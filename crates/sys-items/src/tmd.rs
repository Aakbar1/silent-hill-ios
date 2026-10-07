// SPDX-License-Identifier: GPL-3.0-only
// PsyQ TMD records from pinned include/psyq/libgs.h and formats/tmd.h.
use crate::DecodeError;

#[repr(C)]
pub struct NativeTmdObject {
    pub vertices: *mut u32,
    pub vertex_count: u32,
    pub normals: *mut u32,
    pub normal_count: u32,
    pub primitives: *mut u32,
    pub primitive_count: u32,
    pub scale: u32,
}
#[repr(C)]
pub struct NativeTmdHeader {
    pub id: i32,
    pub flags: i32,
    pub model_count: i32,
    pub models: *mut NativeTmdObject,
}
const _: () = assert!(std::mem::size_of::<NativeTmdObject>() == 48);
const _: () = assert!(std::mem::offset_of!(NativeTmdObject, normals) == 16);
const _: () = assert!(std::mem::offset_of!(NativeTmdObject, primitives) == 32);
const _: () = assert!(std::mem::size_of::<NativeTmdHeader>() == 24);
struct Tables {
    vertices: Box<[u32]>,
    normals: Box<[u32]>,
    primitives: Box<[u32]>,
}
pub struct Tmd {
    pub header: NativeTmdHeader,
    models: Box<[NativeTmdObject]>,
    tables: Vec<Tables>,
}
impl Tmd {
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        fn word(bytes: &[u8], offset: usize) -> Result<u32, DecodeError> {
            let end = offset
                .checked_add(4)
                .ok_or(DecodeError("TMD offset overflow"))?;
            Ok(u32::from_le_bytes(
                bytes
                    .get(offset..end)
                    .ok_or(DecodeError("TMD word outside file"))?
                    .try_into()
                    .unwrap(),
            ))
        }
        fn words(bytes: &[u8], offset: usize, length: usize) -> Result<Box<[u32]>, DecodeError> {
            let end = offset
                .checked_add(length)
                .ok_or(DecodeError("TMD span overflow"))?;
            let span = bytes
                .get(offset..end)
                .ok_or(DecodeError("TMD span outside file"))?;
            if !span.len().is_multiple_of(4) {
                return Err(DecodeError("TMD span not word-sized"));
            }
            Ok(span
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| u32::from_le_bytes(*b))
                .collect())
        }
        let id = word(bytes, 0)?;
        let flags = word(bytes, 4)?;
        let count = word(bytes, 8)? as usize;
        if id != 0x41 || flags != 0 || count > 1024 {
            return Err(DecodeError("TMD requires unrelocated 0x41 wire data"));
        }
        let table_end = 12usize
            .checked_add(count * 28)
            .ok_or(DecodeError("TMD object count overflow"))?;
        if table_end > bytes.len() {
            return Err(DecodeError("TMD object table truncated"));
        }
        let mut tables = Vec::with_capacity(count);
        let mut models = Vec::with_capacity(count);
        for index in 0..count {
            let base = 12 + index * 28;
            let vc = word(bytes, base + 4)?;
            let nc = word(bytes, base + 12)?;
            let pc = word(bytes, base + 20)?;
            if vc > 65535 || nc > 65535 || pc > 65535 {
                return Err(DecodeError("TMD count outside bounded decoder"));
            }
            // Original relative pointers are based at the first object, byte 12.
            let address = |field: usize| -> Result<usize, DecodeError> {
                12usize
                    .checked_add(word(bytes, field)? as usize)
                    .ok_or(DecodeError("TMD relative pointer overflow"))
            };
            let v = address(base)?;
            let n = address(base + 8)?;
            let p = address(base + 16)?;
            if (vc != 0 && v < table_end)
                || (nc != 0 && n < table_end)
                || (pc != 0 && p < table_end)
            {
                return Err(DecodeError("TMD leaf overlaps object headers"));
            }
            let vertices = words(bytes, v, vc as usize * 8)?;
            let normals = words(bytes, n, nc as usize * 8)?;
            let mut end = p;
            for _ in 0..pc {
                let header = word(bytes, end)?;
                let input_words = ((header >> 8) & 255) as usize;
                if input_words == 0 {
                    return Err(DecodeError("empty TMD primitive input"));
                }
                end = end
                    .checked_add(4 + input_words * 4)
                    .ok_or(DecodeError("TMD primitive size overflow"))?;
                if end > bytes.len() {
                    return Err(DecodeError("TMD primitive outside file"));
                }
            }
            let primitives = words(bytes, p, end - p)?;
            let mut owned = Tables {
                vertices,
                normals,
                primitives,
            };
            models.push(NativeTmdObject {
                vertices: owned.vertices.as_mut_ptr(),
                vertex_count: vc,
                normals: owned.normals.as_mut_ptr(),
                normal_count: nc,
                primitives: owned.primitives.as_mut_ptr(),
                primitive_count: pc,
                scale: word(bytes, base + 24)?,
            });
            tables.push(owned);
        }
        let mut models = models.into_boxed_slice();
        let header = NativeTmdHeader {
            id: 0x41,
            flags: 1,
            model_count: count as i32,
            models: models.as_mut_ptr(),
        };
        Ok(Self {
            header,
            models,
            tables,
        })
    }
    pub fn model_count(&self) -> usize {
        self.models.len()
    }
    pub fn decoded_word_count(&self) -> usize {
        self.tables
            .iter()
            .map(|t| t.vertices.len() + t.normals.len() + t.primitives.len())
            .sum()
    }
}
