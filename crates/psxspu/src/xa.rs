//! XA sectors decode off the realtime thread. Stateful history is per selected
//! file/channel and must reset on seek. Output is CD input at 44.1 kHz.
use crate::{Error, Frame, mul, sat};
use psxmedia::{AudioFormat, Sector, XaDecoder};
#[path = "zigzag.rs"]
mod zigzag;

#[derive(Default, Clone)]
pub struct XaResampler {
    ring: [[i16; 32]; 2],
    position: usize,
    six: usize,
}
impl XaResampler {
    /// Each group of six input frames yields seven. Retains phase/history
    /// across sectors. 18.9 kHz duplicates into the 37.8 kHz input clock.
    pub fn process(
        &mut self,
        pcm: &[i16],
        format: AudioFormat,
        output: &mut Vec<Frame>,
    ) -> Result<(), Error> {
        if !matches!(format.sample_rate, 18_900 | 37_800)
            || !matches!(format.channels, 1 | 2)
            || !pcm.len().is_multiple_of(usize::from(format.channels))
        {
            return Err(Error::Invalid("XA resampler input"));
        }
        for input in pcm.chunks_exact(usize::from(format.channels)) {
            let frame = [input[0], *input.get(1).unwrap_or(&input[0])];
            for _ in 0..if format.sample_rate == 18_900 { 2 } else { 1 } {
                for (ch, &sample) in frame.iter().enumerate() {
                    self.ring[ch][self.position] = sample;
                }
                self.position = (self.position + 1) & 31;
                self.six += 1;
                if self.six == 6 {
                    self.six = 0;
                    for table in zigzag::ZIGZAG {
                        output.push(std::array::from_fn(|ch| {
                            let mut sum = 0;
                            for (i, coef) in table.into_iter().enumerate() {
                                sum += mul(
                                    i32::from(
                                        self.ring[ch][self.position.wrapping_sub(i + 1) & 31],
                                    ),
                                    coef,
                                );
                            }
                            sat(sum)
                        }));
                    }
                }
            }
        }
        Ok(())
    }
}
pub struct XaStream {
    pub filter: (u8, u8),
    decoder: XaDecoder,
    resampler: XaResampler,
    format: Option<AudioFormat>,
    pub paused: bool,
    pub ended: bool,
    pub decoded_frames: u64,
    /// CD controller LL, LR, RL, RR volumes, 0x80 is unity.
    pub matrix: [u8; 4],
}
impl XaStream {
    pub fn new(file: u8, channel: u8) -> Self {
        Self {
            filter: (file, channel),
            decoder: XaDecoder::default(),
            resampler: XaResampler::default(),
            format: None,
            paused: false,
            ended: false,
            decoded_frames: 0,
            matrix: [128, 0, 0, 128],
        }
    }
    pub fn seek(&mut self, file: u8, channel: u8) {
        let matrix = self.matrix;
        *self = Self::new(file, channel);
        self.matrix = matrix;
    }
    pub fn feed_sector(&mut self, bytes: &[u8], output: &mut Vec<Frame>) -> Result<bool, Error> {
        if self.paused || self.ended {
            return Ok(false);
        }
        let sector = Sector::parse(bytes)?;
        if !sector.is_audio() || (sector.file, sector.channel) != self.filter {
            return Ok(false);
        }
        let format = AudioFormat::from_xa_coding(sector.coding)?;
        if self.format.is_some_and(|f| f != format) {
            return Err(Error::Invalid("XA coding changed without seek"));
        }
        let pcm = self.decoder.decode(sector.payload, format)?;
        self.format = Some(format);
        let start = output.len();
        self.resampler.process(&pcm, format, output)?;
        let [ll, lr, rl, rr] = self.matrix.map(i32::from);
        for frame in &mut output[start..] {
            let [l, r] = frame.map(i32::from);
            *frame = [sat((l * ll + r * rl) >> 7), sat((l * lr + r * rr) >> 7)];
        }
        self.decoded_frames += (output.len() - start) as u64;
        self.ended = sector.submode & 0x80 != 0;
        Ok(true)
    }
}
