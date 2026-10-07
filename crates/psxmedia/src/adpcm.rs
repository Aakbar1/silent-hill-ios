use crate::Error;

const FILTERS: [(i32, i32); 5] = [(0, 0), (60, 0), (115, -52), (98, -55), (122, -60)];

/// Native sample rate/channel count. PCM is always i16 and interleaved if stereo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u8,
    /// Encoded source depth (4 or 8); decoded PCM samples always have 16 bits.
    pub adpcm_bits_per_sample: u8,
}

impl AudioFormat {
    pub fn from_xa_coding(coding: u8) -> Result<Self, Error> {
        if coding & 0xaa != 0 {
            return Err(Error::Invalid("reserved XA coding bits"));
        }
        if coding & 0x40 != 0 {
            return Err(Error::Unsupported("XA emphasis filter"));
        }
        Ok(Self {
            sample_rate: if coding & 4 == 0 { 37_800 } else { 18_900 },
            channels: if coding & 1 == 0 { 1 } else { 2 },
            adpcm_bits_per_sample: if coding & 16 == 0 { 4 } else { 8 },
        })
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct History {
    previous: i32,
    older: i32,
}
impl History {
    fn sample(&mut self, signed: i32, shift: u8, filter: usize) -> i16 {
        let (positive, negative) = FILTERS[filter];
        let predicted = (self.previous * positive + self.older * negative + 32) >> 6;
        let value = ((signed >> shift) + predicted).clamp(-32768, 32767);
        self.older = self.previous;
        self.previous = value;
        value as i16
    }
}

fn parameter(p: u8, filters: usize) -> Result<(u8, usize), Error> {
    let filter = (p >> 4) as usize;
    if filter >= filters {
        return Err(Error::Invalid("ADPCM predictor"));
    }
    // Reserved shifts 13..15 behave as shift 9 on PlayStation hardware.
    let shift = if p & 15 > 12 { 9 } else { p & 15 };
    Ok((shift, filter))
}

/// Stateful XA decoder. Use a separate instance for every CD file/channel pair.
#[derive(Debug, Default, Clone)]
pub struct XaDecoder {
    history: [History; 2],
}
impl XaDecoder {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Decode the 2304-byte sound-group region (excluding the 8-byte subheader).
    /// The additional 20 padding bytes in a Form 2 payload are ignored.
    pub fn decode(&mut self, data: &[u8], format: AudioFormat) -> Result<Vec<i16>, Error> {
        if data.len() < 2304 {
            return Err(Error::Truncated);
        }
        if !matches!(format.channels, 1 | 2) || !matches!(format.adpcm_bits_per_sample, 4 | 8) {
            return Err(Error::Invalid("XA format"));
        }
        let units = if format.adpcm_bits_per_sample == 4 {
            8
        } else {
            4
        };
        // Validate the entire sector before changing predictor state.
        for group in data[..2304].as_chunks::<128>().0 {
            for unit in 0..units {
                parameter(group[4 + unit], 4)?;
            }
        }
        let mut pcm = Vec::with_capacity(18 * units * 28);
        for group in data[..2304].as_chunks::<128>().0 {
            for pair in (0..units).step_by(format.channels as usize) {
                let mut decoded = [[0i16; 28]; 2];
                for (channel, samples) in decoded
                    .iter_mut()
                    .enumerate()
                    .take(format.channels as usize)
                {
                    let unit = pair + channel;
                    let (shift, filter) = parameter(group[4 + unit], 4)?;
                    for (i, output) in samples.iter_mut().enumerate() {
                        let signed = if format.adpcm_bits_per_sample == 4 {
                            let byte = group[16 + i * 4 + unit / 2];
                            let nibble = (byte >> ((unit & 1) * 4)) & 15;
                            ((nibble as i32) << 28) >> 16
                        } else {
                            (group[16 + i * 4 + unit] as i8 as i32) << 8
                        };
                        *output = self.history[channel].sample(signed, shift, filter);
                    }
                }
                for (&left, &right) in decoded[0].iter().zip(&decoded[1]) {
                    pcm.push(left);
                    if format.channels == 2 {
                        pcm.push(right);
                    }
                }
            }
        }
        Ok(pcm)
    }
}

/// One SPU block: 28 decoded samples and the three independent loop flags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpuBlock {
    pub pcm: [i16; 28],
    pub end: bool,
    pub repeat: bool,
    pub loop_start: bool,
}

/// Stateful raw SPU-ADPCM decoder. Reset for a new sample/voice.
#[derive(Debug, Default, Clone)]
pub struct SpuDecoder {
    history: History,
}
impl SpuDecoder {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    pub fn decode_block(&mut self, block: &[u8]) -> Result<SpuBlock, Error> {
        if block.len() != 16 {
            return Err(Error::Invalid("SPU block must have 16 bytes"));
        }
        let (shift, filter) = parameter(block[0], 5)?;
        let mut pcm = [0; 28];
        for (i, sample) in pcm.iter_mut().enumerate() {
            let nibble = (block[2 + i / 2] >> ((i & 1) * 4)) & 15;
            *sample = self
                .history
                .sample(((nibble as i32) << 28) >> 16, shift, filter);
        }
        Ok(SpuBlock {
            pcm,
            end: block[1] & 1 != 0,
            repeat: block[1] & 2 != 0,
            loop_start: block[1] & 4 != 0,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VagSample {
    pub sample_rate: u32,
    pub pcm: Vec<i16>,
    /// Sample indices for the inclusive start and exclusive end of the loop.
    pub loop_range: Option<std::ops::Range<usize>>,
    pub ended: bool,
}

/// Decode a standard mono `VAGp` file. Loop repetitions are metadata, not expanded.
/// Raw VAB sample bodies should instead be fed to [`SpuDecoder`].
pub fn decode_vag(data: &[u8]) -> Result<VagSample, Error> {
    if data.len() < 48 {
        return Err(Error::Truncated);
    }
    if &data[..4] != b"VAGp" {
        return Err(Error::Invalid("VAG magic"));
    }
    let size = u32::from_be_bytes(data[12..16].try_into().expect("header")) as usize;
    let sample_rate = u32::from_be_bytes(data[16..20].try_into().expect("header"));
    if sample_rate == 0 || !size.is_multiple_of(16) {
        return Err(Error::Invalid("VAG size/rate"));
    }
    let body = data.get(48..48 + size).ok_or(Error::Truncated)?;
    let mut decoder = SpuDecoder::default();
    let mut sample = VagSample {
        sample_rate,
        pcm: Vec::new(),
        loop_range: None,
        ended: false,
    };
    let mut loop_start = 0;
    for block in body.as_chunks::<16>().0 {
        let block = decoder.decode_block(block)?;
        if block.loop_start {
            loop_start = sample.pcm.len();
        }
        sample.pcm.extend_from_slice(&block.pcm);
        if block.end {
            sample.ended = true;
            if block.repeat {
                sample.loop_range = Some(loop_start..sample.pcm.len());
            }
            break;
        }
    }
    Ok(sample)
}
