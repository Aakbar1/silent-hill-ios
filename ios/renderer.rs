// SPDX-License-Identifier: GPL-3.0-only
//! Real shared wgpu backend, measured phone policy and lossless Raster fallback.
use crate::{
    backend::GpuBackend,
    gpu_wgpu::{self, WgpuGpu},
    raster::Raster,
};
use std::{cell::RefCell, time::Instant};

pub const MAX_SCALE: u32 = 3;
pub const BUDGET_MS: f64 = 12.0;
const BACKEND_NAME: &str = if cfg!(target_vendor = "apple") {
    "Metal"
} else if cfg!(target_os = "windows") {
    "D3D12"
} else {
    "Vulkan"
};

pub fn requested_scale(width: f32, height: f32, pixel_ratio: f32) -> u32 {
    if !width.is_finite()
        || !height.is_finite()
        || !pixel_ratio.is_finite()
        || width <= 0.0
        || height <= 0.0
        || pixel_ratio <= 0.0
    {
        return 1;
    }
    ((width / 320.0).min(height / 224.0) * pixel_ratio)
        .ceil()
        .clamp(1.0, MAX_SCALE as f32) as u32
}

pub struct PhoneGpu {
    metal: RefCell<Option<WgpuGpu>>,
    mirror: Raster,
    pub scale: u32,
    strict: bool,
}
impl PhoneGpu {
    pub fn new(requested: u32, strict: bool) -> Result<Self, String> {
        for scale in (1..=requested.clamp(1, MAX_SCALE)).rev() {
            let attempt = (|| -> Result<(WgpuGpu, f64), String> {
                let mut gpu = WgpuGpu::new(scale)?;
                let mut worst = 0.0f64;
                let mut total = 0.0;
                for index in 0..5 {
                    let start = Instant::now();
                    synthetic_scene(&mut gpu);
                    let native = gpu.frame(0, 0, 320, 224);
                    let (_, _, display) = gpu_wgpu::screenshot(320, 224, &native)?;
                    if let Some(error) = gpu.error() {
                        return Err(error);
                    }
                    if display.iter().all(|p| *p == 0) {
                        return Err("blank GPU startup probe".into());
                    }
                    if index != 0 {
                        let ms = start.elapsed().as_secs_f64() * 1000.0;
                        total += ms;
                        worst = worst.max(ms);
                    }
                }
                println!(
                    "IOS_GPU probe scale={scale} samples=4 mean_ms={:.3} max_ms={worst:.3} scaled_vram_mib={} budget_ms={BUDGET_MS}",
                    total / 4.0,
                    2 * scale * scale
                );
                Ok((gpu, worst))
            })();
            let (mut gpu, worst) = match attempt {
                Ok(pair) => pair,
                Err(error) => {
                    if strict {
                        return Err(error);
                    }
                    eprintln!("IOS_GPU fallback=Raster init_error={error}");
                    gpu_wgpu::reset_for_raster_fallback();
                    return Ok(Self {
                        metal: RefCell::new(None),
                        mirror: Raster::default(),
                        scale: 1,
                        strict,
                    });
                }
            };
            if worst <= BUDGET_MS || scale == 1 {
                // No synthetic pixels survive into the game worker.
                gpu.clear([0, 0, 1024, 512], [0, 0, 0]);
                gpu_wgpu::frame_texture(false);
                println!(
                    "IOS_GPU selected=wgpu backend={BACKEND_NAME} scale={scale} stats=false wide=false"
                );
                return Ok(Self {
                    metal: RefCell::new(Some(gpu)),
                    mirror: Raster::default(),
                    scale,
                    strict,
                });
            }
        }
        unreachable!()
    }
    pub fn accelerated(&self) -> bool {
        self.metal.borrow().is_some()
    }
    fn fallback(&self, error: &str) {
        if !self.strict {
            eprintln!("IOS_GPU fallback=Raster runtime_error={error}");
            self.metal.borrow_mut().take();
            gpu_wgpu::reset_for_raster_fallback();
        }
    }
    fn check_failure(&self) {
        let error = self.metal.borrow().as_ref().and_then(WgpuGpu::error);
        if let Some(error) = error {
            self.fallback(&error);
        }
    }
    pub fn display(&self, w: u32, h: u32, native: &[u32]) -> Result<(u32, u32, Vec<u32>), String> {
        if self.accelerated() {
            match gpu_wgpu::frame_texture(true)
                .ok_or_else(|| "Metal frame missing".to_owned())
                .and_then(|f| f.read_pixels())
            {
                Ok(frame) => return Ok(frame),
                Err(error) if self.strict => return Err(error),
                Err(error) => self.fallback(&error),
            }
        }
        Ok((w, h, native.to_vec()))
    }
    fn scanout(&self, x: i32, y: i32, w: u32, h: u32, rgb24: bool) -> Vec<u32> {
        let result = self.metal.borrow().as_ref().map(|gpu| {
            let frame = if rgb24 {
                gpu.frame_rgb24(x, y, w, h)
            } else {
                gpu.frame(x, y, w, h)
            };
            (frame, gpu.error())
        });
        if let Some((pixels, error)) = result {
            if let Some(error) = error {
                self.fallback(&error);
            } else {
                return pixels;
            }
        }
        if rgb24 {
            self.mirror.frame_rgb24(x, y, w, h)
        } else {
            self.mirror.frame(x, y, w, h)
        }
    }
}
impl GpuBackend for PhoneGpu {
    fn begin_ordering_table(&mut self) {
        self.mirror.begin_ordering_table();
        if let Some(gpu) = self.metal.get_mut() {
            gpu.begin_ordering_table();
        }
        self.check_failure();
    }
    fn packet(&mut self, words: &[u32]) {
        self.mirror.packet(words);
        if let Some(gpu) = self.metal.get_mut() {
            gpu.packet(words);
        }
        self.check_failure();
    }
    fn packet_precise(&mut self, words: &[u32], positions: &[[f32; 2]]) {
        self.mirror.packet(words);
        if let Some(gpu) = self.metal.get_mut() {
            gpu.packet_precise(words, positions);
        }
        self.check_failure();
    }
    fn end_ordering_table(&mut self) {
        self.mirror.end_ordering_table();
        if let Some(gpu) = self.metal.get_mut() {
            gpu.end_ordering_table();
        }
        self.check_failure();
    }
    fn env(&mut self, clip: [i32; 4], offset: [i32; 2]) {
        self.mirror.env(clip, offset);
        if let Some(gpu) = self.metal.get_mut() {
            gpu.env(clip, offset);
        }
        self.check_failure();
    }
    fn load(&mut self, [x, y, w, h]: [i32; 4], pixels: &[u16]) {
        self.mirror.load(x, y, w, h, pixels);
        if let Some(gpu) = self.metal.get_mut() {
            gpu.load([x, y, w, h], pixels);
        }
        self.check_failure();
    }
    fn read(&self, rect: [i32; 4]) -> Vec<u16> {
        let result = self
            .metal
            .borrow()
            .as_ref()
            .map(|gpu| (gpu.read(rect), gpu.error()));
        if let Some((pixels, error)) = result {
            if let Some(error) = error {
                self.fallback(&error);
            } else {
                return pixels;
            }
        }
        self.mirror.read(rect)
    }
    fn clear(&mut self, rect: [i32; 4], color: [u8; 3]) {
        self.mirror.clear(rect, color);
        if let Some(gpu) = self.metal.get_mut() {
            gpu.clear(rect, color);
        }
        self.check_failure();
    }
    fn frame(&self, x: i32, y: i32, w: u32, h: u32) -> Vec<u32> {
        self.scanout(x, y, w, h, false)
    }
    fn frame_rgb24(&self, x: i32, y: i32, w: u32, h: u32) -> Vec<u32> {
        self.scanout(x, y, w, h, true)
    }
    fn primitives(&self) -> u64 {
        self.mirror.primitives
    }
}

/// Original test geometry only; no game data, assets, shaders or disc access.
pub fn synthetic_scene(gpu: &mut dyn GpuBackend) {
    gpu.env([0, 0, 320, 224], [0, 0]);
    gpu.clear([0, 0, 320, 224], [24, 40, 64]);
    gpu.begin_ordering_table();
    gpu.packet(&[
        0x04000000,
        0x200020f0,
        32 | (32 << 16),
        280 | (40 << 16),
        100 | (200 << 16),
    ]);
    gpu.packet(&[
        0x04000000,
        0x20f0c020,
        180 | (60 << 16),
        300 | (190 << 16),
        140 | (190 << 16),
    ]);
    gpu.end_ordering_table();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn phone_scale_caps_full_vram_and_uses_live_pixel_density() {
        assert_eq!(requested_scale(600.0, 420.0, 3.0), 3);
        assert_eq!(requested_scale(320.0, 224.0, 1.0), 1);
        assert_eq!(requested_scale(320.0, 224.0, 2.0), 2);
        assert_eq!(requested_scale(f32::NAN, 224.0, 3.0), 1);
        assert_eq!(requested_scale(0.0, 224.0, 3.0), 1);
    }
    #[test]
    fn runtime_fallback_preserves_vram_and_packet_rendering() {
        let mut gpu = PhoneGpu::new(3, false).unwrap();
        assert!(gpu.accelerated());
        synthetic_scene(&mut gpu);
        let native = gpu.frame(0, 0, 320, 224);
        let display = gpu.display(320, 224, &native).unwrap();
        assert_eq!((display.0, display.1), (320 * gpu.scale, 224 * gpu.scale));
        gpu.load([900, 400, 2, 1], &[0x1234, 0x4321]);
        gpu.fallback("injected device loss");
        assert_eq!(gpu.read([900, 400, 2, 1]), [0x1234, 0x4321]);
        let pixels = gpu.frame(0, 0, 320, 224);
        assert!(pixels.iter().any(|p| *p & 255 > 128));
        assert!(pixels.iter().any(|p| *p >> 16 > 128));
        assert_eq!(gpu.display(320, 224, &pixels).unwrap().2, pixels);
    }
}
