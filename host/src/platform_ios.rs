// SPDX-License-Identifier: GPL-3.0-only
//! UIKit owns the sole application loop. The existing native C worker owns game
//! state; the UIKit presenter consumes the same raster output as the desktop.
#[path = "../../ios/importer.rs"]
mod importer;

use crate::{
    backend::{GpuBackend, SilentSpu},
    disc::GameDisc,
    native::Backends,
    pad::{KeyboardController, LiveInput},
    raster::Raster,
    saves::SaveStore,
};
use std::{
    cell::Cell,
    collections::HashMap,
    ffi::{CStr, CString, c_char},
    fs,
    path::PathBuf,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, SystemTime},
};

unsafe extern "C" {
    fn sh_ios_application_main();
    fn sh_ios_status(text: *const c_char, progress: f64, busy: bool);
    fn sh_ios_frame(rgba: *const u8, width: u32, height: u32);
    fn sh_ios_game_started();
}

struct State {
    documents: PathBuf,
    support: PathBuf,
    importing: AtomicBool,
    source_bytes: AtomicU64,
    game_started: AtomicBool,
    active: AtomicBool,
    cancel: Arc<AtomicBool>,
    files: Mutex<HashMap<PathBuf, WatchedFile>>,
    input: Arc<LiveInput>,
}
#[derive(Clone, Copy)]
struct WatchedFile {
    bytes: u64,
    modified: Option<SystemTime>,
    polls: u8,
}
static STATE: OnceLock<State> = OnceLock::new();

fn status(message: &str, progress: f64, busy: bool) {
    let text = CString::new(message.replace('\0', " ")).expect("sanitized status");
    // SAFETY: ObjC copies this string before returning and updates UIKit on main.
    unsafe { sh_ios_status(text.as_ptr(), progress, busy) };
}

pub fn run() {
    std::panic::set_hook(Box::new(|info| eprintln!("panic: {info}")));
    // SAFETY: main thread, exactly once; UIApplicationMain owns the lifetime.
    unsafe { sh_ios_application_main() };
}

// SAFETY: UIKit supplies live NUL-terminated filesystem paths for this call.
unsafe fn path(pointer: *const c_char) -> PathBuf {
    PathBuf::from(
        unsafe { CStr::from_ptr(pointer) }
            .to_string_lossy()
            .as_ref(),
    )
}

#[unsafe(no_mangle)]
unsafe extern "C" fn sh_ios_initialize(documents: *const c_char, support: *const c_char) {
    // SAFETY: Both pointers follow the documented path contract above.
    let documents = unsafe { path(documents) };
    let support = unsafe { path(support) };
    if let Err(error) =
        fs::create_dir_all(documents.join("saves")).and_then(|()| fs::create_dir_all(&support))
    {
        status(
            &format!("Could not create app storage: {error}"),
            -1.0,
            false,
        );
        return;
    }
    let _ = STATE.set(State {
        documents,
        support,
        importing: AtomicBool::new(false),
        source_bytes: AtomicU64::new(0),
        game_started: AtomicBool::new(false),
        active: AtomicBool::new(true),
        cancel: Arc::new(AtomicBool::new(false)),
        files: Mutex::new(HashMap::new()),
        input: Arc::new(LiveInput::default()),
    });
    let state = STATE.get().expect("initialized once");
    let saved = importer::imported(&state.support);
    if saved.exists() {
        status("Checking your imported disc…", -1.0, true);
        std::thread::spawn(move || match GameDisc::open(&saved) {
            Ok(disc) => start_game(disc),
            Err(_) => {
                // Preserve damaged app-owned bytes where the player can inspect
                // them; never rename/delete the player's selected source.
                let rejected = state.documents.join(format!(
                    "RejectedImport-{}.invalid",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                ));
                if fs::rename(&saved, rejected).is_err() {
                    status(
                        "The saved import is damaged and could not be moved. Check storage and restart.",
                        -1.0,
                        true,
                    );
                } else {
                    status(
                        "The saved copy is damaged. Choose your original .bin to import again.",
                        -1.0,
                        false,
                    );
                }
            }
        });
    }
}

#[unsafe(no_mangle)]
extern "C" fn sh_ios_wants_import() -> bool {
    STATE.get().is_some_and(|s| {
        !s.game_started.load(Ordering::Relaxed)
            && !s.importing.load(Ordering::Relaxed)
            && !importer::imported(&s.support).exists()
    })
}

/// Called synchronously within NSFileCoordinator's background read accessor.
/// Security-scoped access remains held until the verified copy is complete.
#[unsafe(no_mangle)]
unsafe extern "C" fn sh_ios_import(source: *const c_char) {
    let Some(state) = STATE.get() else { return };
    if state.game_started.load(Ordering::Relaxed) || state.importing.swap(true, Ordering::AcqRel) {
        return;
    }
    // SAFETY: NSFileCoordinator provides a live local filesystem path.
    let source = unsafe { path(source) };
    state.source_bytes.store(
        fs::metadata(&source).map_or(0, |m| m.len()),
        Ordering::Relaxed,
    );
    status("Verifying US v1.1 and copying your disc…", -1.0, true);
    println!("Disc import started; source opened read-only");
    let result = importer::import_disc(&source, &state.support);
    match result {
        Ok(target) => match GameDisc::open(target) {
            Ok(disc) => start_game(disc),
            Err(_) => status(
                "The imported copy could not be reopened. Restart and try again.",
                -1.0,
                false,
            ),
        },
        Err(error) => {
            println!("Disc import rejected or failed");
            status(&error, -1.0, false);
        }
    }
    state.importing.store(false, Ordering::Release);
}

/// UIKit's timer reports actual staged bytes; verification remains indeterminate.
#[unsafe(no_mangle)]
extern "C" fn sh_ios_poll() {
    let Some(state) = STATE.get() else { return };
    if state.importing.load(Ordering::Acquire) {
        if let Ok(meta) = fs::metadata(importer::staging(&state.support).join("disc.bin")) {
            let total = state.source_bytes.load(Ordering::Relaxed);
            let progress = if total == 0 {
                -1.0
            } else {
                (meta.len() as f64 / total as f64).min(1.0)
            };
            status(
                &format!(
                    "Copied {:.1} MiB — verifying the copy before starting…",
                    meta.len() as f64 / 1048576.0
                ),
                progress,
                true,
            );
        }
        return;
    }
    if !sh_ios_wants_import() {
        return;
    }
    let mut files = state.files.lock().expect("Files polling lock");
    let candidates = importer::candidates(&state.documents);
    files.retain(|p, _| candidates.contains(p));
    for candidate in candidates {
        let Ok(meta) = fs::metadata(&candidate) else {
            continue;
        };
        let value = WatchedFile {
            bytes: meta.len(),
            modified: meta.modified().ok(),
            polls: 0,
        };
        let entry = files.entry(candidate.clone()).or_insert(value);
        if entry.bytes != value.bytes || entry.modified != value.modified {
            *entry = value;
        }
        // Three unchanged 1-second polls avoid starting during most Files copies.
        // psxdisc rejects incomplete images; a later size/mtime change retries.
        if entry.polls < 3 {
            entry.polls += 1;
        } else {
            continue;
        }
        if entry.polls == 3 && entry.bytes > 0 {
            let text = CString::new(candidate.to_string_lossy().as_bytes())
                .expect("filesystem path has no NUL");
            // Ask ObjC to coordinate app-local Files reads as well as picker URLs.
            unsafe extern "C" {
                fn sh_ios_import_local(path: *const c_char);
            }
            // SAFETY: The Objective-C side copies the path immediately.
            unsafe { sh_ios_import_local(text.as_ptr()) };
            break;
        }
    }
}

#[unsafe(no_mangle)]
extern "C" fn sh_ios_active(active: bool) {
    if let Some(state) = STATE.get() {
        state.active.store(active, Ordering::Relaxed);
        state.input.focus(active);
    }
}

fn start_game(disc: GameDisc<crate::disc::DiscImage<fs::File>>) {
    let state = STATE.get().expect("initialized storage");
    if state.game_started.swap(true, Ordering::AcqRel) {
        return;
    }
    println!("Verified imported US v1.1 disc; starting shared native C host");
    // SAFETY: The function schedules UI work on the main queue.
    unsafe { sh_ios_game_started() };
    std::thread::spawn(move || {
        let backends = Backends {
            gpu: Box::new(IosGpu {
                raster: Raster::default(),
                first: Cell::new(true),
            }),
            spu: Box::<SilentSpu>::default(),
            pad: Box::new(KeyboardController::new(state.input.clone())),
        };
        if let Err(error) = crate::native::run_ios_worker(
            disc,
            SaveStore::new(state.documents.join("saves")),
            backends,
            state.cancel.clone(),
        ) {
            eprintln!("Native host stopped: {error}");
            status(&error, -1.0, false);
        }
    });
}

struct IosGpu {
    raster: Raster,
    first: Cell<bool>,
}
impl IosGpu {
    fn present(&self, pixels: Vec<u32>, width: u32, height: u32) -> Vec<u32> {
        let rgba: Vec<u8> = pixels
            .iter()
            .flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8, 255])
            .collect();
        // SAFETY: UIKit immediately copies exactly width*height*4 live bytes.
        unsafe { sh_ios_frame(rgba.as_ptr(), width, height) };
        if self.first.replace(false) {
            println!("first game frame presented to UIKit");
        }
        pixels
    }
}

pub(crate) fn wait_for_active() -> bool {
    let state = STATE.get().expect("initialized host");
    let mut paused = false;
    while !state.active.load(Ordering::Relaxed) {
        paused = true;
        std::thread::sleep(Duration::from_millis(50));
    }
    paused
}
impl GpuBackend for IosGpu {
    fn packet(&mut self, words: &[u32]) {
        self.raster.packet(words);
    }
    fn env(&mut self, clip: [i32; 4], offset: [i32; 2]) {
        self.raster.env(clip, offset);
    }
    fn load(&mut self, [x, y, w, h]: [i32; 4], pixels: &[u16]) {
        self.raster.load(x, y, w, h, pixels);
    }
    fn read(&self, rect: [i32; 4]) -> Vec<u16> {
        self.raster.read(rect)
    }
    fn clear(&mut self, rect: [i32; 4], color: [u8; 3]) {
        self.raster.clear(rect, color);
    }
    fn frame(&self, x: i32, y: i32, w: u32, h: u32) -> Vec<u32> {
        self.present(self.raster.frame(x, y, w, h), w, h)
    }
    fn frame_rgb24(&self, x: i32, y: i32, w: u32, h: u32) -> Vec<u32> {
        self.present(GpuBackend::frame_rgb24(&self.raster, x, y, w, h), w, h)
    }
    fn primitives(&self) -> u64 {
        self.raster.primitives
    }
}
