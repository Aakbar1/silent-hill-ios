//! PS1 Schroeder reverb, shared sound RAM, alternating L/R at 22.05 kHz,
//! signed Q15 coefficients, and the hardware 39-tap FIR in both directions.
use crate::{RAM_BYTES, mul, sat};
#[path = "presets.rs"]
mod presets;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Preset {
    Off,
    Room,
    StudioA,
    StudioB,
    StudioC,
    Hall,
    Space,
    Echo,
    Delay,
    Pipe,
}
impl Preset {
    pub fn from_id(id: u8) -> Option<Self> {
        Some(match id {
            0 => Self::Off,
            1 => Self::Room,
            2 => Self::StudioA,
            3 => Self::StudioB,
            4 => Self::StudioC,
            5 => Self::Hall,
            6 => Self::Space,
            7 => Self::Echo,
            8 => Self::Delay,
            9 => Self::Pipe,
            _ => return None,
        })
    }
    pub fn registers(self) -> [u16; 32] {
        presets::PRESETS[self as usize].1
    }
    pub fn work_bytes(self) -> usize {
        presets::PRESETS[self as usize].0
    }
    pub fn base(self) -> u16 {
        ((RAM_BYTES - self.work_bytes()) / 8) as u16
    }
}

pub const FIR: [i32; 39] = [
    -1, 0, 2, 0, -10, 0, 35, 0, -103, 0, 266, 0, -616, 0, 1332, 0, -2960, 0, 10246, 16384, 10246,
    0, -2960, 0, 1332, 0, -616, 0, 266, 0, -103, 0, 35, 0, -10, 0, 2, 0, -1,
];

#[derive(Clone)]
pub struct Reverb {
    pub registers: [u16; 32],
    pub volume: [i16; 2],
    base: usize,
    cursor: usize,
    channel: usize,
    input: [[i16; 39]; 2],
    input_pos: usize,
    wet: [[i16; 20]; 2],
    wet_pos: [usize; 2],
}
impl Default for Reverb {
    fn default() -> Self {
        Self {
            registers: [0; 32],
            volume: [0; 2],
            base: RAM_BYTES / 2 - 8,
            cursor: RAM_BYTES / 2 - 8,
            channel: 0,
            input: [[0; 39]; 2],
            input_pos: 0,
            wet: [[0; 20]; 2],
            wet_pos: [0; 2],
        }
    }
}
impl Reverb {
    pub fn set_base(&mut self, value: u16) {
        self.base = usize::from(value) * 4;
        self.cursor = self.base;
    }
    pub fn base(&self) -> u16 {
        (self.base / 4) as u16
    }
    pub fn cursor(&self) -> usize {
        self.cursor * 2
    }
    pub fn set_preset(&mut self, preset: Preset) {
        self.registers = preset.registers();
        self.set_base(preset.base());
    }
    pub fn clear(&mut self, ram: &mut [u8]) {
        ram[self.base * 2..RAM_BYTES].fill(0);
    }
    fn address(&self, offset: i32, extra: i32) -> usize {
        let length = (RAM_BYTES / 2 - self.base) as i32;
        (self.base as i32
            + (self.cursor as i32 - self.base as i32 + offset * 4 + extra).rem_euclid(length))
            as usize
            * 2
    }
    fn read(&self, ram: &[u8], offset: i32, extra: i32) -> i32 {
        let a = self.address(offset, extra);
        i32::from(i16::from_le_bytes([ram[a], ram[a + 1]]))
    }
    fn write(&self, ram: &mut [u8], offset: i32, value: i32) {
        let a = self.address(offset, 0);
        ram[a..a + 2].copy_from_slice(&sat(value).to_le_bytes());
    }
    /// One half-rate channel, exposed for deterministic coefficient fixtures.
    pub fn tick_channel(
        &mut self,
        ram: &mut [u8],
        input: i16,
        ch: usize,
        write_enabled: bool,
    ) -> i16 {
        let r = self.registers;
        let coefficient = |i: usize| i32::from(r[i] as i16);
        let offset = |i: usize| i32::from(r[i]);
        let incoming = mul(i32::from(input), coefficient(30 + ch));
        if write_enabled {
            for (source, dest) in [(16 + ch, 10 + ch), (24 + (1 - ch), 18 + ch)] {
                let reflected = i32::from(sat(
                    incoming + mul(self.read(ram, offset(source), 0), coefficient(7))
                ));
                let previous = self.read(ram, offset(dest), -1);
                let value = mul(reflected, coefficient(2)) + mul(previous, 32768 - coefficient(2));
                // psx-spx documents a sign inversion for the vIIR=-0x8000 edge.
                self.write(
                    ram,
                    offset(dest),
                    if r[2] == 0x8000 { -value } else { value },
                );
            }
        }
        let mut value = 0;
        for (coef, tap) in [(3, 12 + ch), (4, 14 + ch), (5, 20 + ch), (6, 22 + ch)] {
            value += mul(self.read(ram, offset(tap), 0), coefficient(coef));
        }
        value = i32::from(sat(value));
        for (dest, disp, coef) in [(26 + ch, 0, 8), (28 + ch, 1, 9)] {
            let tap = self.read(ram, offset(dest) - offset(disp), 0);
            value = i32::from(sat(value - mul(tap, coefficient(coef))));
            if write_enabled {
                self.write(ram, offset(dest), value);
            }
            value = i32::from(sat(mul(value, coefficient(coef)) + tap));
        }
        sat(mul(value, i32::from(self.volume[ch])))
    }
    /// No allocations. ATTR bit 7 gates RAM writes, never the wet output.
    pub fn process(&mut self, ram: &mut [u8], input: [i16; 2], write_enabled: bool) -> [i16; 2] {
        for (ch, sample) in input.into_iter().enumerate() {
            self.input[ch][self.input_pos] = sample;
        }
        let ch = self.channel;
        let mut acc = 0i64;
        for (tap, &coef) in FIR.iter().enumerate() {
            acc += i64::from(coef) * i64::from(self.input[ch][(self.input_pos + 39 - tap) % 39]);
        }
        let wet = self.tick_channel(ram, sat((acc >> 15) as i32), ch, write_enabled);
        self.wet_pos[ch] = (self.wet_pos[ch] + 1) % 20;
        self.wet[ch][self.wet_pos[ch]] = wet;
        let output = std::array::from_fn(|c| {
            if c != ch {
                return self.wet[c][(self.wet_pos[c] + 20 - 9) % 20];
            }
            let mut acc = 0i64;
            for tap in (0..39).step_by(2) {
                acc += i64::from(FIR[tap])
                    * i64::from(self.wet[c][(self.wet_pos[c] + 20 - tap / 2) % 20]);
            }
            sat((acc >> 14) as i32)
        });
        if ch == 1 {
            self.cursor += 1;
            if self.cursor == RAM_BYTES / 2 {
                self.cursor = self.base;
            }
        }
        self.channel ^= 1;
        self.input_pos = (self.input_pos + 1) % 39;
        output
    }
}
