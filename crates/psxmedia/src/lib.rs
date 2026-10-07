//! Native, bounded PlayStation media decoding. No emulator or native dependencies.
//! Feed raw Mode 2 sectors to [`StreamDecoder`]; schedule output by its timestamps.
//! Video timing is supplied by the caller because STR headers contain no frame rate.

mod adpcm;
mod mdec;
mod sector;
mod stream;
mod tables;

pub use adpcm::{AudioFormat, SpuBlock, SpuDecoder, VagSample, XaDecoder, decode_vag};
pub use mdec::MdecDecoder;
pub use sector::{Sector, StrHeader};
pub use stream::{AudioPacket, Event, Frame, FrameRate, StreamConfig, StreamDecoder, Timestamp};

/// Invalid, truncated or unsupported media. Malformed input never invokes unsafe code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Truncated,
    Invalid(&'static str),
    Unsupported(&'static str),
    IncompleteFrame(u32),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated => f.write_str("truncated media"),
            Self::Invalid(s) => write!(f, "invalid media: {s}"),
            Self::Unsupported(s) => write!(f, "unsupported media: {s}"),
            Self::IncompleteFrame(n) => write!(f, "incomplete STR frame {n}"),
        }
    }
}
impl std::error::Error for Error {}

pub(crate) fn le16(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}
pub(crate) fn le32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().expect("checked header"))
}
