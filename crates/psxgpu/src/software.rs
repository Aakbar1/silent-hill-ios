// SPDX-License-Identifier: GPL-3.0-only
use crate::{raster, validate_pixels, Primitive, Rect, Renderer, Result, VRAM_WORDS};

/// Unfiltered 1x reference path; VRAM is always authoritative and inspectable.
pub struct SoftwareRenderer {
    vram: Vec<u16>,
}
impl Default for SoftwareRenderer {
    fn default() -> Self {
        Self::new()
    }
}
impl SoftwareRenderer {
    pub fn new() -> Self {
        Self {
            vram: vec![0; VRAM_WORDS],
        }
    }
    pub fn vram(&self) -> &[u16] {
        &self.vram
    }
    fn write(&mut self, index: usize, value: u16, mask: u32) {
        if mask & 2 == 0 || self.vram[index] & 0x8000 == 0 {
            self.vram[index] = value | if mask & 1 != 0 { 0x8000 } else { 0 };
        }
    }
}
impl Renderer for SoftwareRenderer {
    fn draw(&mut self, p: Primitive) -> Result<()> {
        if let Some([x0, y0, x1, y1]) = p.bounds() {
            // PORT: primitive snapshots make feedback deterministic; hardware cache residency is not simulated.
            let source = if p.textured() {
                Some(self.vram.clone())
            } else {
                None
            };
            let coefficients = raster::planes(p);
            for y in y0..y1 {
                for x in x0..x1 {
                    if let Some((color, uv)) = raster::fragment(p, &coefficients, x, y) {
                        let index = (y * 1024 + x) as usize;
                        let texel = source.as_ref().map_or(0, |v| raster::texture(v, p, uv));
                        if let Some(value) = raster::shade(p, self.vram[index], color, texel, x, y)
                        {
                            self.vram[index] = value;
                        }
                    }
                }
            }
        }
        Ok(())
    }
    fn fill(&mut self, rect: Rect, color: u16) -> Result<()> {
        validate_pixels(rect, rect.len())?;
        for i in 0..rect.len() {
            self.vram[rect.index(i)] = color & 0x7fff;
        }
        Ok(())
    }
    fn upload(&mut self, rect: Rect, pixels: &[u16], mask: u32) -> Result<()> {
        validate_pixels(rect, pixels.len())?;
        for (i, &value) in pixels.iter().enumerate() {
            self.write(rect.index(i), value, mask);
        }
        Ok(())
    }
    fn copy(&mut self, src: Rect, x: u16, y: u16, mask: u32) -> Result<()> {
        validate_pixels(src, src.len())?;
        let dst = Rect::new(x, y, src.width, src.height);
        // Read a 128-halfword burst before writing it, then advance forward.
        // Mirrors the established Mednafen transfer model (see NOTICE.md).
        for row in 0..usize::from(src.height) {
            for start in (0..usize::from(src.width)).step_by(128) {
                let count = (usize::from(src.width) - start).min(128);
                let index = row * usize::from(src.width) + start;
                let mut burst = [0u16; 128];
                for (j, pixel) in burst.iter_mut().enumerate().take(count) {
                    *pixel = self.vram[src.index(index + j)];
                }
                for (j, &pixel) in burst.iter().enumerate().take(count) {
                    self.write(dst.index(index + j), pixel, mask);
                }
            }
        }
        Ok(())
    }
    fn read(&mut self, rect: Rect) -> Result<Vec<u16>> {
        validate_pixels(rect, rect.len())?;
        Ok((0..rect.len()).map(|i| self.vram[rect.index(i)]).collect())
    }
    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
