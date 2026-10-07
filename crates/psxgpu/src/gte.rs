// SPDX-License-Identifier: GPL-3.0-only
//! Per-context native C GTE. Integer registers always follow the original contract.
use crate::{Error, Result};

#[repr(C)]
#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Gte {
    data: [u32; 32],
    control: [u32; 32],
}
extern "C" {
    fn psxgpu_gte_read_data(state: *mut Gte, reg: u32) -> u32;
    fn psxgpu_gte_write_data(state: *mut Gte, reg: u32, value: u32);
    fn psxgpu_gte_read_control(state: *const Gte, reg: u32) -> u32;
    fn psxgpu_gte_write_control(state: *mut Gte, reg: u32, value: u32);
    fn psxgpu_gte_execute(state: *mut Gte, opcode: u32) -> i32;
}
impl Gte {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn read_data(&mut self, reg: u8) -> u32 {
        assert!(reg < 32);
        // SAFETY: repr(C) context is valid, exclusive, and register is bounded.
        unsafe { psxgpu_gte_read_data(self, u32::from(reg)) }
    }
    pub fn write_data(&mut self, reg: u8, value: u32) {
        assert!(reg < 32);
        // SAFETY: same bounded per-context ABI as read_data.
        unsafe { psxgpu_gte_write_data(self, u32::from(reg), value) }
    }
    pub fn read_control(&self, reg: u8) -> u32 {
        assert!(reg < 32);
        // SAFETY: read-only bounded control register access.
        unsafe { psxgpu_gte_read_control(self, u32::from(reg)) }
    }
    pub fn write_control(&mut self, reg: u8, value: u32) {
        assert!(reg < 32);
        // SAFETY: exclusive valid context and bounded index.
        unsafe { psxgpu_gte_write_control(self, u32::from(reg), value) }
    }
    pub fn execute(&mut self, opcode: u32) -> Result<()> {
        // SAFETY: C dispatcher validates opcode before changing this context.
        if unsafe { psxgpu_gte_execute(self, opcode) } == 0 {
            Ok(())
        } else {
            Err(Error(format!("undefined GTE opcode {opcode:#x}")))
        }
    }
    pub fn registers(&self) -> (&[u32; 32], &[u32; 32]) {
        (&self.data, &self.control)
    }

    /// PORT: presentation-only unrounded projection, computed before RTPS/RTPT.
    /// Uses the original transform, screen offset, H and saturation limits.
    /// Integer command results/flags remain exact; pass this beside GP0 vertices.
    pub fn project_precise(&self, vector: usize) -> Result<[f32; 2]> {
        if vector >= 3 {
            return Err(Error("GTE vector index must be 0..2".into()));
        }
        let s16 = |v: u32| f64::from(v as i16);
        let xyz = [
            s16(self.data[vector * 2]),
            s16(self.data[vector * 2] >> 16),
            s16(self.data[vector * 2 + 1]),
        ];
        let matrix: [f64; 9] = std::array::from_fn(|i| s16(self.control[i / 2] >> ((i & 1) * 16)));
        let t: [f64; 3] = std::array::from_fn(|r| {
            f64::from(self.control[5 + r] as i32)
                + (0..3)
                    .map(|c| matrix[r * 3 + c] * xyz[c] / 4096.0)
                    .sum::<f64>()
        });
        let h = f64::from(self.control[26] as u16);
        let ratio = if t[2] > 0.0 {
            (h / t[2]).min(131071.0 / 65536.0)
        } else {
            131071.0 / 65536.0
        };
        Ok(std::array::from_fn(|i| {
            ((f64::from(self.control[24 + i] as i32) / 65536.0
                + t[i].clamp(-32768.0, 32767.0) * ratio)
                .clamp(-1024.0, 1023.0)) as f32
        }))
    }
}
