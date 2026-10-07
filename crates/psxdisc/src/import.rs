// SPDX-License-Identifier: GPL-3.0-only
use crate::{
    DiscImage, Error, GameDisc, ImageFormat, IsoEntry, Result, Sector, SectorReader,
    disc::RAW_SIZE,
    error::{bounds, invalid},
    game::FILES,
};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const MAGIC: &[u8; 8] = b"PSXPACK1";
const HEADER_SIZE: u64 = 16 + 4 * 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImportMode {
    /// Copy the data track into app-owned storage and index it on opening.
    KeepImage,
    /// Four contiguous file regions in one file; 2048-byte data, original 2352-byte XA.
    #[default]
    Packed,
}

#[derive(Debug)]
pub struct ImportStats {
    pub mode: ImportMode,
    pub path: PathBuf,
    pub source_bytes: u64,
    pub bytes_written: u64,
    /// Includes release verification, conversion/copy, flush and sync_all.
    pub elapsed: Duration,
}

/// Import only after release verification; never overwrite an existing import.
/// The caller supplies an app-private directory (on iOS, Application Support).
/// The source Files/security-scoped URL can be released after this returns.
pub fn import(
    source: impl AsRef<Path>,
    app_data: impl AsRef<Path>,
    mode: ImportMode,
) -> Result<ImportStats> {
    let start = Instant::now();
    let mut game = GameDisc::open(source)?;
    let source_bytes = game.source_mut().image_bytes();
    let format = game.source_mut().format();
    if format != ImageFormat::Raw2352 {
        return Err(Error::Unsupported("a 2048-byte ISO has already lost the XA audio/video sector bytes; import your raw BIN/CUE for the full game".into()));
    }
    fs::create_dir_all(app_data.as_ref())?;
    let name = match mode {
        ImportMode::Packed => "game.psxpack",
        ImportMode::KeepImage => "disc.bin",
    };
    let path = app_data.as_ref().join(name);
    let mut out = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    let result = (|| {
        let bytes = match mode {
            ImportMode::KeepImage => game.source_mut().copy_image(&mut out)?,
            ImportMode::Packed => {
                let files = game.files.clone();
                write_packed(game.source_mut(), &files, &mut out)?
            }
        };
        out.flush()?;
        out.sync_all()?;
        Ok(bytes)
    })();
    drop(out);
    match result {
        Ok(bytes_written) => Ok(ImportStats {
            mode,
            path,
            source_bytes,
            bytes_written,
            elapsed: start.elapsed(),
        }),
        Err(error) => {
            // Remove only the file this invocation successfully created.
            let _ = fs::remove_file(&path);
            Err(error)
        }
    }
}

fn write_packed<R: Read + Seek>(
    source: &mut DiscImage<R>,
    files: &[IsoEntry; 4],
    out: &mut dyn Write,
) -> Result<u64> {
    let mut header = Vec::new();
    header.extend_from_slice(MAGIC);
    header.extend_from_slice(&4_u32.to_le_bytes());
    header.extend_from_slice(&source.sector_count().to_le_bytes());
    let mut offset = HEADER_SIZE;
    for (i, file) in files.iter().enumerate() {
        let width = if i == 3 { 2352_u32 } else { 2048 };
        header.extend_from_slice(&file.extent.to_le_bytes());
        header.extend_from_slice(&file.size.to_le_bytes());
        header.extend_from_slice(&width.to_le_bytes());
        header.extend_from_slice(&offset.to_le_bytes());
        offset += file.size.div_ceil(2048) * u64::from(width);
    }
    out.write_all(&header)?;
    for (i, file) in files.iter().enumerate() {
        let count =
            u32::try_from(file.size.div_ceil(2048)).map_err(|_| invalid("container too large"))?;
        let mut copied = 0;
        while copied < count {
            // Bulk reads avoid a seek/syscall for every sector during import.
            let take = (count - copied).min(512);
            let bytes = source.read_image_range(file.extent + copied, take)?;
            if i == 3 {
                out.write_all(&bytes)?;
            } else {
                let mut logical = Vec::with_capacity(take as usize * 2048);
                for (n, sector) in bytes.as_chunks::<RAW_SIZE>().0.iter().enumerate() {
                    let parsed = Sector::parse(
                        file.extent + copied + n as u32,
                        sector.to_vec(),
                        ImageFormat::Raw2352,
                    )?;
                    logical.extend_from_slice(parsed.logical_data()?);
                }
                out.write_all(&logical)?;
            }
            copied += take;
        }
    }
    Ok(offset)
}

struct Region {
    file: IsoEntry,
    width: u32,
    offset: u64,
}

/// Packed data retain original LBAs and the executable table; no asset extraction/decryption.
pub struct PackedImage {
    file: File,
    regions: [Region; 4],
    sectors: u32,
}

impl PackedImage {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut file = File::open(path)?;
        let length = file.metadata()?.len();
        let mut header = [0; HEADER_SIZE as usize];
        file.read_exact(&mut header)?;
        if &header[..8] != MAGIC || read32(&header[8..12])? != 4 {
            return Err(invalid("packed image magic/version/region count"));
        }
        let sectors = read32(&header[12..16])?;
        if sectors == 0 {
            return Err(invalid("empty packed image"));
        }
        let mut regions = Vec::new();
        let mut expected_offset = HEADER_SIZE;
        let mut previous_end = 0;
        for (i, name) in FILES.iter().enumerate() {
            let bytes = &header[16 + i * 24..16 + (i + 1) * 24];
            let extent = read32(&bytes[..4])?;
            let size = read64(&bytes[4..12])?;
            let width = read32(&bytes[12..16])?;
            let offset = read64(&bytes[16..24])?;
            if width != if i == 3 { 2352 } else { 2048 } || offset != expected_offset {
                return Err(invalid("packed image region width/order/offset"));
            }
            let count = size.div_ceil(2048);
            bounds(extent.into(), count, sectors.into())?;
            if u64::from(extent) < previous_end || size == 0 {
                return Err(invalid("overlapping or empty packed regions"));
            }
            previous_end = u64::from(extent) + count;
            let stored = count
                .checked_mul(width.into())
                .ok_or_else(|| invalid("packed region overflow"))?;
            bounds(offset, stored, length)?;
            expected_offset += stored;
            regions.push(Region {
                file: IsoEntry {
                    name: (*name).into(),
                    extent,
                    size,
                    is_directory: false,
                },
                width,
                offset,
            });
        }
        if expected_offset != length {
            return Err(invalid("packed image has trailing/unindexed bytes"));
        }
        Ok(Self {
            file,
            regions: regions
                .try_into()
                .map_err(|_| invalid("packed region count"))?,
            sectors,
        })
    }
}

impl SectorReader for PackedImage {
    fn sector_count(&self) -> u32 {
        self.sectors
    }
    fn read_sector(&mut self, lba: u32) -> Result<Sector> {
        bounds(lba.into(), 1, self.sectors.into())?;
        let region = self
            .regions
            .iter()
            .find(|r| {
                lba >= r.file.extent && u64::from(lba - r.file.extent) < r.file.size.div_ceil(2048)
            })
            .ok_or_else(|| {
                Error::NotFound(format!("sector {lba}, omitted from the packed game data"))
            })?;
        self.file.seek(SeekFrom::Start(
            region.offset + u64::from(lba - region.file.extent) * u64::from(region.width),
        ))?;
        let mut bytes = vec![0; region.width as usize];
        self.file.read_exact(&mut bytes)?;
        Sector::parse(
            lba,
            bytes,
            if region.width == 2352 {
                ImageFormat::Raw2352
            } else {
                ImageFormat::Iso2048
            },
        )
    }

    fn copy_xa(
        &mut self,
        start: u32,
        offset: u64,
        length: u64,
        out: &mut dyn Write,
    ) -> Result<u64> {
        crate::disc::copy_xa_blocks(self.sectors, start, offset, length, out, |first, count| {
            let region = &self.regions[3];
            if first < region.file.extent {
                return Err(invalid("XA read begins before HILL"));
            }
            bounds(
                u64::from(first - region.file.extent),
                count.into(),
                region.file.size.div_ceil(2048),
            )?;
            self.file.seek(SeekFrom::Start(
                region.offset + u64::from(first - region.file.extent) * 2352,
            ))?;
            let mut bytes = crate::error::buffer(u64::from(count) * 2352)?;
            self.file.read_exact(&mut bytes)?;
            Ok(bytes)
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
            .ok_or_else(|| invalid("packed read overflow"))?;
        let skip = offset % 2048;
        let count = length
            .checked_add(skip)
            .ok_or_else(|| invalid("packed read overflow"))?
            .div_ceil(2048);
        bounds(first, count, self.sectors.into())?;
        if length == 0 {
            return Ok(0);
        }
        let region = self
            .regions
            .iter()
            .find(|r| {
                first >= u64::from(r.file.extent)
                    && first - u64::from(r.file.extent) < r.file.size.div_ceil(2048)
            })
            .ok_or_else(|| Error::NotFound(format!("packed sector {first}")))?;
        if region.width != 2048 {
            return Err(Error::Unsupported(
                "HILL contains XA stream sectors; use the XA/raw streaming API".into(),
            ));
        }
        let relative = (first - u64::from(region.file.extent)) * 2048 + skip;
        bounds(relative, length, region.file.size.div_ceil(2048) * 2048)?;
        self.file.seek(SeekFrom::Start(region.offset + relative))?;
        let mut bytes = crate::error::buffer(length.min(512 * 2048))?;
        let mut remaining = length;
        while remaining != 0 {
            let amount = remaining.min(bytes.len() as u64) as usize;
            self.file.read_exact(&mut bytes[..amount])?;
            out.write_all(&bytes[..amount])?;
            remaining -= amount as u64;
        }
        Ok(length)
    }
}

impl GameDisc<PackedImage> {
    /// Reverify the retained executable before interpreting its index.
    pub fn open_packed(path: impl AsRef<Path>) -> Result<Self> {
        let source = PackedImage::open(path)?;
        let files = std::array::from_fn(|i| source.regions[i].file.clone());
        Self::from_parts(source, files)
    }
}

fn read32(bytes: &[u8]) -> Result<u32> {
    Ok(u32::from_le_bytes(
        bytes.try_into().map_err(|_| invalid("packed integer"))?,
    ))
}
fn read64(bytes: &[u8]) -> Result<u64> {
    Ok(u64::from_le_bytes(
        bytes.try_into().map_err(|_| invalid("packed integer"))?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn fixture() -> (DiscImage<Cursor<Vec<u8>>>, [IsoEntry; 4]) {
        let mut bytes = vec![0; 8 * 2352];
        for (i, raw) in bytes.as_chunks_mut::<2352>().0.iter_mut().enumerate() {
            raw[1..11].fill(255);
            raw[15] = 2;
            if i >= 4 {
                raw[18] = 0x64;
                raw[22] = 0x64;
                raw[24..2348].fill(i as u8);
            } else {
                raw[24..2072].fill(i as u8);
            }
        }
        let files = std::array::from_fn(|i| IsoEntry {
            name: FILES[i].into(),
            extent: i as u32 + 1,
            size: if i == 3 {
                3 * 2048
            } else {
                if i == 0 { 68 } else { 2048 }
            },
            is_directory: false,
        });
        (
            DiscImage::new(Cursor::new(bytes), ImageFormat::Raw2352).unwrap(),
            files,
        )
    }

    #[test]
    fn synthetic_packed_roundtrip_preserves_data_raw_xa_and_rejects_corruption() {
        let (mut image, files) = fixture();
        let mut packed = Vec::new();
        let size = write_packed(&mut image, &files, &mut packed).unwrap();
        assert_eq!(size, HEADER_SIZE + 3 * 2048 + 3 * 2352);
        let path = std::env::temp_dir().join(format!(
            "psxdisc-{}-packed-test.psxpack",
            std::process::id()
        ));
        fs::write(&path, &packed).unwrap();
        let mut reopened = PackedImage::open(&path).unwrap();
        for lba in 1..4 {
            assert_eq!(
                reopened.read_sector(lba).unwrap().logical_data().unwrap(),
                image.read_sector(lba).unwrap().logical_data().unwrap()
            );
        }
        for lba in 4..7 {
            assert_eq!(
                reopened.read_sector(lba).unwrap().raw().unwrap(),
                image.read_sector(lba).unwrap().raw().unwrap()
            );
        }
        let mut partial = Vec::new();
        reopened.copy_logical(2, 31, 1000, &mut partial).unwrap();
        assert_eq!(partial, [2; 1000]);
        assert!(reopened.copy_logical(2, 0, 2049, &mut Vec::new()).is_err());
        assert!(reopened.read_sector(0).is_err());
        assert!(reopened.read_sector(7).is_err());
        assert!(reopened.read_sector(1).unwrap().raw().is_err());
        let mut xa = Vec::new();
        reopened.copy_xa(4, 2320, 48, &mut xa).unwrap();
        let expected = [
            image.read_sector(4).unwrap().xa_raw().unwrap().to_vec(),
            image.read_sector(5).unwrap().xa_raw().unwrap().to_vec(),
        ]
        .concat();
        assert_eq!(xa, expected[2320..2368]);
        assert!(reopened.copy_xa(6, 0, 2337, &mut Vec::new()).is_err());
        assert!(GameDisc::open_packed(&path).is_err());
        drop(reopened);
        for index in [0, 8, 16, 28, 32] {
            let mut damaged = packed.clone();
            damaged[index] ^= 0x80;
            fs::write(&path, damaged).unwrap();
            assert!(PackedImage::open(&path).is_err(), "header offset {index}");
        }
        fs::write(&path, &packed[..packed.len() - 1]).unwrap();
        assert!(PackedImage::open(&path).is_err());
        let mut trailing = packed.clone();
        trailing.push(0);
        fs::write(&path, trailing).unwrap();
        assert!(PackedImage::open(&path).is_err());
        fs::remove_file(path).unwrap();
    }
}
