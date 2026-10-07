// SPDX-License-Identifier: GPL-3.0-only
use std::{fmt, io};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Invalid(String),
    Unsupported(String),
    OutOfBounds {
        start: u64,
        count: u64,
        available: u64,
    },
    NotFound(String),
    Ambiguous(String),
    WrongRelease {
        actual_sha256: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "Could not read or write the game data: {e}"),
            Self::Invalid(s) => write!(f, "The image or index is damaged or invalid: {s}"),
            Self::Unsupported(s) => write!(f, "This image format is not supported: {s}"),
            Self::OutOfBounds {
                start,
                count,
                available,
            } => write!(
                f,
                "Read {start}+{count} exceeds the available {available} units"
            ),
            Self::NotFound(s) => write!(
                f,
                "Could not find {s}. Please choose your Silent Hill US v1.1 disc image."
            ),
            Self::Ambiguous(s) => write!(
                f,
                "More than one entry is named {s}; use its full path or game file ID"
            ),
            Self::WrongRelease { actual_sha256 } => write!(
                f,
                "This copy of Silent Hill is not the supported US v1.1 release. Please import your US v1.1 disc (SLUS_007.07). Executable SHA-256: {actual_sha256}"
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

pub(crate) fn invalid(s: impl Into<String>) -> Error {
    Error::Invalid(s.into())
}

pub(crate) fn bounds(start: u64, count: u64, available: u64) -> Result<()> {
    if start > available || count > available - start {
        return Err(Error::OutOfBounds {
            start,
            count,
            available,
        });
    }
    Ok(())
}

pub(crate) fn buffer(size: u64) -> Result<Vec<u8>> {
    let len = usize::try_from(size).map_err(|_| invalid("read is too large for this device"))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(len)
        .map_err(|_| invalid("not enough memory for this read; use the streaming API"))?;
    bytes.resize(len, 0);
    Ok(bytes)
}
