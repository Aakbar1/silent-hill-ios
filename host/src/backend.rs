// SPDX-License-Identifier: GPL-3.0-only
//! Hardware seams: PS1 packets/VRAM and SPU registers/RAM, never game-level audio tasks.
use crate::raster::Raster;

/// Packet slices include the 32-bit tag followed by GP0 words. OT links are resolved
/// by C; packets arrive in traversal order between begin/end_ordering_table.
pub trait GpuBackend: Send {
    fn begin_ordering_table(&mut self) {}
    fn packet(&mut self, words: &[u32]);
    fn end_ordering_table(&mut self) {}
    fn env(&mut self, clip: [i32; 4], offset: [i32; 2]);
    fn load(&mut self, rect: [i32; 4], pixels: &[u16]);
    fn read(&self, rect: [i32; 4]) -> Vec<u16>;
    fn clear(&mut self, rect: [i32; 4], color: [u8; 3]);
    fn frame(&self, x: i32, y: i32, w: u32, h: u32) -> Vec<u32>;
    fn primitives(&self) -> u64;
}

impl GpuBackend for Raster {
    fn packet(&mut self, words: &[u32]) {
        self.packet(words);
    }
    fn env(&mut self, clip: [i32; 4], offset: [i32; 2]) {
        self.env(clip, offset);
    }
    fn load(&mut self, [x, y, w, h]: [i32; 4], pixels: &[u16]) {
        self.load(x, y, w, h, pixels);
    }
    fn read(&self, rect: [i32; 4]) -> Vec<u16> {
        self.read(rect)
    }
    fn clear(&mut self, rect: [i32; 4], color: [u8; 3]) {
        self.clear(rect, color);
    }
    fn frame(&self, x: i32, y: i32, w: u32, h: u32) -> Vec<u32> {
        self.frame(x, y, w, h)
    }
    fn primitives(&self) -> u64 {
        self.primitives
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VoiceRegisters {
    pub volume_left: u16,
    pub volume_right: u16,
    pub pitch: u16,
    pub start_address: u16,
    pub adsr_low: u16,
    pub adsr_high: u16,
    pub envelope: u16,
    pub repeat_address: u16,
}

/// Registers use even byte offsets from 0x1f801c00; sample addresses are SPU RAM
/// byte offsets. Register address units inside voice registers remain PS1 units.
pub trait SpuBackend: Send {
    fn reset(&mut self);
    fn write_register(&mut self, offset: u16, value: u16) -> Result<(), String>;
    fn read_register(&self, offset: u16) -> Result<u16, String>;
    fn transfer_write(&mut self, address: u32, bytes: &[u8]) -> Result<(), String>;
    fn transfer_read(&self, address: u32, bytes: &mut [u8]) -> Result<(), String>;
    fn voice(&mut self, index: u8, voice: VoiceRegisters) -> Result<(), String> {
        if index >= 24 {
            return Err("SPU voice index exceeds 23".into());
        }
        for (i, value) in [
            voice.volume_left,
            voice.volume_right,
            voice.pitch,
            voice.start_address,
            voice.adsr_low,
            voice.adsr_high,
            voice.envelope,
            voice.repeat_address,
        ]
        .into_iter()
        .enumerate()
        {
            self.write_register(u16::from(index) * 16 + i as u16 * 2, value)?;
        }
        Ok(())
    }
}

/// PORT: Boot fallback stores hardware state but produces no sound. The game-owned
/// sequencer is not replaced; psxspu can implement this trait when integrated.
pub struct SilentSpu {
    registers: [u16; 256],
    ram: Vec<u8>,
}
impl Default for SilentSpu {
    fn default() -> Self {
        Self {
            registers: [0; 256],
            ram: vec![0; 512 * 1024],
        }
    }
}
impl SpuBackend for SilentSpu {
    fn reset(&mut self) {
        self.registers.fill(0);
        self.ram.fill(0);
    }
    fn write_register(&mut self, offset: u16, value: u16) -> Result<(), String> {
        if offset & 1 != 0 {
            return Err("unaligned SPU register".into());
        }
        let slot = self
            .registers
            .get_mut(usize::from(offset / 2))
            .ok_or("SPU register outside range")?;
        *slot = value;
        Ok(())
    }
    fn read_register(&self, offset: u16) -> Result<u16, String> {
        if offset & 1 != 0 {
            return Err("unaligned SPU register".into());
        }
        self.registers
            .get(usize::from(offset / 2))
            .copied()
            .ok_or_else(|| "SPU register outside range".into())
    }
    fn transfer_write(&mut self, address: u32, bytes: &[u8]) -> Result<(), String> {
        let start = address as usize;
        let end = start
            .checked_add(bytes.len())
            .ok_or("SPU transfer overflow")?;
        self.ram
            .get_mut(start..end)
            .ok_or("SPU transfer outside RAM")?
            .copy_from_slice(bytes);
        Ok(())
    }
    fn transfer_read(&self, address: u32, bytes: &mut [u8]) -> Result<(), String> {
        let start = address as usize;
        let end = start
            .checked_add(bytes.len())
            .ok_or("SPU transfer overflow")?;
        bytes.copy_from_slice(self.ram.get(start..end).ok_or("SPU transfer outside RAM")?);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spu_voice_units_transfer_bounds_and_reset() {
        let mut spu = SilentSpu::default();
        spu.voice(
            23,
            VoiceRegisters {
                pitch: 0x1000,
                start_address: 17,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(spu.read_register(23 * 16 + 4).unwrap(), 0x1000);
        assert_eq!(spu.read_register(23 * 16 + 6).unwrap(), 17);
        assert!(spu.write_register(1, 0).is_err());
        assert!(spu.voice(24, VoiceRegisters::default()).is_err());
        spu.transfer_write(512 * 1024 - 2, &[1, 2]).unwrap();
        assert!(spu.transfer_write(512 * 1024 - 1, &[1, 2]).is_err());
        let mut bytes = [0; 2];
        spu.transfer_read(512 * 1024 - 2, &mut bytes).unwrap();
        assert_eq!(bytes, [1, 2]);
        spu.reset();
        spu.transfer_read(512 * 1024 - 2, &mut bytes).unwrap();
        assert_eq!(bytes, [0; 2]);
    }
    #[test]
    fn software_backend_vram_roundtrip() {
        let mut gpu: Box<dyn GpuBackend> = Box::<Raster>::default();
        gpu.load([100, 20, 2, 1], &[0x1234, 0x5678]);
        assert_eq!(gpu.read([100, 20, 2, 1]), [0x1234, 0x5678]);
    }
}
