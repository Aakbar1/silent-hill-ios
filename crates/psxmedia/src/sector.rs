use crate::{Error, le16, le32};

/// Borrowed CD sector. No disc filesystem or device emulation is involved.
#[derive(Debug, Clone, Copy)]
pub struct Sector<'a> {
    pub file: u8,
    pub channel: u8,
    pub submode: u8,
    pub coding: u8,
    pub payload: &'a [u8],
}
impl<'a> Sector<'a> {
    /// Accept raw Mode 2/2352 or Mode 2/2336 sectors, or 2048-byte STR user data.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Error> {
        let data = match bytes.len() {
            2352 => {
                const SYNC: [u8; 12] = [0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0];
                if bytes[..12] != SYNC || bytes[15] != 2 {
                    return Err(Error::Invalid("CD sync/mode"));
                }
                &bytes[16..]
            }
            2336 => bytes,
            2048 => {
                return Ok(Self {
                    file: 0,
                    channel: 0,
                    submode: 8,
                    coding: 0,
                    payload: bytes,
                });
            }
            _ => return Err(Error::Invalid("sector length")),
        };
        if data[..4] != data[4..8] {
            return Err(Error::Invalid("XA subheader copies disagree"));
        }
        let payload_size = if data[2] & 0x20 != 0 { 2324 } else { 2048 };
        Ok(Self {
            file: data[0],
            channel: data[1],
            submode: data[2],
            coding: data[3],
            payload: &data[8..8 + payload_size],
        })
    }
    pub fn is_audio(self) -> bool {
        self.submode & 0x64 == 0x64
    }
    pub fn str_header(self) -> Result<Option<StrHeader>, Error> {
        if self.is_audio() || self.payload.get(..4) != Some(&[0x60, 1, 1, 0x80]) {
            return Ok(None);
        }
        StrHeader::parse(self.payload).map(Some)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrHeader {
    pub chunk: u16,
    pub chunks: u16,
    pub frame_number: u32,
    pub frame_size: u32,
    pub width: u16,
    pub height: u16,
}
impl StrHeader {
    pub fn parse(payload: &[u8]) -> Result<Self, Error> {
        if payload.len() < 32 {
            return Err(Error::Truncated);
        }
        if payload[..4] != [0x60, 1, 1, 0x80] {
            return Err(Error::Invalid("STR signature/type"));
        }
        let h = Self {
            chunk: le16(payload, 4),
            chunks: le16(payload, 6),
            frame_number: le32(payload, 8),
            frame_size: le32(payload, 12),
            width: le16(payload, 16),
            height: le16(payload, 18),
        };
        if h.chunks == 0
            || h.chunks > 256
            || h.chunk >= h.chunks
            || h.frame_size < 8
            || h.frame_size as usize > h.chunks as usize * 2016
        {
            return Err(Error::Invalid("STR chunk/size bounds"));
        }
        if h.width == 0 || h.height == 0 || h.width > 1024 || h.height > 512 {
            return Err(Error::Invalid("STR dimensions"));
        }
        Ok(h)
    }
}
