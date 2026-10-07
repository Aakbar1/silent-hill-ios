// SPDX-License-Identifier: GPL-3.0-only
mod geometry;
mod logging;
mod renderer;

use std::{collections::BTreeMap, sync::Arc};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, TouchPhase, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Fullscreen, Window, WindowId},
};

#[derive(Default)]
struct App {
    renderer: Option<renderer::Renderer>,
    touches: BTreeMap<u64, [f32; 2]>,
    cursor: [f32; 2],
    mouse_down: bool,
    frames: u32,
    smoke_frames: Option<u32>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_some() {
            log::info!("resumed");
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Silent Hill Port — shell test")
            .with_fullscreen(Some(Fullscreen::Borderless(None)));
        #[cfg(target_os = "ios")]
        let attrs = {
            use winit::platform::ios::{ScreenEdge, ValidOrientations, WindowAttributesExtIOS};
            attrs
                .with_valid_orientations(ValidOrientations::Landscape)
                .with_prefers_status_bar_hidden(true)
                .with_prefers_home_indicator_hidden(true)
                .with_preferred_screen_edges_deferring_system_gestures(ScreenEdge::ALL)
        };
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));
        self.renderer = Some(pollster::block_on(renderer::Renderer::new(window)));
        log::info!("window ready");
    }

    fn suspended(&mut self, _: &ActiveEventLoop) {
        self.touches.clear();
        self.mouse_down = false;
        log::info!("suspended; touches cleared");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => renderer.resize(),
            WindowEvent::Touch(touch) => {
                match touch.phase {
                    TouchPhase::Started | TouchPhase::Moved => {
                        self.touches
                            .insert(touch.id, [touch.location.x as f32, touch.location.y as f32]);
                    }
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        self.touches.remove(&touch.id);
                    }
                }
                // Do not log positions/identifiers: the log is a diagnostics file.
                if touch.phase != TouchPhase::Moved {
                    log::info!("touch {:?}; active={}", touch.phase, self.touches.len());
                }
                renderer.window.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = [position.x as f32, position.y as f32];
                renderer.window.request_redraw();
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.mouse_down = state == ElementState::Pressed;
                renderer.window.request_redraw();
            }
            WindowEvent::Focused(false) => {
                self.touches.clear();
                self.mouse_down = false;
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.logical_key
                    == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape)
                    && event.state == ElementState::Pressed =>
            {
                event_loop.exit()
            }
            WindowEvent::RedrawRequested => {
                let mut points: Vec<_> = self.touches.values().copied().collect();
                if self.mouse_down {
                    points.push(self.cursor);
                }
                if renderer.draw(&points) {
                    self.frames += 1;
                    if self.frames == 1 {
                        log::info!("first frame presented");
                    }
                    if self.smoke_frames.is_some_and(|n| self.frames >= n) {
                        log::info!("smoke complete; frames={}", self.frames);
                        event_loop.exit();
                    }
                }
            }
            _ => {}
        }
    }
}

fn main() {
    logging::init().expect("open Documents/shell.log");
    std::panic::set_hook(Box::new(|info| log::error!("panic: {info}")));
    log::info!(
        "shell {} startup on {}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS
    );
    let smoke_frames = std::env::var("SHELL_SMOKE_FRAMES")
        .ok()
        .map(|n| n.parse().expect("positive frame count"));
    let mut app = App {
        smoke_frames,
        ..App::default()
    };
    EventLoop::new()
        .expect("event loop")
        .run_app(&mut app)
        .expect("run app");
    log::info!("clean exit");
}
