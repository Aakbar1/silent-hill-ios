// SPDX-License-Identifier: GPL-3.0-only
//! Bounded STR playback, scheduled on the game's virtual 60 Hz VBlank clock.
use crate::{
    backend::{GpuBackend, SpuBackend},
    disc::{Archive, GameDisc, SectorReader},
};
use psxmedia::{AudioPacket, Event, Frame, StreamConfig, StreamDecoder, Timestamp};
use std::collections::VecDeque;

pub struct Movie {
    id: u32,
    sector: u32,
    sectors: u32,
    start_lba: u32,
    start_tick: u64,
    last_frame: u32,
    decoder: StreamDecoder,
    next_video: Option<Frame>,
    audio: VecDeque<AudioPacket>,
    pub decoded_frames: u32,
    pub audio_samples: u64,
    logged_silent: bool,
    finished: bool,
}

fn due(pts: Timestamp, ticks: u64) -> bool {
    u128::from(pts.ticks) * 60 <= u128::from(ticks) * u128::from(pts.timescale)
}

/// Pack RGB24 into PS1 VRAM words, padding each odd-width row separately.
pub fn pack_rgb24(frame: &Frame) -> Vec<u16> {
    let mut words =
        Vec::with_capacity((usize::from(frame.width) * 3).div_ceil(2) * usize::from(frame.height));
    for row in frame.rgba.chunks_exact(usize::from(frame.width) * 4) {
        let rgb: Vec<u8> = row
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect();
        for pair in rgb.chunks(2) {
            words.push(u16::from_le_bytes([pair[0], *pair.get(1).unwrap_or(&0)]));
        }
    }
    words
}

impl Movie {
    pub fn new<S: SectorReader>(
        disc: &GameDisc<S>,
        id: u32,
        last_frame: u32,
        tick: u64,
    ) -> Result<Self, String> {
        let entry = disc.entry(id).map_err(|e| e.to_string())?;
        if entry.archive != Archive::Hill || entry.block_count < 7 {
            return Err("movie requires a nonempty HILL STR entry".into());
        }
        let last_frame = if last_frame == 0 {
            u32::from(entry.block_count) - 7
        } else {
            last_frame
        };
        println!(
            "MOVIE begin id={id} last_frame={last_frame} sectors={} tick={tick}",
            entry.sector_count
        );
        Ok(Self {
            id,
            sector: 0,
            sectors: entry.sector_count,
            start_lba: entry.start_sector,
            start_tick: tick,
            last_frame,
            decoder: StreamDecoder::new(StreamConfig::default()).map_err(|e| e.to_string())?,
            next_video: None,
            audio: VecDeque::new(),
            decoded_frames: 0,
            audio_samples: 0,
            logged_silent: false,
            finished: false,
        })
    }

    /// Returns true after the original frame limit. Decodes at most one frame ahead.
    pub fn tick<S: SectorReader>(
        &mut self,
        disc: &mut GameDisc<S>,
        gpu: &mut dyn GpuBackend,
        spu: &mut dyn SpuBackend,
        tick: u64,
    ) -> Result<bool, String> {
        let elapsed = tick.saturating_sub(self.start_tick);
        loop {
            if self.next_video.is_none() && !self.finished {
                while self.sector < self.sectors {
                    let mut bytes = Vec::with_capacity(2352);
                    disc.copy_entry_raw_sectors(self.id, self.sector, 1, &mut bytes)
                        .map_err(|e| e.to_string())?;
                    let lba = u64::from(self.start_lba + self.sector);
                    self.sector += 1;
                    match self
                        .decoder
                        .feed_sector(&bytes, lba)
                        .map_err(|e| format!("movie {} LBA {lba}: {e}", self.id))?
                    {
                        Some(Event::Video(frame)) => {
                            if frame.width == 0 || frame.width > 320 || frame.height > 240 {
                                return Err("movie dimensions exceed native STREAM display".into());
                            }
                            self.next_video = Some(frame);
                            break;
                        }
                        Some(Event::Audio(packet)) => {
                            if self.audio.len() >= 64 {
                                return Err("movie XA queue exceeds bound".into());
                            }
                            self.audio.push_back(packet);
                        }
                        None => {}
                    }
                }
                if self.next_video.is_none() {
                    self.decoder.finish().map_err(|e| e.to_string())?;
                    self.finished = true;
                }
            }
            while self.audio.front().is_some_and(|a| due(a.pts, elapsed)) {
                let audio = self.audio.pop_front().expect("checked audio queue");
                self.audio_samples += (audio.pcm.len() / usize::from(audio.format.channels)) as u64;
                if !spu.cd_input(&audio.pcm, audio.format.sample_rate, audio.format.channels)?
                    && !self.logged_silent
                {
                    println!(
                        "MOVIE XA decoded; SpuBackend has no CD input sink ({} Hz, {} channels)",
                        audio.format.sample_rate, audio.format.channels
                    );
                    self.logged_silent = true;
                }
            }
            if self
                .next_video
                .as_ref()
                .is_some_and(|v| due(v.pts, elapsed))
            {
                let video = self.next_video.take().expect("checked video queue");
                let words = pack_rgb24(&video);
                gpu.load(
                    [
                        0,
                        16,
                        (u32::from(video.width) * 3).div_ceil(2) as i32,
                        i32::from(video.height),
                    ],
                    &words,
                );
                self.decoded_frames += 1;
                if video.number >= self.last_frame {
                    self.finished = true;
                }
                if self.finished {
                    return Ok(true);
                }
            } else {
                return Ok(self.finished && self.audio.is_empty());
            }
        }
    }

    pub fn end(self, spu: &mut dyn SpuBackend, skipped: bool) {
        spu.cd_stop();
        println!(
            "MOVIE end id={} decoded={} xa_samples={} skipped={skipped}",
            self.id, self.decoded_frames, self.audio_samples
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raster::Raster;
    #[test]
    fn rgb24_uses_vram_and_preserves_channels_and_row_padding() {
        let frame = Frame {
            number: 1,
            width: 1,
            height: 2,
            rgba: vec![255, 17, 3, 255, 7, 128, 254, 255],
            pts: Timestamp {
                ticks: 0,
                timescale: 15,
            },
            duration: Timestamp {
                ticks: 1,
                timescale: 15,
            },
            first_sector: 0,
            last_sector: 0,
        };
        let words = pack_rgb24(&frame);
        assert_eq!(words, [0x11ff, 3, 0x8007, 254]);
        let mut gpu = Raster::default();
        GpuBackend::load(&mut gpu, [0, 16, 2, 2], &words);
        assert_eq!(gpu.frame_rgb24(0, 16, 1, 2), [0xff1103, 0x0780fe]);
    }
    #[test]
    fn timestamps_follow_virtual_vblanks() {
        let pts = Timestamp {
            ticks: 7,
            timescale: 15,
        };
        assert!(!due(pts, 27));
        assert!(due(pts, 28));
        assert!(due(
            Timestamp {
                ticks: u64::MAX,
                timescale: u32::MAX
            },
            u64::MAX
        ));
    }
}
