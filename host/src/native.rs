// SPDX-License-Identifier: GPL-3.0-only
use crate::{
    asset_store::{AssetInfo, AssetKind, AssetStore, NativeSpan},
    backend::{GpuBackend, SilentSpu, SpuBackend},
    disc::{DiscImage, GameDisc},
    pad::{KeyboardController, LiveInput, PadSource, ReplayPad},
    raster::Raster,
};
use std::{
    cell::RefCell,
    fs::File,
    num::NonZeroU32,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    keyboard::PhysicalKey,
    window::{Window, WindowId},
};

unsafe extern "C" {
    fn port_run_game() -> i32;
}

struct Host {
    disc: GameDisc<DiscImage<File>>,
    gpu: Box<dyn GpuBackend>,
    spu: Box<dyn SpuBackend>,
    pad: Box<dyn PadSource>,
    assets: AssetStore,
    proxy: EventLoopProxy<Frame>,
    frames: u64,
    limit: u64,
    screenshot: Option<PathBuf>,
    start: Instant,
    next: Instant,
    cancel: Arc<AtomicBool>,
    error: Option<String>,
    first_logo: Option<u64>,
    last_frame: Option<(u32, u32, Vec<u32>)>,
}
thread_local! { static HOST: RefCell<Option<Host>> = const {RefCell::new(None)}; }

fn host<R>(f: impl FnOnce(&mut Host) -> R) -> R {
    HOST.with(|cell| {
        f(cell
            .borrow_mut()
            .as_mut()
            .expect("native worker host installed"))
    })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn port_read_file(id: u32, bytes: u32, destination: *mut u8) -> i32 {
    host(|h| {
        // PORT: Refuse raw pointer-bearing images at the legacy byte-copy API.
        // Native consumers must use AssetStore rather than widening disk slots.
        if h.disc
            .entry(id)
            .is_ok_and(|entry| AssetKind::try_from(entry.file_type).is_ok())
        {
            h.error = Some(format!(
                "asset {id} requires port_asset_open/native views; raw relocation is forbidden"
            ));
            return 1;
        }
        match h.disc.read_entry_range(id, 0, u64::from(bytes)) {
            Ok(data) => {
                // SAFETY: The C job validates its destination capacity before calling this function.
                unsafe {
                    std::ptr::copy_nonoverlapping(data.as_ptr(), destination, data.len());
                }
                0
            }
            Err(e) => {
                h.error = Some(format!("disc read file {id}: {e}"));
                1
            }
        }
    })
}
#[unsafe(no_mangle)]
extern "C" fn port_asset_open(id: u32) -> u32 {
    host(|h| match h.assets.open(&mut h.disc, id) {
        Ok(handle) => handle,
        Err(error) => {
            h.error = Some(error);
            0
        }
    })
}
#[unsafe(no_mangle)]
unsafe extern "C" fn port_asset_info(handle: u32, output: *mut AssetInfo) -> i32 {
    if output.is_null() {
        return 1;
    }
    host(|h| match h.assets.info(handle) {
        Ok(info) => {
            // SAFETY: C caller supplies one aligned writable native info record.
            unsafe {
                output.write(info);
            }
            0
        }
        Err(error) => {
            h.error = Some(error);
            1
        }
    })
}
#[unsafe(no_mangle)]
extern "C" fn port_asset_close(handle: u32) -> i32 {
    host(|h| match h.assets.close(handle) {
        Ok(()) => 0,
        Err(error) => {
            h.error = Some(error);
            1
        }
    })
}
#[unsafe(no_mangle)]
unsafe extern "C" fn port_asset_span(
    handle: u32,
    section: u32,
    model: u32,
    mesh: u32,
    output: *mut NativeSpan,
) -> i32 {
    if output.is_null() {
        return 1;
    }
    host(|h| {
        match h
            .assets
            .span(handle, section, model as usize, mesh as usize)
        {
            Ok((bytes, stride)) => {
                let span = NativeSpan {
                    data: bytes.as_ptr(),
                    count: bytes.len() / stride,
                    stride,
                };
                // SAFETY: C supplies writable native span storage; the backing file
                // stays owned by AssetStore until port_asset_close. No wire writes.
                unsafe {
                    output.write(span);
                }
                0
            }
            Err(error) => {
                h.error = Some(error);
                1
            }
        }
    })
}
#[unsafe(no_mangle)]
unsafe extern "C" fn port_load_vram(x: i32, y: i32, w: i32, h: i32, data: *const u16) {
    if w <= 0 || h <= 0 || w > 1024 || h > 512 {
        return;
    }
    // SAFETY: C validates the TIM block bounds and number of 16-bit pixels before calling.
    let pixels = unsafe { std::slice::from_raw_parts(data, (w * h) as usize) };
    host(|state| state.gpu.load([x, y, w, h], pixels));
}
#[unsafe(no_mangle)]
extern "C" fn port_clear_vram(x: i32, y: i32, w: i32, h: i32, r: u8, g: u8, b: u8) {
    host(|state| state.gpu.clear([x, y, w, h], [r, g, b]));
}
#[unsafe(no_mangle)]
unsafe extern "C" fn port_draw_packet(words: *const u32, count: u32) {
    if !(2..=256).contains(&count) {
        return;
    }
    // SAFETY: C supplies a packet allocated by the real boot code, with its length tag.
    let packet = unsafe { std::slice::from_raw_parts(words, count as usize) };
    host(|state| state.gpu.packet(packet));
}
#[unsafe(no_mangle)]
extern "C" fn port_draw_env(x: i32, y: i32, w: i32, h: i32, ox: i32, oy: i32) {
    host(|state| state.gpu.env([x, y, w, h], [ox, oy]));
}
#[unsafe(no_mangle)]
extern "C" fn port_begin_ot() {
    host(|h| h.gpu.begin_ordering_table());
}
#[unsafe(no_mangle)]
extern "C" fn port_end_ot() {
    host(|h| h.gpu.end_ordering_table());
}
#[unsafe(no_mangle)]
unsafe extern "C" fn port_store_vram(
    x: i32,
    y: i32,
    w: i32,
    height: i32,
    destination: *mut u16,
) -> i32 {
    if w <= 0 || height <= 0 || w > 1024 || height > 512 || destination.is_null() {
        return 1;
    }
    let pixels = host(|h| h.gpu.read([x, y, w, height]));
    if pixels.len() != (w * height) as usize {
        return 1;
    }
    // SAFETY: StoreImage's caller supplies space for the requested VRAM rectangle.
    unsafe {
        std::ptr::copy_nonoverlapping(pixels.as_ptr(), destination, pixels.len());
    }
    0
}
#[unsafe(no_mangle)]
extern "C" fn port_move_vram(x: i32, y: i32, w: i32, height: i32, dx: i32, dy: i32) -> i32 {
    if w <= 0 || height <= 0 || w > 1024 || height > 512 {
        return 1;
    }
    host(|h| {
        let pixels = h.gpu.read([x, y, w, height]);
        if pixels.len() != (w * height) as usize {
            return 1;
        }
        h.gpu.load([dx, dy, w, height], &pixels);
        0
    })
}
#[unsafe(no_mangle)]
unsafe extern "C" fn port_pad_read(destination: *mut u8) {
    let packet = host(|h| h.pad.sample(h.frames));
    // SAFETY: libpad shim supplies an eight-byte writable buffer.
    unsafe {
        std::ptr::copy_nonoverlapping(packet.as_ptr(), destination, packet.len());
    }
}
#[unsafe(no_mangle)]
extern "C" fn port_spu_reset() {
    host(|h| h.spu.reset());
}
#[unsafe(no_mangle)]
extern "C" fn port_spu_write(offset: u16, value: u16) -> i32 {
    host(|h| match h.spu.write_register(offset, value) {
        Ok(()) => 0,
        Err(error) => {
            h.error = Some(error);
            1
        }
    })
}
#[unsafe(no_mangle)]
extern "C" fn port_spu_read(offset: u16) -> u16 {
    host(|h| match h.spu.read_register(offset) {
        Ok(value) => value,
        Err(error) => {
            h.error = Some(error);
            0
        }
    })
}
#[unsafe(no_mangle)]
unsafe extern "C" fn port_spu_transfer(address: u32, data: *const u8, count: u32) -> i32 {
    if count > 512 * 1024 || data.is_null() {
        return 1;
    }
    // SAFETY: C transfer shim must provide count readable bytes for this synchronous call.
    let bytes = unsafe { std::slice::from_raw_parts(data, count as usize) };
    host(|h| match h.spu.transfer_write(address, bytes) {
        Ok(()) => 0,
        Err(error) => {
            h.error = Some(error);
            1
        }
    })
}
#[unsafe(no_mangle)]
extern "C" fn port_present(x: i32, y: i32, w: i32, h: i32, state: i32, step: i32) -> i32 {
    host(|host| {
        host.frames += 1;
        let display_enabled = w > 0 && h > 0;
        let w = if w > 0 { w as u32 } else { 320 };
        let h = if h > 0 { h as u32 } else { 240 };
        let pixels = if display_enabled {
            host.gpu.frame(x, y, w, h)
        } else {
            vec![0; (w * h) as usize]
        };
        if state == 1 && host.first_logo.is_none() {
            host.first_logo = Some(host.frames);
            println!("Entered B_KONAMI at frame {}", host.frames);
        }
        host.last_frame = Some((w, h, pixels.clone()));
        let finished = host.frames >= host.limit || host.cancel.load(Ordering::Relaxed);
        if finished {
            if let Some(path) = &host.screenshot
                && let Err(e) = save_png(path, w, h, &pixels)
            {
                host.error = Some(e.to_string());
            }
            println!(
                "RUN frames={} seconds={:.3} state={} step={} first_logo={:?} primitives={}",
                host.frames,
                host.start.elapsed().as_secs_f64(),
                state,
                step,
                host.first_logo,
                host.gpu.primitives()
            );
        }
        if host
            .proxy
            .send_event(Frame {
                w,
                h,
                pixels,
                finished,
            })
            .is_err()
        {
            return 1;
        }
        host.next += Duration::from_secs_f64(1.0 / 60.0);
        if let Some(wait) = host.next.checked_duration_since(Instant::now()) {
            std::thread::sleep(wait);
        }
        i32::from(finished)
    })
}

fn save_png(path: &Path, w: u32, h: u32, pixels: &[u32]) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::create(path)?;
    let mut encoder = png::Encoder::new(file, w, h);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    let bytes: Vec<u8> = pixels
        .iter()
        .flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8])
        .collect();
    writer.write_image_data(&bytes)?;
    Ok(())
}

struct Frame {
    w: u32,
    h: u32,
    pixels: Vec<u32>,
    finished: bool,
}
struct App {
    input: Arc<LiveInput>,
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    frame: Option<Frame>,
    cancel: Arc<AtomicBool>,
    failure: Option<String>,
    presented: u64,
    final_presented: bool,
}
impl ApplicationHandler<Frame> for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        match el.create_window(
            Window::default_attributes()
                .with_title("Silent Hill native C — boot spike")
                .with_inner_size(PhysicalSize::new(640, 448)),
        ) {
            Ok(window) => {
                let window = Rc::new(window);
                let result = softbuffer::Context::new(window.clone())
                    .and_then(|context| softbuffer::Surface::new(&context, window.clone()));
                match result {
                    Ok(surface) => {
                        self.window = Some(window);
                        self.surface = Some(surface);
                    }
                    Err(e) => {
                        self.failure = Some(e.to_string());
                        self.cancel.store(true, Ordering::Relaxed);
                        el.exit();
                    }
                }
            }
            Err(e) => {
                self.failure = Some(e.to_string());
                self.cancel.store(true, Ordering::Relaxed);
                el.exit();
            }
        }
    }
    fn user_event(&mut self, el: &ActiveEventLoop, frame: Frame) {
        self.frame = Some(frame);
        if let Some(window) = &self.window {
            window.request_redraw();
        } else {
            self.cancel.store(true, Ordering::Relaxed);
            el.exit();
        }
    }
    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    self.input.key(code, event.state.is_pressed());
                }
            }
            WindowEvent::Focused(focused) => self.input.focus(focused),
            WindowEvent::CloseRequested => {
                self.cancel.store(true, Ordering::Relaxed);
                el.exit();
            }
            WindowEvent::RedrawRequested => {
                let (Some(frame), Some(surface), Some(window)) =
                    (&self.frame, &mut self.surface, &self.window)
                else {
                    return;
                };
                let size = window.inner_size();
                let (Some(w), Some(h)) =
                    (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                else {
                    return;
                };
                let result = (|| {
                    surface.resize(w, h)?;
                    let mut buffer = surface.buffer_mut()?;
                    for y in 0..size.height {
                        for x in 0..size.width {
                            buffer[(y * size.width + x) as usize] =
                                frame.pixels[((y * frame.h / size.height) * frame.w
                                    + x * frame.w / size.width)
                                    as usize];
                        }
                    }
                    buffer.present()
                })();
                if let Err(e) = result {
                    self.failure = Some(e.to_string());
                    self.cancel.store(true, Ordering::Relaxed);
                    el.exit();
                } else {
                    self.presented += 1;
                    if frame.finished {
                        self.final_presented = true;
                    }
                }
                if frame.finished {
                    el.exit();
                }
            }
            _ => {}
        }
    }
}

pub struct Backends {
    pub gpu: Box<dyn GpuBackend>,
    pub spu: Box<dyn SpuBackend>,
    pub pad: Box<dyn PadSource>,
}
pub fn run(
    disc: GameDisc<DiscImage<File>>,
    limit: u64,
    screenshot: Option<PathBuf>,
    replay: Option<ReplayPad>,
) -> Result<(), String> {
    let input = Arc::new(LiveInput::default());
    let pad: Box<dyn PadSource> = match replay {
        Some(replay) => Box::new(replay),
        None => Box::new(KeyboardController::new(input.clone())),
    };
    run_with_backends(
        disc,
        limit,
        screenshot,
        input,
        Backends {
            gpu: Box::<Raster>::default(),
            spu: Box::<SilentSpu>::default(),
            pad,
        },
    )
}
pub fn run_with_backends(
    disc: GameDisc<DiscImage<File>>,
    limit: u64,
    screenshot: Option<PathBuf>,
    input: Arc<LiveInput>,
    backends: Backends,
) -> Result<(), String> {
    let event_loop = EventLoop::<Frame>::with_user_event()
        .build()
        .map_err(|e| e.to_string())?;
    let proxy = event_loop.create_proxy();
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = cancel.clone();
    let worker = std::thread::spawn(move || {
        let now = Instant::now();
        HOST.with(|cell| {
            *cell.borrow_mut() = Some(Host {
                disc,
                gpu: backends.gpu,
                spu: backends.spu,
                pad: backends.pad,
                assets: AssetStore::default(),
                proxy,
                frames: 0,
                limit,
                screenshot,
                start: now,
                next: now,
                cancel: worker_cancel,
                error: None,
                first_logo: None,
                last_frame: None,
            })
        });
        // SAFETY: This is the sole game worker. C's exit jump only crosses C frames after Rust callbacks return.
        let code = unsafe { port_run_game() };
        host(|h| {
            if code != 0 {
                if let (Some(path), Some((w, height, pixels))) = (&h.screenshot, &h.last_frame)
                    && let Err(error) = save_png(path, *w, *height, pixels)
                {
                    h.error = Some(error.to_string());
                }
                let (w, height, pixels) =
                    h.last_frame
                        .take()
                        .unwrap_or((320, 240, vec![0; 320 * 240]));
                println!(
                    "STOPPED code={code} frames={} primitives={}",
                    h.frames,
                    h.gpu.primitives()
                );
                let _ = h.proxy.send_event(Frame {
                    w,
                    h: height,
                    pixels,
                    finished: true,
                });
            }
            if let Some(e) = h.error.take() {
                Err(e)
            } else if code != 0 {
                Err(format!(
                    "native boot stopped: code {code}, frames {}",
                    h.frames
                ))
            } else if h.frames < h.limit {
                Err(format!(
                    "window closed after {} of {} frames",
                    h.frames, h.limit
                ))
            } else {
                Ok(())
            }
        })
    });
    let mut app = App {
        input,
        window: None,
        surface: None,
        frame: None,
        cancel: cancel.clone(),
        failure: None,
        presented: 0,
        final_presented: false,
    };
    let loop_result = event_loop.run_app(&mut app).map_err(|e| e.to_string());
    cancel.store(true, Ordering::Relaxed);
    let worker_result = worker
        .join()
        .map_err(|_| "native worker panicked".to_owned())?;
    loop_result?;
    if let Some(failure) = app.failure {
        return Err(failure);
    }
    println!(
        "WINDOW presents={} final_presented={}",
        app.presented, app.final_presented
    );
    worker_result
}
