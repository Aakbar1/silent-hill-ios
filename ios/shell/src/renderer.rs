// SPDX-License-Identifier: GPL-3.0-only
use crate::geometry;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;

pub struct Renderer {
    pub window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pattern: wgpu::RenderPipeline,
    circles: wgpu::RenderPipeline,
    frame: wgpu::Buffer,
    bind: wgpu::BindGroup,
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> Self {
        #[cfg(target_os = "ios")]
        let backends = wgpu::Backends::METAL;
        #[cfg(target_os = "windows")]
        let backends = wgpu::Backends::DX12;
        #[cfg(not(any(target_os = "ios", target_os = "windows")))]
        let backends = wgpu::Backends::PRIMARY;
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });
        let surface = instance
            .create_surface(window.clone())
            .expect("GPU surface");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .expect("GPU adapter");
        log::info!("GPU {:?}", adapter.get_info());
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .expect("GPU device");
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .expect("surface configuration");
        config.present_mode = wgpu::PresentMode::Fifo;
        surface.configure(&device, &config);
        let frame = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("frame"),
            contents: bytemuck::cast_slice(&[0f32; 8]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: frame.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shell test pattern"),
            source: wgpu::ShaderSource::Wgsl(include_str!("pattern.wgsl").into()),
        });
        let make_pipeline = |vertex, fragment, buffers| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(vertex),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(vertex),
                    compilation_options: Default::default(),
                    buffers,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fragment),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview: None,
                cache: None,
            })
        };
        let pattern = make_pipeline("background", "pattern", &[]);
        let circles = make_pipeline(
            "circle",
            "touch_color",
            &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<geometry::Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2],
            }],
        );
        Self {
            window,
            surface,
            device,
            queue,
            config,
            pattern,
            circles,
            frame,
            bind,
        }
    }

    pub fn resize(&mut self) {
        let size = self.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
        log::info!(
            "resize {}x{} scale={}",
            size.width,
            size.height,
            self.window.scale_factor()
        );
        self.window.request_redraw();
    }

    pub fn draw(&mut self, points: &[[f32; 2]]) -> bool {
        let size = self.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return false;
        }
        let output = match self.surface.get_current_texture() {
            Ok(output) => output,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.resize();
                return false;
            }
            Err(wgpu::SurfaceError::Timeout) => {
                self.window.request_redraw();
                return false;
            }
            Err(error) => panic!("surface: {error}"),
        };
        let safe = self.safe_insets();
        self.queue.write_buffer(
            &self.frame,
            0,
            bytemuck::cast_slice(&[
                size.width as f32,
                size.height as f32,
                0.,
                0.,
                safe[0],
                safe[1],
                safe[2],
                safe[3],
            ]),
        );
        let vertices = geometry::circles(
            points,
            [size.width as f32, size.height as f32],
            24.0 * self.window.scale_factor() as f32,
        );
        let buffer = (!vertices.is_empty()).then(|| {
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("touch circles"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                })
        });
        let view = output.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
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
            pass.set_bind_group(0, &self.bind, &[]);
            pass.set_pipeline(&self.pattern);
            pass.draw(0..3, 0..1);
            if let Some(buffer) = &buffer {
                pass.set_pipeline(&self.circles);
                pass.set_vertex_buffer(0, buffer.slice(..));
                pass.draw(0..vertices.len() as u32, 0..1);
            }
        }
        self.queue.submit(Some(encoder.finish()));
        self.window.pre_present_notify();
        output.present();
        self.window.request_redraw();
        true
    }

    fn safe_insets(&self) -> [f32; 4] {
        #[cfg(target_os = "ios")]
        {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            let RawWindowHandle::UiKit(handle) =
                self.window.window_handle().expect("UIKit handle").as_raw()
            else {
                panic!("not UIKit")
            };
            // SAFETY: winit owns this UIView and draw runs on the UIKit main thread.
            let insets = unsafe {
                handle
                    .ui_view
                    .cast::<objc2_ui_kit::UIView>()
                    .as_ref()
                    .safeAreaInsets()
            };
            let scale = self.window.scale_factor() as f32;
            [
                insets.left as f32 * scale,
                insets.top as f32 * scale,
                insets.right as f32 * scale,
                insets.bottom as f32 * scale,
            ]
        }
        #[cfg(not(target_os = "ios"))]
        {
            [0.; 4]
        }
    }
}
