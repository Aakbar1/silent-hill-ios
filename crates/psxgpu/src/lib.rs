// SPDX-License-Identifier: GPL-3.0-only
//! Standalone, ordered PS1 GPU packet processing. See INTEGRATION.md.
#[cfg(feature = "wgpu")]
mod accelerated;
pub mod capture;
pub mod gte;
mod packet;
mod raster;
mod software;

#[cfg(feature = "wgpu")]
pub use accelerated::WgpuRenderer;
pub use packet::{Display, DrawState, Processor};
pub use raster::{Primitive, Vertex};
pub use software::SoftwareRenderer;

pub const VRAM_WIDTH: usize = 1024;
pub const VRAM_HEIGHT: usize = 512;
pub const VRAM_WORDS: usize = VRAM_WIDTH * VRAM_HEIGHT;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self(e.to_string())
    }
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}
impl Rect {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    pub fn len(self) -> usize {
        usize::from(self.width) * usize::from(self.height)
    }
    pub fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }
    pub(crate) fn index(self, i: usize) -> usize {
        (((usize::from(self.y) + i / usize::from(self.width)) & 511) * 1024)
            + ((usize::from(self.x) + i % usize::from(self.width)) & 1023)
    }
}

/// Target implemented independently of the host's GpuBackend trait.
/// Calls are ordered; reads and transfers observe all preceding primitives.
pub trait Renderer {
    fn draw(&mut self, primitive: Primitive) -> Result<()>;
    fn fill(&mut self, rect: Rect, color: u16) -> Result<()>;
    fn fill_field(&mut self, rect: Rect, color: u16, skip_line: Option<bool>) -> Result<()> {
        if let Some(parity) = skip_line {
            for row in 0..rect.height {
                let y = rect.y.wrapping_add(row) & 511;
                if (y & 1 != 0) != parity {
                    self.fill(Rect::new(rect.x, y, rect.width, 1), color)?;
                }
            }
            Ok(())
        } else {
            self.fill(rect, color)
        }
    }
    fn upload(&mut self, rect: Rect, pixels: &[u16], mask: u32) -> Result<()>;
    fn copy(&mut self, src: Rect, x: u16, y: u16, mask: u32) -> Result<()>;
    fn read(&mut self, rect: Rect) -> Result<Vec<u16>>;
    fn flush(&mut self) -> Result<()>;
}

pub(crate) fn validate_pixels(rect: Rect, len: usize) -> Result<()> {
    if rect.len() != len || rect.width > 1024 || rect.height > 512 {
        return Err(Error("invalid VRAM rectangle/pixel count".into()));
    }
    Ok(())
}

pub fn rgb888(pixel: u16) -> [u8; 3] {
    [pixel & 31, (pixel >> 5) & 31, (pixel >> 10) & 31].map(|v| ((v << 3) | (v >> 2)) as u8)
}

/// Decode display pixels, including packed 24-bit MDEC output. No filtering.
pub fn scanout(vram: &[u16], display: Display, field: bool) -> Result<Vec<u8>> {
    if vram.len() != VRAM_WORDS {
        return Err(Error("scanout needs full VRAM".into()));
    }
    let [width, height] = display.dimensions();
    let mut rgba = vec![0u8; width as usize * height as usize * 4];
    for y in 0..height as usize {
        for x in 0..width as usize {
            let sx = if display.mode & 128 != 0 {
                width as usize - 1 - x
            } else {
                x
            };
            let row = (usize::from(display.y) + y) & 511;
            let color = if display.disabled {
                [0; 3]
            } else if display.mode & 16 != 0 {
                let byte = usize::from(display.x) * 2 + sx * 3;
                std::array::from_fn(|c| {
                    let n = (byte + c) & 2047;
                    (vram[row * 1024 + n / 2] >> ((n & 1) * 8)) as u8
                })
            } else {
                rgb888(vram[row * 1024 + ((usize::from(display.x) + sx) & 1023)])
            };
            // A field scanout leaves the opposite field black; host may weave it.
            let color = if display.mode & 36 == 36 && (y & 1 != usize::from(field)) {
                [0; 3]
            } else {
                color
            };
            let i = (y * width as usize + x) * 4;
            rgba[i..i + 3].copy_from_slice(&color);
            rgba[i + 3] = 255;
        }
    }
    Ok(rgba)
}
