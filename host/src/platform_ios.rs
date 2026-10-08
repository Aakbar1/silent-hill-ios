// SPDX-License-Identifier: GPL-3.0-only
//! UIKit owns the sole application loop. The existing native C worker owns game
//! state; the UIKit presenter consumes the shared scaled Metal renderer.
#[path = "../../ios/audio.rs"]
mod audio;
#[path = "../../ios/importer.rs"]
mod importer;

use crate::{
    backend::{GpuBackend, SilentSpu, SpuBackend},
    disc::GameDisc,
    ios_renderer::{PhoneGpu, requested_scale, synthetic_scene},
    native::Backends,
    pad::LiveInput,
    pad_touch::PlatformTouch,
    saves::SaveStore,
};
use sh_touch::{Insets, Phase, Viewport};
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
    time::{Duration, Instant, SystemTime},
};

unsafe extern "C" {
    fn sh_ios_application_main();
    fn sh_ios_status(text: *const c_char, progress: f64, busy: bool);
    fn sh_ios_frame(rgba: *const u8, width: u32, height: u32);
    fn sh_ios_game_started();
    fn sh_ios_worker_paused();
    fn sh_ios_refresh_view();
    fn sh_ios_synthetic_touch(id: u64, phase: u32, x: f32, y: f32);
    fn sh_ios_synthetic_audio_notifications();
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
    interrupted: AtomicBool,
    recover_audio: AtomicBool,
    audio_recoveries: AtomicU64,
    worker_paused: AtomicBool,
    clock_reset: AtomicBool,
    ui_ready: AtomicBool,
    audio: Mutex<Option<Arc<Mutex<crate::spu_cpal::SpuCpal>>>>,
    failure: Mutex<Option<String>>,
    touch: Mutex<Option<PlatformTouch>>,
    view: Mutex<(Viewport, f32)>,
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
        interrupted: AtomicBool::new(false),
        recover_audio: AtomicBool::new(false),
        audio_recoveries: AtomicU64::new(0),
        worker_paused: AtomicBool::new(false),
        clock_reset: AtomicBool::new(false),
        ui_ready: AtomicBool::new(false),
        audio: Mutex::new(None),
        failure: Mutex::new(None),
        touch: Mutex::new(None),
        view: Mutex::new((Viewport::default(), 1.0)),
    });
    let state = STATE.get().expect("initialized once");
    // SAFETY: initialize is on main after its window/controller was installed.
    unsafe { sh_ios_refresh_view() };
    if std::env::var("SH_IOS_SMOKE").as_deref() == Ok("1") {
        state.game_started.store(true, Ordering::Release);
        unsafe { sh_ios_game_started() };
        std::thread::spawn(|| {
            if let Err(error) = synthetic_smoke() {
                eprintln!("IOS_SMOKE FAIL {error}");
                status(&format!("Synthetic smoke failed: {error}"), -1.0, false);
            }
        });
        return;
    }
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
        if !active {
            if let Some(touch) = &*state.touch.lock().expect("touch handle") {
                touch.cancel();
            }
        }
        println!("IOS_LIFECYCLE active={active}");
    }
}

#[unsafe(no_mangle)]
extern "C" fn sh_ios_interrupted(interrupted: bool) {
    if let Some(state) = STATE.get() {
        if state.interrupted.swap(interrupted, Ordering::AcqRel) != interrupted {
            state.recover_audio.store(true, Ordering::Release);
            if let Some(touch) = &*state.touch.lock().expect("touch handle") {
                touch.cancel();
            }
        }
        println!("IOS_LIFECYCLE interrupted={interrupted}");
    }
}
#[unsafe(no_mangle)]
extern "C" fn sh_ios_audio_route_changed() {
    if let Some(state) = STATE.get() {
        state.recover_audio.store(true, Ordering::Release);
        if let Some(touch) = &*state.touch.lock().expect("touch handle") {
            touch.cancel();
        }
        println!("IOS_AUDIO route/reset recovery requested");
    }
}

#[unsafe(no_mangle)]
extern "C" fn sh_ios_view(
    width: f32,
    height: f32,
    top: f32,
    right: f32,
    bottom: f32,
    left: f32,
    ratio: f32,
) {
    if let Some(state) = STATE.get() {
        let view = Viewport {
            width,
            height,
            safe: Insets {
                top,
                right,
                bottom,
                left,
            },
        };
        *state.view.lock().expect("UIKit geometry") = (view, ratio);
        println!(
            "IOS_VIEW points={width:.1}x{height:.1} safe={top:.1},{right:.1},{bottom:.1},{left:.1} pixel_ratio={ratio:.1} requested_scale={}",
            requested_scale(view.content().width, view.content().height, ratio)
        );
        if let Some(touch) = &*state.touch.lock().expect("touch handle") {
            touch.viewport(view);
        }
    }
}
#[unsafe(no_mangle)]
extern "C" fn sh_ios_touch(id: u64, phase: u32, x: f32, y: f32) {
    let phase = match phase {
        0 => Phase::Down,
        1 => Phase::Move,
        2 => Phase::Up,
        3 => Phase::Cancel,
        _ => return,
    };
    if let Some(state) = STATE.get()
        && is_running(state)
        && let Some(touch) = &*state.touch.lock().expect("touch handle")
    {
        touch.touch(id, phase, x, y);
    }
}

fn is_running(state: &State) -> bool {
    state.active.load(Ordering::Acquire) && !state.interrupted.load(Ordering::Acquire)
}

#[unsafe(no_mangle)]
extern "C" fn sh_ios_ready() {
    if let Some(state) = STATE.get() {
        state.ui_ready.store(true, Ordering::Release);
    }
}
fn wait_for_ui() {
    let state = STATE.get().expect("UI state");
    while !state.ui_ready.load(Ordering::Acquire) {
        std::thread::sleep(Duration::from_millis(5));
    }
}
pub(crate) fn take_clock_reset() -> bool {
    STATE
        .get()
        .expect("clock state")
        .clock_reset
        .swap(false, Ordering::AcqRel)
}

pub(crate) use audio::open_audio;

fn start_game(disc: GameDisc<crate::disc::DiscImage<fs::File>>) {
    let state = STATE.get().expect("initialized storage");
    if state.game_started.swap(true, Ordering::AcqRel) {
        return;
    }
    println!("Verified imported US v1.1 disc; starting shared native C host");
    // SAFETY: The function schedules UI work on the main queue.
    unsafe { sh_ios_game_started() };
    std::thread::spawn(move || {
        wait_for_ui();
        wait_for_active();
        let (view, ratio) = *state.view.lock().expect("UIKit geometry");
        let (touch, pad) = match PlatformTouch::new(view) {
            Ok(pair) => pair,
            Err(error) => {
                status(&error, -1.0, false);
                return;
            }
        };
        *state.touch.lock().expect("touch handle") = Some(touch.clone());
        let gpu = match PhoneGpu::new(
            requested_scale(view.content().width, view.content().height, ratio),
            false,
        ) {
            Ok(gpu) => gpu,
            Err(error) => {
                status(&error, -1.0, false);
                return;
            }
        };
        let backends = Backends {
            gpu: Box::new(IosGpu {
                inner: gpu,
                touch,
                first: Cell::new(true),
            }),
            spu: Box::<SilentSpu>::default(),
            // The Apple worker factory replaces this placeholder with the
            // lifecycle-aware SpuCpal before any C/game callbacks execute.
            pad: Box::new(pad),
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
    inner: PhoneGpu,
    touch: PlatformTouch,
    first: Cell<bool>,
}
impl IosGpu {
    fn present(&self, pixels: Vec<u32>, width: u32, height: u32) -> Vec<u32> {
        let (display_w, display_h, mut display) = match self.inner.display(width, height, &pixels) {
            Ok(frame) => frame,
            Err(error) => {
                eprintln!("IOS_GPU display failure: {error}");
                return pixels;
            }
        };
        self.touch.draw(&mut display, display_w, display_h);
        let rgba: Vec<u8> = display
            .iter()
            .flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8, 255])
            .collect();
        // SAFETY: UIKit immediately copies exactly width*height*4 live bytes.
        unsafe { sh_ios_frame(rgba.as_ptr(), display_w, display_h) };
        if self.first.replace(false) {
            println!("first game frame presented to UIKit");
        }
        pixels
    }
}

pub(crate) fn wait_for_active() -> bool {
    let state = STATE.get().expect("initialized host");
    let mut paused = false;
    while !is_running(state) {
        paused = true;
        if !state.worker_paused.swap(true, Ordering::AcqRel) {
            state.clock_reset.store(true, Ordering::Release);
            if let Some(audio) = &*state.audio.lock().expect("audio handle") {
                if let Err(error) = audio.lock().expect("audio worker").ios_suspend() {
                    *state.failure.lock().expect("lifecycle failure") = Some(error);
                }
                state.recover_audio.store(true, Ordering::Release);
            }
            println!("IOS_LIFECYCLE worker paused; synchronous save callbacks completed");
            unsafe { sh_ios_worker_paused() };
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    if state.recover_audio.swap(false, Ordering::AcqRel)
        && let Some(audio) = &*state.audio.lock().expect("audio handle")
    {
        state.clock_reset.store(true, Ordering::Release);
        let mut audio = audio.lock().expect("audio worker");
        if let Err(error) = audio.ios_suspend().and_then(|()| audio.ios_recover()) {
            eprintln!("IOS_AUDIO recovery failed: {error}");
            *state.failure.lock().expect("lifecycle failure") = Some(error);
        } else {
            state.audio_recoveries.fetch_add(1, Ordering::Release);
            println!("IOS_AUDIO recovered; SPU state and sample clock preserved");
        }
    }
    if state.worker_paused.swap(false, Ordering::AcqRel) {
        println!("IOS_LIFECYCLE worker resumed");
    }
    paused
}
impl GpuBackend for IosGpu {
    fn begin_ordering_table(&mut self) {
        wait_for_active();
        self.inner.begin_ordering_table();
    }
    fn end_ordering_table(&mut self) {
        wait_for_active();
        self.inner.end_ordering_table();
    }
    fn packet_precise(&mut self, words: &[u32], positions: &[[f32; 2]]) {
        wait_for_active();
        self.inner.packet_precise(words, positions);
    }
    fn packet(&mut self, words: &[u32]) {
        wait_for_active();
        self.inner.packet(words);
    }
    fn env(&mut self, clip: [i32; 4], offset: [i32; 2]) {
        wait_for_active();
        self.inner.env(clip, offset);
    }
    fn load(&mut self, [x, y, w, h]: [i32; 4], pixels: &[u16]) {
        wait_for_active();
        self.inner.load([x, y, w, h], pixels);
    }
    fn read(&self, rect: [i32; 4]) -> Vec<u16> {
        wait_for_active();
        self.inner.read(rect)
    }
    fn clear(&mut self, rect: [i32; 4], color: [u8; 3]) {
        wait_for_active();
        self.inner.clear(rect, color);
    }
    fn frame(&self, x: i32, y: i32, w: u32, h: u32) -> Vec<u32> {
        wait_for_active();
        self.present(self.inner.frame(x, y, w, h), w, h)
    }
    fn frame_rgb24(&self, x: i32, y: i32, w: u32, h: u32) -> Vec<u32> {
        wait_for_active();
        self.present(self.inner.frame_rgb24(x, y, w, h), w, h)
    }
    fn primitives(&self) -> u64 {
        self.inner.primitives()
    }
}

fn synthetic_smoke() -> Result<(), String> {
    wait_for_ui();
    let state = STATE.get().expect("smoke storage");
    println!("IOS_SMOKE synthetic-only; disc importer and native C worker bypassed");
    let (view, ratio) = *state.view.lock().expect("UIKit geometry");
    let (touch, mut pad) = PlatformTouch::new(view)?;
    *state.touch.lock().expect("touch handle") = Some(touch.clone());
    touch.smoke_check_with(&mut pad, |id, phase, x, y| {
        let phase = match phase {
            Phase::Down => 0,
            Phase::Move => 1,
            Phase::Up => 2,
            Phase::Cancel => 3,
        };
        // SAFETY: The Objective-C data-free probe calls the SAME Rust entry as
        // SHGameView's touch delivery. No UIKit objects or C state are touched.
        unsafe { sh_ios_synthetic_touch(id, phase, x, y) };
    })?;
    let mut gpu = IosGpu {
        inner: PhoneGpu::new(
            requested_scale(view.content().width, view.content().height, ratio),
            true,
        )?,
        touch,
        first: Cell::new(true),
    };
    let mut audio = open_audio()?;
    // Stable synthetic SPU RAM/register sentinels prove device rebuilds do not
    // reset hardware state. These are original fixture bytes, not game assets.
    audio.transfer_write(0x7fff0, &[1, 2, 3, 4])?;
    audio.write_register(4, 0x1234)?;
    let saves = SaveStore::new(state.support.join("synthetic-save-check"));
    let mut payload = [0u8; 636];
    saves.write(0, &payload)?;
    payload[0] = 7;
    saves.write(0, &payload)?;
    if saves.write(0, &payload[..635]).is_ok() || saves.read(0)? != Some(payload) {
        return Err("synthetic atomic save replacement/preservation failed".into());
    }
    fs::remove_file(saves.root().join("slot-000.shs")).map_err(|e| e.to_string())?;
    fs::remove_dir(saves.root()).map_err(|e| e.to_string())?;
    let start = Instant::now();
    for frame in 1..=90 {
        audio.advance_to(frame * 735)?;
        synthetic_scene(&mut gpu);
        let pixels = gpu.frame(0, 0, 320, 224);
        if let Some(error) = crate::gpu_wgpu::frame_error() {
            return Err(error);
        }
        if !gpu.inner.accelerated() || pixels.iter().all(|p| *p == 0) {
            return Err("synthetic Metal path was blank or fell back".into());
        }
        std::thread::sleep(Duration::from_millis(16));
    }
    let callbacks = state
        .audio
        .lock()
        .expect("audio handle")
        .as_ref()
        .expect("smoke audio")
        .lock()
        .expect("smoke audio worker")
        .ios_callbacks();
    if callbacks == 0 {
        return Err("real audio device delivered no callbacks".into());
    }
    println!(
        "IOS_SMOKE PASS rendered=90 audio_frames=66150 audio_callbacks={callbacks} touch=PASS saves=PASS seconds={:.3}",
        start.elapsed().as_secs_f64()
    );
    // SAFETY: Schedules clearly labelled synthetic OS-notification fixtures on
    // main, exercising the production observers without game data or a call.
    unsafe { sh_ios_synthetic_audio_notifications() };
    // Keep servicing lifecycle after the scene is drawn, so CI can background
    // and foreground the real app without an imported disc.
    let mut sample = 66150;
    let mut observed_recovery = 0;
    loop {
        wait_for_active();
        sample += 735;
        audio.advance_to(sample)?;
        synthetic_scene(&mut gpu);
        gpu.frame(0, 0, 320, 224);
        if let Some(error) = crate::gpu_wgpu::frame_error() {
            return Err(error);
        }
        let generation = state.audio_recoveries.load(Ordering::Acquire);
        let callbacks = state
            .audio
            .lock()
            .expect("audio handle")
            .as_ref()
            .expect("smoke audio")
            .lock()
            .expect("smoke audio worker")
            .ios_callbacks();
        if generation > observed_recovery && callbacks > 0 {
            let mut sentinel = [0; 4];
            audio.transfer_read(0x7fff0, &mut sentinel)?;
            if sentinel != [1, 2, 3, 4] || audio.read_register(4)? != 0x1234 {
                return Err("SPU RAM/register state changed during device recovery".into());
            }
            println!(
                "IOS_SMOKE audio recovery progress generation={generation} sample={sample} callbacks={callbacks} spu_state=PASS"
            );
            observed_recovery = generation;
        }
        std::thread::sleep(Duration::from_millis(16));
    }
}
