// SPDX-License-Identifier: GPL-3.0-only
use crate::{
    Error, Result, SectorReader,
    error::{bounds, invalid},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsoEntry {
    /// Identifier with the ISO version suffix removed. Trailing dots are retained.
    pub name: String,
    pub extent: u32,
    pub size: u64,
    pub is_directory: bool,
}

#[derive(Debug, Clone)]
pub struct IsoFileSystem {
    pub root: IsoEntry,
    pub volume_sectors: u32,
}

impl IsoFileSystem {
    pub fn open(source: &mut impl SectorReader) -> Result<Self> {
        for lba in 16..source.sector_count().min(16 + 256) {
            let sector = source.read_sector(lba)?;
            let pvd = sector.logical_data()?;
            if &pvd[1..6] != b"CD001" || pvd[6] != 1 {
                return Err(invalid("ISO9660 volume descriptor signature/version"));
            }
            match pvd[0] {
                1 => {
                    if both16(&pvd[128..132])? != 2048 {
                        return Err(Error::Unsupported(
                            "ISO logical block size must be 2048".into(),
                        ));
                    }
                    if both16(&pvd[120..124])? != 1 || both16(&pvd[124..128])? != 1 {
                        return Err(Error::Unsupported("multi-volume ISO".into()));
                    }
                    let volume_sectors = both32(&pvd[80..88])?;
                    if volume_sectors == 0 || volume_sectors > source.sector_count() {
                        return Err(invalid("ISO volume exceeds the image"));
                    }
                    let size = usize::from(pvd[156]);
                    if !(34..=2048 - 156).contains(&size) {
                        return Err(invalid("ISO root record length"));
                    }
                    let root = record(&pvd[156..156 + size], volume_sectors)?;
                    if !root.is_directory {
                        return Err(invalid("ISO root is not a directory"));
                    }
                    return Ok(Self {
                        root,
                        volume_sectors,
                    });
                }
                255 => break,
                0 | 2 | 3 => {}
                _ => return Err(invalid("unknown ISO volume descriptor")),
            }
        }
        Err(invalid("missing ISO9660 primary volume descriptor"))
    }

    pub fn read_directory(
        &self,
        source: &mut impl SectorReader,
        entry: &IsoEntry,
    ) -> Result<Vec<IsoEntry>> {
        if !entry.is_directory {
            return Err(invalid("entry is not a directory"));
        }
        // PORT: bound metadata allocation for malformed player-supplied images.
        if entry.size > 16 * 1024 * 1024 {
            return Err(Error::Unsupported("ISO directory exceeds 16 MiB".into()));
        }
        bounds(
            entry.extent.into(),
            entry.size.div_ceil(2048),
            self.volume_sectors.into(),
        )?;
        let data = source.read_logical(entry.extent, entry.size)?;
        let mut result = Vec::new();
        let mut offset = 0;
        while offset < data.len() {
            let length = usize::from(data[offset]);
            if length == 0 {
                let end = ((offset / 2048 + 1) * 2048).min(data.len());
                if data[offset..end].iter().any(|b| *b != 0) {
                    return Err(invalid("nonzero ISO directory padding"));
                }
                offset = end;
                continue;
            }
            if length < 34 || offset + length > data.len() || offset % 2048 + length > 2048 {
                return Err(invalid(
                    "ISO directory record crosses a sector or is truncated",
                ));
            }
            let bytes = &data[offset..offset + length];
            let parsed = record(bytes, self.volume_sectors)?;
            if bytes[32] != 1 || !matches!(bytes[33], 0 | 1) {
                result.push(parsed);
            }
            offset += length;
        }
        Ok(result)
    }

    pub fn find(&self, source: &mut impl SectorReader, path: &str) -> Result<IsoEntry> {
        let path = path.replace('\\', "/");
        let parts: Vec<_> = path.split('/').filter(|p| !p.is_empty()).collect();
        if parts.len() > 32 || parts.iter().any(|p| matches!(*p, "." | "..")) {
            return Err(invalid("invalid or excessively deep ISO path"));
        }
        let mut entry = self.root.clone();
        for component in parts {
            let name = component.split(';').next().unwrap_or(component);
            entry = self
                .read_directory(source, &entry)?
                .into_iter()
                .find(|e| e.name.eq_ignore_ascii_case(name))
                .ok_or_else(|| Error::NotFound(path.clone()))?;
        }
        Ok(entry)
    }

    pub fn read_file(&self, source: &mut impl SectorReader, path: &str) -> Result<Vec<u8>> {
        let entry = self.find(source, path)?;
        if entry.is_directory {
            return Err(invalid("requested ISO file is a directory"));
        }
        source.read_logical(entry.extent, entry.size)
    }
}

fn record(bytes: &[u8], sectors: u32) -> Result<IsoEntry> {
    if bytes.len() < 34 || usize::from(bytes[0]) != bytes.len() {
        return Err(invalid("ISO record length"));
    }
    let name_len = usize::from(bytes[32]);
    if name_len == 0 || 33 + name_len > bytes.len() {
        return Err(invalid("ISO identifier length"));
    }
    if bytes[26] != 0 || bytes[27] != 0 || bytes[25] & 0x80 != 0 {
        return Err(Error::Unsupported(
            "interleaved or multi-extent ISO files".into(),
        ));
    }
    if both16(&bytes[28..32])? != 1 {
        return Err(Error::Unsupported("ISO file on another volume".into()));
    }
    let extent = both32(&bytes[2..10])?
        .checked_add(bytes[1].into())
        .ok_or_else(|| invalid("extended-attribute offset overflow"))?;
    let size = u64::from(both32(&bytes[10..18])?);
    bounds(extent.into(), size.div_ceil(2048), sectors.into())?;
    let raw_name = &bytes[33..33 + name_len];
    let name = match raw_name {
        [0] => ".".to_owned(),
        [1] => "..".to_owned(),
        _ => {
            if !raw_name.iter().all(|b| (32..=126).contains(b)) {
                return Err(invalid("non-ASCII ISO identifier"));
            }
            String::from_utf8(raw_name.to_vec())
                .map_err(|_| invalid("ISO identifier encoding"))?
                .split(';')
                .next()
                .unwrap_or_default()
                .to_owned()
        }
    };
    Ok(IsoEntry {
        name,
        extent,
        size,
        is_directory: bytes[25] & 2 != 0,
    })
}

fn both16(bytes: &[u8]) -> Result<u16> {
    let le = u16::from_le_bytes([bytes[0], bytes[1]]);
    let be = u16::from_be_bytes([bytes[2], bytes[3]]);
    if le != be {
        return Err(invalid("ISO 16-bit endian copies disagree"));
    }
    Ok(le)
}
fn both32(bytes: &[u8]) -> Result<u32> {
    let le = u32::from_le_bytes(bytes[..4].try_into().map_err(|_| invalid("ISO integer"))?);
    let be = u32::from_be_bytes(bytes[4..].try_into().map_err(|_| invalid("ISO integer"))?);
    if le != be {
        return Err(invalid("ISO 32-bit endian copies disagree"));
    }
    Ok(le)
}
