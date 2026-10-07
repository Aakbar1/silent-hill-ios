// SPDX-License-Identifier: GPL-3.0-only
// Table layout/name and size rules derived from silent-hill-decomp:
// Copyright (C) 2026 shdecompilations, GPL-3.0-only. See NOTICE.md and LICENSE.
use crate::{
    DiscImage, Error, IsoEntry, IsoFileSystem, Result, SectorReader,
    error::{bounds, buffer, invalid},
};
use sha2::{Digest, Sha256};
use std::{fs::File, io::Write, path::Path};

pub const US_11_SHA256: &str = "e73859ccd2e8000d259c6fe640bb8a6d55fed6044f67fbf071e3d86c0f202398";
const TABLE_OFFSET: usize = 0xb91c;
const ENTRY_COUNT: usize = 2074;
const DIRS: [&str; 11] = [
    "1ST", "ANIM", "BG", "CHARA", "ITEM", "MISC", "SND", "TEST", "TIM", "VIN", "XA",
];
const EXTS: [&str; 12] = [
    "TIM", "VAB", "BIN", "DMS", "ANM", "PLM", "IPD", "ILM", "TMD", "DAT", "KDT", "CMP",
];
pub(crate) const FILES: [&str; 4] = ["SYSTEM.CNF", "SLUS_007.07", "SILENT.", "HILL."];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Release {
    Us11,
}

/// Hash the entire ISO boot executable, not its filename or a prefix.
pub fn verify(source: &mut impl SectorReader) -> Result<Release> {
    let iso = IsoFileSystem::open(source)?;
    let executable = iso.find(source, "SLUS_007.07")?;
    if executable.is_directory || executable.size > 16 * 1024 * 1024 {
        return Err(invalid("boot executable is not a file of at most 16 MiB"));
    }
    verify_executable(&source.read_logical(executable.extent, executable.size)?)
}

fn verify_executable(executable: &[u8]) -> Result<Release> {
    let actual = format!("{:x}", Sha256::digest(executable));
    if actual != US_11_SHA256 {
        return Err(Error::WrongRelease {
            actual_sha256: actual,
        });
    }
    Ok(Release::Us11)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Archive {
    Silent,
    Hill,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: u32,
    pub name: String,
    pub path: String,
    pub archive: Archive,
    /// Absolute LBA, exactly the game's startSector field. No lead-in subtraction.
    pub start_sector: u32,
    pub block_count: u16,
    /// Fs_GetFileSize = block_count * 256. For XA this is stream metadata, not byte length.
    pub table_size: u32,
    /// Data: rounded 2048-byte allocation. XA: distance to next XA entry/container end.
    pub sector_count: u32,
    /// Data: table_size. XA: sector_count * 2336, including subheaders/EDC/ECC.
    pub size: u64,
    pub file_type: u8,
}

pub struct GameDisc<S> {
    source: S,
    entries: Vec<Entry>,
    pub(crate) files: [IsoEntry; 4],
}

impl<R: std::io::Read + std::io::Seek> GameDisc<DiscImage<R>> {
    /// Only creates an index after the release hash has passed.
    pub fn from_image(mut source: DiscImage<R>) -> Result<Self> {
        let iso = IsoFileSystem::open(&mut source)?;
        let mut files = Vec::new();
        for name in FILES {
            files.push(iso.find(&mut source, name)?);
        }
        Self::from_parts(
            source,
            files
                .try_into()
                .map_err(|_| invalid("missing disc files"))?,
        )
    }
}

impl GameDisc<DiscImage<File>> {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_image(DiscImage::open(path)?)
    }
}

impl<S: SectorReader> GameDisc<S> {
    pub(crate) fn from_parts(mut source: S, files: [IsoEntry; 4]) -> Result<Self> {
        for file in &files {
            if file.is_directory {
                return Err(invalid("game container is a directory"));
            }
            bounds(
                file.extent.into(),
                file.size.div_ceil(2048),
                source.sector_count().into(),
            )?;
        }
        // PORT: avoid allocating an unbounded executable on an untrusted image.
        if files[1].size > 16 * 1024 * 1024 {
            return Err(Error::Unsupported("boot executable exceeds 16 MiB".into()));
        }
        let executable = source.read_logical(files[1].extent, files[1].size)?;
        verify_executable(&executable)?;
        let table = executable
            .get(TABLE_OFFSET..TABLE_OFFSET + ENTRY_COUNT * 12)
            .ok_or_else(|| invalid("truncated game table"))?;
        let entries = parse_table(table, &files[2], &files[3])?;
        Ok(Self {
            source,
            entries,
            files,
        })
    }
    pub fn release(&self) -> Release {
        Release::Us11
    }
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }
    pub fn into_source(self) -> S {
        self.source
    }
    pub fn entry(&self, id: u32) -> Result<&Entry> {
        self.entries
            .get(id as usize)
            .ok_or_else(|| Error::NotFound(format!("game file ID {id}")))
    }
    /// Full path is preferred. A bare name is accepted only if unique.
    pub fn entry_by_name(&self, name: &str) -> Result<&Entry> {
        let name = name.replace('\\', "/");
        let name = name.trim_start_matches('/');
        let mut matches = self.entries.iter().filter(|e| {
            if name.contains('/') {
                e.path.eq_ignore_ascii_case(name)
            } else {
                e.name.eq_ignore_ascii_case(name)
            }
        });
        let first = matches
            .next()
            .ok_or_else(|| Error::NotFound(name.to_owned()))?;
        if matches.next().is_some() {
            return Err(Error::Ambiguous(name.to_owned()));
        }
        Ok(first)
    }
    pub fn read_entry_by_name(&mut self, name: &str) -> Result<Vec<u8>> {
        let id = self.entry_by_name(name)?.id;
        self.read_entry(id)
    }
    pub fn read_entry(&mut self, id: u32) -> Result<Vec<u8>> {
        let size = self.entry(id)?.size;
        let mut bytes = buffer(size)?;
        self.copy_entry_range(id, 0, size, &mut bytes.as_mut_slice())?;
        Ok(bytes)
    }
    pub fn read_entry_range(&mut self, id: u32, offset: u64, length: u64) -> Result<Vec<u8>> {
        bounds(offset, length, self.entry(id)?.size)?;
        let mut bytes = buffer(length)?;
        self.copy_entry_range(id, offset, length, &mut bytes.as_mut_slice())?;
        Ok(bytes)
    }
    pub fn copy_entry(&mut self, id: u32, out: &mut dyn Write) -> Result<u64> {
        let size = self.entry(id)?.size;
        self.copy_entry_range(id, 0, size, out)
    }
    pub fn copy_entry_range(
        &mut self,
        id: u32,
        offset: u64,
        length: u64,
        out: &mut dyn Write,
    ) -> Result<u64> {
        let entry = self.entry(id)?;
        bounds(offset, length, entry.size)?;
        let start = entry.start_sector;
        let archive = entry.archive;
        if length == 0 {
            return Ok(0);
        }
        if archive == Archive::Silent {
            return self.source.copy_logical(start, offset, length, out);
        }
        self.source.copy_xa(start, offset, length, out)
    }
    /// Complete 2352-byte sectors for XA decoding/streaming, with bounded entry-relative seeks.
    pub fn copy_entry_raw_sectors(
        &mut self,
        id: u32,
        offset: u32,
        count: u32,
        out: &mut dyn Write,
    ) -> Result<u64> {
        let entry = self.entry(id)?;
        bounds(offset.into(), count.into(), entry.sector_count.into())?;
        let start = entry.start_sector + offset;
        for i in 0..count {
            out.write_all(self.source.read_sector(start + i)?.raw()?)?;
        }
        Ok(u64::from(count) * 2352)
    }
    /// ISO container/executable reads. HILL's XA sectors require the stream API.
    pub fn read_disc_file(&mut self, name: &str) -> Result<Vec<u8>> {
        if name.eq_ignore_ascii_case("HILL.") {
            return Err(Error::Unsupported(
                "HILL contains XA sectors; read its entries with the XA/raw streaming API".into(),
            ));
        }
        let file = self
            .files
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| Error::NotFound(name.into()))?;
        self.source.read_logical(file.extent, file.size)
    }
}

fn parse_table(table: &[u8], silent: &IsoEntry, hill: &IsoEntry) -> Result<Vec<Entry>> {
    if !table.len().is_multiple_of(12) {
        return Err(invalid("partial file table entry"));
    }
    let mut entries = Vec::new();
    for (id, bytes) in table.as_chunks::<12>().0.iter().enumerate() {
        let meta = u32::from_le_bytes(
            bytes[..4]
                .try_into()
                .map_err(|_| invalid("table metadata"))?,
        );
        let part1 = u32::from_le_bytes(bytes[4..8].try_into().map_err(|_| invalid("table name"))?);
        let part2 = u32::from_le_bytes(bytes[8..12].try_into().map_err(|_| invalid("table name"))?);
        if meta >> 31 != 0 || part1 >> 28 != 0 || part2 >> 28 != 0 {
            return Err(invalid("reserved file-table bits"));
        }
        let directory = (part1 & 15) as usize;
        let dir = DIRS
            .get(directory)
            .ok_or_else(|| invalid("file-table directory index"))?;
        let file_type = ((part2 >> 24) & 15) as u8;
        let mut name = String::new();
        for i in 0..8 {
            let part = if i < 4 { part1 >> 4 } else { part2 };
            let c = (part >> (6 * (i % 4))) & 63;
            if c == 0 {
                break;
            }
            name.push((c as u8 + 32) as char);
        }
        if name.is_empty() || name.contains(['/', '\\']) || matches!(name.as_str(), "." | "..") {
            return Err(invalid("invalid game filename"));
        }
        if file_type != 15 {
            let ext = EXTS
                .get(file_type as usize)
                .ok_or_else(|| invalid("file-table extension index"))?;
            name.push('.');
            name.push_str(ext);
        }
        let archive = if directory == 10 {
            Archive::Hill
        } else {
            Archive::Silent
        };
        if (archive == Archive::Hill) != (file_type == 15) {
            return Err(invalid("XA directory/type disagree"));
        }
        let start_sector = meta & 0x7ffff;
        let block_count = ((meta >> 19) & 0xfff) as u16;
        let table_size = u32::from(block_count) * 256;
        let sector_count = table_size.div_ceil(2048);
        let container = if archive == Archive::Hill {
            hill
        } else {
            silent
        };
        if start_sector < container.extent {
            return Err(invalid("entry begins before its container"));
        }
        bounds(
            u64::from(start_sector - container.extent) * 2048,
            if archive == Archive::Hill {
                0
            } else {
                table_size.into()
            },
            container.size,
        )?;
        entries.push(Entry {
            id: id as u32,
            path: format!("{dir}/{name}"),
            name,
            archive,
            start_sector,
            block_count,
            table_size,
            sector_count,
            size: table_size.into(),
            file_type,
        });
    }
    let xa: Vec<_> = entries
        .iter()
        .filter(|e| e.archive == Archive::Hill)
        .map(|e| (e.id as usize, e.start_sector))
        .collect();
    let end = u64::from(hill.extent) + hill.size.div_ceil(2048);
    for (i, (id, start)) in xa.iter().enumerate() {
        let next = xa.get(i + 1).map_or(end, |e| u64::from(e.1));
        if next <= u64::from(*start) {
            return Err(invalid("XA entries are not strictly increasing"));
        }
        let count =
            u32::try_from(next - u64::from(*start)).map_err(|_| invalid("XA span overflow"))?;
        entries[*id].sector_count = count;
        entries[*id].size = u64::from(count) * 2336;
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn container(name: &str, lba: u32, size: u64) -> IsoEntry {
        IsoEntry {
            name: name.into(),
            extent: lba,
            size,
            is_directory: false,
        }
    }
    fn record(start: u32, blocks: u32, dir: u32, name: &str, kind: u32) -> Vec<u8> {
        let mut parts = [0; 2];
        for (i, c) in name.bytes().enumerate().take(8) {
            parts[i / 4] |= u32::from(c - 32) << (6 * (i % 4));
        }
        [
            start | (blocks << 19),
            dir | (parts[0] << 4),
            parts[1] | kind << 24,
        ]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect()
    }
    #[test]
    fn table_bitfields_ids_zero_lengths_and_xa_spans() {
        let mut table = record(64, 9, 0, "BODYPROG", 2);
        table.extend(record(66, 0, 0, "EMPTY", 0));
        table.extend(record(100, 1000, 10, "STREAM", 15));
        table.extend(record(110, 1, 10, "LAST", 15));
        let entries = parse_table(
            &table,
            &container("SILENT.", 64, 4096),
            &container("HILL.", 100, 20 * 2048),
        )
        .unwrap();
        assert_eq!(
            (
                entries[0].id,
                entries[0].table_size,
                entries[0].sector_count
            ),
            (0, 2304, 2)
        );
        assert_eq!(entries[0].path, "1ST/BODYPROG.BIN");
        assert_eq!(entries[1].size, 0);
        assert_eq!(
            (entries[2].table_size, entries[2].size),
            (256000, 10 * 2336)
        );
        assert_eq!(entries[3].size, 10 * 2336);
    }
    #[test]
    fn table_rejects_invalid_indices_bounds_and_order() {
        let silent = container("SILENT.", 64, 2048);
        let hill = container("HILL.", 100, 20 * 2048);
        for bytes in [
            record(64, 1, 15, "BAD", 0),
            record(63, 1, 0, "BAD", 0),
            record(64, 9, 0, "BAD", 0),
            record(100, 1, 10, "BAD", 12),
            [record(110, 1, 10, "XA1", 15), record(100, 1, 10, "XA2", 15)].concat(),
        ] {
            assert!(parse_table(&bytes, &silent, &hill).is_err());
        }
    }
    #[test]
    fn wrong_release_is_friendly() {
        let error = verify_executable(b"PS-X EXE wrong version").unwrap_err();
        assert!(matches!(error, Error::WrongRelease { .. }));
        assert!(error.to_string().contains("US v1.1"));
    }
}
