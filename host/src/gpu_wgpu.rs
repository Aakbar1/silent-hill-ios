// SPDX-License-Identifier: GPL-3.0-only
//! Host adapter, display scaling and presentation for the ordered psxgpu renderer.
use crate::{backend::GpuBackend, raster::Raster};
use psxgpu::{Display, Processor, Rect, Renderer, WgpuRenderer, block_on, wgpu};
use std::{
    cell::RefCell,
    ffi::{OsStr, OsString},
    num::NonZeroU32,
    sync::{Arc, Mutex, OnceLock},
    time::Instant,
};
use winit::{dpi::PhysicalSize, window::Window};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RendererKind {
    Soft,
    Wgpu,
}

#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub renderer: RendererKind,
    pub scale: u32,
    pub stats: bool,
    pub widescreen: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            // Switch only after the available game replays have been compared at 1x.
            renderer: RendererKind::Soft,
            scale: 4,
            stats: false,
            widescreen: false,
        }
    }
}
impl Options {
    pub fn argument(
        &mut self,
        flag: &OsStr,
        args: &mut impl Iterator<Item = OsString>,
    ) -> Result<(), String> {
        match flag.to_str() {
            Some("--renderer") => {
                self.renderer = match args.next().as_deref().and_then(OsStr::to_str) {
                    Some("wgpu") => RendererKind::Wgpu,
                    Some("soft") => RendererKind::Soft,
                    _ => return Err("--renderer requires wgpu or soft".into()),
                };
            }
            Some("--scale") => {
                self.scale = args
                    .next()
                    .as_deref()
                    .and_then(OsStr::to_str)
                    .and_then(|value| value.parse().ok())
                    .filter(|value| (1..=8).contains(value))
                    .ok_or("--scale requires an integer from 1 to 8")?;
            }
            Some("--stats") => self.stats = true,
            Some("--wide") | Some("--16:9") => self.widescreen = true,
            _ => return Err("unknown renderer option".into()),
        }
        Ok(())
    }
}
static OPTIONS: OnceLock<Options> = OnceLock::new();
static CONTEXT: Mutex<Option<Arc<Context>>> = Mutex::new(None);
pub fn configure(options: Options) -> Result<(), String> {
    if !(1..=8).contains(&options.scale) {
        return Err("--scale requires an integer from 1 to 8".into());
    }
    OPTIONS
        .set(options)
        .map_err(|_| "renderer already configured".into())
}
pub fn options() -> Options {
    OPTIONS.get().copied().unwrap_or_default()
}
pub fn window_size() -> PhysicalSize<u32> {
    let options = options();
    let height = 224 * options.scale;
    PhysicalSize::new(
        if options.widescreen {
            height * 16 / 9
        } else {
            320 * options.scale
        },
        height,
    )
}
pub fn backend() -> Result<Box<dyn GpuBackend>, String> {
    FRAME.with(|frame| *frame.borrow_mut() = None);
    ERROR.with(|error| *error.borrow_mut() = None);
    STATS.with(|stats| *stats.borrow_mut() = FrameStats::default());
    let options = options();
    match options.renderer {
        RendererKind::Soft => Ok(Box::<Raster>::default()),
        RendererKind::Wgpu => {
            let gpu = WgpuGpu::new(options.scale)?;
            *CONTEXT.lock().expect("GPU context mutex") = Some(gpu.context.clone());
            Ok(Box::new(gpu))
        }
    }
}

struct Context {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}
impl Context {
    async fn new(scale: u32) -> Result<Self, String> {
        #[cfg(target_os = "windows")]
        let backends = wgpu::Backends::DX12;
        #[cfg(target_vendor = "apple")]
        let backends = wgpu::Backends::METAL;
        #[cfg(not(any(target_os = "windows", target_vendor = "apple")))]
        let backends = wgpu::Backends::VULKAN;
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .ok_or("no wgpu hardware adapter")?;
        let limits = wgpu::Limits {
            max_storage_buffers_per_shader_stage: 6,
            max_storage_buffer_binding_size: (1024 * 512 * 4 * scale * scale)
                .max(128 * 1024 * 1024),
            ..Default::default()
        };
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Silent Hill GPU"),
                    required_features: wgpu::Features::empty(),
                    required_limits: limits,
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .map_err(|e| e.to_string())?;
        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }
}

/// A separate texture per queued frame prevents worker/UI races during presentation.
#[derive(Clone)]
pub struct FrameTexture {
    context: Arc<Context>,
    texture: wgpu::Texture,
}
thread_local! {
    static FRAME: RefCell<Option<FrameTexture>> = const { RefCell::new(None) };
    static ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
    static STATS: RefCell<FrameStats> = RefCell::new(FrameStats::default());
}
pub fn frame_texture(enabled: bool) -> Option<FrameTexture> {
    FRAME.with(|frame| {
        if !enabled {
            *frame.borrow_mut() = None;
        }
        frame.borrow().clone()
    })
}

pub struct WgpuGpu {
    gpu: Mutex<Processor<WgpuRenderer>>,
    context: Arc<Context>,
    primitives: u64,
    error: Mutex<Option<String>>,
}
impl WgpuGpu {
    pub fn new(scale: u32) -> Result<Self, String> {
        if !(1..=8).contains(&scale) {
            return Err("scale must be 1..=8".into());
        }
        let context = Arc::new(block_on(Context::new(scale))?);
        let renderer = block_on(WgpuRenderer::from_device(
            context.device.clone(),
            context.queue.clone(),
            context.adapter.get_info(),
            scale,
        ))
        .map_err(|e| e.to_string())?;
        println!(
            "GPU wgpu adapter={} scale={scale}",
            context.adapter.get_info().name
        );
        let mut gpu = Processor::new(renderer);
        gpu.gp0_words(&[0xe3000000, 0xe407ffff])
            .map_err(|e| e.to_string())?;
        Ok(Self {
            gpu: Mutex::new(gpu),
            context,
            primitives: 0,
            error: Mutex::new(None),
        })
    }
    /// The existing trait has no Result; retain errors for the host's frame boundary.
    fn remember<T>(&self, result: psxgpu::Result<T>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                let mut failure = self.error.lock().expect("GPU error mutex");
                if failure.is_none() {
                    eprintln!("GPU ERROR: {error}");
                    *failure = Some(error.to_string());
                    ERROR.with(|failure| *failure.borrow_mut() = Some(error.to_string()));
                }
                None
            }
        }
    }
    pub fn error(&self) -> Option<String> {
        self.error.lock().expect("GPU error mutex").clone()
    }
    fn scanout(&self, x: i32, y: i32, w: u32, h: u32, rgb24: bool) -> Vec<u32> {
        if w == 0 || h == 0 || w > 1024 || h > 512 {
            return Vec::new();
        }
        let mut gpu = self.gpu.lock().expect("GPU mutex");
        let display = Display {
            disabled: false,
            x: (x & 1023) as u16,
            y: (y & 511) as u16,
            // Mode 3 uses four GPU clocks per pixel; retain the host's literal size.
            horizontal: [0, (w * 4) as u16],
            vertical: [0, h as u16],
            mode: 3 | if rgb24 { 16 } else { 0 },
        };
        let renderer = gpu.renderer_mut();
        let Some(texture) = self.remember(renderer.create_scanout_texture(display)) else {
            return Vec::new();
        };
        if self
            .remember(renderer.render_scanout(display, false, &texture))
            .is_none()
        {
            return Vec::new();
        }
        FRAME.with(|frame| {
            *frame.borrow_mut() = Some(FrameTexture {
                context: self.context.clone(),
                texture,
            })
        });
        // Native pixels serve existing replay/screenshot/visibility checks. The window
        // consumes the scaled GPU texture directly, preserving subpixel rasterization.
        let Some(vram) = self.remember(renderer.read_vram()) else {
            return Vec::new();
        };
        let Some(rgba) = self.remember(psxgpu::scanout(&vram, display, false)) else {
            return Vec::new();
        };
        rgba.as_chunks::<4>()
            .0
            .iter()
            .map(|p| (u32::from(p[0]) << 16) | (u32::from(p[1]) << 8) | u32::from(p[2]))
            .collect()
    }
}
impl GpuBackend for WgpuGpu {
    fn packet(&mut self, words: &[u32]) {
        let Some(payload) = words.get(1..) else {
            return;
        };
        if payload.is_empty() {
            return;
        }
        let mut gpu = self.gpu.lock().expect("GPU mutex");
        if !gpu.idle() {
            self.remember(Err::<(), _>(psxgpu::Error(
                "incomplete previous host GPU packet".into(),
            )));
            return;
        }
        if self.remember(gpu.gp0_words(payload)).is_some() && !gpu.idle() {
            self.remember(Err::<(), _>(psxgpu::Error(
                "incomplete host GPU packet".into(),
            )));
        }
        if matches!(payload[0] >> 29, 1..=3) {
            self.primitives += 1;
        }
    }
    fn end_ordering_table(&mut self) {
        self.remember(self.gpu.lock().expect("GPU mutex").renderer_mut().flush());
    }
    fn env(&mut self, [x, y, w, h]: [i32; 4], offset: [i32; 2]) {
        let mut gpu = self.gpu.lock().expect("GPU mutex");
        // The host passes a literal rectangle, while E3/E4 use inclusive endpoints.
        gpu.draw_state.area = [
            x.max(0),
            y.max(0),
            x.saturating_add(w).min(1024) - 1,
            y.saturating_add(h).min(512) - 1,
        ];
        gpu.draw_state.offset = offset;
    }
    fn load(&mut self, [x, y, w, h]: [i32; 4], pixels: &[u16]) {
        if w <= 0 || h <= 0 || w > 1024 || h > 512 {
            return;
        }
        self.remember(self.gpu.lock().expect("GPU mutex").renderer_mut().upload(
            Rect::new((x & 1023) as u16, (y & 511) as u16, w as u16, h as u16),
            pixels,
            0,
        ));
    }
    fn read(&self, [x, y, w, h]: [i32; 4]) -> Vec<u16> {
        if w <= 0 || h <= 0 || w > 1024 || h > 512 {
            return Vec::new();
        }
        self.remember(
            self.gpu
                .lock()
                .expect("GPU mutex")
                .renderer_mut()
                .read(Rect::new(
                    (x & 1023) as u16,
                    (y & 511) as u16,
                    w as u16,
                    h as u16,
                )),
        )
        .unwrap_or_default()
    }
    fn clear(&mut self, [x, y, w, h]: [i32; 4], color: [u8; 3]) {
        let left = x.max(0);
        let top = y.max(0);
        let right = x.saturating_add(w).min(1024);
        let bottom = y.saturating_add(h).min(512);
        if right <= left || bottom <= top {
            return;
        }
        let [r, g, b] = color.map(|c| u16::from(c >> 3));
        self.remember(self.gpu.lock().expect("GPU mutex").renderer_mut().fill(
            Rect::new(
                left as u16,
                top as u16,
                (right - left) as u16,
                (bottom - top) as u16,
            ),
            r | (g << 5) | (b << 10),
        ));
    }
    fn frame(&self, x: i32, y: i32, w: u32, h: u32) -> Vec<u32> {
        self.scanout(x, y, w, h, false)
    }
    fn frame_rgb24(&self, x: i32, y: i32, w: u32, h: u32) -> Vec<u32> {
        self.scanout(x, y, w, h, true)
    }
    fn primitives(&self) -> u64 {
        self.primitives
    }
}

#[derive(Default)]
struct FrameStats {
    previous: Option<Instant>,
    count: u64,
    total_ms: f64,
    max_ms: f64,
}
pub fn record_frame(finished: bool) {
    if !options().stats {
        return;
    }
    STATS.with(|stats| {
        let mut stats = stats.borrow_mut(); let now = Instant::now();
        if let Some(previous) = stats.previous.replace(now) {
            let ms = now.duration_since(previous).as_secs_f64() * 1000.0;
            stats.count += 1; stats.total_ms += ms; stats.max_ms = stats.max_ms.max(ms);
        }
        if stats.count > 0 && (finished || stats.count % 120 == 0) {
            println!("STATS frames={} mean_ms={:.3} max_ms={:.3} fps={:.2} (frame intervals; includes pacing, decode and native readback)", stats.count, stats.total_ms / stats.count as f64, stats.max_ms, stats.count as f64 * 1000.0 / stats.total_ms);
        }
    });
}

/// Native host must stop on errors rather than accept a black/partial GPU frame.
pub fn frame_error() -> Option<String> {
    ERROR.with(|error| error.borrow().clone())
}

/// PORT: optional 16:9 stretches presentation only; camera/clip/game logic stay original.
fn viewport(width: u32, height: u32) -> [u32; 4] {
    viewport_for(width, height, options().widescreen)
}
fn viewport_for(width: u32, height: u32, widescreen: bool) -> [u32; 4] {
    // Preserve the existing 640x448 host window's aspect across 320/640-wide
    // and progressive/interlaced framebuffers; storage dimensions are not aspect.
    let (aspect_w, aspect_h): (u32, u32) = if widescreen { (16, 9) } else { (320, 224) };
    let w = width
        .min((u64::from(height) * u64::from(aspect_w) / u64::from(aspect_h)) as u32)
        .max(1);
    let h = height
        .min((u64::from(width) * u64::from(aspect_h) / u64::from(aspect_w)) as u32)
        .max(1);
    [(width - w) / 2, (height - h) / 2, w, h]
}

pub struct Presentation {
    window: Arc<Window>,
    soft: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
    accelerated: Option<Surface>,
}
impl Presentation {
    pub fn new(window: Arc<Window>) -> Result<Self, String> {
        let size = window.inner_size();
        println!(
            "WINDOW initial={}x{} renderer={:?} wide={}",
            size.width,
            size.height,
            options().renderer,
            options().widescreen
        );
        let soft = if options().renderer == RendererKind::Soft {
            let context = softbuffer::Context::new(window.clone()).map_err(|e| e.to_string())?;
            Some(softbuffer::Surface::new(&context, window.clone()).map_err(|e| e.to_string())?)
        } else {
            None
        };
        let accelerated = if soft.is_none() {
            let context = CONTEXT
                .lock()
                .expect("GPU context mutex")
                .clone()
                .ok_or("wgpu backend must be created before presentation")?;
            Some(Surface::new(window.clone(), context)?)
        } else {
            None
        };
        Ok(Self {
            window,
            soft,
            accelerated,
        })
    }
    pub fn present(
        &mut self,
        w: u32,
        h: u32,
        pixels: &[u32],
        frame: Option<&FrameTexture>,
    ) -> Result<(), String> {
        let size = self.window.inner_size();
        let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        else {
            return Ok(());
        };
        if let Some(soft) = &mut self.soft {
            soft.resize(width, height).map_err(|e| e.to_string())?;
            let mut buffer = soft.buffer_mut().map_err(|e| e.to_string())?;
            buffer.fill(0);
            let [left, top, vw, vh] = viewport(size.width, size.height);
            for y in 0..vh {
                for x in 0..vw {
                    buffer[((y + top) * size.width + x + left) as usize] =
                        pixels[((y * h / vh) * w + x * w / vw) as usize];
                }
            }
            buffer.present().map_err(|e| e.to_string())
        } else {
            self.accelerated
                .as_mut()
                .expect("surface initialized")
                .present(size, frame)
        }
    }
}
struct Surface {
    surface: wgpu::Surface<'static>,
    context: Arc<Context>,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
}
impl Surface {
    fn new(window: Arc<Window>, context: Arc<Context>) -> Result<Self, String> {
        let surface = context
            .instance
            .create_surface(window)
            .map_err(|e| e.to_string())?;
        let caps = surface.get_capabilities(&context.adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .ok_or("surface has no unorm format")?;
        let mut config = surface
            .get_default_config(&context.adapter, 1, 1)
            .ok_or("GPU cannot present to window")?;
        config.format = format;
        config.present_mode = wgpu::PresentMode::Fifo;
        let shader = context
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("PS1 nearest blit"),
                source: wgpu::ShaderSource::Wgsl(BLIT.into()),
            });
        let pipeline = context
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("PS1 unorm blit"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vertex"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fragment"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            });
        Ok(Self {
            surface,
            context,
            config,
            pipeline,
        })
    }
    fn present(
        &mut self,
        size: PhysicalSize<u32>,
        frame: Option<&FrameTexture>,
    ) -> Result<(), String> {
        if self.config.width != size.width || self.config.height != size.height {
            self.config.width = size.width;
            self.config.height = size.height;
            self.surface.configure(&self.context.device, &self.config);
        }
        let output = match self.surface.get_current_texture() {
            Ok(output) => output,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.context.device, &self.config);
                self.surface
                    .get_current_texture()
                    .map_err(|e| e.to_string())?
            }
            Err(wgpu::SurfaceError::Timeout) => return Err("GPU surface timed out".into()),
            Err(e) => return Err(e.to_string()),
        };
        let group = frame.map(|frame| {
            self.context
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("PS1 frame"),
                    layout: &self.pipeline.get_bind_group_layout(0),
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &frame.texture.create_view(&Default::default()),
                        ),
                    }],
                })
        });
        let mut encoder = self
            .context
            .device
            .create_command_encoder(&Default::default());
        {
            let view = output.texture.create_view(&Default::default());
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("PS1 present"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if let Some(group) = &group {
                let [x, y, vw, vh] = viewport(size.width, size.height);
                pass.set_viewport(x as f32, y as f32, vw as f32, vh as f32, 0.0, 1.0);
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, group, &[]);
                pass.draw(0..3, 0..1);
            }
        }
        self.context.queue.submit([encoder.finish()]);
        output.present();
        Ok(())
    }
}
const BLIT: &str = r#"
struct Output { @builtin(position) position: vec4f, @location(0) uv: vec2f }
@vertex fn vertex(@builtin(vertex_index) index: u32) -> Output {
    let uv = vec2f(f32((index << 1u) & 2u), f32(index & 2u));
    return Output(vec4f(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0), uv);
}
@group(0) @binding(0) var image: texture_2d<f32>;
@fragment fn fragment(input: Output) -> @location(0) vec4f {
    let size = textureDimensions(image);
    return textureLoad(image, vec2i(min(vec2u(input.uv * vec2f(size)), size - 1u)), 0);
}
"#;

pub fn screenshot(w: u32, h: u32, pixels: &[u32]) -> Result<(u32, u32, Vec<u32>), String> {
    if let Some(frame) = frame_texture(true) {
        return frame.read_pixels();
    }
    let scale = options().scale;
    let (width, height) = (w * scale, h * scale);
    let result = (0..height)
        .flat_map(|y| (0..width).map(move |x| pixels[((y / scale) * w + x / scale) as usize]))
        .collect();
    Ok((width, height, result))
}
impl FrameTexture {
    pub fn read_pixels(&self) -> Result<(u32, u32, Vec<u32>), String> {
        let (w, h) = (self.texture.width(), self.texture.height());
        let stride = (w * 4).div_ceil(256) * 256;
        let buffer = self.context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("private screenshot"),
            size: u64::from(stride) * u64::from(h),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .context
            .device
            .create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride),
                    rows_per_image: Some(h),
                },
            },
            self.texture.size(),
        );
        self.context.queue.submit([encoder.finish()]);
        let slice = buffer.slice(..);
        let (send, receive) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = send.send(result);
        });
        let _ = self.context.device.poll(wgpu::Maintain::Wait);
        receive
            .recv()
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
        let mapped = slice.get_mapped_range();
        let pixels = mapped
            .chunks_exact(stride as usize)
            .flat_map(|row| {
                row[..w as usize * 4]
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|p| (u32::from(p[0]) << 16) | (u32::from(p[1]) << 8) | u32::from(p[2]))
            })
            .collect();
        drop(mapped);
        buffer.unmap();
        Ok((w, h, pixels))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interlaced_aspect_letterbox_and_wide_presentation() {
        assert_eq!(viewport_for(1280, 896, false), [0, 0, 1280, 896]);
        assert_eq!(viewport_for(1280, 896, true), [0, 88, 1280, 720]);
        assert_eq!(viewport_for(1280, 960, false), [0, 32, 1280, 896]);
    }
    #[test]
    fn arguments_reject_bad_renderer_and_scale() {
        let mut options = Options::default();
        for value in ["0", "9", "-1", "abc"] {
            assert!(
                options
                    .argument(
                        OsStr::new("--scale"),
                        &mut [OsString::from(value)].into_iter()
                    )
                    .is_err()
            );
        }
        assert!(
            options
                .argument(
                    OsStr::new("--renderer"),
                    &mut [OsString::from("unknown")].into_iter()
                )
                .is_err()
        );
        options
            .argument(
                OsStr::new("--scale"),
                &mut [OsString::from("8")].into_iter(),
            )
            .unwrap();
        assert_eq!(options.scale, 8);
    }
    #[test]
    fn host_packets_match_psx_reference_for_gradients_and_diagonal_lines() {
        let mut gpu = WgpuGpu::new(1).unwrap();
        let mut reference = Processor::new(psxgpu::SoftwareRenderer::new());
        let mut legacy = Raster::default();
        gpu.env([0, 0, 64, 64], [-2, 3]);
        legacy.env([0, 0, 64, 64], [-2, 3]);
        reference.draw_state.area = [0, 0, 63, 63];
        reference.draw_state.offset = [-2, 3];
        let packets: &[&[u32]] = &[
            &[0, 0xe1000200],
            &[
                0, 0x38ffffff, 0x00010001, 0x00102080, 0x00010011, 0x0060a0ff, 0x00110001,
                0x00102030, 0x00110011,
            ],
            &[0, 0x50ff8000, 0x00140028, 0x00ffff00, 0x00240018],
            &[0, 0x50ff8000, 0x002c0028, 0x00ffff00, 0x001c0018],
        ];
        for packet in packets {
            gpu.packet(packet);
            legacy.packet(packet);
            reference.gp0_words(&packet[1..]).unwrap();
        }
        let actual = gpu.read([0, 0, 64, 64]);
        assert_eq!(
            actual,
            reference
                .renderer_mut()
                .read(Rect::new(0, 0, 64, 64))
                .unwrap()
        );
        // The boot fallback samples polygons at half-pixel centers, omits dither,
        // and uses different line rounding/endpoints. It is not a PS1 golden.
        assert_ne!(actual, legacy.read([0, 0, 64, 64]));
        assert!(gpu.error().is_none());
    }
    #[test]
    fn adapter_ordering_reads_and_rgb24_scanout() {
        let mut gpu = WgpuGpu::new(1).unwrap();
        gpu.load([100, 20, 2, 1], &[0x1234, 0x5678]);
        assert_eq!(gpu.read([100, 20, 2, 1]), [0x1234, 0x5678]);
        gpu.env([2, 1, 3, 2], [2, 1]);
        gpu.packet(&[0, 0x600000ff, 0, 0x00040004]);
        gpu.end_ordering_table();
        assert_eq!(gpu.frame(2, 1, 4, 1), [0xff0000, 0xff0000, 0xff0000, 0]);
        gpu.load(
            [50, 30, 3, 2],
            &[0x0201, 0x0403, 0x0605, 0x0807, 0x0a09, 0x0c0b],
        );
        assert_eq!(
            gpu.frame_rgb24(50, 30, 2, 2),
            [0x010203, 0x040506, 0x070809, 0x0a0b0c]
        );
        assert_eq!(
            frame_texture(true).unwrap().read_pixels().unwrap().2,
            [0x010203, 0x040506, 0x070809, 0x0a0b0c]
        );
        assert!(gpu.error().is_none());
        assert_eq!(gpu.primitives(), 1);
        gpu.load(
            [1022, 40, 5, 2],
            &[
                0x0201, 0x0403, 0x0605, 0x0807, 0xff09, 0x0b0a, 0x0d0c, 0x0f0e, 0x1110, 0xff12,
            ],
        );
        assert_eq!(
            gpu.frame_rgb24(1022, 40, 3, 2),
            [0x010203, 0x040506, 0x070809, 0x0a0b0c, 0x0d0e0f, 0x101112]
        );
        assert!(frame_texture(false).is_none());
        gpu.packet(&[0, 0x200000ff]);
        assert_eq!(gpu.error().as_deref(), Some("incomplete host GPU packet"));
        assert_eq!(frame_error(), gpu.error());
    }
}
