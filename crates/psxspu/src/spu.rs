use crate::{
    Adsr, AdsrPhase, Error, Frame, RAM_BYTES, VOICES, Volume, gaussian::GAUSS, mul, reverb::Reverb,
    sat,
};
use psxmedia::SpuDecoder;

pub const REGISTER_BASE: u32 = 0x1f80_1c00;
const RAM_MASK: usize = RAM_BYTES - 1;

#[derive(Default, Clone)]
struct Voice {
    registers: [u16; 8],
    volume: [Volume; 2],
    envelope: Adsr,
    decoder: SpuDecoder,
    decoded: [i16; 31],
    phase: u32,
    address: usize,
    loaded: bool,
    flags: u8,
    keyed: bool,
    pending_key_on: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct VoiceState {
    pub registers: [u16; 8],
    pub volume: [i16; 2],
    pub envelope: u16,
    pub phase: AdsrPhase,
    pub keyed: bool,
    pub pending_key_on: bool,
    pub address: usize,
    pub pitch_counter: u32,
}

pub struct Spu {
    ram: Box<[u8]>,
    voices: [Voice; VOICES],
    registers: [u16; 320],
    main_volume: [Volume; 2],
    pub reverb: Reverb,
    endx: u32,
    transfer: usize,
    noise_timer: i32,
    noise_level: u16,
    capture: usize,
    irq: bool,
    samples: u64,
    decode_errors: u64,
}
impl Default for Spu {
    fn default() -> Self {
        Self::new()
    }
}
impl Spu {
    pub fn new() -> Self {
        Self {
            ram: vec![0; RAM_BYTES].into_boxed_slice(),
            voices: std::array::from_fn(|_| Voice::default()),
            registers: [0; 320],
            main_volume: [Volume::default(); 2],
            reverb: Reverb::default(),
            endx: 0,
            transfer: 0,
            noise_timer: 0,
            noise_level: 0,
            capture: 0,
            irq: false,
            samples: 0,
            decode_errors: 0,
        }
    }
    pub fn ram(&self) -> &[u8] {
        &self.ram
    }
    pub fn reset(&mut self) {
        self.voices = std::array::from_fn(|_| Voice::default());
        self.registers.fill(0);
        self.main_volume = [Volume::default(); 2];
        self.reverb = Reverb::default();
        self.endx = 0;
        self.transfer = 0;
        self.noise_timer = 0;
        self.noise_level = 0;
        self.capture = 0;
        self.irq = false;
        self.samples = 0;
        self.decode_errors = 0;
    }
    pub fn clear_reverb(&mut self) {
        self.reverb.clear(&mut self.ram);
    }
    pub fn clear_reverb_work_area(&mut self, preset: crate::reverb::Preset) {
        self.ram[RAM_BYTES - preset.work_bytes()..].fill(0);
    }
    pub fn sample_clock(&self) -> u64 {
        self.samples
    }
    pub fn decode_errors(&self) -> u64 {
        self.decode_errors
    }
    pub fn irq_pending(&self) -> bool {
        self.irq
    }
    pub fn endx(&self) -> u32 {
        self.endx
    }
    pub fn voice(&self, index: usize) -> Option<VoiceState> {
        self.voices.get(index).map(|v| VoiceState {
            registers: v.registers,
            volume: v.volume.map(|v| v.current),
            envelope: v.envelope.level,
            phase: v.envelope.phase,
            keyed: v.keyed,
            pending_key_on: v.pending_key_on,
            address: v.address,
            pitch_counter: v.phase,
        })
    }
    fn index(address: u32) -> Result<usize, Error> {
        let offset = if address >= REGISTER_BASE {
            address - REGISTER_BASE
        } else {
            address
        };
        if offset >= 640 || offset & 1 != 0 {
            return Err(Error::Invalid("SPU register address"));
        }
        Ok(offset as usize / 2)
    }
    /// Accept full MMIO addresses or byte offsets from 0x1f801c00.
    pub fn read_register(&self, address: u32) -> Result<u16, Error> {
        let i = Self::index(address)?;
        Ok(match i * 2 {
            0..=0x17e => {
                let v = &self.voices[i / 8];
                if i % 8 == 6 {
                    v.envelope.level
                } else {
                    v.registers[i % 8]
                }
            }
            0x19c => self.endx as u16,
            0x19e => (self.endx >> 16) as u16,
            0x1a2 => self.reverb.base(),
            0x1ae => {
                (self.registers[0x1aa / 2] & 63)
                    | match self.registers[0x1aa / 2] & 0x30 {
                        0x20 => 0x180,
                        0x30 => 0x280,
                        _ => 0,
                    }
                    | if self.irq { 64 } else { 0 }
                    | if self.capture >= 0x200 && self.registers[0x1ac / 2] & 12 != 0 {
                        0x800
                    } else {
                        0
                    }
            }
            0x1b8 => self.main_volume[0].current as u16,
            0x1ba => self.main_volume[1].current as u16,
            0x200..=0x25e => self.voices[(i * 2 - 0x200) / 4].volume[i % 2].current as u16,
            0x1c0..=0x1fe => self.reverb.registers[(i * 2 - 0x1c0) / 2],
            0x184 => self.reverb.volume[0] as u16,
            0x186 => self.reverb.volume[1] as u16,
            _ => self.registers[i],
        })
    }
    pub fn write_register(&mut self, address: u32, value: u16) -> Result<(), Error> {
        let i = Self::index(address)?;
        if i < 24 * 8 {
            let v = &mut self.voices[i / 8];
            v.registers[i % 8] = value;
            match i % 8 {
                0 | 1 => v.volume[i % 8].write(value),
                6 => v.envelope.level = value & 0x7fff,
                _ => {}
            }
            return Ok(());
        }
        match i * 2 {
            0x180 | 0x182 => self.main_volume[(i * 2 - 0x180) / 2].write(value),
            0x184 | 0x186 => self.reverb.volume[(i * 2 - 0x184) / 2] = value as i16,
            0x188 => self.key_on(u32::from(value)),
            0x18a => self.key_on(u32::from(value & 255) << 16),
            0x18c => self.key_off(u32::from(value)),
            0x18e => self.key_off(u32::from(value & 255) << 16),
            0x19c | 0x19e | 0x1ae | 0x1b8 | 0x1ba | 0x200..=0x25e => return Ok(()),
            0x1a2 => self.reverb.set_base(value),
            0x1a6 => self.transfer = usize::from(value) * 8,
            0x1a8 => self.dma_write(&value.to_le_bytes()),
            0x1aa => {
                if value & 64 == 0 {
                    self.irq = false;
                }
            }
            0x1c0..=0x1fe => self.reverb.registers[(i * 2 - 0x1c0) / 2] = value,
            _ => {}
        }
        self.registers[i] = value;
        Ok(())
    }
    pub fn key_on(&mut self, mask: u32) {
        self.endx &= !mask;
        for (i, v) in self.voices.iter_mut().enumerate() {
            if mask & (1 << i) == 0 {
                continue;
            }
            v.envelope.key_on();
            v.decoder.reset();
            v.decoded.fill(0);
            v.phase = 0;
            v.address = usize::from(v.registers[3]) * 8;
            v.loaded = false;
            v.flags = 0;
            v.keyed = true;
            v.pending_key_on = true;
            // PORT: native SDK key-status polling observes KON immediately;
            // asynchronous bus-write and silicon key-on startup latency are omitted.
        }
    }
    pub fn key_off(&mut self, mask: u32) {
        for (i, v) in self.voices.iter_mut().enumerate() {
            if mask & (1 << i) != 0 {
                v.keyed = false;
                v.pending_key_on = false;
                v.envelope.key_off();
            }
        }
    }
    fn mask(&self, offset: usize) -> u32 {
        u32::from(self.registers[offset / 2])
            | u32::from(self.registers[offset / 2 + 1] & 255) << 16
    }
    fn touch_irq(&mut self, address: usize, length: usize) {
        if self.registers[0x1aa / 2] & 0x8040 != 0x8040 {
            return;
        }
        let target = usize::from(self.registers[0x1a4 / 2]) * 8;
        if target.wrapping_sub(address) & RAM_MASK < length {
            self.irq = true;
        }
    }
    /// Bounds-checked import/upload. Does not move the DMA cursor.
    pub fn upload(&mut self, address: usize, data: &[u8]) -> Result<(), Error> {
        let end = address
            .checked_add(data.len())
            .ok_or(Error::OutOfSoundRam)?;
        self.ram
            .get_mut(address..end)
            .ok_or(Error::OutOfSoundRam)?
            .copy_from_slice(data);
        self.touch_irq(address, data.len());
        Ok(())
    }
    /// PORT: synchronous native transfers omit FIFO/DMA bus contention/latency.
    /// The data and wraparound are preserved; completion can be reported immediately.
    pub fn dma_write(&mut self, data: &[u8]) {
        self.touch_irq(self.transfer, data.len());
        for &byte in data {
            self.ram[self.transfer] = byte;
            self.transfer = (self.transfer + 1) & RAM_MASK;
        }
    }
    pub fn dma_read(&mut self, output: &mut [u8]) {
        self.touch_irq(self.transfer, output.len());
        for byte in output {
            *byte = self.ram[self.transfer];
            self.transfer = (self.transfer + 1) & RAM_MASK;
        }
    }
    pub fn transfer_address(&self) -> usize {
        self.transfer
    }
    pub fn set_transfer_address(&mut self, address: usize) -> Result<usize, Error> {
        let rounded = address.checked_add(7).ok_or(Error::OutOfSoundRam)? & !7;
        if rounded >= RAM_BYTES {
            return Err(Error::OutOfSoundRam);
        }
        self.transfer = rounded;
        self.registers[0x1a6 / 2] = (rounded / 8) as u16;
        Ok(rounded)
    }
    fn noise(&mut self, control: u16) -> i16 {
        let step = i32::from((control >> 8) & 3) + 4;
        let shift = u32::from((control >> 10) & 15);
        self.noise_timer -= step;
        if self.noise_timer < 0 {
            let n = self.noise_level;
            let parity = ((n >> 15) ^ (n >> 12) ^ (n >> 11) ^ (n >> 10) ^ 1) & 1;
            self.noise_level = (n << 1) | parity;
            self.noise_timer += 0x20000 >> shift;
            if self.noise_timer < 0 {
                self.noise_timer += 0x20000 >> shift;
            }
        }
        self.noise_level as i16
    }
    fn voice_sample(
        &mut self,
        index: usize,
        noise: Option<i16>,
        previous: i32,
        modulate: bool,
    ) -> i32 {
        // PORT: fully inactive voices skip inaudible ADPCM reads, so their idle
        // hardware IRQ activity is not modeled. Game-used active voice IRQs remain.
        if self.voices[index].envelope.phase == AdsrPhase::Off {
            return 0;
        }
        let must_load = !self.voices[index].loaded || self.voices[index].phase >> 12 >= 28;
        if must_load {
            let v = &mut self.voices[index];
            if v.loaded {
                v.phase -= 28 << 12;
                if v.flags & 1 != 0 {
                    self.endx |= 1 << index;
                    v.address = usize::from(v.registers[7]) * 8;
                    if v.flags & 2 == 0 && noise.is_none() {
                        v.envelope.stop();
                        v.pending_key_on = false;
                        return 0;
                    }
                }
                v.decoded.copy_within(28..31, 0);
            }
            let address = v.address;
            self.touch_irq(address, 16);
            let mut block = [0; 16];
            for (i, byte) in block.iter_mut().enumerate() {
                *byte = self.ram[(address + i) & RAM_MASK];
            }
            let v = &mut self.voices[index];
            match v.decoder.decode_block(&block) {
                Ok(decoded) => {
                    v.decoded[3..].copy_from_slice(&decoded.pcm);
                    v.flags = block[1];
                    if decoded.loop_start {
                        v.registers[7] = (address / 8) as u16;
                    }
                }
                Err(_) => {
                    v.envelope.stop();
                    v.pending_key_on = false;
                    self.decode_errors += 1;
                    return 0;
                }
            }
            v.address = (address + 16) & RAM_MASK;
            v.loaded = true;
        }
        let v = &mut self.voices[index];
        let sample = if let Some(noise) = noise {
            i32::from(noise)
        } else {
            let i = ((v.phase >> 4) & 255) as usize;
            let pos = (v.phase >> 12) as usize;
            let s = &v.decoded[pos..pos + 4];
            mul(GAUSS[255 - i], i32::from(s[0]))
                + mul(GAUSS[511 - i], i32::from(s[1]))
                + mul(GAUSS[256 + i], i32::from(s[2]))
                + mul(GAUSS[i], i32::from(s[3]))
        };
        let output = mul(i32::from(sat(sample)), i32::from(v.envelope.level));
        let pitch = v.registers[2];
        let step = if modulate {
            (((i32::from(pitch as i16) * (previous + 32768)) >> 15) & 65535) as u32
        } else {
            u32::from(pitch)
        };
        v.phase += step.min(0x4000);
        v.envelope.tick(v.registers[4], v.registers[5]);
        v.pending_key_on = false;
        output
    }
    /// One native 44.1 kHz frame. CD and external inputs are already resampled
    /// PCM. All state is preallocated; this routine never locks or allocates.
    pub fn next_frame(&mut self, cd: Frame, external: Frame) -> Frame {
        let control = self.registers[0x1aa / 2];
        let noise = self.noise(control);
        let noise_mask = self.mask(0x194);
        let mod_mask = self.mask(0x190);
        let rev_mask = self.mask(0x198);
        let mut dry = [0i32; 2];
        let mut send = [0i32; 2];
        let mut previous = 0;
        let mut captures = [0i16; 2];
        if control & 0x8000 != 0 {
            for i in 0..VOICES {
                let sample = self.voice_sample(
                    i,
                    if noise_mask & (1 << i) != 0 {
                        Some(noise)
                    } else {
                        None
                    },
                    previous,
                    i != 0 && mod_mask & (1 << i) != 0,
                );
                previous = sample;
                if i == 1 {
                    captures[0] = sat(sample);
                } else if i == 3 {
                    captures[1] = sat(sample);
                }
                for ch in 0..2 {
                    let vol = self.voices[i].volume[ch].tick();
                    let value = mul(sample, i32::from(vol));
                    dry[ch] += value;
                    if rev_mask & (1 << i) != 0 {
                        send[ch] += value;
                    }
                }
            }
        } else {
            for v in &mut self.voices {
                v.envelope.stop();
                v.pending_key_on = false;
            }
        }
        // Voice mute does not mute CD/external input.
        if control & 0x4000 == 0 {
            dry = [0; 2];
            send = [0; 2];
        }
        for (input, enable, reverb, volume) in [(cd, 1, 4, 0x1b0), (external, 2, 8, 0x1b4)] {
            if control & enable != 0 {
                for ch in 0..2 {
                    let sample = mul(
                        i32::from(input[ch]),
                        i32::from(self.registers[volume / 2 + ch] as i16),
                    );
                    dry[ch] += sample;
                    if control & reverb != 0 {
                        send[ch] += sample;
                    }
                }
            }
        }
        let wet = self.reverb.process(
            &mut self.ram,
            send.map(sat),
            control & 128 != 0 && self.registers[0x1ac / 2] & 12 != 0,
        );
        for (i, sample) in [cd[0], cd[1], captures[0], captures[1]]
            .into_iter()
            .enumerate()
        {
            let addr = i * 0x400 + self.capture;
            self.ram[addr..addr + 2].copy_from_slice(&sample.to_le_bytes());
            if self.registers[0x1ac / 2] & 12 != 0 {
                self.touch_irq(addr, 2);
            }
        }
        self.capture = (self.capture + 2) & 0x3ff;
        self.samples += 1;
        std::array::from_fn(|ch| {
            sat(mul(
                i32::from(sat(dry[ch] + i32::from(wet[ch]))),
                i32::from(self.main_volume[ch].tick()),
            ))
        })
    }
    pub fn render(&mut self, output: &mut [Frame]) {
        for frame in output {
            *frame = self.next_frame([0; 2], [0; 2]);
        }
    }
    pub fn render_cd(&mut self, input: &[Frame], output: &mut [Frame]) -> Result<(), Error> {
        if input.len() != output.len() {
            return Err(Error::Invalid("CD input/output frame count"));
        }
        for (o, &i) in output.iter_mut().zip(input) {
            *o = self.next_frame(i, [0; 2]);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sustained(pitch: u16) -> Spu {
        let mut s = Spu::new();
        s.init();
        let mut block = [0x11; 16];
        block[0] = 0;
        block[1] = 7;
        s.upload(0x1010, &block).unwrap();
        for i in 0..2 {
            s.write_register(i * 16 + 6, 0x1010 / 8).unwrap();
            s.write_register(i * 16 + 4, pitch).unwrap();
            s.key_on(1 << i);
            s.voices[i as usize].envelope.phase = AdsrPhase::Sustain;
            s.voices[i as usize].envelope.level = 32767;
            s.voices[i as usize].registers[5] = 0x1fc0;
        }
        s
    }
    #[test]
    fn gaussian_constant_q15_products_have_hardware_rounding() {
        let mut s = sustained(4096);
        for _ in 0..4 {
            s.voice_sample(0, None, 0, false);
        }
        // gauss[255,511,256,0] = [4807,22963,4871,-1]. The per-product
        // arithmetic shifts yield 4077, then ENVX=32767 yields 4076.
        assert_eq!(s.voice_sample(0, None, 0, false), 4076);
    }
    #[test]
    fn pitch_modulation_uses_previous_enveloped_voice_not_channel_volume() {
        let mut s = sustained(4096);
        for _ in 0..4 {
            s.voice_sample(0, None, 0, false);
        }
        let previous = s.voice_sample(0, None, 0, false);
        let before = s.voices[1].phase;
        s.voice_sample(1, None, previous, true);
        assert_eq!(s.voices[1].phase - before, 4605);
    }
    #[test]
    fn adpcm_loop_preserves_predictor_history() {
        let mut s = sustained(4096);
        let mut block = [0x11; 16];
        block[0] = 0x1c;
        block[1] = 7;
        s.upload(0x1010, &block).unwrap();
        for _ in 0..29 {
            s.voice_sample(0, None, 0, false);
        }
        // x[n]=1+round(60*x[n-1]/64) reaches 9. Resetting predictor history
        // at the loop would incorrectly start the second block at 1.
        assert_eq!(s.voices[0].decoded[3], 9);
        assert_eq!(s.endx(), 1);
    }
    #[test]
    fn noise_feedback_register_goldens() {
        let mut s = Spu::new();
        let samples = std::array::from_fn::<_, 12, _>(|_| s.noise(0xfc00));
        assert_eq!(
            samples,
            [1, 3, 7, 15, 31, 63, 127, 255, 511, 1023, 2047, 4094]
        );
    }
}
