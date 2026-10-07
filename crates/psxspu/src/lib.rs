//! Native SPU synthesis. No CPU emulator, game bytes, or platform dependencies
//! in the mixer. See INTEGRATION.md for the PsyQ and host ownership boundaries.
#![forbid(unsafe_code)]

mod envelope;
mod gaussian;
#[cfg(feature = "output")]
pub mod output;
pub mod preview;
pub mod reverb;
pub mod sdk;
pub mod sequence;
mod spu;
pub mod vab;
pub mod xa;

pub use envelope::{Adsr, AdsrPhase, Volume};
pub use spu::{Spu, VoiceState};

pub const SAMPLE_RATE: u32 = 44_100;
pub const RAM_BYTES: usize = 512 * 1024;
pub const VOICES: usize = 24;
pub type Frame = [i16; 2];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Truncated,
    Invalid(&'static str),
    Unsupported(&'static str),
    OutOfSoundRam,
    Media(psxmedia::Error),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated => f.write_str("truncated audio data"),
            Self::Invalid(s) => write!(f, "invalid audio data: {s}"),
            Self::Unsupported(s) => write!(f, "unsupported audio data: {s}"),
            Self::OutOfSoundRam => f.write_str("sound RAM allocation exceeds the sample area"),
            Self::Media(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for Error {}
impl From<psxmedia::Error> for Error {
    fn from(e: psxmedia::Error) -> Self {
        Self::Media(e)
    }
}
pub(crate) fn sat(v: i32) -> i16 {
    v.clamp(-32768, 32767) as i16
}
pub(crate) fn mul(a: i32, b: i32) -> i32 {
    ((i64::from(a) * i64::from(b)) >> 15) as i32
}
pub(crate) fn le16(b: &[u8], at: usize) -> Result<u16, Error> {
    Ok(u16::from_le_bytes(
        b.get(at..at + 2)
            .ok_or(Error::Truncated)?
            .try_into()
            .unwrap(),
    ))
}
pub(crate) fn le32(b: &[u8], at: usize) -> Result<u32, Error> {
    Ok(u32::from_le_bytes(
        b.get(at..at + 4)
            .ok_or(Error::Truncated)?
            .try_into()
            .unwrap(),
    ))
}
