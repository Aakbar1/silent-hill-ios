// SPDX-License-Identifier: GPL-3.0-only
//! Host Raster API backed by the PS1 software reference.
use psxgpu::{Processor, Rect, Renderer, SoftwareRenderer};

pub struct Raster {
    gpu: Processor<SoftwareRenderer>,
    pub primitives: u64,
}
impl Default for Raster {
    fn default() -> Self {
        let mut gpu = Processor::new(SoftwareRenderer::new());
        gpu.draw_state.area = [0, 0, 1023, 511];
        Self { gpu, primitives: 0 }
    }
}
impl Raster {
    pub fn env(&mut self, [x, y, w, h]: [i32; 4], offset: [i32; 2]) {
        self.gpu.draw_state.area = [
            x.max(0),
            y.max(0),
            x.saturating_add(w).min(1024) - 1,
            y.saturating_add(h).min(512) - 1,
        ];
        self.gpu.draw_state.offset = offset;
    }
    pub fn packet(&mut self, words: &[u32]) {
        let Some(payload) = words.get(1..).filter(|p| !p.is_empty()) else {
            return;
        };
        assert!(self.gpu.idle(), "incomplete previous host GP0 packet");
        self.gpu
            .gp0_words(payload)
            .expect("invalid host GP0 packet");
        assert!(self.gpu.idle(), "incomplete host GP0 packet");
        if matches!(payload[0] >> 24, 0x20..=0x7f) {
            self.primitives += 1;
        }
    }
    pub fn load(&mut self, x: i32, y: i32, w: i32, h: i32, data: &[u16]) {
        if w <= 0 || h <= 0 || w > 1024 || h > 512 {
            return;
        }
        self.gpu
            .renderer_mut()
            .upload(
                Rect::new((x & 1023) as u16, (y & 511) as u16, w as u16, h as u16),
                data,
                0,
            )
            .expect("invalid host VRAM upload");
    }
    pub fn read(&self, [x, y, w, h]: [i32; 4]) -> Vec<u16> {
        if w <= 0 || h <= 0 || w > 1024 || h > 512 {
            return Vec::new();
        }
        (0..h)
            .flat_map(|row| {
                (0..w).map(move |col| {
                    self.gpu.renderer().vram()
                        [(((y + row) & 511) * 1024 + ((x + col) & 1023)) as usize]
                })
            })
            .collect()
    }
    pub fn clear(&mut self, [x, y, w, h]: [i32; 4], color: [u8; 3]) {
        let left = x.max(0);
        let top = y.max(0);
        let right = x.saturating_add(w).min(1024);
        let bottom = y.saturating_add(h).min(512);
        if right <= left || bottom <= top {
            return;
        }
        let [r, g, b] = color.map(|c| u16::from(c >> 3));
        self.gpu
            .renderer_mut()
            .fill(
                Rect::new(
                    left as u16,
                    top as u16,
                    (right - left) as u16,
                    (bottom - top) as u16,
                ),
                r | (g << 5) | (b << 10),
            )
            .expect("invalid host clear");
    }
    pub fn frame(&self, x: i32, y: i32, w: u32, h: u32) -> Vec<u32> {
        self.read([x, y, w as i32, h as i32])
            .into_iter()
            .map(|pixel| {
                let [r, g, b] = psxgpu::rgb888(pixel).map(u32::from);
                (r << 16) | (g << 8) | b
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn translucent_quad_has_no_double_blended_diagonal_and_clips() {
        let mut raster = Raster::default();
        raster.env([1, 0, 2, 4], [0, 0]);
        raster.packet(&[0, 0xe1000020]); // additive
        raster.packet(&[
            0,
            0x3a080808,
            0,
            0x00080808,
            4,
            0x00080808,
            4 << 16,
            0x00080808,
            (4 << 16) | 4,
        ]);
        for y in 0..4 {
            assert_eq!(raster.read([0, y, 4, 1]), [0, 0x421, 0x421, 0]);
        }
        assert_eq!(raster.primitives, 1);
    }
    #[test]
    fn fixed_sprite_size_reads_no_missing_wh_word() {
        let mut raster = Raster::default();
        raster.packet(&[0, 0x70101010, 0]);
        assert_eq!(raster.read([7, 7, 2, 2]), [0x842, 0, 0, 0]);
    }
    #[test]
    fn sprite_uses_clut_and_skips_transparent_texels() {
        let mut raster = Raster::default();
        raster.load(768, 0, 1, 1, &[0x0010]);
        raster.load(0, 480, 2, 1, &[0, 31]);
        raster.packet(&[0, 0xe100000c]);
        raster.packet(&[0, 0x64808080, 0, (480 << 22), 0x00010004]);
        assert_eq!(raster.frame(0, 0, 4, 1), [0, 0xff0000, 0, 0]);
    }
    #[test]
    fn subtractive_tile_respects_offset_and_clip() {
        let mut raster = Raster::default();
        raster.clear([0, 0, 4, 1], [255, 255, 255]);
        raster.env([0, 0, 4, 1], [1, 0]);
        raster.packet(&[0, 0xe1000040]);
        raster.packet(&[0, 0x62ffffff, 0, 0x00010001]);
        assert_eq!(raster.frame(0, 0, 4, 1), [0xffffff, 0, 0xffffff, 0xffffff]);
    }
}
