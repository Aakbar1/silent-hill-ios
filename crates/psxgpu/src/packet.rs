// SPDX-License-Identifier: GPL-3.0-only
use crate::{Error, Primitive, Rect, Renderer, Result, Vertex};
use std::collections::{HashSet, VecDeque};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DrawState {
    pub mode: u32,
    pub window: u32,
    pub area: [i32; 4],
    pub offset: [i32; 2],
    /// E6 bits: force bit 15, prohibit writes to bit-15 destinations.
    pub mask: u32,
    pub texture_disable_allowed: bool,
    /// In 480i with DFE=0 the currently displayed parity is protected.
    pub skip_line: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Display {
    pub disabled: bool,
    pub x: u16,
    pub y: u16,
    pub horizontal: [u16; 2],
    pub vertical: [u16; 2],
    pub mode: u32,
}
impl Default for Display {
    fn default() -> Self {
        Self {
            disabled: true,
            x: 0,
            y: 0,
            horizontal: [0x200, 0xc00],
            vertical: [0x10, 0x100],
            mode: 0,
        }
    }
}
impl Display {
    pub fn dimensions(self) -> [u32; 2] {
        let divisor = if self.mode & 64 != 0 {
            7
        } else {
            [10, 8, 5, 4][(self.mode & 3) as usize]
        };
        let w = u32::from(self.horizontal[1].saturating_sub(self.horizontal[0])) / divisor;
        let mut h = u32::from(self.vertical[1].saturating_sub(self.vertical[0]));
        if self.mode & 36 == 36 {
            h *= 2;
        }
        [w.min(1024), h.min(512)]
    }
}

struct Upload {
    rect: Rect,
    pixels: Vec<u16>,
}

/// Stateful GP0 FIFO/GP1 control. GP0 writes may split at any word boundary.
pub struct Processor<R: Renderer> {
    renderer: R,
    pub draw_state: DrawState,
    pub display: Display,
    packet: Vec<u32>,
    upload: Option<Upload>,
    readback: VecDeque<u32>,
    read_latch: u32,
    irq: bool,
    dma: u32,
    field: bool,
    polyline_segment: usize,
    precise: Option<Vec<[f32; 2]>>,
    widescreen: bool,
}

fn signed11(n: u32) -> i32 {
    ((n << 21) as i32) >> 21
}
fn transfer_rect(xy: u32, wh: u32) -> Rect {
    Rect::new(
        (xy & 1023) as u16,
        ((xy >> 16) & 511) as u16,
        (((wh.wrapping_sub(1)) & 1023) + 1) as u16,
        ((((wh >> 16).wrapping_sub(1)) & 511) + 1) as u16,
    )
}

impl<R: Renderer> Processor<R> {
    pub fn new(renderer: R) -> Self {
        Self {
            renderer,
            draw_state: DrawState::default(),
            display: Display::default(),
            packet: Vec::new(),
            upload: None,
            readback: VecDeque::new(),
            read_latch: 0,
            irq: false,
            dma: 0,
            field: false,
            polyline_segment: 0,
            precise: None,
            widescreen: false,
        }
    }
    pub fn renderer(&self) -> &R {
        &self.renderer
    }
    pub fn renderer_mut(&mut self) -> &mut R {
        &mut self.renderer
    }
    pub fn into_renderer(self) -> R {
        self.renderer
    }
    pub fn idle(&self) -> bool {
        self.packet.is_empty() && self.upload.is_none()
    }
    pub fn set_field(&mut self, field: bool) {
        self.field = field;
    }
    /// PORT: opt-in wider clipping; core must also submit geometry outside 4:3.
    pub fn set_widescreen(&mut self, enabled: bool) {
        self.widescreen = enabled;
    }

    pub fn gp0_words(&mut self, words: &[u32]) -> Result<()> {
        for &w in words {
            self.gp0(w)?;
        }
        Ok(())
    }

    /// PORT: precise positions travel beside an original complete polygon packet.
    /// Never changes GTE integer registers or the captured GP0 wire data.
    pub fn polygon_with_positions(&mut self, words: &[u32], positions: &[[f32; 2]]) -> Result<()> {
        if !self.idle() || words.is_empty() || words[0] >> 29 != 1 {
            return Err(Error(
                "precise positions require an idle, complete polygon packet".into(),
            ));
        }
        let count = if words[0] & (1 << 27) != 0 { 4 } else { 3 };
        if positions.len() != count
            || positions
                .iter()
                .flatten()
                .any(|v| !v.is_finite() || v.abs() > 1024.0)
            || words.len() != packet_len((words[0] >> 24) as u8)
        {
            return Err(Error("invalid precise polygon metadata".into()));
        }
        // Metadata must belong to these wire vertices, before drawing offset.
        let mut index = 1;
        for (i, pos) in positions.iter().enumerate() {
            if i > 0 && words[0] & (1 << 28) != 0 {
                index += 1;
            }
            let xy = words[index];
            if (pos[0] - signed11(xy) as f32).abs() >= 1.0
                || (pos[1] - signed11(xy >> 16) as f32).abs() >= 1.0
            {
                return Err(Error("precise vertex does not match GP0 coordinate".into()));
            }
            index += 1 + usize::from(words[0] & (1 << 26) != 0);
        }
        self.precise = Some(positions.to_vec());
        let result = self.gp0_words(words);
        self.precise = None;
        result
    }

    pub fn gp0(&mut self, word: u32) -> Result<()> {
        if let Some(upload) = &mut self.upload {
            upload.pixels.push(word as u16);
            if upload.pixels.len() < upload.rect.len() {
                upload.pixels.push((word >> 16) as u16);
            }
            if upload.pixels.len() == upload.rect.len() {
                let upload = self.upload.take().expect("active upload");
                self.renderer
                    .upload(upload.rect, &upload.pixels, self.draw_state.mask)?;
            }
            return Ok(());
        }
        if let Some(&first) = self.packet.first() {
            let op = (first >> 24) as u8;
            if op & 0xe8 == 0x48 {
                let shaded = op & 16 != 0;
                let boundary = !shaded || self.packet.len() == 2 && self.polyline_segment > 0;
                if boundary && word & 0xf000_f000 == 0x5000_5000 {
                    self.packet.clear();
                    self.polyline_segment = 0;
                    return Ok(());
                }
            }
        }
        self.packet.push(word);
        let op = (self.packet[0] >> 24) as u8;
        if op & 0xe8 == 0x48 {
            let shaded = op & 16 != 0;
            let size = if shaded { 4 } else { 3 };
            if self.packet.len() == size {
                let p = self.packet.clone();
                self.line(&p, false)?;
                self.polyline_segment += 1;
                self.packet = if shaded {
                    vec![(p[2] & 0xffffff) | (u32::from(op) << 24), p[3]]
                } else {
                    vec![p[0], p[2]]
                };
            }
            return Ok(());
        }
        if self.packet.len() == packet_len(op) {
            let words = std::mem::take(&mut self.packet);
            self.execute(&words)?;
        }
        Ok(())
    }

    pub fn gp1(&mut self, word: u32) -> Result<()> {
        if word >> 24 <= 1 {
            // A FIFO reset aborts the transfer, retaining pixels already supplied.
            if let Some(upload) = self.upload.take() {
                let width = usize::from(upload.rect.width);
                for (row, pixels) in upload.pixels.chunks(width).enumerate() {
                    self.renderer.upload(
                        Rect::new(
                            upload.rect.x,
                            ((usize::from(upload.rect.y) + row) & 511) as u16,
                            pixels.len() as u16,
                            1,
                        ),
                        pixels,
                        self.draw_state.mask,
                    )?;
                }
            }
        }
        match word >> 24 {
            0 => {
                self.renderer.flush()?;
                let allow = self.draw_state.texture_disable_allowed;
                self.draw_state = DrawState::default();
                self.draw_state.texture_disable_allowed = allow;
                self.display = Display::default();
                self.packet.clear();
                self.upload = None;
                self.readback.clear();
                self.polyline_segment = 0;
                self.irq = false;
                self.dma = 0;
            }
            1 => {
                self.packet.clear();
                self.upload = None;
                self.readback.clear();
                self.polyline_segment = 0;
            }
            2 => self.irq = false,
            3 => self.display.disabled = word & 1 != 0,
            4 => self.dma = word & 3,
            5 => {
                self.display.x = (word & 1023) as u16;
                self.display.y = ((word >> 10) & 511) as u16;
            }
            6 => self.display.horizontal = [(word & 4095) as u16, ((word >> 12) & 4095) as u16],
            7 => self.display.vertical = [(word & 1023) as u16, ((word >> 10) & 1023) as u16],
            8 => self.display.mode = word & 255,
            9 => self.draw_state.texture_disable_allowed = word & 1 != 0,
            0x10..=0x1f => {
                self.read_latch = match word & 15 {
                    2 => self.draw_state.window,
                    3 => self.draw_state.area[0] as u32 | (self.draw_state.area[1] as u32) << 10,
                    4 => self.draw_state.area[2] as u32 | (self.draw_state.area[3] as u32) << 10,
                    5 => {
                        self.draw_state.offset[0] as u32 & 2047
                            | (self.draw_state.offset[1] as u32 & 2047) << 11
                    }
                    7 => 2,
                    8 => 0,
                    _ => self.read_latch,
                };
            }
            _ => {} // Undefined GP1 opcodes do not alter register state.
        }
        Ok(())
    }
    pub fn read_gp0(&mut self) -> u32 {
        if let Some(word) = self.readback.pop_front() {
            self.read_latch = word;
        }
        self.read_latch
    }
    pub fn status(&self) -> u32 {
        let mode = self.display.mode;
        let ready = u32::from(!self.readback.is_empty());
        let request = match self.dma {
            1 | 2 => 1,
            3 => ready,
            _ => 0,
        };
        (self.draw_state.mode & 0x7ff)
            | ((self.draw_state.mask & 3) << 11)
            | (u32::from(!self.field) << 13)
            | ((mode & 128) << 7)
            | (u32::from(
                self.draw_state.texture_disable_allowed && self.draw_state.mode & 0x800 != 0,
            ) << 15)
            | ((mode & 64) << 10)
            | ((mode & 3) << 17)
            | ((mode & 60) << 17)
            | (u32::from(self.display.disabled) << 23)
            | (u32::from(self.irq) << 24)
            | (request << 25)
            | (1 << 26)
            | (ready << 27)
            | (1 << 28)
            | (self.dma << 29)
            | (u32::from(self.field) << 31)
    }

    /// Walk a disk32 DMA linked list. Addresses are byte offsets, never host pointers.
    pub fn walk_ot(&mut self, ram: &[u8], head: u32, max_packets: usize) -> Result<usize> {
        let mut next = head & 0x00ff_ffff;
        let mut visited = HashSet::new();
        let mut count = 0;
        while next != 0x00ff_ffff {
            if count == max_packets || !visited.insert(next) {
                return Err(Error("OT cycle or packet budget exhausted".into()));
            }
            if next & 3 != 0 {
                return Err(Error("unaligned OT link".into()));
            }
            let offset = next as usize;
            let header = ram
                .get(offset..offset + 4)
                .ok_or_else(|| Error("OT header outside RAM".into()))?;
            let tag = u32::from_le_bytes(header.try_into().expect("four bytes"));
            let len = (tag >> 24) as usize;
            let data = ram
                .get(offset + 4..offset + 4 + len * 4)
                .ok_or_else(|| Error("OT packet outside RAM".into()))?;
            for w in data.as_chunks::<4>().0 {
                self.gp0(u32::from_le_bytes(*w))?;
            }
            next = tag & 0x00ff_ffff;
            count += 1;
        }
        Ok(count)
    }

    fn state(&self) -> DrawState {
        let mut state = self.draw_state;
        state.skip_line = if self.display.mode & 36 == 36 && state.mode & 1024 == 0 {
            Some((self.display.y + u16::from(self.field)) & 1 != 0)
        } else {
            None
        };
        if self.widescreen {
            // PORT: add one sixth of the original width on either side (4:3 -> 16:9).
            let width = (state.area[2] - state.area[0] + 1).max(0);
            let pad = width / 6;
            let expanded = (width + pad * 2).min(1024);
            state.area[0] = (state.area[0] - pad).clamp(0, 1024 - expanded);
            state.area[2] = state.area[0] + expanded - 1;
        }
        state
    }
    fn vertex(&self, xy: u32, color: u32, uv: u32) -> Vertex {
        Vertex::new(
            signed11(xy) + self.draw_state.offset[0],
            signed11(xy >> 16) + self.draw_state.offset[1],
            color,
            uv,
        )
    }
    fn line(&mut self, w: &[u32], skip_first: bool) -> Result<()> {
        let shaded = w[0] & (1 << 28) != 0;
        let a = self.vertex(w[1], w[0], 0);
        let b = self.vertex(
            w[if shaded { 3 } else { 2 }],
            if shaded { w[2] } else { w[0] },
            0,
        );
        self.renderer.draw(Primitive::line(
            a,
            b,
            (w[0] >> 24) as u8,
            self.state(),
            skip_first,
        ))
    }
    fn execute(&mut self, w: &[u32]) -> Result<()> {
        let op = (w[0] >> 24) as u8;
        match op {
            0x00 | 0x01 | 0x03..=0x1e => {}
            0x1f => self.irq = true,
            0x02 => {
                let color = ((w[0] & 255) >> 3)
                    | (((w[0] >> 8) & 255) >> 3) << 5
                    | (((w[0] >> 16) & 255) >> 3) << 10;
                let rect = Rect::new(
                    (w[1] & 0x3f0) as u16,
                    ((w[1] >> 16) & 511) as u16,
                    (((w[2] & 1023) + 15) & !15) as u16,
                    ((w[2] >> 16) & 511) as u16,
                );
                self.renderer
                    .fill_field(rect, color as u16, self.state().skip_line)?;
            }
            0x20..=0x3f => {
                let textured = op & 4 != 0;
                let shaded = op & 16 != 0;
                let count = if op & 8 != 0 { 4 } else { 3 };
                let mut vertices = [Vertex::default(); 4];
                let mut index = 1;
                let mut clut = 0;
                for (i, vertex) in vertices.iter_mut().enumerate().take(count) {
                    let color = if shaded && i != 0 {
                        let c = w[index];
                        index += 1;
                        c
                    } else {
                        w[0]
                    };
                    let xy = w[index];
                    index += 1;
                    let uv = if textured {
                        let uv = w[index];
                        index += 1;
                        uv
                    } else {
                        0
                    };
                    if textured && i == 0 {
                        clut = uv >> 16;
                    }
                    if textured && i == 1 {
                        self.draw_state.mode =
                            (self.draw_state.mode & !0x1ff) | ((uv >> 16) & 0x1ff);
                    }
                    *vertex = self.vertex(xy, color, uv);
                    if let Some(positions) = &self.precise {
                        vertex.precise = [
                            positions[i][0] + self.draw_state.offset[0] as f32,
                            positions[i][1] + self.draw_state.offset[1] as f32,
                        ];
                    }
                }
                let state = self.state();
                self.renderer.draw(Primitive::triangle(
                    [vertices[0], vertices[1], vertices[2]],
                    op,
                    state,
                    clut,
                    self.precise.is_some(),
                ))?;
                if count == 4 {
                    self.renderer.draw(Primitive::triangle(
                        [vertices[1], vertices[2], vertices[3]],
                        op,
                        state,
                        clut,
                        self.precise.is_some(),
                    ))?;
                }
            }
            0x40..=0x5f => self.line(w, false)?,
            0x60..=0x7f => {
                let textured = op & 4 != 0;
                let uv = if textured { w[2] } else { 0 };
                let wh = match (op >> 3) & 3 {
                    0 => w[if textured { 3 } else { 2 }],
                    1 => 0x10001,
                    2 => 0x80008,
                    _ => 0x100010,
                };
                let vertex = self.vertex(w[1], w[0], uv);
                self.renderer.draw(Primitive::rectangle(
                    vertex,
                    [(wh & 1023) as i32, ((wh >> 16) & 511) as i32],
                    op,
                    self.state(),
                    uv >> 16,
                ))?;
            }
            0x80..=0x9f => {
                let src = transfer_rect(w[1], w[3]);
                self.renderer.copy(
                    src,
                    (w[2] & 1023) as u16,
                    ((w[2] >> 16) & 511) as u16,
                    self.draw_state.mask,
                )?;
            }
            0xa0..=0xbf => {
                self.upload = Some(Upload {
                    rect: transfer_rect(w[1], w[2]),
                    pixels: Vec::new(),
                })
            }
            0xc0..=0xdf => {
                let rect = transfer_rect(w[1], w[2]);
                let mut pixels = self.renderer.read(rect)?;
                if pixels.len() & 1 != 0 {
                    let trailing = Rect::new(
                        (rect.x + rect.width) & 1023,
                        (rect.y + rect.height - 1) & 511,
                        1,
                        1,
                    );
                    pixels.push(self.renderer.read(trailing)?[0]);
                }
                self.readback = pixels
                    .chunks(2)
                    .map(|p| u32::from(p[0]) | (u32::from(*p.get(1).unwrap_or(&0)) << 16))
                    .collect();
            }
            0xe1 => self.draw_state.mode = w[0] & 0x3fff,
            0xe2 => self.draw_state.window = w[0] & 0xfffff,
            0xe3 => {
                self.draw_state.area[0] = (w[0] & 1023) as i32;
                self.draw_state.area[1] = ((w[0] >> 10) & 511) as i32;
            }
            0xe4 => {
                self.draw_state.area[2] = (w[0] & 1023) as i32;
                self.draw_state.area[3] = ((w[0] >> 10) & 511) as i32;
            }
            0xe5 => self.draw_state.offset = [signed11(w[0]), signed11(w[0] >> 11)],
            0xe6 => self.draw_state.mask = w[0] & 3,
            _ => return Err(Error(format!("unsupported GP0 opcode {op:#04x}"))),
        }
        Ok(())
    }
}

fn packet_len(op: u8) -> usize {
    match op {
        2 | 0xa0..=0xdf => 3,
        0x20..=0x3f => {
            let vertices = if op & 8 != 0 { 4 } else { 3 };
            1 + vertices * (1 + usize::from(op & 4 != 0))
                + if op & 16 != 0 { vertices - 1 } else { 0 }
        }
        0x40..=0x5f => {
            if op & 16 != 0 {
                4
            } else {
                3
            }
        }
        0x60..=0x7f => 2 + usize::from(op & 4 != 0) + usize::from(op & 0x18 == 0),
        0x80..=0x9f => 4,
        _ => 1,
    }
}
