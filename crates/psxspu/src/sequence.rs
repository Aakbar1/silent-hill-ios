// Copyright (C) 2026 shdecompilations. GPL version 3.
// Adapted from Silent Hill's smf_mid.c and smf_main.c (d9e28f8).
// This program is distributed WITHOUT ANY WARRANTY; see ../NOTICE.md.
//! Game sequence event clock. Keep the original libsd voice allocator and MIDI
//! synthesis controllers in core; this parser also supports standalone previews.
use crate::{Error, SAMPLE_RATE, le16, le32};

pub const TIMER_TARGET: u32 = 7328;
pub const SYSTEM_CLOCK: u64 = 33_868_800;
pub const TIMER_DIVIDER: u64 = 8;

/// The original smf_timer tests >=11 before resetting, then increments after
/// reset: first housekeeping at interrupt 12, then every 11 interrupts.
#[derive(Default)]
pub struct HousekeepingClock {
    counter: u8,
}
impl HousekeepingClock {
    pub fn tick(&mut self) -> bool {
        let due = self.counter >= 11;
        if due {
            self.counter = 0;
        }
        self.counter += 1;
        due
    }
}

/// Rational CPU-cycle clock, independent of host refresh/device block size.
/// Core may supply a measured target period (including reset delay) in cycles.
#[derive(Debug, Clone)]
pub struct SequenceClock {
    phase: u64,
    period: u64,
    pub ticks: u64,
}
impl Default for SequenceClock {
    fn default() -> Self {
        Self::new(u64::from(TIMER_TARGET) * TIMER_DIVIDER).unwrap()
    }
}
impl SequenceClock {
    pub fn new(period_cpu_cycles: u64) -> Result<Self, Error> {
        if period_cpu_cycles == 0
            || period_cpu_cycles > (u64::MAX - SYSTEM_CLOCK) / u64::from(SAMPLE_RATE)
        {
            return Err(Error::Invalid("timer period"));
        }
        Ok(Self {
            phase: 0,
            period: period_cpu_cycles * u64::from(SAMPLE_RATE),
            ticks: 0,
        })
    }
    /// Call before producing the next SPU frame. Returns elapsed timer events.
    pub fn advance_frame(&mut self) -> u32 {
        self.phase += SYSTEM_CLOCK;
        let events = self.phase / self.period;
        self.phase %= self.period;
        self.ticks += events;
        events as u32
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    NoteOn {
        channel: u8,
        note: u8,
        velocity: u8,
    },
    NoteOff {
        channel: u8,
        note: u8,
    },
    Program {
        channel: u8,
        program: u8,
    },
    Control {
        channel: u8,
        controller: u8,
        value: u8,
    },
    Bend {
        channel: u8,
        value: u8,
    },
    Pressure {
        channel: u8,
        note: Option<u8>,
        value: u8,
    },
    Tempo(u16),
    End {
        track: u8,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Kdt,
    Seq,
    Midi,
}
#[derive(Debug, Default, Clone, Copy)]
struct Track {
    cursor: usize,
    end: usize,
    delta: u16,
    remainder: u16,
    tempo: u16,
    accumulator: u16,
    channel: u8,
    status: u8,
    ended: bool,
    loop_count: u8,
}
#[derive(Debug, Default, Clone, Copy)]
struct Snapshot {
    cursor: usize,
    delta: u16,
    remainder: u16,
    status: u8,
    ended: bool,
}
pub struct Sequence<'a> {
    data: &'a [u8],
    tracks: [Track; 32],
    count: usize,
    pub division: u16,
    pub format: Format,
    snapshots: [Snapshot; 32],
    loop_pending: u8,
    nrpn: [u8; 16],
    pub paused: bool,
    pub interrupts: u64,
}

fn byte(data: &[u8], t: &mut Track) -> Result<u8, Error> {
    if t.cursor >= t.end {
        return Err(Error::Truncated);
    }
    let b = data[t.cursor];
    t.cursor += 1;
    Ok(b)
}
fn vlq(data: &[u8], t: &mut Track) -> Result<u32, Error> {
    let mut result = 0;
    for _ in 0..4 {
        let b = byte(data, t)?;
        result = (result << 7) | u32::from(b & 127);
        if b & 128 == 0 {
            return Ok(result);
        }
    }
    Err(Error::Invalid("sequence VLQ"))
}
fn big16(data: &[u8], at: usize) -> Result<u16, Error> {
    Ok(u16::from_be_bytes(
        data.get(at..at + 2)
            .ok_or(Error::Truncated)?
            .try_into()
            .unwrap(),
    ))
}
fn big32(data: &[u8], at: usize) -> Result<u32, Error> {
    Ok(u32::from_be_bytes(
        data.get(at..at + 4)
            .ok_or(Error::Truncated)?
            .try_into()
            .unwrap(),
    ))
}

/// The original uses u16 arithmetic, retaining fractional remainder per track.
pub fn convert_delta(delta: u16, division: u16, remainder: &mut u16) -> u16 {
    let (factor, denominator, retain) = match division {
        48 => (10, 4, true),
        96 => (5, 4, true),
        192 | 240 => (1, 2, true),
        288 | 360 => (1, 3, false),
        384 | 480 => (1, 4, true),
        768 | 960 => (1, 8, true),
        _ => (1, 1, false),
    };
    let value = delta
        .wrapping_mul(factor)
        .wrapping_add(if retain { *remainder } else { 0 });
    if retain {
        *remainder = value % denominator;
    }
    value / denominator
}
fn tempo_value(us: u32, division: u16) -> Result<u16, Error> {
    if us == 0 {
        return Err(Error::Invalid("zero sequence tempo"));
    }
    let mut tempo = ((60_000_000 / us) * 100 / 115).min(255);
    if matches!(division, 24 | 60) {
        tempo /= 2;
    } else if division == 30 {
        tempo /= 4;
    }
    Ok(tempo as u16)
}

impl<'a> Sequence<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, Error> {
        let mut result = Self {
            data,
            tracks: [Track::default(); 32],
            count: 0,
            division: 0,
            format: Format::Kdt,
            snapshots: [Snapshot::default(); 32],
            loop_pending: 0,
            nrpn: [0; 16],
            paused: false,
            interrupts: 0,
        };
        let mut spans = [(0usize, 0usize); 32];
        let mut tempo = 104;
        match data.get(..4).ok_or(Error::Truncated)? {
            b"KDT1" | b"KDT " => {
                let size = le32(data, 4)? as usize;
                let division = le32(data, 8)?;
                let count = le32(data, 12)? as usize;
                if count == 0 || count > 32 || division == 0 || division > u16::MAX.into() {
                    return Err(Error::Invalid("KDT track count/time base"));
                }
                let mut start = if data[3] == b'1' { 16 + count * 2 } else { 80 };
                if size > data.len() || size < start {
                    return Err(Error::Truncated);
                }
                for (i, span) in spans.iter_mut().enumerate().take(count) {
                    let length = usize::from(le16(data, 16 + i * 2)?);
                    let end = start.checked_add(length).ok_or(Error::Truncated)?;
                    if end > size {
                        return Err(Error::Truncated);
                    }
                    *span = (start, end);
                    start = end;
                }
                result.count = count;
                result.division = division as u16;
            }
            b"pQES" => {
                result.format = Format::Seq;
                result.count = 1;
                result.division = big16(data, 8)?;
                let b = data.get(10..13).ok_or(Error::Truncated)?;
                tempo = tempo_value(u32::from_be_bytes([0, b[0], b[1], b[2]]), result.division)?;
                spans[0] = (15, data.len());
            }
            b"MThd" => {
                result.format = Format::Midi;
                let length = big32(data, 4)? as usize;
                result.count = usize::from(big16(data, 10)?);
                result.division = big16(data, 12)?;
                if length < 6
                    || result.count == 0
                    || result.count > 32
                    || big16(data, 8)? > 1
                    || result.division & 0x8000 != 0
                {
                    return Err(Error::Unsupported("MIDI format/division"));
                }
                let mut start = 8 + length;
                for span in spans.iter_mut().take(result.count) {
                    if data.get(start..start + 4) != Some(b"MTrk") {
                        return Err(Error::Invalid("MIDI track header"));
                    }
                    let length = big32(data, start + 4)? as usize;
                    start += 8;
                    let end = start.checked_add(length).ok_or(Error::Truncated)?;
                    if end > data.len() {
                        return Err(Error::Truncated);
                    }
                    *span = (start, end);
                    start = end;
                }
            }
            _ => return Err(Error::Invalid("sequence magic")),
        }
        if result.division == 0 {
            return Err(Error::Invalid("sequence division"));
        }
        for (i, &(cursor, end)) in spans.iter().enumerate().take(result.count) {
            let mut track = Track {
                cursor,
                end,
                tempo,
                accumulator: tempo,
                ..Track::default()
            };
            track.delta = convert_delta(
                vlq(data, &mut track)? as u16,
                result.division,
                &mut track.remainder,
            );
            result.tracks[i] = track;
        }
        Ok(result)
    }
    pub fn ended(&self) -> bool {
        self.tracks[..self.count].iter().all(|t| t.ended)
    }
    pub fn stop(&mut self) {
        for t in &mut self.tracks[..self.count] {
            t.ended = true;
        }
    }
    fn control(&mut self, ch: u8, controller: u8, value: u8, emit: &mut impl FnMut(Event)) {
        if controller == 99 {
            self.nrpn[usize::from(ch)] = value;
            if value == 20 {
                self.loop_pending = 2;
            } else if value == 30 {
                self.loop_pending = 1;
            }
        } else if controller == 6 && self.nrpn[usize::from(ch)] == 20 {
            for t in &mut self.tracks[..self.count] {
                t.loop_count = value;
            }
        }
        emit(Event::Control {
            channel: ch,
            controller,
            value,
        });
    }
    fn set_tempo(&mut self, value: u16, emit: &mut impl FnMut(Event)) {
        for t in &mut self.tracks[..self.count] {
            t.tempo = value;
        }
        emit(Event::Tempo(value));
    }
    fn kdt_event(&mut self, index: usize, emit: &mut impl FnMut(Event)) -> Result<bool, Error> {
        let t = &mut self.tracks[index];
        let code = byte(self.data, t)?;
        let channel = t.channel;
        if code & 128 == 0 {
            let velocity = byte(self.data, t)?;
            t.status = code;
            if velocity & 127 == 0 {
                emit(Event::NoteOff {
                    channel,
                    note: code,
                });
            } else {
                emit(Event::NoteOn {
                    channel,
                    note: code,
                    velocity: velocity & 127,
                });
            }
            return Ok(velocity & 128 != 0);
        }
        match code & 127 {
            0x4a | 0x4b => {
                emit(Event::NoteOff {
                    channel,
                    note: t.status,
                });
                Ok(code & 127 == 0x4b)
            }
            0x47 => {
                let value = byte(self.data, t)?;
                self.set_tempo(u16::from(value & 127) * 2 + 2, emit);
                Ok(value & 128 != 0)
            }
            0x48 => {
                let value = byte(self.data, t)?;
                emit(Event::Bend {
                    channel,
                    value: value & 127,
                });
                Ok(value & 128 != 0)
            }
            0x49 => {
                let value = byte(self.data, t)?;
                emit(Event::Program {
                    channel,
                    program: value & 127,
                });
                Ok(value & 128 != 0)
            }
            0x46 => {
                let value = byte(self.data, t)?;
                t.channel = value & 15;
                Ok(value & 128 != 0)
            }
            0x7f => {
                let value = byte(self.data, t)?;
                t.ended = true;
                emit(Event::End { track: index as u8 });
                Ok(value & 128 != 0)
            }
            controller => {
                let value = byte(self.data, t)?;
                self.control(channel, controller, value & 127, emit);
                Ok(value & 128 != 0)
            }
        }
    }
    fn midi_event(&mut self, index: usize, emit: &mut impl FnMut(Event)) -> Result<(), Error> {
        let t = &mut self.tracks[index];
        let code = byte(self.data, t)?;
        if code == 0xff {
            let kind = byte(self.data, t)?;
            // Konami's SEQ meta tempo omits the usual MIDI length byte.
            let length = if self.format == Format::Seq && kind == 0x51 {
                3
            } else {
                vlq(self.data, t)? as usize
            };
            if t.cursor.checked_add(length).is_none_or(|end| end > t.end) {
                return Err(Error::Truncated);
            }
            if kind == 0x51 {
                if length != 3 {
                    return Err(Error::Invalid("MIDI tempo length"));
                }
                let b = &self.data[t.cursor..t.cursor + 3];
                let us = u32::from_be_bytes([0, b[0], b[1], b[2]]);
                t.cursor += 3;
                let tempo = tempo_value(us, self.division)?;
                self.set_tempo(tempo, emit);
                // A MIDI meta tempo resets the fractional accumulator as in libsd.
                for t in &mut self.tracks[..self.count] {
                    t.accumulator = tempo;
                }
            } else {
                t.cursor += length;
                if kind == 0x2f {
                    t.ended = true;
                    emit(Event::End { track: index as u8 });
                }
            }
            return Ok(());
        }
        if code == 0xf0 || code == 0xf7 {
            let n = vlq(self.data, t)? as usize;
            t.cursor = t
                .cursor
                .checked_add(n)
                .filter(|&end| end <= t.end)
                .ok_or(Error::Truncated)?;
            return Ok(());
        }
        let (status, first) = if code & 128 != 0 {
            t.status = code;
            (code, byte(self.data, t)?)
        } else {
            (t.status, code)
        };
        if !(0x80..0xf0).contains(&status) || first & 128 != 0 {
            return Err(Error::Invalid("MIDI running status/data"));
        }
        let ch = status & 15;
        let second = if matches!(status & 0xf0, 0xc0 | 0xd0) {
            0
        } else {
            let b = byte(self.data, t)?;
            if b & 128 != 0 {
                return Err(Error::Invalid("MIDI data"));
            }
            b
        };
        match status & 0xf0 {
            0x80 => emit(Event::NoteOff {
                channel: ch,
                note: first,
            }),
            0x90 if second == 0 => emit(Event::NoteOff {
                channel: ch,
                note: first,
            }),
            0x90 => emit(Event::NoteOn {
                channel: ch,
                note: first,
                velocity: second,
            }),
            0xa0 => emit(Event::Pressure {
                channel: ch,
                note: Some(first),
                value: second,
            }),
            0xb0 => self.control(ch, first, second, emit),
            0xc0 => emit(Event::Program {
                channel: ch,
                program: first,
            }),
            0xd0 => emit(Event::Pressure {
                channel: ch,
                note: None,
                value: first,
            }),
            0xe0 => emit(Event::Bend {
                channel: ch,
                value: second,
            }),
            _ => unreachable!(),
        }
        Ok(())
    }
    /// One smf_timer/midi_smf_main interrupt. No allocation, events delivered
    /// directly to the retained libsd or the preview synthesizer.
    pub fn tick(&mut self, mut emit: impl FnMut(Event)) -> Result<(), Error> {
        if self.paused {
            return Ok(());
        }
        self.interrupts += 1;
        for i in 0..self.count {
            let t = &mut self.tracks[i];
            if t.ended {
                continue;
            }
            t.accumulator = t.accumulator.wrapping_add(t.tempo);
            if t.accumulator <= 255 {
                continue;
            }
            t.accumulator &= 255;
            if t.delta != 0 {
                t.delta -= 1;
                continue;
            }
            let mut budget = 4096;
            loop {
                if budget == 0 {
                    return Err(Error::Invalid("unbounded sequence event chain"));
                }
                budget -= 1;
                let chained = if self.format == Format::Kdt {
                    self.kdt_event(i, &mut emit)?
                } else {
                    self.midi_event(i, &mut emit)?;
                    false
                };
                let t = &mut self.tracks[i];
                if t.ended {
                    break;
                }
                if !chained {
                    t.delta = vlq(self.data, t)? as u16;
                }
                if t.delta != 0 {
                    t.delta = convert_delta(t.delta, self.division, &mut t.remainder);
                }
                if t.delta != 0 {
                    t.delta -= 1;
                    break;
                }
            }
        }
        if self.loop_pending == 2 {
            for (t, s) in self.tracks[..self.count]
                .iter_mut()
                .zip(&mut self.snapshots)
            {
                *s = Snapshot {
                    cursor: t.cursor,
                    delta: t.delta,
                    remainder: t.remainder,
                    status: t.status,
                    ended: t.ended,
                };
                t.loop_count = 127;
            }
        } else if self.loop_pending == 1 {
            for (t, s) in self.tracks[..self.count].iter_mut().zip(&self.snapshots) {
                if t.loop_count != 0 {
                    if t.loop_count < 127 {
                        t.loop_count -= 1;
                    }
                    t.cursor = s.cursor;
                    t.delta = s.delta;
                    t.remainder = s.remainder;
                    t.status = s.status;
                    t.ended = s.ended;
                }
            }
        }
        self.loop_pending = 0;
        Ok(())
    }
}
