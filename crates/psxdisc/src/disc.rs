// SPDX-License-Identifier: GPL-3.0-only
use crate::{
    Error, Result, cue,
    error::{bounds, buffer, invalid},
};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

pub const RAW_SIZE: usize = 2352;
pub const LOGICAL_SIZE: usize = 2048;
const SYNC: [u8; 12] = [0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Raw2352,
    Iso2048,
}
impl ImageFormat {
    pub fn sector_size(self) -> usize {
        match self {
            Self::Raw2352 => RAW_SIZE,
            Self::Iso2048 => LOGICAL_SIZE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectorKind {
    Mode1,
    Mode2Form1,
    Mode2Form2,
    Iso2048,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sector {
    pub lba: u32,
    pub kind: SectorKind,
    bytes: Vec<u8>,
}
impl Sector {
    pub(crate) fn parse(lba: u32, bytes: Vec<u8>, format: ImageFormat) -> Result<Self> {
        if bytes.len() != format.sector_size() {
            return Err(invalid("incomplete sector"));
        }
        let kind = match format {
            ImageFormat::Iso2048 => SectorKind::Iso2048,
            ImageFormat::Raw2352 => {
                if bytes[..12] != SYNC {
                    return Err(invalid(format!("missing CD sync at sector {lba}")));
                }
                match bytes[15] {
                    1 => SectorKind::Mode1,
                    2 => {
                        if bytes[16..20] != bytes[20..24] {
                            return Err(invalid(format!(
                                "XA subheader copies disagree at sector {lba}"
                            )));
                        }
                        if bytes[18] & 0x20 != 0 {
                            SectorKind::Mode2Form2
                        } else {
                            SectorKind::Mode2Form1
                        }
                    }
                    mode => {
                        return Err(Error::Unsupported(format!(
                            "CD sector mode {mode} at sector {lba}"
                        )));
                    }
                }
            }
        };
        Ok(Self { lba, kind, bytes })
    }
    /// The original 2352 bytes, including header, subheader and EDC/ECC.
    pub fn raw(&self) -> Result<&[u8]> {
        if self.kind == SectorKind::Iso2048 {
            return Err(Error::Unsupported(
                "2048-byte sectors have no raw XA/header/ECC bytes".into(),
            ));
        }
        Ok(&self.bytes)
    }
    /// MODE1/Form1/ISO user data. Form2 is explicitly rejected, never truncated.
    pub fn logical_data(&self) -> Result<&[u8]> {
        match self.kind {
            SectorKind::Mode1 => Ok(&self.bytes[16..2064]),
            SectorKind::Mode2Form1 => Ok(&self.bytes[24..2072]),
            SectorKind::Iso2048 => Ok(&self.bytes),
            SectorKind::Mode2Form2 => Err(Error::Unsupported(
                "XA Form2 is a 2324-byte stream sector; use raw() or xa_data()".into(),
            )),
        }
    }
    /// XA sector without the 16-byte sync/header: 2336 bytes, matching .xa exports.
    pub fn xa_raw(&self) -> Result<&[u8]> {
        match self.kind {
            SectorKind::Mode2Form1 | SectorKind::Mode2Form2 => Ok(&self.bytes[16..]),
            _ => Err(Error::Unsupported("this sector has no XA subheader".into())),
        }
    }
    pub fn xa_subheader(&self) -> Result<&[u8]> {
        Ok(&self.xa_raw()?[..8])
    }
    /// 2324 bytes for Form2, 2048 bytes for Form1, excluding the XA subheader.
    pub fn xa_data(&self) -> Result<&[u8]> {
        let xa = self.xa_raw()?;
        let size = if self.kind == SectorKind::Mode2Form2 {
            2324
        } else {
            2048
        };
        Ok(&xa[8..8 + size])
    }
}

/// Mutable reads keep seek positions explicit; create separate handles for concurrent streams.
pub trait SectorReader {
    fn sector_count(&self) -> u32;
    fn read_sector(&mut self, lba: u32) -> Result<Sector>;

    /// Range in concatenated 2336-byte XA sectors, retaining subheaders/EDC/ECC.
    fn copy_xa(
        &mut self,
        start: u32,
        offset: u64,
        length: u64,
        out: &mut dyn Write,
    ) -> Result<u64> {
        copy_xa_blocks(
            self.sector_count(),
            start,
            offset,
            length,
            out,
            |first, count| {
                let mut bytes = buffer(u64::from(count) * 2352)?;
                for i in 0..count {
                    bytes[i as usize * 2352..(i as usize + 1) * 2352]
                        .copy_from_slice(self.read_sector(first + i)?.raw()?);
                }
                Ok(bytes)
            },
        )
    }

    fn read_sectors(&mut self, start: u32, count: u32) -> Result<Vec<Sector>> {
        bounds(start.into(), count.into(), self.sector_count().into())?;
        let mut result = Vec::new();
        result
            .try_reserve_exact(count as usize)
            .map_err(|_| invalid("sector range too large; stream it instead"))?;
        for lba in start..start + count {
            result.push(self.read_sector(lba)?);
        }
        Ok(result)
    }

    fn copy_logical(
        &mut self,
        start: u32,
        offset: u64,
        length: u64,
        out: &mut dyn Write,
    ) -> Result<u64> {
        let first = u64::from(start)
            .checked_add(offset / 2048)
            .ok_or_else(|| invalid("sector offset overflow"))?;
        let mut skip = (offset % 2048) as usize;
        let sector_count = (length
            .checked_add(skip as u64)
            .ok_or_else(|| invalid("read length overflow"))?)
        .div_ceil(2048);
        bounds(first, sector_count, self.sector_count().into())?;
        let mut remaining = length;
        for i in 0..sector_count {
            let sector = self.read_sector((first + i) as u32)?;
            let data = sector.logical_data()?;
            let take = remaining.min((2048 - skip) as u64) as usize;
            out.write_all(&data[skip..skip + take])?;
            remaining -= take as u64;
            skip = 0;
        }
        Ok(length)
    }

    fn read_logical(&mut self, start: u32, length: u64) -> Result<Vec<u8>> {
        bounds(
            start.into(),
            length.div_ceil(2048),
            self.sector_count().into(),
        )?;
        let mut result = buffer(length)?;
        self.copy_logical(start, 0, length, &mut result.as_mut_slice())?;
        Ok(result)
    }
}

pub struct DiscImage<R> {
    reader: R,
    format: ImageFormat,
    offset: u64,
    sectors: u32,
}

impl<R: Read + Seek> DiscImage<R> {
    pub fn new(mut reader: R, format: ImageFormat) -> Result<Self> {
        let length = reader.seek(SeekFrom::End(0))?;
        Self::with_region(reader, format, 0, length)
    }
    /// Detect using sync and the PVD position, including ambiguous file lengths.
    pub fn detect(mut reader: R) -> Result<Self> {
        let length = reader.seek(SeekFrom::End(0))?;
        let mut probe = [0; 12];
        reader.seek(SeekFrom::Start(0))?;
        if length >= 12 {
            reader.read_exact(&mut probe)?;
        }
        if probe == SYNC && length.is_multiple_of(2352) {
            return Self::new(reader, ImageFormat::Raw2352);
        }
        if length >= 17 * 2048 && length.is_multiple_of(2048) {
            reader.seek(SeekFrom::Start(16 * 2048 + 1))?;
            let mut id = [0; 5];
            reader.read_exact(&mut id)?;
            if &id == b"CD001" {
                return Self::new(reader, ImageFormat::Iso2048);
            }
        }
        Err(Error::Unsupported(
            "choose a raw 2352-byte BIN, a 2048-byte ISO, or a single-data-track CUE".into(),
        ))
    }
    pub(crate) fn with_region(
        mut reader: R,
        format: ImageFormat,
        offset: u64,
        length: u64,
    ) -> Result<Self> {
        let width = format.sector_size() as u64;
        let total = reader.seek(SeekFrom::End(0))?;
        bounds(offset, length, total)?;
        if length == 0 || !length.is_multiple_of(width) {
            return Err(invalid("image has a partial or empty sector"));
        }
        let sectors =
            u32::try_from(length / width).map_err(|_| invalid("image has too many sectors"))?;
        Ok(Self {
            reader,
            format,
            offset,
            sectors,
        })
    }
    pub fn format(&self) -> ImageFormat {
        self.format
    }
    pub fn image_bytes(&self) -> u64 {
        u64::from(self.sectors) * self.format.sector_size() as u64
    }
    /// Read contiguous on-image bytes (raw bytes for BIN; logical bytes for ISO).
    pub fn read_image_range(&mut self, start: u32, count: u32) -> Result<Vec<u8>> {
        bounds(start.into(), count.into(), self.sectors.into())?;
        let width = self.format.sector_size() as u64;
        let mut bytes = buffer(u64::from(count) * width)?;
        self.reader
            .seek(SeekFrom::Start(self.offset + u64::from(start) * width))?;
        self.reader.read_exact(&mut bytes)?;
        Ok(bytes)
    }
    pub fn copy_image(&mut self, out: &mut dyn Write) -> Result<u64> {
        self.reader.seek(SeekFrom::Start(self.offset))?;
        let expected = self.image_bytes();
        let copied = std::io::copy(&mut self.reader.by_ref().take(expected), out)?;
        if copied != expected {
            return Err(invalid("image changed or was truncated during import"));
        }
        Ok(copied)
    }
}

impl DiscImage<File> {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if path
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("cue"))
        {
            cue::open(path)
        } else {
            Self::detect(File::open(path)?)
        }
    }
}

impl<R: Read + Seek> SectorReader for DiscImage<R> {
    fn sector_count(&self) -> u32 {
        self.sectors
    }
    fn read_sector(&mut self, lba: u32) -> Result<Sector> {
        Sector::parse(lba, self.read_image_range(lba, 1)?, self.format)
    }

    fn copy_xa(
        &mut self,
        start: u32,
        offset: u64,
        length: u64,
        out: &mut dyn Write,
    ) -> Result<u64> {
        if self.format != ImageFormat::Raw2352 {
            return Err(Error::Unsupported(
                "ISO sectors have no XA stream bytes; use your raw BIN/CUE".into(),
            ));
        }
        copy_xa_blocks(self.sectors, start, offset, length, out, |first, count| {
            self.read_image_range(first, count)
        })
    }

    fn copy_logical(
        &mut self,
        start: u32,
        offset: u64,
        length: u64,
        out: &mut dyn Write,
    ) -> Result<u64> {
        let first = u64::from(start)
            .checked_add(offset / 2048)
            .ok_or_else(|| invalid("sector offset overflow"))?;
        let mut skip = (offset % 2048) as usize;
        let count = length
            .checked_add(skip as u64)
            .ok_or_else(|| invalid("read length overflow"))?
            .div_ceil(2048);
        bounds(first, count, self.sectors.into())?;
        if length == 0 {
            return Ok(0);
        }
        if self.format == ImageFormat::Iso2048 {
            self.reader
                .seek(SeekFrom::Start(self.offset + first * 2048 + skip as u64))?;
            let copied = std::io::copy(&mut self.reader.by_ref().take(length), out)?;
            if copied != length {
                return Err(invalid("ISO was truncated during read"));
            }
            return Ok(copied);
        }
        let mut completed = 0_u64;
        let mut remaining = length;
        while completed < count {
            let take = (count - completed).min(256) as u32;
            let bytes = self.read_image_range((first + completed) as u32, take)?;
            for (i, raw) in bytes.as_chunks::<RAW_SIZE>().0.iter().enumerate() {
                let sector = Sector::parse(
                    (first + completed + i as u64) as u32,
                    raw.to_vec(),
                    ImageFormat::Raw2352,
                )?;
                let logical = sector.logical_data()?;
                let amount = remaining.min((2048 - skip) as u64) as usize;
                out.write_all(&logical[skip..skip + amount])?;
                remaining -= amount as u64;
                skip = 0;
            }
            completed += u64::from(take);
        }
        Ok(length)
    }
}

pub(crate) fn copy_xa_blocks(
    sectors: u32,
    start: u32,
    offset: u64,
    length: u64,
    out: &mut dyn Write,
    mut read: impl FnMut(u32, u32) -> Result<Vec<u8>>,
) -> Result<u64> {
    let first = u64::from(start)
        .checked_add(offset / 2336)
        .ok_or_else(|| invalid("XA offset overflow"))?;
    let mut skip = (offset % 2336) as usize;
    let count = length
        .checked_add(skip as u64)
        .ok_or_else(|| invalid("XA read overflow"))?
        .div_ceil(2336);
    bounds(first, count, sectors.into())?;
    if length == 0 {
        return Ok(0);
    }
    let mut completed = 0;
    let mut remaining = length;
    while completed < count {
        let take = (count - completed).min(256) as u32;
        let bytes = read((first + completed) as u32, take)?;
        if bytes.len() != take as usize * 2352 {
            return Err(invalid("incomplete raw XA range"));
        }
        for (i, raw) in bytes.as_chunks::<RAW_SIZE>().0.iter().enumerate() {
            let sector = Sector::parse(
                (first + completed + i as u64) as u32,
                raw.to_vec(),
                ImageFormat::Raw2352,
            )?;
            let xa = sector.xa_raw()?;
            let amount = remaining.min((2336 - skip) as u64) as usize;
            out.write_all(&xa[skip..skip + amount])?;
            remaining -= amount as u64;
            skip = 0;
        }
        completed += u64::from(take);
    }
    Ok(length)
}
