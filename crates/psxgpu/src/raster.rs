// SPDX-License-Identifier: GPL-3.0-only
use crate::DrawState;

#[derive(Clone, Copy, Debug, Default)]
pub struct Vertex {
    pub x: i32,
    pub y: i32,
    pub color: [i32; 3],
    pub uv: [i32; 2],
    pub precise: [f32; 2],
}
impl Vertex {
    pub fn new(x: i32, y: i32, color: u32, uv: u32) -> Self {
        Self {
            x,
            y,
            color: [
                (color & 255) as i32,
                ((color >> 8) & 255) as i32,
                ((color >> 16) & 255) as i32,
            ],
            uv: [(uv & 255) as i32, ((uv >> 8) & 255) as i32],
            precise: [x as f32, y as f32],
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Primitive {
    pub vertices: [Vertex; 3],
    pub state: DrawState,
    pub clut: u32,
    pub opcode: u8,
    /// 0 triangle, 1 rectangle, 2 line
    pub kind: u32,
    pub size: [i32; 2],
    pub subpixel: bool,
    pub skip_first: bool,
}
impl Primitive {
    pub fn triangle(
        mut v: [Vertex; 3],
        opcode: u8,
        state: DrawState,
        clut: u32,
        subpixel: bool,
    ) -> Self {
        // The PS1 interpolation anchor is the leftmost original vertex; tie order matters.
        let core = if v[1].x <= v[0].x {
            if v[2].x <= v[1].x {
                2
            } else {
                1
            }
        } else if v[2].x < v[0].x {
            2
        } else {
            0
        };
        v.rotate_left(core);
        if edge(v[0], v[1], v[2].x, v[2].y) < 0 {
            v.swap(1, 2);
        }
        Self {
            vertices: v,
            opcode,
            state,
            clut,
            kind: 0,
            size: [0; 2],
            subpixel,
            skip_first: false,
        }
    }
    pub fn rectangle(v: Vertex, size: [i32; 2], opcode: u8, state: DrawState, clut: u32) -> Self {
        Self {
            vertices: [v; 3],
            opcode,
            state,
            clut,
            kind: 1,
            size,
            subpixel: false,
            skip_first: false,
        }
    }
    pub fn line(
        mut a: Vertex,
        mut b: Vertex,
        opcode: u8,
        state: DrawState,
        skip_first: bool,
    ) -> Self {
        if a.x > b.x {
            std::mem::swap(&mut a, &mut b);
        }
        Self {
            vertices: [a, b, b],
            opcode,
            state,
            clut: 0,
            kind: 2,
            size: [0; 2],
            subpixel: false,
            skip_first,
        }
    }
    pub fn textured(self) -> bool {
        self.opcode & 4 != 0
            && self.kind != 2
            && !(self.state.texture_disable_allowed && self.state.mode & 0x800 != 0)
    }
    pub fn bounds(self) -> Option<[i32; 4]> {
        let v = self.vertices;
        let mut b = if self.kind == 1 {
            [v[0].x, v[0].y, v[0].x + self.size[0], v[0].y + self.size[1]]
        } else {
            let mut b = [i32::MAX, i32::MAX, i32::MIN, i32::MIN];
            for p in &v[..if self.kind == 2 { 2 } else { 3 }] {
                b[0] = b[0].min(p.x);
                b[1] = b[1].min(p.y);
                b[2] = b[2].max(p.x);
                b[3] = b[3].max(p.y);
            }
            if b[2] - b[0] > 1023 || b[3] - b[1] > 511 {
                return None;
            }
            if self.kind == 2 || self.subpixel {
                b[2] += 1;
                b[3] += 1;
            }
            b
        };
        b[0] = b[0].max(self.state.area[0]).max(0);
        b[1] = b[1].max(self.state.area[1]).max(0);
        b[2] = b[2].min(self.state.area[2] + 1).min(1024);
        b[3] = b[3].min(self.state.area[3] + 1).min(512);
        (b[2] > b[0] && b[3] > b[1]).then_some(b)
    }
    /// Conservative source footprint, including a full CLUT, for feedback barriers.
    #[cfg(feature = "wgpu")]
    pub(crate) fn texture_bounds(self) -> [i32; 4] {
        let depth = (self.state.mode >> 7) & 3;
        let x = ((self.state.mode & 15) * 64) as i32;
        let y = ((self.state.mode & 16) * 16) as i32;
        [
            x,
            y,
            x + if depth == 0 {
                64
            } else if depth == 1 {
                128
            } else {
                256
            },
            y + 256,
        ]
    }
}

fn edge(a: Vertex, b: Vertex, x: i32, y: i32) -> i32 {
    (b.x - a.x) * (y - a.y) - (b.y - a.y) * (x - a.x)
}
fn top_left(a: Vertex, b: Vertex) -> bool {
    b.y < a.y || b.y == a.y && b.x > a.x
}

/// Integer affine interpolation; the GPU path uses the same precomputed planes.
/// 12 fractional bits retain PS1-style truncation instead of float interpolation.
pub(crate) fn planes(p: Primitive) -> [[i32; 3]; 5] {
    let v = p.vertices;
    let area = i64::from(edge(v[0], v[1], v[2].x, v[2].y));
    std::array::from_fn(|c| {
        let at = |i: usize| if c < 3 { v[i].color[c] } else { v[i].uv[c - 3] };
        if area == 0 {
            return [at(0) * 4096, 0, 0];
        }
        let dx = (i64::from(at(1) - at(0)) * i64::from(v[2].y - v[0].y)
            - i64::from(at(2) - at(0)) * i64::from(v[1].y - v[0].y))
            * 4096
            / area;
        let dy = (i64::from(v[1].x - v[0].x) * i64::from(at(2) - at(0))
            - i64::from(v[2].x - v[0].x) * i64::from(at(1) - at(0)))
            * 4096
            / area;
        [at(0) * 4096 + 2048, dx as i32, dy as i32]
    })
}

pub(crate) fn fragment(
    p: Primitive,
    coefficients: &[[i32; 3]; 5],
    x: i32,
    y: i32,
) -> Option<([i32; 3], [i32; 2])> {
    if p.state.skip_line == Some(y & 1 != 0) {
        return None;
    }
    let v = p.vertices;
    match p.kind {
        1 => {
            let dx = x - v[0].x;
            let dy = y - v[0].y;
            if dx < 0 || dy < 0 || dx >= p.size[0] || dy >= p.size[1] {
                return None;
            }
            let flip = p.state.mode >> 12;
            Some((
                v[0].color,
                [
                    v[0].uv[0] + if flip & 1 != 0 { -dx } else { dx },
                    v[0].uv[1] + if flip & 2 != 0 { -dy } else { dy },
                ],
            ))
        }
        2 => {
            let dx = v[1].x - v[0].x;
            let dy = v[1].y - v[0].y;
            let n = dx.abs().max(dy.abs());
            let i = if dx.abs() >= dy.abs() {
                (x - v[0].x) * dx.signum()
            } else {
                (y - v[0].y) * dy.signum()
            };
            if i < i32::from(p.skip_first) || i > n {
                return None;
            }
            let div = n.max(1);
            let xx = v[0].x + (i * dx * 2 + div - 1).div_euclid(div * 2);
            let yy = v[0].y + (i * dy * 2 + div - i32::from(dy < 0)).div_euclid(div * 2);
            if x != xx || y != yy {
                return None;
            }
            Some((
                std::array::from_fn(|c| {
                    (v[0].color[c] * 4096
                        + 2048
                        + ((v[1].color[c] - v[0].color[c]) * 4096 / div) * i)
                        >> 12
                }),
                [0; 2],
            ))
        }
        _ => {
            let area = edge(v[0], v[1], v[2].x, v[2].y);
            if area <= 0 {
                return None;
            }
            for (a, b) in [(v[0], v[1]), (v[1], v[2]), (v[2], v[0])] {
                let e = edge(a, b, x, y);
                if e < 0 || e == 0 && !top_left(a, b) {
                    return None;
                }
            }
            let planes = coefficients;
            let a: [i32; 5] = std::array::from_fn(|c| {
                // Wrapping here models the 32-bit interpolation accumulators.
                planes[c][0]
                    .wrapping_add(planes[c][1].wrapping_mul(x - v[0].x))
                    .wrapping_add(planes[c][2].wrapping_mul(y - v[0].y))
                    >> 12
            });
            Some(([a[0], a[1], a[2]], [a[3], a[4]]))
        }
    }
}

pub(crate) fn texture(vram: &[u16], p: Primitive, uv: [i32; 2]) -> u16 {
    let w = p.state.window;
    let u = (uv[0] as u32 & !((w & 31) << 3)) | (((w >> 10) & (w & 31)) << 3);
    let v = (uv[1] as u32 & !(((w >> 5) & 31) << 3)) | (((w >> 15) & ((w >> 5) & 31)) << 3);
    let u = u & 255;
    let v = v & 255;
    let page = p.state.mode;
    let depth = (page >> 7) & 3;
    let x = (page & 15) * 64
        + if depth == 0 {
            u / 4
        } else if depth == 1 {
            u / 2
        } else {
            u
        };
    let y = ((page & 16) * 16 + v) & 511;
    let word = vram[(y * 1024 + (x & 1023)) as usize];
    if depth >= 2 {
        return word;
    }
    let index = if depth == 0 {
        (word >> ((u & 3) * 4)) & 15
    } else {
        (word >> ((u & 1) * 8)) & 255
    };
    let cx = ((p.clut & 63) * 16 + u32::from(index)) & 1023;
    let cy = (p.clut >> 6) & 511;
    vram[(cy * 1024 + cx) as usize]
}

pub(crate) fn shade(
    p: Primitive,
    dst: u16,
    color: [i32; 3],
    texel: u16,
    x: i32,
    y: i32,
) -> Option<u16> {
    if p.state.mask & 2 != 0 && dst & 0x8000 != 0 {
        return None;
    }
    let textured = p.textured();
    if textured && texel == 0 {
        return None;
    }
    let raw = p.opcode & 1 != 0;
    let dither = p.state.mode & 512 != 0
        && p.kind != 1
        && (p.kind == 2 || p.opcode & 16 != 0 || textured && !raw);
    const DITHER: [i32; 16] = [-4, 0, -3, 1, 2, -2, 3, -1, -3, 1, -4, 0, 3, -1, 2, -2];
    let d = if dither {
        DITHER[((y & 3) * 4 + (x & 3)) as usize]
    } else {
        0
    };
    let blend = p.opcode & 2 != 0 && (!textured || texel & 0x8000 != 0);
    let mut result = 0;
    for (c, &col) in color.iter().enumerate() {
        let t = i32::from((texel >> (c * 5)) & 31);
        let value = if textured && raw {
            t
        } else {
            ((if textured {
                (t * col.clamp(0, 255)) >> 4
            } else {
                col
            } + d)
                .clamp(0, 255))
                >> 3
        };
        let b = i32::from((dst >> (c * 5)) & 31);
        let value = if blend {
            match (p.state.mode >> 5) & 3 {
                0 => (b + value) >> 1,
                1 => b + value,
                2 => b - value,
                _ => b + (value >> 2),
            }
            .clamp(0, 31)
        } else {
            value
        };
        result |= (value as u16) << (c * 5);
    }
    Some(
        result
            | if p.state.mask & 1 != 0 {
                0x8000
            } else if textured {
                texel & 0x8000
            } else {
                0
            },
    )
}
