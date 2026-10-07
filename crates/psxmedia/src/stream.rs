use crate::{AudioFormat, Error, MdecDecoder, Sector, StrHeader, XaDecoder};
use std::collections::BTreeMap;

/// Exact media time, independent of wall-clock decode speed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timestamp {
    pub ticks: u64,
    pub timescale: u32,
}
impl Timestamp {
    pub fn seconds(self) -> f64 {
        self.ticks as f64 / self.timescale as f64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameRate {
    pub numerator: u32,
    pub denominator: u32,
}
impl FrameRate {
    pub const FPS_15: Self = Self {
        numerator: 15,
        denominator: 1,
    };
}

#[derive(Debug, Clone, Copy)]
pub struct StreamConfig {
    pub frame_rate: FrameRate,
    /// CD file/channel; None emits all video streams (one active frame at a time).
    pub video_filter: Option<(u8, u8)>,
    /// None emits all XA channels, each with independent PCM state/time.
    pub audio_filter: Option<(u8, u8)>,
}
impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            frame_rate: FrameRate::FPS_15,
            video_filter: None,
            audio_filter: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub number: u32,
    pub width: u16,
    pub height: u16,
    pub rgba: Vec<u8>,
    pub pts: Timestamp,
    pub duration: Timestamp,
    pub first_sector: u64,
    pub last_sector: u64,
}
#[derive(Debug, Clone)]
pub struct AudioPacket {
    pub file: u8,
    pub channel: u8,
    pub format: AudioFormat,
    pub pcm: Vec<i16>,
    /// The first decoded sample starts at zero, like the first video frame.
    pub pts: Timestamp,
    pub source_sector: u64,
}
#[derive(Debug, Clone)]
pub enum Event {
    Video(Frame),
    Audio(AudioPacket),
}

#[derive(Debug)]
struct Assembly {
    header: StrHeader,
    file_channel: (u8, u8),
    data: Vec<u8>,
    seen: Vec<bool>,
    count: usize,
    first_sector: u64,
    last_sector: u64,
}
#[derive(Debug)]
struct AudioState {
    decoder: XaDecoder,
    samples: u64,
    format: AudioFormat,
}

/// Demuxer and decoder. One ordered video stream, with any number of XA channels.
/// Start a new instance (or reset) on seeks; feed every sector in the chosen extent.
/// Never infer playback rate from processing speed or STR's padding/sector counts.
#[derive(Debug)]
pub struct StreamDecoder {
    config: StreamConfig,
    current: Option<Assembly>,
    first_frame: Option<u32>,
    last_frame: Option<u32>,
    audio: BTreeMap<(u8, u8), AudioState>,
}
impl StreamDecoder {
    pub fn new(config: StreamConfig) -> Result<Self, Error> {
        if config.frame_rate.numerator == 0 || config.frame_rate.denominator == 0 {
            return Err(Error::Invalid("zero frame rate"));
        }
        Ok(Self {
            config,
            current: None,
            first_frame: None,
            last_frame: None,
            audio: BTreeMap::new(),
        })
    }
    pub fn reset(&mut self) {
        self.current = None;
        self.first_frame = None;
        self.last_frame = None;
        self.audio.clear();
    }
    /// Emit at most one event per sector. `sector_index` is caller-owned source LBA.
    pub fn feed_sector(&mut self, bytes: &[u8], sector_index: u64) -> Result<Option<Event>, Error> {
        let sector = Sector::parse(bytes)?;
        let key = (sector.file, sector.channel);
        if sector.is_audio() {
            if self.config.audio_filter.is_some_and(|filter| filter != key) {
                return Ok(None);
            }
            let format = AudioFormat::from_xa_coding(sector.coding)?;
            let state = self.audio.entry(key).or_insert_with(|| AudioState {
                decoder: XaDecoder::default(),
                samples: 0,
                format,
            });
            if state.format != format {
                return Err(Error::Invalid(
                    "XA format changed; reset on stream boundary",
                ));
            }
            let pcm = state.decoder.decode(sector.payload, format)?;
            let pts = Timestamp {
                ticks: state.samples,
                timescale: format.sample_rate,
            };
            state.samples += (pcm.len() / format.channels as usize) as u64;
            return Ok(Some(Event::Audio(AudioPacket {
                file: sector.file,
                channel: sector.channel,
                format,
                pcm,
                pts,
                source_sector: sector_index,
            })));
        }
        if self.config.video_filter.is_some_and(|filter| filter != key) {
            return Ok(None);
        }
        let Some(header) = sector.str_header()? else {
            return Ok(None);
        };
        if sector.payload.len() < 2048 {
            return Err(Error::Truncated);
        }
        if let Some(current) = &self.current {
            if current.header.frame_number != header.frame_number {
                return Err(Error::IncompleteFrame(current.header.frame_number));
            }
            let mut expected = current.header;
            expected.chunk = header.chunk;
            if header != expected || current.file_channel != key {
                return Err(Error::Invalid("inconsistent STR headers"));
            }
        } else {
            if self
                .last_frame
                .is_some_and(|last| header.frame_number <= last)
            {
                return Err(Error::Invalid("STR frame order; reset on seek"));
            }
            self.current = Some(Assembly {
                header,
                file_channel: key,
                data: vec![0; header.chunks as usize * 2016],
                seen: vec![false; header.chunks as usize],
                count: 0,
                first_sector: sector_index,
                last_sector: sector_index,
            });
        }
        let current = self.current.as_mut().expect("assembly initialized");
        let chunk = header.chunk as usize;
        let source = &sector.payload[32..2048];
        let destination = &mut current.data[chunk * 2016..(chunk + 1) * 2016];
        if current.seen[chunk] {
            if destination != source {
                return Err(Error::Invalid("conflicting duplicate STR chunk"));
            }
            return Ok(None);
        }
        destination.copy_from_slice(source);
        current.seen[chunk] = true;
        current.count += 1;
        current.first_sector = current.first_sector.min(sector_index);
        current.last_sector = current.last_sector.max(sector_index);
        if current.count != header.chunks as usize {
            return Ok(None);
        }
        let mut current = self.current.take().expect("complete assembly");
        current.data.truncate(header.frame_size as usize);
        let rgba = MdecDecoder.decode(&current.data, header.width, header.height)?;
        let first = *self.first_frame.get_or_insert(header.frame_number);
        self.last_frame = Some(header.frame_number);
        let duration = Timestamp {
            ticks: self.config.frame_rate.denominator as u64,
            timescale: self.config.frame_rate.numerator,
        };
        let pts = Timestamp {
            ticks: (header.frame_number - first) as u64 * duration.ticks,
            timescale: duration.timescale,
        };
        Ok(Some(Event::Video(Frame {
            number: header.frame_number,
            width: header.width,
            height: header.height,
            rgba,
            pts,
            duration,
            first_sector: current.first_sector,
            last_sector: current.last_sector,
        })))
    }
    /// Call at the extent's end; missing chunks must not silently count as success.
    pub fn finish(&self) -> Result<(), Error> {
        self.current.as_ref().map_or(Ok(()), |a| {
            Err(Error::IncompleteFrame(a.header.frame_number))
        })
    }
}
