// SPDX-License-Identifier: GPL-3.0-only
use crate::{raster, validate_pixels, Error, Primitive, Rect, Renderer, Result, VRAM_WORDS};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

const TILE: i32 = 4;
const COLS: i32 = 1024 / TILE;
const TILES: usize = (COLS * (512 / TILE)) as usize;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Draw {
    meta: [u32; 4],
    state: [u32; 4],
    bounds: [i32; 4],
    vertices: [[i32; 4]; 3],
    colors: [[i32; 4]; 3],
    planes: [[i32; 4]; 5],
    precise: [[f32; 4]; 3],
}
impl From<Primitive> for Draw {
    fn from(p: Primitive) -> Self {
        Self {
            meta: [
                p.kind,
                u32::from(p.opcode) | (u32::from(p.subpixel) << 8) | (u32::from(p.skip_first) << 9),
                p.state.mode
                    | if p.state.skip_line.is_some() {
                        1 << 17
                    } else {
                        0
                    }
                    | if p.state.skip_line == Some(true) {
                        1 << 18
                    } else {
                        0
                    }
                    | if p.state.texture_disable_allowed {
                        1 << 16
                    } else {
                        0
                    },
                p.state.window,
            ],
            state: [p.state.mask, p.clut, p.size[0] as u32, p.size[1] as u32],
            bounds: p.bounds().unwrap_or([0; 4]),
            vertices: p.vertices.map(|v| [v.x, v.y, v.uv[0], v.uv[1]]),
            colors: p.vertices.map(|v| [v.color[0], v.color[1], v.color[2], 0]),
            planes: raster::planes(p).map(|v| [v[0], v[1], v[2], 0]),
            precise: p.vertices.map(|v| [v.precise[0], v.precise[1], 0.0, 0.0]),
        }
    }
}

/// Ordered tiled compute rasterizer. Integer VRAM is separate from sharp output.
pub struct WgpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    info: wgpu::AdapterInfo,
    scale: u32,
    native: wgpu::Buffer,
    source: wgpu::Buffer,
    scaled: wgpu::Buffer,
    draw_pipeline: wgpu::ComputePipeline,
    transfer_pipeline: wgpu::ComputePipeline,
    scanout_pipeline: wgpu::ComputePipeline,
    pending: Vec<Primitive>,
    dirty: [bool; TILES],
    dirty_bounds: [i32; 4],
}

fn storage(device: &wgpu::Device, label: &str, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
fn data_buffer<T: Pod>(
    device: &wgpu::Device,
    label: &str,
    value: &[T],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytemuck::cast_slice(value),
        usage,
    })
}
fn intersects(a: [i32; 4], b: [i32; 4]) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

impl WgpuRenderer {
    pub async fn new(scale: u32) -> Result<Self> {
        #[cfg(target_os = "windows")]
        let backends = wgpu::Backends::DX12;
        #[cfg(target_vendor = "apple")]
        let backends = wgpu::Backends::METAL;
        #[cfg(not(any(target_os = "windows", target_vendor = "apple")))]
        let backends = wgpu::Backends::VULKAN;
        Self::with_backends(scale, backends).await
    }
    pub async fn with_backends(scale: u32, backends: wgpu::Backends) -> Result<Self> {
        if !(1..=8).contains(&scale) {
            return Err(Error("scale must be 1..=8".into()));
        }
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
            .ok_or_else(|| Error("no wgpu hardware adapter".into()))?;
        let limits = wgpu::Limits {
            max_storage_buffers_per_shader_stage: 6,
            ..Default::default()
        };
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("psxgpu"),
                    required_features: wgpu::Features::empty(),
                    required_limits: limits,
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .map_err(|e| Error(e.to_string()))?;
        Self::from_device(device, queue, adapter.get_info(), scale).await
    }

    /// Use the host's device/queue for zero-copy presentation. Needs 6 storage bindings.
    pub async fn from_device(
        device: wgpu::Device,
        queue: wgpu::Queue,
        info: wgpu::AdapterInfo,
        scale: u32,
    ) -> Result<Self> {
        if !(1..=8).contains(&scale) {
            return Err(Error("scale must be 1..=8".into()));
        }
        let size = (VRAM_WORDS as u64) * 4;
        if device.limits().max_storage_buffers_per_shader_stage < 6
            || u64::from(device.limits().max_storage_buffer_binding_size)
                < size * u64::from(scale * scale)
        {
            return Err(Error(
                "device limits too small for psxgpu scale/storage bindings".into(),
            ));
        }
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::include_wgsl!("raster.wgsl"));
        let draw_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("PS1 tiled raster"),
            layout: None,
            module: &shader,
            entry_point: Some("draw"),
            compilation_options: Default::default(),
            cache: None,
        });
        let transfer_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("PS1 transfer"),
            layout: None,
            module: &shader,
            entry_point: Some("transfer"),
            compilation_options: Default::default(),
            cache: None,
        });
        let scanout_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("PS1 scanout"),
            layout: None,
            module: &shader,
            entry_point: Some("scanout"),
            compilation_options: Default::default(),
            cache: None,
        });
        if let Some(e) = device.pop_error_scope().await {
            return Err(Error(e.to_string()));
        }
        Ok(Self {
            native: storage(&device, "native 1024x512x16 VRAM", size),
            source: storage(&device, "ordered texture snapshot", size),
            scaled: storage(
                &device,
                "scaled integer framebuffer",
                size * u64::from(scale * scale),
            ),
            device,
            queue,
            info,
            scale,
            draw_pipeline,
            transfer_pipeline,
            scanout_pipeline,
            pending: Vec::new(),
            dirty: [false; TILES],
            dirty_bounds: [1024, 512, 0, 0],
        })
    }
    pub fn adapter_info(&self) -> &wgpu::AdapterInfo {
        &self.info
    }
    pub fn scale(&self) -> u32 {
        self.scale
    }
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
    /// Flush first. Buffer is row-major u32 BGR555, stride 1024*scale, bit15 mask.
    pub fn scaled_buffer(&self) -> &wgpu::Buffer {
        &self.scaled
    }
    pub fn native_buffer(&self) -> &wgpu::Buffer {
        &self.native
    }
    pub fn wait_idle(&self) {
        let _ = self.device.poll(wgpu::Maintain::Wait);
    }

    fn hazards(&self, p: Primitive) -> bool {
        if !p.textured() {
            return false;
        }
        let page = p.texture_bounds();
        let depth = (p.state.mode >> 7) & 3;
        let cx = ((p.clut & 63) * 16) as i32;
        let cy = ((p.clut >> 6) & 511) as i32;
        let clut = [cx, cy, cx + if depth == 0 { 16 } else { 256 }, cy + 1];
        let wrap_page = [page[0] - 1024, page[1], page[2] - 1024, page[3]];
        let wrap_clut = [clut[0] - 1024, clut[1], clut[2] - 1024, clut[3]];
        if !intersects(self.dirty_bounds, page)
            && !intersects(self.dirty_bounds, wrap_page)
            && (depth >= 2
                || !intersects(self.dirty_bounds, clut)
                    && !intersects(self.dirty_bounds, wrap_clut))
        {
            return false;
        }
        self.dirty.iter().enumerate().any(|(i, &dirty)| {
            if !dirty {
                return false;
            }
            let x = (i % COLS as usize) as i32 * TILE;
            let y = (i / COLS as usize) as i32 * TILE;
            let tile = [x, y, x + TILE, y + TILE];
            [page, [page[0] - 1024, page[1], page[2] - 1024, page[3]]]
                .iter()
                .any(|&r| intersects(tile, r))
                || depth < 2
                    && [clut, [clut[0] - 1024, clut[1], clut[2] - 1024, clut[3]]]
                        .iter()
                        .any(|&r| intersects(tile, r))
        })
    }

    fn transfer_command(&mut self, params: [u32; 12], pixels: &[u32]) -> Result<()> {
        self.flush()?;
        let config = data_buffer(
            &self.device,
            "transfer params",
            &params,
            wgpu::BufferUsages::UNIFORM,
        );
        let upload = data_buffer(
            &self.device,
            "transfer pixels",
            if pixels.is_empty() { &[0] } else { pixels },
            wgpu::BufferUsages::STORAGE,
        );
        let layout = self.transfer_pipeline.get_bind_group_layout(0);
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.native.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.scaled.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: config.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: upload.as_entire_binding(),
                },
            ],
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.transfer_pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(
                (params[4] * self.scale).div_ceil(8),
                (params[5] * self.scale).div_ceil(8),
                1,
            );
        }
        self.queue.submit([encoder.finish()]);
        Ok(())
    }

    fn read_buffer(&mut self, scaled: bool) -> Result<Vec<u16>> {
        self.flush()?;
        let buffer = if scaled { &self.scaled } else { &self.native };
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("VRAM readback"),
            size: buffer.size(),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, buffer.size());
        self.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        staging.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.wait_idle();
        rx.recv()
            .map_err(|e| Error(e.to_string()))?
            .map_err(|e| Error(e.to_string()))?;
        let mapped = staging.slice(..).get_mapped_range();
        let pixels = bytemuck::cast_slice::<u8, u32>(&mapped)
            .iter()
            .map(|&v| v as u16)
            .collect();
        drop(mapped);
        staging.unmap();
        Ok(pixels)
    }
    pub fn read_vram(&mut self) -> Result<Vec<u16>> {
        self.read_buffer(false)
    }
    pub fn read_scaled(&mut self) -> Result<Vec<u16>> {
        self.read_buffer(true)
    }

    pub fn create_scanout_texture(&self, display: crate::Display) -> Result<wgpu::Texture> {
        let [w, h] = display.dimensions();
        if w == 0 || h == 0 {
            return Err(Error("empty display range".into()));
        }
        Ok(self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("PS1 nearest scanout"),
            size: wgpu::Extent3d {
                width: w * self.scale,
                height: h * self.scale,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        }))
    }
    /// Reuse a texture from create_scanout_texture until GP1 changes dimensions.
    /// Core blits it to its surface with a nearest sampler; no CPU readback needed.
    pub fn render_scanout(
        &mut self,
        display: crate::Display,
        field: bool,
        target: &wgpu::Texture,
    ) -> Result<()> {
        let [w, h] = display.dimensions();
        if w == 0
            || h == 0
            || target.width() != w * self.scale
            || target.height() != h * self.scale
            || target.format() != wgpu::TextureFormat::Rgba8Unorm
            || !target
                .usage()
                .contains(wgpu::TextureUsages::STORAGE_BINDING)
        {
            return Err(Error(
                "scanout target dimensions/format/usage mismatch".into(),
            ));
        }
        self.flush()?;
        let config = data_buffer(
            &self.device,
            "scanout params",
            &[
                self.scale,
                u32::from(display.x),
                u32::from(display.y),
                display.mode,
                w,
                h,
                u32::from(display.disabled),
                u32::from(field),
                0,
                0,
                0,
                0,
            ],
            wgpu::BufferUsages::UNIFORM,
        );
        let view = target.create_view(&Default::default());
        let layout = self.scanout_pipeline.get_bind_group_layout(0);
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.native.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.scaled.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: config.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
            ],
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.scanout_pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(
                (w * self.scale).div_ceil(8),
                (h * self.scale).div_ceil(8),
                1,
            );
        }
        self.queue.submit([encoder.finish()]);
        Ok(())
    }
}

impl Renderer for WgpuRenderer {
    fn draw(&mut self, p: Primitive) -> Result<()> {
        let Some(b) = p.bounds() else {
            return Ok(());
        };
        if self.pending.len() >= 4096 || self.hazards(p) {
            self.flush()?;
        }
        self.dirty_bounds[0] = self.dirty_bounds[0].min(b[0]);
        self.dirty_bounds[1] = self.dirty_bounds[1].min(b[1]);
        self.dirty_bounds[2] = self.dirty_bounds[2].max(b[2]);
        self.dirty_bounds[3] = self.dirty_bounds[3].max(b[3]);
        for ty in b[1] / TILE..=(b[3] - 1) / TILE {
            for tx in b[0] / TILE..=(b[2] - 1) / TILE {
                self.dirty[(ty * COLS + tx) as usize] = true;
            }
        }
        self.pending.push(p);
        Ok(())
    }
    fn fill(&mut self, rect: Rect, color: u16) -> Result<()> {
        self.fill_field(rect, color, None)
    }
    fn fill_field(&mut self, rect: Rect, color: u16, skip_line: Option<bool>) -> Result<()> {
        validate_pixels(rect, rect.len())?;
        if rect.is_empty() {
            return Ok(());
        }
        self.transfer_command(
            [
                0,
                self.scale,
                u32::from(rect.x),
                u32::from(rect.y),
                u32::from(rect.width),
                u32::from(rect.height),
                0,
                0,
                0,
                u32::from(color & 0x7fff),
                u32::from(skip_line.is_some()) | (u32::from(skip_line == Some(true)) << 1),
                0,
            ],
            &[],
        )
    }
    fn upload(&mut self, rect: Rect, pixels: &[u16], mask: u32) -> Result<()> {
        validate_pixels(rect, pixels.len())?;
        if rect.is_empty() {
            return Ok(());
        }
        let data: Vec<u32> = pixels.iter().map(|&p| u32::from(p)).collect();
        self.transfer_command(
            [
                1,
                self.scale,
                u32::from(rect.x),
                u32::from(rect.y),
                u32::from(rect.width),
                u32::from(rect.height),
                0,
                0,
                mask,
                0,
                0,
                0,
            ],
            &data,
        )
    }
    fn copy(&mut self, src: Rect, x: u16, y: u16, mask: u32) -> Result<()> {
        validate_pixels(src, src.len())?;
        if src.is_empty() {
            return Ok(());
        }
        // Wrapped or overlapping copies use a deterministic forward CPU transfer.
        let source = [
            i32::from(src.x),
            i32::from(src.y),
            i32::from(src.x) + i32::from(src.width),
            i32::from(src.y) + i32::from(src.height),
        ];
        let dest = [
            i32::from(x),
            i32::from(y),
            i32::from(x) + i32::from(src.width),
            i32::from(y) + i32::from(src.height),
        ];
        if intersects(source, dest)
            || source[2] > 1024
            || source[3] > 512
            || dest[2] > 1024
            || dest[3] > 512
        {
            let mut vram = self.read_vram()?;
            let mut sharp = self.read_scaled()?;
            let scale = self.scale as usize;
            let dst = Rect::new(x, y, src.width, src.height);
            for row in 0..usize::from(src.height) {
                for start in (0..usize::from(src.width)).step_by(128) {
                    let count = (usize::from(src.width) - start).min(128);
                    let index = row * usize::from(src.width) + start;
                    let mut burst = [0u16; 128];
                    for (j, pixel) in burst.iter_mut().enumerate().take(count) {
                        *pixel = vram[src.index(index + j)];
                    }
                    for (j, &pixel) in burst.iter().enumerate().take(count) {
                        let n = dst.index(index + j);
                        if mask & 2 == 0 || vram[n] & 0x8000 == 0 {
                            vram[n] = pixel | if mask & 1 != 0 { 0x8000 } else { 0 };
                        }
                    }
                    // Snapshot every subpixel in the same native 128-word burst.
                    let sharp_index = |rect: Rect, j: usize, sx: usize, sy: usize| {
                        let n = rect.index(index + j);
                        (n / 1024 * scale + sy) * (1024 * scale) + (n % 1024 * scale + sx)
                    };
                    let mut sharp_burst = Vec::with_capacity(count * scale * scale);
                    for j in 0..count {
                        for sy in 0..scale {
                            for sx in 0..scale {
                                sharp_burst.push(sharp[sharp_index(src, j, sx, sy)]);
                            }
                        }
                    }
                    for j in 0..count {
                        for sy in 0..scale {
                            for sx in 0..scale {
                                let n = sharp_index(dst, j, sx, sy);
                                if mask & 2 == 0 || sharp[n] & 0x8000 == 0 {
                                    sharp[n] = sharp_burst[(j * scale + sy) * scale + sx]
                                        | if mask & 1 != 0 { 0x8000 } else { 0 };
                                }
                            }
                        }
                    }
                }
            }
            let native_data: Vec<u32> = vram.iter().map(|&p| u32::from(p)).collect();
            self.queue
                .write_buffer(&self.native, 0, bytemuck::cast_slice(&native_data));
            let mut data = Vec::with_capacity(src.len() * scale * scale);
            for yy in 0..usize::from(dst.height) * scale {
                for xx in 0..usize::from(dst.width) * scale {
                    let row = (usize::from(dst.y) * scale + yy) % (512 * scale);
                    let col = (usize::from(dst.x) * scale + xx) % (1024 * scale);
                    data.push(u32::from(sharp[row * (1024 * scale) + col]));
                }
            }
            return self.transfer_command(
                [
                    3,
                    self.scale,
                    u32::from(x),
                    u32::from(y),
                    u32::from(src.width),
                    u32::from(src.height),
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                ],
                &data,
            );
        }
        self.transfer_command(
            [
                2,
                self.scale,
                u32::from(x),
                u32::from(y),
                u32::from(src.width),
                u32::from(src.height),
                u32::from(src.x),
                u32::from(src.y),
                mask,
                0,
                0,
                0,
            ],
            &[],
        )
    }
    fn read(&mut self, rect: Rect) -> Result<Vec<u16>> {
        validate_pixels(rect, rect.len())?;
        let vram = self.read_vram()?;
        Ok((0..rect.len()).map(|i| vram[rect.index(i)]).collect())
    }
    fn flush(&mut self) -> Result<()> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let mut tiles: Vec<Vec<u32>> = (0..TILES).map(|_| Vec::new()).collect();
        let mut bounds = [1024, 512, 0, 0];
        let mut draws = Vec::with_capacity(self.pending.len());
        for (index, p) in self.pending.iter().copied().enumerate() {
            let b = p.bounds().expect("visible queued primitive");
            bounds[0] = bounds[0].min(b[0]);
            bounds[1] = bounds[1].min(b[1]);
            bounds[2] = bounds[2].max(b[2]);
            bounds[3] = bounds[3].max(b[3]);
            for ty in b[1] / TILE..=(b[3] - 1) / TILE {
                for tx in b[0] / TILE..=(b[2] - 1) / TILE {
                    tiles[(ty * COLS + tx) as usize].push(index as u32);
                }
            }
            draws.push(Draw::from(p));
        }
        let mut headers = Vec::<[u32; 2]>::with_capacity(TILES);
        let mut indices = Vec::<u32>::new();
        for tile in tiles {
            headers.push([indices.len() as u32, tile.len() as u32]);
            indices.extend(tile);
        }
        let draws = data_buffer(
            &self.device,
            "primitives",
            &draws,
            wgpu::BufferUsages::STORAGE,
        );
        let headers = data_buffer(
            &self.device,
            "tile offsets",
            &headers,
            wgpu::BufferUsages::STORAGE,
        );
        let indices = data_buffer(
            &self.device,
            "tile indices",
            &indices,
            wgpu::BufferUsages::STORAGE,
        );
        let config = data_buffer(
            &self.device,
            "draw config",
            &[
                self.scale,
                bounds[0] as u32 * self.scale,
                bounds[1] as u32 * self.scale,
                (bounds[2] - bounds[0]) as u32 * self.scale,
                (bounds[3] - bounds[1]) as u32 * self.scale,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            wgpu::BufferUsages::UNIFORM,
        );
        let layout = self.draw_pipeline.get_bind_group_layout(0);
        let buffers = [
            &self.native,
            &self.source,
            &self.scaled,
            &draws,
            &headers,
            &indices,
            &config,
        ];
        let entries: Vec<_> = buffers
            .iter()
            .enumerate()
            .map(|(i, b)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &entries,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        // PORT: same uncached primitive snapshot contract as the software reference.
        encoder.copy_buffer_to_buffer(&self.native, 0, &self.source, 0, (VRAM_WORDS as u64) * 4);
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.draw_pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(
                ((bounds[2] - bounds[0]) as u32 * self.scale).div_ceil(8),
                ((bounds[3] - bounds[1]) as u32 * self.scale).div_ceil(8),
                1,
            );
        }
        self.queue.submit([encoder.finish()]);
        self.pending.clear();
        self.dirty.fill(false);
        self.dirty_bounds = [1024, 512, 0, 0];
        Ok(())
    }
}
