// SPDX-License-Identifier: GPL-3.0-only
//! Pointer-free wire decoding and a standalone original-C preparation harness.
use std::fmt;
#[path = "../../../port/sys/items/host_sidecars.rs"]
mod host_sidecars;
pub use host_sidecars::{
    read as read_sidecar_record, storage::SidecarStore, write as write_sidecar_record,
};
mod tmd;
pub use tmd::{NativeTmdHeader, NativeTmdObject, Tmd};
mod disc_tables {
    include!(concat!(env!("OUT_DIR"), "/disc_tables.rs"));
}
pub use disc_tables::DISC_TABLES;

pub const SAVE_SIZE: usize = 636;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError(pub &'static str);
impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for DecodeError {}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InventoryItem {
    pub id: u8,
    pub count: u8,
    pub command: u8,
    pub order: u8,
}
const _: () = assert!(std::mem::size_of::<InventoryItem>() == 4);

#[derive(Clone, PartialEq, Eq)]
pub struct SavePayload {
    bytes: [u8; SAVE_SIZE],
}
impl SavePayload {
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {
            bytes: bytes
                .try_into()
                .map_err(|_| DecodeError("save payload must be exactly 636 bytes"))?,
        })
    }
    pub fn encode(&self) -> [u8; SAVE_SIZE] {
        self.bytes
    }
    pub fn items(&self) -> [InventoryItem; 40] {
        std::array::from_fn(|i| {
            let b = &self.bytes[i * 4..i * 4 + 4];
            InventoryItem {
                id: b[0],
                count: b[1],
                command: b[2],
                order: b[3],
            }
        })
    }
    pub fn toggles(&self) -> u32 {
        u32::from_le_bytes(self.bytes[0xac..0xb0].try_into().unwrap())
    }
    pub fn event_flags(&self) -> [u32; 52] {
        std::array::from_fn(|i| {
            u32::from_le_bytes(self.bytes[0x168 + i * 4..0x16c + i * 4].try_into().unwrap())
        })
    }
    pub fn difficulty(&self) -> i8 {
        (self.bytes[0x263] as i8) >> 4
    }
    pub fn health(&self) -> i32 {
        i32::from_le_bytes(self.bytes[0x240..0x244].try_into().unwrap())
    }
    pub fn endings(&self) -> (u8, u8, i8) {
        (
            self.bytes[0x24a],
            self.bytes[0x24b],
            self.bytes[0x27a] as i8,
        )
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PianoTables {
    pub thresholds: [i8; 11],
    pub sequence: [i8; 5],
}
const _: () = assert!(std::mem::size_of::<PianoTables>() == 16);
impl PianoTables {
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        if bytes.len() != 17 {
            return Err(DecodeError("piano table block must be 17 bytes"));
        }
        let result = Self {
            thresholds: std::array::from_fn(|i| bytes[i] as i8),
            sequence: std::array::from_fn(|i| bytes[12 + i] as i8),
        };
        if result.thresholds.windows(2).any(|pair| pair[0] >= pair[1])
            || result
                .sequence
                .iter()
                .any(|k| ![1, 2, 7, 9, 10].contains(k))
        {
            return Err(DecodeError("invalid piano thresholds or silent key index"));
        }
        Ok(result)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelCell {
    pub x: u8,
    pub y: u8,
}
pub fn decode_panel_cells(bytes: &[u8], count: usize) -> Result<Vec<PanelCell>, DecodeError> {
    if count > 27 || count.checked_mul(2) != Some(bytes.len()) {
        return Err(DecodeError("invalid panel cell count/length"));
    }
    Ok(bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| PanelCell { x: b[0], y: b[1] })
        .collect())
}
pub fn decode_points(bytes: &[u8], count: usize) -> Result<Vec<(i16, i16)>, DecodeError> {
    if count > 4096 || count.checked_mul(4) != Some(bytes.len()) {
        return Err(DecodeError("invalid point count/length"));
    }
    Ok(bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| {
            (
                i16::from_le_bytes([b[0], b[1]]),
                i16::from_le_bytes([b[2], b[3]]),
            )
        })
        .collect())
}

/// PS1 addresses are resolved against an explicit immutable data image; they
/// never become host pointers. Owned decoded tables survive image release.
pub fn resolve_data(
    image: &[u8],
    base: u32,
    address: u32,
    length: usize,
) -> Result<&[u8], DecodeError> {
    let offset = address
        .checked_sub(base)
        .ok_or(DecodeError("address below image base"))? as usize;
    let end = offset
        .checked_add(length)
        .ok_or(DecodeError("data range overflow"))?;
    image
        .get(offset..end)
        .ok_or(DecodeError("data range outside image"))
}
/// Decode only a bounded numeric data leaf from the encrypted BODYPROG image.
/// Derived from GPL `src/main/fileinfo.c::Fs_DecryptOverlay` at the pinned
/// revision. No program bytes preceding the requested leaf are decrypted.
pub fn encrypted_data_leaf(
    image: &[u8],
    offset: usize,
    length: usize,
) -> Result<Vec<u8>, DecodeError> {
    let end = offset
        .checked_add(length)
        .ok_or(DecodeError("encrypted leaf range overflow"))?;
    if image.len() > 16 * 1024 * 1024 || end > image.len() || !image.len().is_multiple_of(4) {
        return Err(DecodeError("encrypted leaf outside bounded word image"));
    }
    if length == 0 {
        return Ok(vec![]);
    }
    let first_word = offset / 4;
    let end_word = end.div_ceil(4);
    let mut seed = 0u32;
    let mut words = Vec::with_capacity((end_word - first_word) * 4);
    for i in 0..end_word {
        // PORT: explicit PS1 word wrapping, replacing signed-overflow UB.
        seed = seed.wrapping_add(0x01309125).wrapping_mul(0x03a452f7);
        if i >= first_word {
            let raw = u32::from_le_bytes(image[i * 4..i * 4 + 4].try_into().unwrap());
            words.extend_from_slice(&(raw ^ seed).to_le_bytes());
        }
    }
    Ok(words[offset % 4..offset % 4 + length].to_vec())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditState {
    pub x: i16,
    pub y: i16,
    pub margin: i16,
    pub line_height: i8,
    pub semi_trans: i8,
    pub color: u32,
    pub tpage: u16,
    pub clut: i16,
    pub uv: u32,
    pub widths: Vec<i16>,
    pub colors: Vec<u32>,
}
#[repr(C)]
pub struct NativeCreditState {
    pub x: i16,
    pub y: i16,
    pub margin: i16,
    pub line_height: i8,
    pub semi_trans: i8,
    pub color: u32,
    pub widths: *mut i16,
    pub colors: *mut u32,
    pub tpage: u16,
    pub clut: i16,
    pub uv: u32,
}
const _: () = assert!(std::mem::size_of::<NativeCreditState>() == 40);
const _: () = assert!(std::mem::offset_of!(NativeCreditState, widths) == 16);
const _: () = assert!(std::mem::offset_of!(NativeCreditState, colors) == 24);
impl CreditState {
    /// The caller must retain this owner and must not resize its tables while
    /// C holds these pointers. C may mutate the decoded tables, never wire bytes.
    pub fn native(&mut self) -> NativeCreditState {
        NativeCreditState {
            x: self.x,
            y: self.y,
            margin: self.margin,
            line_height: self.line_height,
            semi_trans: self.semi_trans,
            color: self.color,
            widths: self.widths.as_mut_ptr(),
            colors: self.colors.as_mut_ptr(),
            tpage: self.tpage,
            clut: self.clut,
            uv: self.uv,
        }
    }
}
#[repr(C)]
pub struct NativeCredit3d {
    pub text: NativeCreditState,
    pub field_1c: i32,
    pub field_20: i16,
    pub field_22: i16,
    pub tail: [i32; 13],
}
const _: () = assert!(std::mem::size_of::<NativeCredit3d>() == 104);
pub struct Credit3dState {
    pub text: CreditState,
    pub field_1c: i32,
    pub field_20: i16,
    pub field_22: i16,
    pub tail: [i32; 13],
}
impl Credit3dState {
    pub fn decode(wire: &[u8], image: &[u8], base: u32) -> Result<Self, DecodeError> {
        if wire.len() != 88 {
            return Err(DecodeError("3d credit state must be 88 bytes"));
        }
        Ok(Self {
            text: CreditState::decode(&wire[..28], image, base)?,
            field_1c: i32::from_le_bytes(wire[28..32].try_into().unwrap()),
            field_20: i16::from_le_bytes(wire[32..34].try_into().unwrap()),
            field_22: i16::from_le_bytes(wire[34..36].try_into().unwrap()),
            tail: std::array::from_fn(|i| {
                i32::from_le_bytes(wire[36 + i * 4..40 + i * 4].try_into().unwrap())
            }),
        })
    }
    pub fn native(&mut self) -> NativeCredit3d {
        NativeCredit3d {
            text: self.text.native(),
            field_1c: self.field_1c,
            field_20: self.field_20,
            field_22: self.field_22,
            tail: self.tail,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapMarkings {
    pub first_flag: i16,
    pub end_flag: i16,
    pub atlas: Vec<[u8; 4]>,
    pub entries: Vec<[u8; 4]>,
}
impl MapMarkings {
    pub fn decode(wire: &[u8], image: &[u8], base: u32) -> Result<Self, DecodeError> {
        Self::decode_image(wire, image, base, false)
    }
    pub fn decode_bodyprog(
        wire: &[u8],
        encrypted_image: &[u8],
        base: u32,
    ) -> Result<Self, DecodeError> {
        Self::decode_image(wire, encrypted_image, base, true)
    }
    fn decode_image(
        wire: &[u8],
        image: &[u8],
        base: u32,
        encrypted: bool,
    ) -> Result<Self, DecodeError> {
        if wire.len() != 12 {
            return Err(DecodeError("map marking record must be 12 bytes"));
        }
        let u32_at = |i| u32::from_le_bytes(wire[i..i + 4].try_into().unwrap());
        let first_flag = i16::from_le_bytes(wire[8..10].try_into().unwrap());
        let end_flag = i16::from_le_bytes(wire[10..12].try_into().unwrap());
        if u32_at(0) == 0 || u32_at(4) == 0 {
            return Ok(Self {
                first_flag,
                end_flag,
                atlas: vec![],
                entries: vec![],
            });
        }
        if first_flag < 0 || end_flag < first_flag || end_flag > 1664 {
            return Err(DecodeError("map marking flag range invalid"));
        }
        let count = (end_flag - first_flag) as usize / 2;
        let read = |address: u32, length: usize| -> Result<Vec<u8>, DecodeError> {
            let offset = address
                .checked_sub(base)
                .ok_or(DecodeError("data address below encrypted image"))?
                as usize;
            if encrypted {
                encrypted_data_leaf(image, offset, length)
            } else {
                Ok(resolve_data(image, base, address, length)?.to_vec())
            }
        };
        let entries: Vec<[u8; 4]> = read(u32_at(4), count * 4)?.as_chunks::<4>().0.to_vec();
        let atlas_count = entries
            .iter()
            .flat_map(|e| [e[2], e[3]])
            .max()
            .map_or(0, |n| usize::from(n) + 1);
        let atlas = read(u32_at(0), atlas_count * 4)?
            .as_chunks::<4>()
            .0
            .to_vec();
        Ok(Self {
            first_flag,
            end_flag,
            atlas,
            entries,
        })
    }
}
#[repr(C)]
pub struct NativeMapMarkings {
    pub atlas: *mut u32,
    pub entries: *mut u32,
    pub first_flag: i16,
    pub end_flag: i16,
}
const _: () = assert!(std::mem::size_of::<NativeMapMarkings>() == 24);
pub struct OwnedNativeMapMarkings {
    pub record: NativeMapMarkings,
    atlas: Box<[u32]>,
    entries: Box<[u32]>,
}
impl MapMarkings {
    pub fn native(&self) -> OwnedNativeMapMarkings {
        let mut atlas: Box<[u32]> = self.atlas.iter().map(|b| u32::from_le_bytes(*b)).collect();
        let mut entries: Box<[u32]> = self
            .entries
            .iter()
            .map(|b| u32::from_le_bytes(*b))
            .collect();
        OwnedNativeMapMarkings {
            record: NativeMapMarkings {
                atlas: if atlas.is_empty() {
                    std::ptr::null_mut()
                } else {
                    atlas.as_mut_ptr()
                },
                entries: if entries.is_empty() {
                    std::ptr::null_mut()
                } else {
                    entries.as_mut_ptr()
                },
                first_flag: self.first_flag,
                end_flag: self.end_flag,
            },
            atlas,
            entries,
        }
    }
}
impl OwnedNativeMapMarkings {
    pub fn table_lengths(&self) -> (usize, usize) {
        (self.atlas.len(), self.entries.len())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveRow {
    pub total: i16,
    pub count: i16,
    pub kind: i8,
    pub device: i8,
    pub file: i8,
    pub element: i8,
    pub location: i8,
    pub metadata: Option<SaveMetadata>,
}
#[repr(C)]
pub struct NativeSaveRow {
    pub total: i16,
    pub count: i16,
    pub kind: i8,
    pub device: i8,
    pub file: i8,
    pub element: i8,
    pub location: i8,
    pub metadata: *mut SaveMetadata,
}
const _: () = assert!(std::mem::size_of::<NativeSaveRow>() == 24);
const _: () = assert!(std::mem::offset_of!(NativeSaveRow, metadata) == 16);
pub struct OwnedNativeSaveRow {
    pub record: NativeSaveRow,
    metadata: Option<Box<SaveMetadata>>,
}
impl SaveRow {
    pub fn native(&self) -> OwnedNativeSaveRow {
        let mut metadata = self.metadata.clone().map(Box::new);
        OwnedNativeSaveRow {
            record: NativeSaveRow {
                total: self.total,
                count: self.count,
                kind: self.kind,
                device: self.device,
                file: self.file,
                element: self.element,
                location: self.location,
                metadata: metadata
                    .as_mut()
                    .map_or(std::ptr::null_mut(), |m| m.as_mut() as *mut SaveMetadata),
            },
            metadata,
        }
    }
}
impl OwnedNativeSaveRow {
    pub fn metadata(&self) -> Option<&SaveMetadata> {
        self.metadata.as_deref()
    }
}
impl SaveRow {
    pub fn decode(wire: &[u8], image: &[u8], base: u32) -> Result<Self, DecodeError> {
        if wire.len() != 16 {
            return Err(DecodeError("save row must be 16 wire bytes"));
        }
        let address = u32::from_le_bytes(wire[12..16].try_into().unwrap());
        let metadata = if address == 0 {
            None
        } else {
            Some(SaveMetadata::decode(resolve_data(
                image, base, address, 12,
            )?)?)
        };
        Ok(Self {
            total: i16::from_le_bytes([wire[0], wire[1]]),
            count: i16::from_le_bytes([wire[2], wire[3]]),
            kind: wire[4] as i8,
            device: wire[5] as i8,
            file: wire[6] as i8,
            element: wire[7] as i8,
            location: wire[8] as i8,
            metadata,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndingParams {
    pub audio_id: i16,
    pub scroll_delay: i16,
    pub duration: i16,
}
impl EndingParams {
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        if bytes.len() != 6 {
            return Err(DecodeError("ending parameter must be six bytes"));
        }
        Ok(Self {
            audio_id: i16::from_le_bytes([bytes[0], bytes[1]]),
            scroll_delay: i16::from_le_bytes([bytes[2], bytes[3]]),
            duration: i16::from_le_bytes([bytes[4], bytes[5]]),
        })
    }
}
impl CreditState {
    pub fn decode(wire: &[u8], image: &[u8], base: u32) -> Result<Self, DecodeError> {
        if wire.len() != 28 {
            return Err(DecodeError("credit state must be 28 bytes"));
        }
        let u16_at = |i| u16::from_le_bytes([wire[i], wire[i + 1]]);
        let u32_at = |i| u32::from_le_bytes(wire[i..i + 4].try_into().unwrap());
        let widths = resolve_data(image, base, u32_at(12), 512)?
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| i16::from_le_bytes([b[0], b[1]]))
            .collect();
        let colors = resolve_data(image, base, u32_at(16), 28)?
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| u32::from_le_bytes(*b))
            .collect();
        Ok(Self {
            x: u16_at(0) as i16,
            y: u16_at(2) as i16,
            margin: u16_at(4) as i16,
            line_height: wire[6] as i8,
            semi_trans: wire[7] as i8,
            color: u32_at(8),
            tpage: u16_at(20),
            clut: u16_at(22) as i16,
            uv: u32_at(24),
            widths,
            colors,
        })
    }
}

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveMetadata {
    pub total_count: i32,
    pub timer: u32,
    pub count: u16,
    pub location: u8,
    pub packed: u8,
}
const _: () = assert!(std::mem::size_of::<SaveMetadata>() == 12);
impl SaveMetadata {
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        if bytes.len() != 12 {
            return Err(DecodeError("save metadata must be 12 bytes"));
        }
        Ok(Self {
            total_count: i32::from_le_bytes(bytes[0..4].try_into().unwrap()),
            timer: u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            count: u16::from_le_bytes(bytes[8..10].try_into().unwrap()),
            location: bytes[10],
            packed: bytes[11],
        })
    }
}

#[cfg(test)]
mod assets {
    pub fn decode_save(bytes: &[u8]) -> Result<super::SavePayload, super::DecodeError> {
        super::SavePayload::decode(bytes)
    }
}
#[cfg(test)]
#[path = "../../../host/src/saves.rs"]
#[rustfmt::skip]
mod host_saves;
#[cfg(test)]
mod tests;
