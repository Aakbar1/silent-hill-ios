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
    fn put(&mut self, x: i32, y: i32, mut source: [u16; 3], blend: bool) {
        if x < self.clip[0].max(0)
            || y < self.clip[1].max(0)
            || x >= (self.clip[0] + self.clip[2]).min(1024)
            || y >= (self.clip[1] + self.clip[3]).min(512)
        {
            return;
        }
        let at = (y * 1024 + x) as usize;
        if blend {
            let destination = rgb(self.vram[at]);
            for i in 0..3 {
                source[i] = match (self.page >> 5) & 3 {
                    0 => destination[i] / 2 + source[i] / 2,
                    1 => (destination[i] + source[i]).min(31),
                    2 => destination[i].saturating_sub(source[i]),
                    _ => (destination[i] + source[i] / 4).min(31),
                };
            }
        }
        self.vram[at] = packed(source);
    }
    // PORT: Menu fallback uses a native top-left raster. PS1 subpixel/dither
    // calibration remains with the full GPU backend lane.
    fn triangle(&mut self, mut vertices: [(i32, i32, [u16; 3]); 3], blend: bool) {
        fn edge(a: (i32, i32, [u16; 3]), b: (i32, i32, [u16; 3]), x: i32, y: i32) -> i64 {
            i64::from(b.0 - a.0) * i64::from(y - a.1) - i64::from(b.1 - a.1) * i64::from(x - a.0)
        }
        for vertex in &mut vertices {
            vertex.0 *= 2;
            vertex.1 *= 2;
        }
        let mut area = edge(vertices[0], vertices[1], vertices[2].0, vertices[2].1);
        if area == 0 {
            return;
        }
        if area < 0 {
            vertices.swap(1, 2);
            area = -area;
        }
        let left = (vertices.iter().map(|v| v.0).min().unwrap() / 2)
            .max(self.clip[0])
            .max(0);
        let right = (vertices.iter().map(|v| v.0).max().unwrap() / 2)
            .min(self.clip[0] + self.clip[2])
            .min(1024);
        let top = (vertices.iter().map(|v| v.1).min().unwrap() / 2)
            .max(self.clip[1])
            .max(0);
        let bottom = (vertices.iter().map(|v| v.1).max().unwrap() / 2)
            .min(self.clip[1] + self.clip[3])
            .min(512);
        let top_left = |a: (i32, i32, [u16; 3]), b: (i32, i32, [u16; 3])| {
            b.1 < a.1 || (b.1 == a.1 && b.0 > a.0)
        };
        for y in top..bottom {
            for x in left..right {
                let weights = [
                    edge(vertices[1], vertices[2], x * 2 + 1, y * 2 + 1),
                    edge(vertices[2], vertices[0], x * 2 + 1, y * 2 + 1),
                    edge(vertices[0], vertices[1], x * 2 + 1, y * 2 + 1),
                ];
                if (0..3).any(|i| {
                    weights[i] < 0
                        || (weights[i] == 0
                            && !top_left(vertices[(i + 1) % 3], vertices[(i + 2) % 3]))
                }) {
                    continue;
                }
                let color = std::array::from_fn(|c| {
                    ((0..3)
                        .map(|i| weights[i] * i64::from(vertices[i].2[c]))
                        .sum::<i64>()
                        / area
                        / 8)
                    .clamp(0, 31) as u16
                });
                self.put(x, y, color, blend);
            }
        }
    }
    fn polygon(&mut self, words: &[u32], code: u8) {
        let count = if code & 8 != 0 { 4 } else { 3 };
        let gouraud = code & 16 != 0;
        if words.len() < if gouraud { 1 + count * 2 } else { 2 + count } {
            return;
        }
        let mut vertices = [(0, 0, [0; 3]); 4];
        for (i, vertex) in vertices.iter_mut().enumerate().take(count) {
            let color = words[if gouraud { 1 + i * 2 } else { 1 }];
            let xy = words[if gouraud { 2 + i * 2 } else { 2 + i }];
            *vertex = (
                i32::from(xy as i16) + self.offset[0],
                i32::from((xy >> 16) as i16) + self.offset[1],
                [
                    (color & 255) as u16,
                    ((color >> 8) & 255) as u16,
                    ((color >> 16) & 255) as u16,
                ],
            );
        }
        self.primitives += 1;
        self.triangle([vertices[0], vertices[1], vertices[2]], code & 2 != 0);
        if count == 4 {
            self.triangle([vertices[1], vertices[2], vertices[3]], code & 2 != 0);
        }
    }
    fn line(&mut self, words: &[u32], code: u8) {
        let gouraud = code & 16 != 0;
        if words.len() < if gouraud { 5 } else { 4 } {
            return;
        }
        let a = words[2];
        let b = words[if gouraud { 4 } else { 3 }];
        let from = [i32::from(a as i16), i32::from((a >> 16) as i16)];
        let to = [i32::from(b as i16), i32::from((b >> 16) as i16)];
        let colors = [words[1], words[if gouraud { 3 } else { 1 }]];
        let steps = (to[0] - from[0]).abs().max((to[1] - from[1]).abs()).max(1);
        self.primitives += 1;
        for t in 0..=steps {
            let x = from[0] + (to[0] - from[0]) * t / steps + self.offset[0];
            let y = from[1] + (to[1] - from[1]) * t / steps + self.offset[1];
            let color = std::array::from_fn(|c| {
                let a = ((colors[0] >> (c * 8)) & 255) as i32;
                let b = ((colors[1] >> (c * 8)) & 255) as i32;
                ((a + (b - a) * t / steps) / 8) as u16
            });
            self.put(x, y, color, code & 2 != 0);
        }
    }
    pub fn read(&self, [x, y, w, h]: [i32; 4]) -> Vec<u16> {
        if w <= 0 || h <= 0 || w > 1024 || h > 512 {
            return Vec::new();
        }
        (0..h)
            .flat_map(|row| {
                (0..w).map(move |col| {
                    if (0..1024).contains(&(x + col)) && (0..512).contains(&(y + row)) {
                        self.vram[((y + row) * 1024 + x + col) as usize]
                    } else {
                        0
                    }
                })
            })
            .collect()
    }
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
        if code & 0xe0 == 0x20 && code & 4 == 0 {
            self.polygon(words, code);
            return;
        }
        if code & 0xe0 == 0x40 && code & 8 == 0 {
            self.line(words, code);
            return;
        }
        if code & 0xe0 != 0x60 {
            // PORT: Unsupported GPU commands are boot-only stubs, logged once per opcode.
            if !self.unknown[usize::from(code)] {
                eprintln!("STUB GPU opcode {code:#04x}");
                self.unknown[usize::from(code)] = true;
            }
            return;
        }
        let textured = code & 4 != 0;
        let size = (code >> 3) & 3;
        if words.len()
            < if textured {
                if size == 0 { 5 } else { 4 }
            } else if size == 0 {
                4
            } else {
                3
            }
        {
            return;
        }
        self.primitives += 1;
        let x = i32::from(words[2] as i16) + self.offset[0];
        let y = i32::from((words[2] >> 16) as i16) + self.offset[1];
        let wh = if size == 0 {
            words[if textured { 4 } else { 3 }]
        } else {
            let n = match size {
                1 => 1,
                2 => 8,
                _ => 16,
            };
            n | (n << 16)
        };
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
