//! Safe Rust PsyQ service boundary; core converts its native C structs here.
//! Never transmute LP64 C longs or pointer-bearing game structs into these types.
use crate::{Error, Spu, VOICES, reverb::Preset};

#[derive(Debug, Clone, Copy, Default)]
pub struct VoiceAttr {
    pub voices: u32,
    pub mask: u32,
    pub volume: [i16; 2],
    pub volume_mode: [u8; 2],
    pub pitch: u16,
    pub address: u32,
    pub loop_address: u32,
    pub attack_mode: u8,
    pub sustain_mode: u8,
    pub release_mode: u8,
    pub attack_rate: u16,
    pub decay_rate: u16,
    pub sustain_rate: u16,
    pub release_rate: u16,
    pub sustain_level: u16,
    pub adsr1: u16,
    pub adsr2: u16,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct CommonAttr {
    pub mask: u32,
    pub main_volume: [i16; 2],
    pub main_mode: [u8; 2],
    pub cd_volume: [i16; 2],
    pub cd_mix: bool,
    pub cd_reverb: bool,
    pub external_volume: [i16; 2],
    pub external_mix: bool,
    pub external_reverb: bool,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct ReverbAttr {
    pub mask: u32,
    pub mode: u16,
    pub depth: [i16; 2],
    pub delay: u8,
    pub feedback: u8,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum KeyStatus {
    InvalidMask = -1,
    Off = 0,
    On = 1,
    OffEnvelopeOn = 2,
    OnEnvelopeOff = 3,
}

fn volume_register(volume: i16, mode: u8) -> Result<u16, Error> {
    let mode = match mode {
        0 => return Ok(volume as u16 & 0x7fff),
        1 => 0x8000,
        2 => 0x9000,
        3 => 0xa000,
        4 => 0xb000,
        5 => 0xc000,
        6 => 0xd000,
        7 => 0xe000,
        _ => return Err(Error::Invalid("PsyQ volume mode")),
    };
    Ok(mode | (volume as u16 & 127))
}
fn selected(mask: u32, bit: usize) -> bool {
    mask == 0 || mask & (1 << bit) != 0
}
fn signed_volume(value: u16) -> i16 {
    ((value << 1) as i16) / 2
}
fn volume_mode(register: u16) -> u8 {
    match register >> 12 {
        8 => 1,
        9 => 2,
        10 => 3,
        11 => 4,
        12 => 5,
        13 => 6,
        14 => 7,
        _ => 0,
    }
}

impl Spu {
    pub fn init(&mut self) {
        self.reset();
        self.key_off(0xffffff);
        for i in 0..VOICES {
            for reg in 0..8 {
                self.write_register((i * 16 + reg * 2) as u32, 0).unwrap();
            }
        }
        for offset in (0x180..0x280).step_by(2) {
            self.write_register(offset, 0).unwrap();
        }
        self.write_register(0x180, 0x3fff).unwrap();
        self.write_register(0x182, 0x3fff).unwrap();
        self.write_register(0x1ac, 4).unwrap();
        self.write_register(0x1aa, 0xc000).unwrap();
        self.reverb.set_preset(Preset::Off);
        self.clear_reverb();
    }
    pub fn quit(&mut self) {
        self.key_off(0xffffff);
        self.write_register(0x1aa, 0).unwrap();
    }
    pub fn set_voice_attr(&mut self, a: &VoiceAttr) -> Result<(), Error> {
        if a.mask & 0x60 != 0 {
            return Err(Error::Unsupported(
                "PsyQ NOTE/SAMPLE_NOTE: use game Note2Pitch and PITCH",
            ));
        }
        // Validate before changing any voice.
        for ch in 0..2 {
            if selected(a.mask, ch + 2) {
                volume_register(a.volume[ch], a.volume_mode[ch])?;
            }
        }
        if selected(a.mask, 7) && a.address.div_ceil(8) > u16::MAX.into()
            || selected(a.mask, 16) && a.loop_address.div_ceil(8) > u16::MAX.into()
        {
            return Err(Error::OutOfSoundRam);
        }
        for i in 0..VOICES {
            if a.voices & (1 << i) == 0 {
                continue;
            }
            let base = i as u32 * 16;
            for ch in 0..2 {
                if selected(a.mask, ch) || selected(a.mask, ch + 2) {
                    let old = self.read_register(base + ch as u32 * 2)?;
                    let volume = if selected(a.mask, ch) {
                        a.volume[ch]
                    } else {
                        signed_volume(old)
                    };
                    let mode = if selected(a.mask, ch + 2) {
                        a.volume_mode[ch]
                    } else {
                        volume_mode(old)
                    };
                    self.write_register(base + ch as u32 * 2, volume_register(volume, mode)?)?;
                }
            }
            if selected(a.mask, 4) {
                self.write_register(base + 4, a.pitch)?;
            }
            if selected(a.mask, 7) {
                self.write_register(base + 6, a.address.div_ceil(8) as u16)?;
            }
            if selected(a.mask, 16) {
                self.write_register(base + 14, a.loop_address.div_ceil(8) as u16)?;
            }
            let mut first = if selected(a.mask, 17) {
                a.adsr1
            } else {
                self.read_register(base + 8)?
            };
            let mut second = if selected(a.mask, 18) {
                a.adsr2
            } else {
                self.read_register(base + 10)?
            };
            for (bit, mask, value) in [
                (11, 0x7f00, (a.attack_rate & 127) << 8),
                (12, 0xf0, (a.decay_rate & 15) << 4),
                (15, 15, a.sustain_level & 15),
            ] {
                if selected(a.mask, bit) {
                    first = (first & !mask) | value;
                }
            }
            for (bit, mask, value) in [
                (13, 0x1fc0, (a.sustain_rate & 127) << 6),
                (14, 31, a.release_rate & 31),
            ] {
                if selected(a.mask, bit) {
                    second = (second & !mask) | value;
                }
            }
            if selected(a.mask, 8) {
                first = (first & 0x7fff) | if a.attack_mode == 5 { 0x8000 } else { 0 };
            }
            if selected(a.mask, 9) {
                second = (second & 0x3fff)
                    | match a.sustain_mode {
                        3 => 0x4000,
                        5 => 0x8000,
                        7 => 0xc000,
                        _ => 0,
                    };
            }
            if selected(a.mask, 10) {
                second = (second & !32) | if a.release_mode == 7 { 32 } else { 0 };
            }
            self.write_register(base + 8, first)?;
            self.write_register(base + 10, second)?;
        }
        Ok(())
    }
    pub fn key_on_with_attr(&mut self, a: &VoiceAttr) -> Result<(), Error> {
        self.set_voice_attr(a)?;
        self.key_on(a.voices);
        Ok(())
    }
    /// PsyQ selects the lowest set bit when several voices are supplied.
    pub fn key_status(&self, mask: u32) -> KeyStatus {
        let Some(v) = self.voice(mask.trailing_zeros() as usize) else {
            return KeyStatus::InvalidMask;
        };
        if v.keyed && (v.envelope != 0 || v.pending_key_on) {
            KeyStatus::On
        } else if v.envelope != 0 {
            KeyStatus::OffEnvelopeOn
        } else if v.keyed {
            KeyStatus::OnEnvelopeOff
        } else {
            KeyStatus::Off
        }
    }
    pub fn get_voice_attr(&self, index: usize) -> Result<VoiceAttr, Error> {
        let v = self.voice(index).ok_or(Error::Invalid("voice index"))?;
        let r = v.registers;
        Ok(VoiceAttr {
            voices: 1 << index,
            mask: 0x7ff9f,
            volume: [signed_volume(r[0]), signed_volume(r[1])],
            volume_mode: [volume_mode(r[0]), volume_mode(r[1])],
            pitch: r[2],
            address: u32::from(r[3]) * 8,
            loop_address: u32::from(r[7]) * 8,
            adsr1: r[4],
            adsr2: r[5],
            attack_rate: (r[4] >> 8) & 127,
            decay_rate: (r[4] >> 4) & 15,
            sustain_level: r[4] & 15,
            sustain_rate: (r[5] >> 6) & 127,
            release_rate: r[5] & 31,
            attack_mode: if r[4] & 0x8000 != 0 { 5 } else { 1 },
            sustain_mode: match r[5] >> 14 {
                0 => 1,
                1 => 3,
                2 => 5,
                _ => 7,
            },
            release_mode: if r[5] & 32 != 0 { 7 } else { 3 },
        })
    }
    pub fn set_common_attr(&mut self, a: &CommonAttr) -> Result<(), Error> {
        for ch in 0..2 {
            if selected(a.mask, ch) || selected(a.mask, ch + 2) {
                let old = self.read_register(0x180 + ch as u32 * 2)?;
                let volume = if selected(a.mask, ch) {
                    a.main_volume[ch]
                } else {
                    signed_volume(old)
                };
                let mode = if selected(a.mask, ch + 2) {
                    a.main_mode[ch]
                } else {
                    volume_mode(old)
                };
                self.write_register(0x180 + ch as u32 * 2, volume_register(volume, mode)?)?;
            }
            if selected(a.mask, ch + 6) {
                self.write_register(0x1b0 + ch as u32 * 2, a.cd_volume[ch] as u16)?;
            }
            if selected(a.mask, ch + 10) {
                self.write_register(0x1b4 + ch as u32 * 2, a.external_volume[ch] as u16)?;
            }
        }
        let mut control = self.read_register(0x1aa)?;
        for (mask, bit, on) in [
            (8, 4, a.cd_reverb),
            (9, 1, a.cd_mix),
            (12, 8, a.external_reverb),
            (13, 2, a.external_mix),
        ] {
            if selected(a.mask, mask) {
                control = (control & !bit) | if on { bit } else { 0 };
            }
        }
        self.write_register(0x1aa, control)
    }
    pub fn set_reverb(&mut self, enabled: bool) {
        let c = self.read_register(0x1aa).unwrap();
        self.write_register(0x1aa, (c & !128) | if enabled { 128 } else { 0 })
            .unwrap();
    }
    pub fn set_reverb_voice(&mut self, enabled: bool, mask: u32) -> u32 {
        let old = self.reverb_voice();
        let new = if enabled { old | mask } else { old & !mask };
        self.write_register(0x198, new as u16).unwrap();
        self.write_register(0x19a, (new >> 16) as u16).unwrap();
        new & 0xffffff
    }
    pub fn reverb_voice(&self) -> u32 {
        u32::from(self.read_register(0x198).unwrap())
            | u32::from(self.read_register(0x19a).unwrap() & 255) << 16
    }
    pub fn set_reverb_preset(&mut self, preset: Preset, clear: bool) {
        self.reverb.set_preset(preset);
        if clear {
            self.clear_reverb();
        }
    }
    pub fn set_reverb_attr(&mut self, a: &ReverbAttr) -> Result<(), Error> {
        if a.mask & 24 != 0 {
            return Err(Error::Unsupported(
                "PsyQ reverb delay/feedback attributes: use register coefficients",
            ));
        }
        if a.mask == 0 || a.mask & 1 != 0 {
            let preset =
                Preset::from_id((a.mode & 255) as u8).ok_or(Error::Invalid("reverb preset"))?;
            self.reverb.volume = [0; 2];
            self.set_reverb_preset(preset, a.mode & 256 != 0);
        }
        for ch in 0..2 {
            if a.mask == 0 || a.mask & (2 << ch) != 0 {
                self.reverb.volume[ch] = a.depth[ch];
            }
        }
        Ok(())
    }
    pub fn set_transfer_mode(&mut self, manual: bool) {
        let c = self.read_register(0x1aa).unwrap();
        self.write_register(0x1aa, (c & !0x30) | if manual { 0x10 } else { 0x20 })
            .unwrap();
    }
    pub fn transfer_completed(&self) -> bool {
        true
    }
}

/// Optional replacement for PsyQ's allocation records. The game-owned SdSpu
/// allocator may instead retain ownership. Addresses are byte offsets, never pointers.
pub struct Allocator {
    blocks: [Option<(usize, usize)>; 64],
    capacity: usize,
    limit: usize,
}
impl Allocator {
    pub fn new(records: usize) -> Result<Self, Error> {
        if records == 0 || records > 64 {
            return Err(Error::Invalid("SPU allocation record count"));
        }
        Ok(Self {
            blocks: [None; 64],
            capacity: records,
            limit: crate::RAM_BYTES,
        })
    }
    pub fn reserve_reverb(&mut self, preset: Preset, reserve: bool) -> Result<(), Error> {
        let limit = if reserve {
            if preset == Preset::Off {
                crate::RAM_BYTES - 128
            } else {
                usize::from(preset.base()) * 8
            }
        } else {
            crate::RAM_BYTES
        };
        if self.blocks.iter().flatten().any(|&(a, n)| a + n > limit) {
            return Err(Error::OutOfSoundRam);
        }
        self.limit = limit;
        Ok(())
    }
    pub fn allocate_at(&mut self, address: usize, size: usize) -> Result<usize, Error> {
        let size = size.checked_add(7).ok_or(Error::OutOfSoundRam)? & !7;
        if size == 0
            || address < 0x1010
            || !address.is_multiple_of(8)
            || address.checked_add(size).is_none_or(|e| e > self.limit)
            || self
                .blocks
                .iter()
                .flatten()
                .any(|&(a, n)| address < a + n && a < address + size)
        {
            return Err(Error::OutOfSoundRam);
        }
        let slot = self.blocks[..self.capacity]
            .iter_mut()
            .find(|b| b.is_none())
            .ok_or(Error::OutOfSoundRam)?;
        *slot = Some((address, size));
        Ok(address)
    }
    pub fn allocate(&mut self, size: usize) -> Result<usize, Error> {
        let size = size.checked_add(7).ok_or(Error::OutOfSoundRam)? & !7;
        let mut address = 0x1010usize;
        loop {
            let end = address.checked_add(size).ok_or(Error::OutOfSoundRam)?;
            if let Some(&(a, n)) = self
                .blocks
                .iter()
                .flatten()
                .find(|&&(a, n)| address < a + n && a < end)
            {
                address = a + n;
            } else {
                return self.allocate_at(address, size);
            }
        }
    }
    pub fn free(&mut self, address: usize) -> bool {
        if let Some(b) = self
            .blocks
            .iter_mut()
            .find(|b| b.is_some_and(|(a, _)| a == address))
        {
            *b = None;
            true
        } else {
            false
        }
    }
}
