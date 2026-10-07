// SPDX-License-Identifier: GPL-3.0-only
use crate::{
    asset_store::{AssetInfo, AssetKind, AssetStore, NativeSpan},
    backend::{GpuBackend, SpuBackend},
    disc::{DiscImage, GameDisc},
    movie::Movie,
    pad::{KeyboardController, LiveInput, PadSource, ReplayPad},
    saves::SaveStore,
};
use std::{
    cell::RefCell,
    fs::File,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    keyboard::PhysicalKey,
    window::{Window, WindowId},
};

unsafe extern "C" {
    fn port_run_game() -> i32;
    fn port_run_world_probe() -> i32;
    fn sh_title_menu_state() -> i32;
    fn sh_option_selected_entry() -> i32;
    static g_ScreenFade_Status: i32;
}

struct Host {
    disc: GameDisc<DiscImage<File>>,
    gpu: Box<dyn GpuBackend>,
    spu: Box<dyn SpuBackend>,
    pad: Box<dyn PadSource>,
    assets: AssetStore,
    saves: SaveStore,
    proxy: Option<EventLoopProxy<Frame>>,
    frames: u64,
    limit: u64,
    screenshot: Option<PathBuf>,
    start: Instant,
    next: Instant,
    cancel: Arc<AtomicBool>,
    error: Option<String>,
    first_logo: Option<u64>,
    last_frame: Option<(u32, u32, Vec<u32>)>,
    movie: Option<Movie>,
    state: i32,
    step: i32,
    movie_frames: u64,
    movie_skips: u32,
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
unsafe extern "C" fn port_water_texture_name(destination: *mut u8) -> i32 {
    if destination.is_null() {
        return 1;
    }
    host(|h| {
        // PORT: D_8002B2CC is eight filename bytes within encrypted BODYPROG,
        // whose pinned USA load address is 0x80024B60. This reads data only.
        const OFFSET: usize = 0x8002_b2cc - 0x8002_4b60;
        let result = (|| {
            let bytes = h
                .disc
                .read_entry_range(3, 0, (OFFSET + 8) as u64)
                .map_err(|error| error.to_string())?;
            let mut seed = 0u32;
            let mut name = [0u8; 8];
            for (index, word) in bytes.as_chunks::<4>().0.iter().enumerate() {
                seed = seed.wrapping_add(0x0130_9125).wrapping_mul(0x03a4_52f7);
                if index * 4 >= OFFSET {
                    let decoded = (u32::from_le_bytes(*word) ^ seed).to_le_bytes();
                    name[index * 4 - OFFSET..index * 4 - OFFSET + 4].copy_from_slice(&decoded);
                }
            }
            if name
                .iter()
                .take_while(|byte| **byte != 0)
                .any(|byte| !(0x20..=0x5f).contains(byte))
            {
                return Err("invalid pinned water texture name".to_owned());
            }
            // SAFETY: C passes its eight-byte filename array.
            unsafe {
                destination.copy_from_nonoverlapping(name.as_ptr(), 8);
            }
            Ok::<(), String>(())
        })();
        match result {
            Ok(()) => 0,
            Err(error) => {
                h.error = Some(error);
                1
            }
        }
    })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn port_map_data_read(
    id: u32,
    offset: u32,
    size: u32,
    destination: *mut u8,
) -> i32 {
    if destination.is_null() {
        return 1;
    }
    host(|h| {
        match h
            .disc
            .read_entry_range(id, u64::from(offset), u64::from(size))
        {
            Ok(bytes) => {
                // SAFETY: The generated map loader passes an array of exactly size
                // bytes. Only numeric room-grid data is read; no code is executed.
                unsafe {
                    destination.copy_from_nonoverlapping(bytes.as_ptr(), bytes.len());
                }
                0
            }
            Err(error) => {
                h.error = Some(error.to_string());
                1
            }
        }
    })
}

// PORT: Read bounded numeric BODYPROG data for native movement/combat records.
// The pinned overlay is encrypted; decode words in the original seed order.
#[unsafe(no_mangle)]
unsafe extern "C" fn port_move_bodyprog_read(offset: u32, size: u32, destination: *mut u8) -> i32 {
    if destination.is_null() || offset % 4 != 0 || size % 4 != 0 || size > 4096 {
        return 1;
    }
    host(|h| {
        let end = u64::from(offset) + u64::from(size);
        match h.disc.read_entry_range(3, 0, end) {
            Ok(bytes) => {
                let mut seed = 0u32;
                for (index, word) in bytes.as_chunks::<4>().0.iter().enumerate() {
                    seed = seed.wrapping_add(0x0130_9125).wrapping_mul(0x03a4_52f7);
                    if index * 4 >= offset as usize {
                        let decoded = (u32::from_le_bytes(*word) ^ seed).to_le_bytes();
                        // SAFETY: C supplies exactly size writable bytes; the read
                        // and aligned word loop cover [offset, offset + size).
                        unsafe {
                            destination.add(index * 4 - offset as usize)
                                .copy_from_nonoverlapping(decoded.as_ptr(), 4);
                        }
                    }
                }
                0
            }
            Err(error) => {
                h.error = Some(error.to_string());
                1
            }
        }
    })
}
#[unsafe(no_mangle)]
unsafe extern "C" fn port_player_map_anim_load(
    id: u32,
    destination: *mut crate::gameplay::NativeAnmHeader,
) -> i32 {
    if destination.is_null() {
        return 1;
    }
    host(|h| {
        let result = (|| {
            let entry = h.disc.entry(id).map_err(|e| e.to_string())?;
            if entry.name != "HB_M0S00.ANM" {
                return Err("map animation fragment identity is not linked".to_owned());
            }
            let base = h
                .disc
                .entries()
                .iter()
                .position(|entry| entry.name == "HB_BASE.ANM")
                .ok_or("player base animation absent")? as u32;
            let handle = h.assets.open(&mut h.disc, base)?;
            let frames = h.disc.read_entry(id).map_err(|e| e.to_string())?;
            let native = h.assets.patch_player_map_animation(handle, &frames)?;
            // SAFETY: C passes the player header's native descriptor storage.
            // The Store retains the patched frame arena for this worker.
            unsafe {
                destination.copy_from_nonoverlapping(native, 1);
            }
            Ok::<(), String>(())
        })();
        match result {
            Ok(()) => 0,
            Err(error) => {
                h.error = Some(error);
                1
            }
        }
    })
}

#[unsafe(no_mangle)]
unsafe extern "C" fn port_asset_load_ipd(
    id: u32,
    destination: *mut crate::maps::NativeMapHeader,
    global_file: i32,
) -> i32 {
    if destination.is_null() {
        return 1;
    }
    host(|h| {
        let result = (|| {
            let handle = h.assets.open(&mut h.disc, id)?;
            let globals = if global_file >= 0 {
                vec![h.assets.open(&mut h.disc, global_file as u32)?]
            } else {
                vec![]
            };
            let graph = h.assets.native_map(handle, &globals)?;
            // SAFETY: C supplies a native IPD descriptor destination. Every
            // pointer refers to separate allocations retained by AssetStore.
            unsafe {
                destination.copy_from_nonoverlapping(graph, 1);
            }
            Ok::<(), String>(())
        })();
        match result {
            Ok(()) => 0,
            Err(error) => {
                h.error = Some(error);
                1
            }
        }
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
unsafe extern "C" fn port_save_read(slot: u32, destination: *mut u8, count: u32) -> i32 {
    if destination.is_null() || count != 636 {
        return 2;
    }
    host(|h| match h.saves.read(slot) {
        Ok(Some(bytes)) => {
            // SAFETY: C supplies one writable 636-byte save record for this call.
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), destination, 636);
            }
            0
        }
        Ok(None) => 1,
        Err(error) => {
            h.error = Some(format!("native save read: {error}"));
            2
        }
    })
}
#[unsafe(no_mangle)]
unsafe extern "C" fn port_save_write(slot: u32, source: *const u8, count: u32) -> i32 {
    if source.is_null() || count != 636 {
        return 2;
    }
    // SAFETY: C supplies one readable 636-byte original-format save record.
    let bytes = unsafe { std::slice::from_raw_parts(source, 636) };
    host(|h| match h.saves.write(slot, bytes) {
        Ok(()) => 0,
        Err(error) => {
            h.error = Some(format!("native save write: {error}"));
            2
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
unsafe extern "C" fn port_asset_load_native(id: u32, destination: *mut u8, kind: u32) -> i32 {
    if destination.is_null() {
        return 1;
    }
    host(|h| {
        let result = (|| {
            let handle = h.assets.open(&mut h.disc, id)?;
            // SAFETY: C's file queue supplies a descriptor-sized destination.
            // The descriptor points into separate stable AssetStore allocations.
            unsafe {
                match kind {
                    3 => {
                        unsafe extern "C" {
                            fn port_move_dms_publish(destination: *mut u8, bytes: *const u8, count: usize) -> i32;
                        }
                        let bytes = h.disc.read_entry(id).map_err(|error| error.to_string())?;
                        crate::assets::decode_dms(&bytes).map_err(|error| error.to_string())?;
                        if port_move_dms_publish(destination, bytes.as_ptr(), bytes.len()) != 0 {
                            return Err("native DMS graph publication failed".into());
                        }
                    }
                    4 => destination
                        .cast::<crate::gameplay::NativeAnmHeader>()
                        .copy_from_nonoverlapping(h.assets.native_animation(handle)?, 1),
                    5 | 7 => destination
                        .cast::<crate::gameplay::NativeLmHeader>()
                        .copy_from_nonoverlapping(h.assets.native_lm(handle)?, 1),
                    _ => return Err("unported native asset destination".into()),
                }
            }
            Ok::<(), String>(())
        })();
        match result {
            Ok(()) => 0,
            Err(e) => {
                h.error = Some(e);
                1
            }
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
    let positions = crate::gte::packet_positions(words, packet);
    host(|state| match positions {
        Some(positions) => state.gpu.packet_precise(packet, &positions),
        None => state.gpu.packet(packet),
    });
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
    crate::gte::end_ordering_table();
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
extern "C" fn port_movie_begin(id: u32, last_frame: u32) -> i32 {
    host(|h| match Movie::new(&h.disc, id, last_frame, h.frames) {
        Ok(movie) => {
            h.movie = Some(movie);
            0
        }
        Err(error) => {
            h.error = Some(error);
            1
        }
    })
}
#[unsafe(no_mangle)]
extern "C" fn port_movie_tick() -> i32 {
    host(|h| {
        let Some(movie) = h.movie.as_mut() else {
            h.error = Some("movie tick without begin".into());
            return 2;
        };
        let before = movie.decoded_frames;
        let result = movie.tick(&mut h.disc, h.gpu.as_mut(), h.spu.as_mut(), h.frames);
        h.movie_frames += u64::from(movie.decoded_frames - before);
        match result {
            Ok(done) => i32::from(done),
            Err(error) => {
                h.error = Some(error);
                2
            }
        }
    })
}
#[unsafe(no_mangle)]
extern "C" fn port_movie_end(skipped: i32) {
    host(|h| {
        if let Some(movie) = h.movie.take() {
            movie.end(h.spu.as_mut(), skipped != 0);
        }
        if skipped != 0 {
            h.movie_skips += 1;
        }
    });
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
extern "C" fn port_present(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    state: i32,
    step: i32,
    rgb24: i32,
) -> i32 {
    host(|host| {
        host.frames += 1;
        // SAFETY: Presentation runs synchronously on the sole C worker.
        // PORT: Optional numeric gameplay telemetry; no game bytes or captures.
        unsafe extern "C" {
            fn port_player_trace(frame: u32);
        }
        unsafe { port_player_trace(host.frames as u32) };
        // PORT: One virtual NTSC VBlank is exactly 735 hardware mixer samples.
        // The original libsd sequencer remains a separate, currently guarded seam.
        if let Err(error) = host.spu.advance_to(host.frames * 735) {
            host.error = Some(error);
            return 1;
        }
        host.state = state;
        host.step = step;
        let display_enabled = w > 0 && h > 0;
        let w = if w > 0 { w as u32 } else { 320 };
        let h = if h > 0 { h as u32 } else { 240 };
        let pixels = if display_enabled {
            if rgb24 != 0 {
                host.gpu.frame_rgb24(x, y, w, h)
            } else {
                host.gpu.frame(x, y, w, h)
            }
        } else {
            vec![0; (w * h) as usize]
        };
        if state == 1 && host.first_logo.is_none() {
            host.first_logo = Some(host.frames);
            println!("Entered B_KONAMI at frame {}", host.frames);
        }
        let texture = crate::gpu_wgpu::frame_texture(display_enabled);
        if let Some(error) = crate::gpu_wgpu::frame_error() {
            host.error = Some(error);
            return 1;
        }
        if pixels.len() != (w * h) as usize {
            host.error = Some("invalid GPU frame size".into());
            return 1;
        }
        host.last_frame = Some((w, h, pixels.clone()));
        let finished = host.frames >= host.limit || host.cancel.load(Ordering::Relaxed);
        crate::gpu_wgpu::record_frame(finished);
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
        if let Some(proxy) = &host.proxy
            && proxy
                .send_event(Frame {
                    w,
                    h,
                    pixels,
                    texture,
                    finished,
                })
                .is_err()
        {
            return 1;
        }
        if host.proxy.is_some() {
            host.next += Duration::from_secs_f64(1.0 / 60.0);
            if let Some(wait) = host.next.checked_duration_since(Instant::now()) {
                std::thread::sleep(wait);
            }
        }
        i32::from(finished)
    })
}

fn save_png(path: &Path, w: u32, h: u32, pixels: &[u32]) -> Result<(), Box<dyn std::error::Error>> {
    let (w, h, pixels) = crate::gpu_wgpu::screenshot(w, h, pixels)?;
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
    texture: Option<crate::gpu_wgpu::FrameTexture>,
    finished: bool,
}
struct App {
    input: Arc<LiveInput>,
    window: Option<Arc<Window>>,
    surface: Option<crate::gpu_wgpu::Presentation>,
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
                .with_inner_size(crate::gpu_wgpu::window_size()),
        ) {
            Ok(window) => {
                let window = Arc::new(window);
                let result = crate::gpu_wgpu::Presentation::new(window.clone());
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
        crate::pad_touch::window_event(&event, self.window.as_deref());
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
                if size.width == 0 || size.height == 0 {
                    return;
                }
                let result =
                    surface.present(frame.w, frame.h, &frame.pixels, frame.texture.as_ref());
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
#[derive(Default)]
pub struct ReplayCheck {
    pub world_probe: bool,
    pub min_lit_pixels: usize,
    pub option_entry: Option<i32>,
    pub menu_state: Option<i32>,
    pub state: Option<i32>,
    pub step: Option<i32>,
    pub min_movie_frames: u64,
    pub movie_skips: Option<u32>,
}

pub fn run_headless(
    disc: GameDisc<DiscImage<File>>,
    limit: u64,
    screenshot: Option<PathBuf>,
    replay: ReplayPad,
    check: ReplayCheck,
) -> Result<(), String> {
    let gpu = crate::gpu_wgpu::backend()?;
    let spu = Box::new(crate::spu_cpal::open_configured()?);
    let saves = SaveStore::app_data()?;
    let now = Instant::now();
    HOST.with(|cell| {
        *cell.borrow_mut() = Some(Host {
            disc,
            gpu,
            spu,
            pad: Box::new(replay),
            assets: AssetStore::default(),
            saves,
            proxy: None,
            frames: 0,
            limit,
            screenshot,
            start: now,
            next: now,
            cancel: Arc::new(AtomicBool::new(false)),
            error: None,
            first_logo: None,
            last_frame: None,
            movie: None,
            state: 0,
            step: 0,
            movie_frames: 0,
            movie_skips: 0,
        })
    });
    // SAFETY: Headless mode is the sole C worker; jumps cross only C stack frames.
    let code = unsafe {
        if check.world_probe {
            port_run_world_probe()
        } else {
            port_run_game()
        }
    };
    let result = host(|h| {
        if let Some(movie) = h.movie.take() {
            movie.end(h.spu.as_mut(), false);
        }
        h.spu.finish()?;
        if let (Some(path), Some((w, height, pixels))) = (&h.screenshot, &h.last_frame) {
            save_png(path, *w, *height, pixels).map_err(|e| e.to_string())?;
        }
        println!(
            "CHECK code={code} frames={} state={} step={} menu={} option_entry={} fade={} movie_frames={} movie_skips={}",
            h.frames,
            h.state,
            h.step,
            unsafe { sh_title_menu_state() },
            unsafe { sh_option_selected_entry() },
            unsafe { g_ScreenFade_Status },
            h.movie_frames,
            h.movie_skips
        );
        let lit = h.last_frame.as_ref().map_or(0, |(_, _, pixels)| {
            pixels.iter().filter(|p| **p != 0).count()
        });
        println!("VISIBLE lit_pixels={lit}");
        if let Some(error) = h.error.take() {
            return Err(error);
        }
        if code != 0 {
            return Err(format!(
                "native game stopped: code {code}, frames {}",
                h.frames
            ));
        }
        if !check.world_probe && h.frames != limit {
            return Err(format!("completed {} of {limit} ticks", h.frames));
        }
        if check.state.is_some_and(|state| state != h.state)
            || lit < check.min_lit_pixels
            || check
                .option_entry
                .is_some_and(|entry| entry != unsafe { sh_option_selected_entry() })
            || check
                .menu_state
                .is_some_and(|menu| menu != unsafe { sh_title_menu_state() })
            || check.step.is_some_and(|step| step != h.step)
            || h.movie_frames < check.min_movie_frames
            || check
                .movie_skips
                .is_some_and(|skips| skips != h.movie_skips)
        {
            return Err("replay milestone expectation failed".into());
        }
        Ok(())
    });
    HOST.with(|cell| *cell.borrow_mut() = None);
    result
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
            gpu: crate::gpu_wgpu::backend()?,
            spu: Box::new(crate::spu_cpal::open_configured()?),
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
    let saves = SaveStore::app_data()?;
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
                saves,
                proxy: Some(proxy),
                frames: 0,
                limit,
                screenshot,
                start: now,
                next: now,
                cancel: worker_cancel,
                error: None,
                first_logo: None,
                last_frame: None,
                movie: None,
                state: 0,
                step: 0,
                movie_frames: 0,
                movie_skips: 0,
            })
        });
        // SAFETY: This is the sole game worker. C's exit jump only crosses C frames after Rust callbacks return.
        let code = unsafe { port_run_game() };
        host(|h| {
            if let Some(movie) = h.movie.take() {
                movie.end(h.spu.as_mut(), false);
            }
            h.spu.finish()?;
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
                let _ = h.proxy.as_ref().expect("window proxy").send_event(Frame {
                    w,
                    h: height,
                    pixels,
                    texture: crate::gpu_wgpu::frame_texture(true),
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

#[cfg(test)]
mod tests {
    #[test]
    fn boot_overlay_namespace_load_and_initial_data_reset() {
        unsafe extern "C" {
            fn port_overlay_activate(file_id: u32) -> i32;
            fn sh_b_konami_reset_probe() -> i32;
            fn sh_save_init_probe() -> i32;
            fn sh_option_reset_probe() -> i32;
            fn port_queue_image_probe() -> i32;
            fn port_player_controls_probe() -> u32;
            fn port_move_native_probe() -> u32;
        }
        // SAFETY: No game worker runs in tests. Only one test accesses these
        // native overlay globals; layout/reader tests have no shared state.
        unsafe {
            assert_eq!(port_overlay_activate(4), 0);
            assert_eq!(sh_b_konami_reset_probe(), 1);
            assert_eq!(sh_save_init_probe(), 1);
            assert_eq!(sh_option_reset_probe(), 1);
            assert_eq!(port_queue_image_probe(), 1);
            assert_eq!(port_player_controls_probe(), 511);
            assert_eq!(port_move_native_probe(), 127);
            assert_eq!(port_overlay_activate(u32::MAX), 1);
            assert_eq!(port_overlay_activate(4), 0);
        }
    }
}

// PORT: Rendering diagnostics are generated by the owned rendering prep.
#[cfg(test)]
include!(concat!(
    env!("OUT_DIR"),
    "/native-source/render_milestone.rs"
));
