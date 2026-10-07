// SPDX-License-Identifier: GPL-3.0-only
//! Raw MODE2/2352 Form 1 sector access and read-only ISO9660 root inspection.
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

pub const RAW_SECTOR_BYTES: usize = 2352;
pub const DATA_BYTES: usize = 2048;
const DATA_OFFSET: usize = 24;
const SYNC: [u8; 12] = [0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0];

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

/// Form 2/XA sectors are deliberately rejected rather than truncated to Form 1 data.
pub fn form1_data(sector: &[u8; RAW_SECTOR_BYTES]) -> io::Result<&[u8]> {
    if sector[..12] != SYNC || sector[15] != 2 {
        return Err(invalid("expected a synced MODE2/2352 sector"));
    }
    if sector[16..20] != sector[20..24] {
        return Err(invalid("MODE2 duplicated subheaders differ"));
    }
    if sector[18] & 0x20 != 0 {
        return Err(invalid("MODE2 Form 2/XA is not a 2048-byte data sector"));
    }
    Ok(&sector[DATA_OFFSET..DATA_OFFSET + DATA_BYTES])
}

pub struct RawDisc {
    file: File,
    sectors: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DirectoryEntry {
    pub name: String,
    pub lba: u32,
    pub bytes: u32,
    pub is_directory: bool,
}

impl RawDisc {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let bytes = file.metadata()?.len();
        if bytes == 0 || !bytes.is_multiple_of(RAW_SECTOR_BYTES as u64) {
            return Err(invalid("disc size must be a nonzero multiple of 2352"));
        }
        Ok(Self {
            file,
            sectors: bytes / RAW_SECTOR_BYTES as u64,
        })
    }

    pub fn sector_count(&self) -> u64 {
        self.sectors
    }

    pub fn read_sector(&mut self, lba: u64) -> io::Result<[u8; DATA_BYTES]> {
        if lba >= self.sectors {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "sector is outside disc",
            ));
        }
        self.file
            .seek(SeekFrom::Start(lba * RAW_SECTOR_BYTES as u64))?;
        let mut raw = [0; RAW_SECTOR_BYTES];
        self.file.read_exact(&mut raw)?;
        let mut data = [0; DATA_BYTES];
        data.copy_from_slice(form1_data(&raw)?);
        Ok(data)
    }

    /// Byte count is exact: padding at the end of the final sector is not returned.
    pub fn read_extent(&mut self, lba: u32, bytes: u32) -> io::Result<Vec<u8>> {
        let count = u64::from(bytes).div_ceil(DATA_BYTES as u64);
        if u64::from(lba) + count > self.sectors {
            return Err(invalid("extent exceeds disc"));
        }
        let mut result = Vec::new();
        result
            .try_reserve_exact(bytes as usize)
            .map_err(io::Error::other)?;
        for sector in u64::from(lba)..u64::from(lba) + count {
            let data = self.read_sector(sector)?;
            let remaining = bytes as usize - result.len();
            result.extend_from_slice(&data[..remaining.min(DATA_BYTES)]);
        }
        Ok(result)
    }

    pub fn root_directory(&mut self) -> io::Result<Vec<DirectoryEntry>> {
        let pvd = self.read_sector(16)?;
        if pvd[..7] != *b"\x01CD001\x01" {
            return Err(invalid("no ISO9660 primary volume descriptor at LBA 16"));
        }
        if both_u16(&pvd[128..132])? != DATA_BYTES as u16 {
            return Err(invalid("expected 2048-byte ISO logical blocks"));
        }
        let len = usize::from(pvd[156]);
        if !(34..=DATA_BYTES - 156).contains(&len) {
            return Err(invalid("invalid ISO root directory record"));
        }
        let root = parse_record(&pvd[156..156 + len])?;
        if !root.is_directory || root.bytes > 2 * 1024 * 1024 {
            return Err(invalid("invalid or oversized root directory"));
        }
        let bytes = self.read_extent(root.lba, root.bytes)?;
        parse_directory(&bytes)
    }
}

fn both_u16(bytes: &[u8]) -> io::Result<u16> {
    let little = u16::from_le_bytes([bytes[0], bytes[1]]);
    let big = u16::from_be_bytes([bytes[2], bytes[3]]);
    if little != big {
        return Err(invalid("ISO little/big endian values differ"));
    }
    Ok(little)
}

fn both_u32(bytes: &[u8]) -> io::Result<u32> {
    let little = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let big = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    if little != big {
        return Err(invalid("ISO little/big endian values differ"));
    }
    Ok(little)
}

fn parse_record(record: &[u8]) -> io::Result<DirectoryEntry> {
    if record.len() < 34 || usize::from(record[0]) != record.len() {
        return Err(invalid("short or mismatched ISO directory record"));
    }
    let name_len = usize::from(record[32]);
    if name_len == 0 || 33 + name_len > record.len() {
        return Err(invalid("invalid ISO identifier length"));
    }
    if record[1] != 0 || record[26] != 0 || record[27] != 0 || record[25] & 0x80 != 0 {
        return Err(invalid(
            "extended, interleaved or multi-extent ISO file unsupported",
        ));
    }
    let name = match &record[33..33 + name_len] {
        [0] => ".".to_owned(),
        [1] => "..".to_owned(),
        bytes => std::str::from_utf8(bytes)
            .map_err(|_| invalid("non-UTF8 ISO identifier"))?
            .to_owned(),
    };
    Ok(DirectoryEntry {
        name,
        lba: both_u32(&record[2..10])?,
        bytes: both_u32(&record[10..18])?,
        is_directory: record[25] & 2 != 0,
    })
}

fn parse_directory(bytes: &[u8]) -> io::Result<Vec<DirectoryEntry>> {
    let mut entries = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let len = usize::from(bytes[offset]);
        if len == 0 {
            offset = (offset / DATA_BYTES + 1) * DATA_BYTES;
            continue;
        }
        if len > DATA_BYTES - offset % DATA_BYTES || offset + len > bytes.len() {
            return Err(invalid(
                "ISO directory record crosses a block or extent boundary",
            ));
        }
        entries.push(parse_record(&bytes[offset..offset + len])?);
        offset += len;
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sector() -> [u8; RAW_SECTOR_BYTES] {
        let mut raw = [0; RAW_SECTOR_BYTES];
        raw[..12].copy_from_slice(&SYNC);
        raw[15] = 2;
        raw[24] = 77;
        raw[24 + DATA_BYTES - 1] = 91;
        raw[24 + DATA_BYTES] = 42;
        raw
    }

    fn record(name: &[u8]) -> Vec<u8> {
        let len = (33 + name.len()).next_multiple_of(2);
        let mut bytes = vec![0; len];
        bytes[0] = len as u8;
        bytes[2..6].copy_from_slice(&25_u32.to_le_bytes());
        bytes[6..10].copy_from_slice(&25_u32.to_be_bytes());
        bytes[10..14].copy_from_slice(&2051_u32.to_le_bytes());
        bytes[14..18].copy_from_slice(&2051_u32.to_be_bytes());
        bytes[32] = name.len() as u8;
        bytes[33..33 + name.len()].copy_from_slice(name);
        bytes
    }

    #[test]
    fn extracts_only_form1_payload() {
        let raw = sector();
        let data = form1_data(&raw).unwrap();
        assert_eq!(data.len(), DATA_BYTES);
        assert_eq!((data[0], data[DATA_BYTES - 1]), (77, 91));
    }

    #[test]
    fn rejects_form2_and_bad_headers() {
        let mut raw = sector();
        raw[18] = 0x20;
        raw[22] = 0x20;
        assert!(form1_data(&raw).is_err());
        raw = sector();
        raw[16] = 1;
        assert!(form1_data(&raw).is_err());
        raw = sector();
        raw[15] = 1;
        assert!(form1_data(&raw).is_err());
        raw = sector();
        raw[0] = 1;
        assert!(form1_data(&raw).is_err());
    }

    #[test]
    fn checks_endian_pairs_and_record_bounds() {
        let mut bytes = record(b"SYSTEM.CNF;1");
        let entry = parse_record(&bytes).unwrap();
        assert_eq!((entry.lba, entry.bytes), (25, 2051));
        assert_eq!(entry.name, "SYSTEM.CNF;1");
        bytes[6] = 1;
        assert!(parse_record(&bytes).is_err());
        assert!(parse_record(&bytes[..20]).is_err());
    }

    #[test]
    fn skips_block_padding_and_rejects_crossing_records() {
        let entry = record(b"HILL.;1");
        let mut bytes = vec![0; DATA_BYTES * 2];
        bytes[..entry.len()].copy_from_slice(&entry);
        bytes[DATA_BYTES..DATA_BYTES + entry.len()].copy_from_slice(&entry);
        assert_eq!(parse_directory(&bytes).unwrap().len(), 2);
        let mut crossing = vec![0; DATA_BYTES];
        for start in (0..2000).step_by(40) {
            let item = record(b"ABCDE;1");
            assert_eq!(item.len(), 40);
            crossing[start..start + 40].copy_from_slice(&item);
        }
        crossing[2000] = 50;
        assert!(parse_directory(&crossing).is_err());
    }

    #[test]
    fn reads_multisector_extents_and_checks_eof() {
        let path = std::env::temp_dir().join(format!("sh-boot-disc-{}.tmp", std::process::id()));
        let mut bytes = sector().to_vec();
        let mut second = sector();
        second[24] = 13;
        bytes.extend_from_slice(&second);
        std::fs::write(&path, &bytes).unwrap();
        let result = (|| {
            let mut disc = RawDisc::open(&path)?;
            assert_eq!(disc.sector_count(), 2);
            let extent = disc.read_extent(0, 2049)?;
            assert_eq!(extent.len(), 2049);
            assert_eq!(extent[2048], 13);
            assert!(disc.read_sector(2).is_err());
            assert!(disc.read_extent(1, 2049).is_err());
            Ok::<(), io::Error>(())
        })();
        std::fs::remove_file(&path).unwrap();
        result.unwrap();
        std::fs::write(&path, [0; 11]).unwrap();
        let rejected = RawDisc::open(&path).is_err();
        std::fs::remove_file(&path).unwrap();
        assert!(rejected);
    }
}
