//! Checked disk32 VAB parser. Borrowed body bytes are imported by the caller;
//! the crate contains no sample assets. Tone blocks are packed by active program.
use crate::{Error, Spu, le16, le32, sdk::VoiceAttr};
#[path = "pitch_table.rs"]
mod pitch_table;

#[derive(Debug, Clone)]
pub struct Program {
    pub volume: u8,
    pub pan: u8,
    pub priority: u8,
    pub tones: Vec<Tone>,
}
#[derive(Debug, Clone, Copy)]
pub struct Tone {
    pub priority: u8,
    pub mode: u8,
    pub volume: u8,
    pub pan: u8,
    pub center: u8,
    pub shift: u8,
    pub min_note: u8,
    pub max_note: u8,
    pub bend_min: u8,
    pub bend_max: u8,
    pub adsr1: u16,
    pub adsr2: u16,
    pub sample: u16,
}
#[derive(Debug)]
pub struct Vab<'a> {
    pub volume: u8,
    pub pan: u8,
    pub programs: Vec<Program>,
    pub samples: Vec<std::ops::Range<usize>>,
    pub body: &'a [u8],
}

#[derive(Debug, Clone, Copy)]
pub struct ToneRequest {
    pub program: usize,
    pub tone: usize,
    pub note: u8,
    pub fine: i16,
    pub voice: usize,
    pub base: usize,
    pub volume: [i16; 2],
}

impl<'a> Vab<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        if bytes.get(..4) != Some(b"pBAV") {
            return Err(Error::Invalid("VAB magic"));
        }
        let size = le32(bytes, 12)? as usize;
        let programs = usize::from(le16(bytes, 18)?);
        let tones = usize::from(le16(bytes, 20)?);
        let samples = usize::from(le16(bytes, 22)?);
        if programs > 128 || tones > programs * 16 || samples > 255 {
            return Err(Error::Invalid("VAB counts"));
        }
        let body_offset = 0xa20 + programs * 512;
        if size < body_offset {
            return Err(Error::Invalid("VAB file size"));
        }
        let body = bytes.get(body_offset..size).ok_or(Error::Truncated)?;
        let table = body_offset - 512;
        let mut offset = usize::from(le16(bytes, table)?) * 8;
        let mut sample_ranges = Vec::with_capacity(samples);
        for i in 1..=samples {
            let length = usize::from(le16(bytes, table + i * 2)?) * 8;
            if !length.is_multiple_of(16) || offset + length > body.len() {
                return Err(Error::Invalid("VAB sample extent"));
            }
            sample_ranges.push(offset..offset + length);
            offset += length;
        }
        let mut result = Self {
            volume: bytes[24],
            pan: bytes[25],
            programs: Vec::with_capacity(128),
            samples: sample_ranges,
            body,
        };
        let mut packed = 0;
        let mut total_tones = 0;
        for prog in 0..128 {
            let at = 32 + prog * 16;
            // Some retail banks retain old nonzero entries beyond the declared
            // packed program count. They have no corresponding tone blocks.
            // The header's ps/ts bounds, not unused table contents, own the data.
            let count = if packed < programs {
                bytes[at] as usize
            } else {
                0
            };
            if count > 16 {
                return Err(Error::Invalid("VAB program tone count"));
            }
            let mut program = Program {
                volume: bytes[at + 1],
                priority: bytes[at + 2],
                pan: bytes[at + 4],
                tones: Vec::with_capacity(count),
            };
            if count > 0 {
                if packed >= programs {
                    return Err(Error::Invalid("VAB packed programs"));
                }
                for tone in 0..count {
                    let at = 0x820 + packed * 512 + tone * 32;
                    let b = bytes.get(at..at + 32).ok_or(Error::Truncated)?;
                    let sample = le16(b, 22)?;
                    if usize::from(sample) > samples {
                        return Err(Error::Invalid("VAB tone sample number"));
                    }
                    program.tones.push(Tone {
                        priority: b[0],
                        mode: b[1],
                        volume: b[2],
                        pan: b[3],
                        center: b[4],
                        shift: b[5],
                        min_note: b[6],
                        max_note: b[7],
                        bend_min: b[12],
                        bend_max: b[13],
                        adsr1: le16(b, 16)?,
                        adsr2: le16(b, 18)?,
                        sample,
                    });
                }
                packed += 1;
                total_tones += count;
            }
            result.programs.push(program);
        }
        if packed != programs || total_tones != tones {
            return Err(Error::Invalid("VAB program/tone total"));
        }
        Ok(result)
    }
    /// Upload a whole body into sample RAM, excluding capture/reverb regions.
    pub fn upload(&self, spu: &mut Spu, address: usize) -> Result<(), Error> {
        if address < 0x1000
            || !address.is_multiple_of(8)
            || address
                .checked_add(self.body.len())
                .is_none_or(|end| end > usize::from(spu.reverb.base()) * 8)
        {
            return Err(Error::OutOfSoundRam);
        }
        spu.upload(address, self.body)
    }
    pub fn tone_attr(&self, request: ToneRequest) -> Result<VoiceAttr, Error> {
        let ToneRequest {
            program,
            tone,
            note,
            fine,
            voice,
            base,
            volume,
        } = request;
        let t = self
            .programs
            .get(program)
            .and_then(|p| p.tones.get(tone))
            .ok_or(Error::Invalid("VAB tone index"))?;
        if t.sample == 0 || note < t.min_note || note > t.max_note || voice >= 24 {
            return Err(Error::Invalid("VAB note/voice"));
        }
        let address = base
            .checked_add(self.samples[usize::from(t.sample) - 1].start)
            .ok_or(Error::OutOfSoundRam)?;
        if address >= crate::RAM_BYTES {
            return Err(Error::OutOfSoundRam);
        }
        Ok(VoiceAttr {
            voices: 1 << voice,
            mask: 0x7019f,
            volume,
            pitch: note_to_pitch(
                i16::from(note),
                fine,
                i16::from(t.center),
                i16::from(t.shift),
            ),
            address: address as u32,
            loop_address: address as u32,
            adsr1: t.adsr1,
            adsr2: t.adsr2,
            attack_mode: if t.adsr1 & 0x80 != 0 { 5 } else { 1 },
            ..VoiceAttr::default()
        })
    }
}

/// Integer lookup from the game's Note2Pitch / PitchTbl. Fine units are 1/128
/// semitone; sample_cent is ADDED, as in the original, rather than subtracted.
pub fn note_to_pitch(note: i16, cent: i16, sample_note: i16, sample_cent: i16) -> u16 {
    let total = i32::from(cent) + i32::from(sample_cent);
    // PORT: normalize negative audition fine offsets instead of the original
    // C routine's out-of-bounds negative table index. Game callers use 0..127.
    let steps = total.div_euclid(128);
    let fine = total.rem_euclid(128) as usize;
    let diff = i32::from(note) + steps - i32::from(sample_note);
    let semitone = diff.rem_euclid(12) as usize;
    let octave = diff.div_euclid(12);
    let pitch = u32::from(pitch_table::PITCH[semitone][fine]);
    if octave >= 0 {
        pitch.checked_shl(octave as u32).unwrap_or(0) as u16
    } else {
        pitch.checked_shr((-octave) as u32).unwrap_or(0) as u16
    }
}
