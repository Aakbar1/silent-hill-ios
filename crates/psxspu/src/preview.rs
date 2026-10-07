// Copyright (C) 2026 shdecompilations. GPL version 3.
// Volume/pitch rules adapted from smf_io.c; see ../NOTICE.md.
//! Standalone audition helper. Production MUST retain game-owned libsd for
//! voice priority, NRPN modulation, portamento, tremolo and random detuning.
use crate::{
    Error, Spu,
    sdk::VoiceAttr,
    sequence::Event,
    vab::{ToneRequest, Vab, note_to_pitch},
};

#[derive(Clone, Copy)]
struct Channel {
    program: u8,
    volume: u8,
    expression: u8,
    pan: u8,
    bend: u8,
    pedal: bool,
    wide: bool,
    reverb: u8,
    nrpn_msb: u8,
    nrpn_lsb: u8,
}
impl Default for Channel {
    fn default() -> Self {
        Self {
            program: 0,
            volume: 127,
            expression: 127,
            pan: 64,
            bend: 64,
            pedal: false,
            wide: false,
            reverb: 0,
            nrpn_msb: 0,
            nrpn_lsb: 0,
        }
    }
}
#[derive(Clone, Copy)]
struct Note {
    channel: u8,
    note: u8,
    tone: usize,
    program: usize,
    velocity: u8,
    released: bool,
}
pub struct PreviewSynth {
    channels: [Channel; 16],
    notes: [Option<Note>; 24],
    pub unsupported_events: u64,
    pub dropped_notes: u64,
    pub started_notes: u64,
    sequence_volume: [u8; 2],
}
impl Default for PreviewSynth {
    fn default() -> Self {
        Self {
            channels: [Channel::default(); 16],
            notes: [None; 24],
            unsupported_events: 0,
            dropped_notes: 0,
            started_notes: 0,
            sequence_volume: [127; 2],
        }
    }
}

impl PreviewSynth {
    pub fn set_sequence_volume(
        &mut self,
        volume: [u8; 2],
        spu: &mut Spu,
        bank: &Vab<'_>,
    ) -> Result<(), Error> {
        if volume.iter().any(|&v| v > 127) {
            return Err(Error::Invalid("sequence volume"));
        }
        self.sequence_volume = volume;
        for i in 0..24 {
            self.update(spu, bank, i)?;
        }
        Ok(())
    }
    /// Equivalent to the game's external SdSetMidiVol layer-volume control.
    pub fn set_channel_volume(
        &mut self,
        channel: u8,
        volume: u8,
        spu: &mut Spu,
        bank: &Vab<'_>,
    ) -> Result<(), Error> {
        if channel >= 16 || volume > 127 {
            return Err(Error::Invalid("preview channel/volume"));
        }
        self.channels[usize::from(channel)].volume = volume;
        self.refresh(spu, bank, channel)
    }
    fn update(&self, spu: &mut Spu, bank: &Vab<'_>, voice: usize) -> Result<(), Error> {
        let Some(n) = self.notes[voice] else {
            return Ok(());
        };
        let c = self.channels[usize::from(n.channel)];
        let p = &bank.programs[n.program];
        let t = p.tones[n.tone];
        let pan = (i32::from(bank.pan) + i32::from(p.pan) + i32::from(t.pan) + i32::from(c.pan)
            - 192)
            .clamp(0, 127);
        let volume =
            (i32::from(bank.volume) * 127 * i32::from(c.expression) * i32::from(c.volume)) >> 14;
        let volume = (volume * i32::from(p.volume) * i32::from(t.volume)) >> 14;
        let (l, r) = if pan >= 64 {
            (volume * (128 - pan) / 64, volume)
        } else {
            (volume, volume * pan / 64)
        };
        let volume = [
            ((((l * i32::from(n.velocity)) >> 7) * i32::from(self.sequence_volume[0])) >> 7) as i16,
            ((((r * i32::from(n.velocity)) >> 7) * i32::from(self.sequence_volume[1])) >> 7) as i16,
        ];
        let volume = if c.wide {
            [volume[0], -volume[1]]
        } else {
            volume
        };
        let range = if c.bend < 64 { t.bend_min } else { t.bend_max };
        let bend = (i16::from(c.bend) - if c.bend > 64 { 63 } else { 64 }) * i16::from(range) * 2;
        let attr = VoiceAttr {
            voices: 1 << voice,
            mask: 31,
            volume,
            pitch: note_to_pitch(
                i16::from(n.note),
                bend,
                i16::from(t.center),
                i16::from(t.shift),
            ),
            ..VoiceAttr::default()
        };
        spu.set_voice_attr(&attr)
    }
    fn refresh(&self, spu: &mut Spu, bank: &Vab<'_>, channel: u8) -> Result<(), Error> {
        for i in 0..24 {
            if self.notes[i].is_some_and(|n| n.channel == channel) {
                self.update(spu, bank, i)?;
            }
        }
        Ok(())
    }
    pub fn event(
        &mut self,
        event: Event,
        spu: &mut Spu,
        bank: &Vab<'_>,
        base: usize,
    ) -> Result<(), Error> {
        match event {
            Event::NoteOn {
                channel,
                note,
                velocity,
            } => {
                let program = usize::from(self.channels[usize::from(channel)].program);
                for (tone, t) in bank.programs[program].tones.iter().enumerate() {
                    if t.sample == 0 || note < t.min_note || note > t.max_note {
                        continue;
                    }
                    let voice = (0..24)
                        .find(|&i| spu.voice(i).unwrap().phase == crate::AdsrPhase::Off)
                        .or_else(|| {
                            (0..24)
                                .filter(|&i| {
                                    spu.voice(i).unwrap().phase == crate::AdsrPhase::Release
                                })
                                .min_by_key(|&i| spu.voice(i).unwrap().envelope)
                        });
                    let Some(voice) = voice else {
                        self.dropped_notes += 1;
                        continue;
                    };
                    self.notes[voice] = Some(Note {
                        channel,
                        note,
                        tone,
                        program,
                        velocity,
                        released: false,
                    });
                    let attr = bank.tone_attr(ToneRequest {
                        program,
                        tone,
                        note,
                        fine: 0,
                        voice,
                        base,
                        volume: [0; 2],
                    })?;
                    spu.key_on_with_attr(&attr)?;
                    self.update(spu, bank, voice)?;
                    let depth = self.channels[usize::from(channel)].reverb;
                    spu.set_reverb_voice(
                        if depth == 0 {
                            t.mode & 4 != 0
                        } else {
                            depth != 1
                        },
                        1 << voice,
                    );
                    self.started_notes += 1;
                }
            }
            Event::NoteOff { channel, note } => {
                for (i, n) in self.notes.iter_mut().enumerate() {
                    if let Some(n) = n
                        && n.channel == channel
                        && n.note == note
                    {
                        n.released = true;
                        if !self.channels[usize::from(channel)].pedal {
                            spu.key_off(1 << i);
                        }
                    }
                }
            }
            Event::Program { channel, program } => {
                self.channels[usize::from(channel)].program = program
            }
            Event::Bend { channel, value } => {
                self.channels[usize::from(channel)].bend = value;
                self.refresh(spu, bank, channel)?;
            }
            Event::Control {
                channel,
                controller,
                value,
            } => {
                let c = &mut self.channels[usize::from(channel)];
                match controller {
                    7 => c.volume = value,
                    10 => c.pan = value.max(1),
                    11 => c.expression = value,
                    64 => {
                        c.pedal = value >= 64;
                        if !c.pedal {
                            for (i, n) in self.notes.iter().enumerate() {
                                if n.is_some_and(|n| n.channel == channel && n.released) {
                                    spu.key_off(1 << i);
                                }
                            }
                        }
                    }
                    120 | 123 => {
                        for (i, n) in self.notes.iter().enumerate() {
                            if n.is_some_and(|n| n.channel == channel) {
                                spu.key_off(1 << i);
                            }
                        }
                    }
                    98 => c.nrpn_lsb = value,
                    99 => c.nrpn_msb = value,
                    6 => {
                        if !matches!(c.nrpn_msb, 20 | 30) && matches!(c.nrpn_lsb, 1..=13 | 23) {
                            self.unsupported_events += 1;
                        }
                    }
                    118 => {}             // Beat/control status only; no audible synthesis.
                    1 if value == 0 => {} // Disabling an already-disabled LFO.
                    91 => {
                        c.reverb = value;
                    }
                    // PORT: this audition helper reports controls needing the
                    // retained game's libsd. It is not a replacement allocator.
                    _ => self.unsupported_events += 1,
                }
                self.refresh(spu, bank, channel)?;
            }
            Event::Pressure { .. } => self.unsupported_events += 1,
            Event::Tempo(_) | Event::End { .. } => {}
        }
        Ok(())
    }
}
