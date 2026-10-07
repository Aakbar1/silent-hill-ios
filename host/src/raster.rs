// SPDX-License-Identifier: GPL-3.0-only
//! Rasterizes the GPU packets used by the real boot code. This is not a CPU emulator.
pub struct Raster {
    vram: Vec<u16>,
    page: u32,
    clip: [i32; 4],
    offset: [i32; 2],
    pub primitives: u64,
    unknown: [bool; 256],
}

fn rgb(color: u16) -> [u16; 3] {
    [color & 31, (color >> 5) & 31, (color >> 10) & 31]
}
fn packed(color: [u16; 3]) -> u16 {
    color[0] | (color[1] << 5) | (color[2] << 10)
}

impl Default for Raster {
    fn default() -> Self {
        Self {
            vram: vec![0; 1024 * 512],
            page: 0,
            clip: [0, 0, 1024, 512],
            offset: [0, 0],
            primitives: 0,
            unknown: [false; 256],
        }
    }
}

impl Raster {
    pub fn env(&mut self, clip: [i32; 4], offset: [i32; 2]) {
        self.clip = clip;
        self.offset = offset;
    }
    pub fn load(&mut self, x: i32, y: i32, w: i32, h: i32, data: &[u16]) {
        for row in 0..h {
            for col in 0..w {
                if (0..1024).contains(&(x + col)) && (0..512).contains(&(y + row)) {
                    self.vram[((y + row) * 1024 + x + col) as usize] =
                        data[(row * w + col) as usize];
                }
            }
        }
    }
    pub fn clear(&mut self, rect: [i32; 4], color: [u8; 3]) {
        let pixel = packed(color.map(|c| u16::from(c >> 3)));
        for y in rect[1].max(0)..(rect[1] + rect[3]).min(512) {
            for x in rect[0].max(0)..(rect[0] + rect[2]).min(1024) {
                self.vram[(y * 1024 + x) as usize] = pixel;
            }
        }
    }
    pub fn packet(&mut self, words: &[u32]) {
        if words.len() < 2 {
            return;
        }
        let code = (words[1] >> 24) as u8;
        if code == 0xe1 {
            self.page = words[1] & 0xffff;
            return;
        }
        if code == 0xe2 && words[1] & 0xffffff == 0 {
            return;
        } // Disabled texture window.
        if code & 0xe0 != 0x60 {
            // PORT: Unsupported GPU commands are boot-only stubs, logged once per opcode.
            if !self.unknown[usize::from(code)] {
                eprintln!("STUB GPU opcode {code:#04x}");
                self.unknown[usize::from(code)] = true;
            }
            return;
        }
        let textured = code & 4 != 0;
        if words.len() < if textured { 5 } else { 4 } {
            return;
        }
        self.primitives += 1;
        let x = i32::from(words[2] as i16) + self.offset[0];
        let y = i32::from((words[2] >> 16) as i16) + self.offset[1];
        let wh = words[if textured { 4 } else { 3 }];
        let w = (wh & 0xffff) as i32;
        let h = (wh >> 16) as i32;
        let color = [
            (words[1] & 255) as u16,
            ((words[1] >> 8) & 255) as u16,
            ((words[1] >> 16) & 255) as u16,
        ];
        for py in y.max(self.clip[1]).max(0)..(y + h).min(self.clip[1] + self.clip[3]).min(512) {
            for px in x.max(self.clip[0]).max(0)..(x + w).min(self.clip[0] + self.clip[2]).min(1024)
            {
                let (mut source, stp) = if textured {
                    let uv = words[3];
                    let u = ((uv & 255) as i32 + px - x) & 255;
                    let v = (((uv >> 8) & 255) as i32 + py - y) & 255;
                    let tx = ((self.page & 15) * 64) as i32;
                    let ty = ((self.page & 16) * 16) as i32;
                    let mode = (self.page >> 7) & 3;
                    let shift = match mode {
                        0 => 2,
                        1 => 1,
                        _ => 0,
                    };
                    let texel = self.vram[((ty + v) * 1024 + (tx + (u >> shift)) % 1024) as usize];
                    let sample = match mode {
                        0 | 1 => {
                            let index = if mode == 0 {
                                (texel >> ((u & 3) * 4)) & 15
                            } else {
                                (texel >> ((u & 1) * 8)) & 255
                            };
                            let clut = uv >> 16;
                            self.vram[(((clut >> 6) * 1024
                                + ((clut & 63) * 16 + u32::from(index)) % 1024)
                                % (1024 * 512)) as usize]
                        }
                        _ => texel,
                    };
                    if sample == 0 {
                        continue;
                    }
                    (rgb(sample), sample & 0x8000 != 0)
                } else {
                    (color.map(|c| c >> 3), true)
                };
                if textured && code & 1 == 0 {
                    for i in 0..3 {
                        source[i] = (source[i] * color[i] / 128).min(31);
                    }
                }
                let at = (py * 1024 + px) as usize;
                if code & 2 != 0 && stp {
                    let destination = rgb(self.vram[at]);
                    let blend = (self.page >> 5) & 3;
                    for i in 0..3 {
                        source[i] = match blend {
                            0 => destination[i] / 2 + source[i] / 2,
                            1 => (destination[i] + source[i]).min(31),
                            2 => destination[i].saturating_sub(source[i]),
                            _ => (destination[i] + source[i] / 4).min(31),
                        };
                    }
                }
                self.vram[at] = packed(source);
            }
        }
    }
    pub fn frame(&self, x: i32, y: i32, w: u32, h: u32) -> Vec<u32> {
        let mut result = vec![0; (w * h) as usize];
        for row in 0..h {
            for col in 0..w {
                let vx = x + col as i32;
                let vy = y + row as i32;
                if (0..1024).contains(&vx) && (0..512).contains(&vy) {
                    let channels = rgb(self.vram[(vy * 1024 + vx) as usize])
                        .map(|c| u32::from((c << 3) | (c >> 2)));
                    result[(row * w + col) as usize] =
                        (channels[0] << 16) | (channels[1] << 8) | channels[2];
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
