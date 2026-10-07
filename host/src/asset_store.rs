// SPDX-License-Identifier: GPL-3.0-only
//! Native load boundary: immutable bytes, decoded graphs and non-reused identities.
use crate::assets::{self, DecodeError, Dms, Lm, Map};
use crate::gameplay::{Animation, NativeAnimation, NativeLm, NativeLmHeader};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum AssetKind {
    Anm = 4,
    Dms = 3,
    Plm = 5,
    Ipd = 6,
    Ilm = 7,
}
impl TryFrom<u8> for AssetKind {
    type Error = String;
    fn try_from(kind: u8) -> Result<Self, String> {
        match kind {
            4 => Ok(Self::Anm),
            3 => Ok(Self::Dms),
            5 => Ok(Self::Plm),
            6 => Ok(Self::Ipd),
            7 => Ok(Self::Ilm),
            _ => Err("file is not a supported pointer-bearing asset".into()),
        }
    }
}
pub enum NativeAsset<'a> {
    Anm(Animation<'a>),
    Lm(Lm<'a>),
    Map(Box<Map<'a>>),
    Dms(Dms<'a>),
}
fn decode(kind: AssetKind, bytes: &[u8]) -> Result<NativeAsset<'_>, DecodeError> {
    match kind {
        AssetKind::Anm => crate::gameplay::decode_animation(bytes).map(NativeAsset::Anm),
        AssetKind::Plm | AssetKind::Ilm => assets::decode_lm(bytes).map(NativeAsset::Lm),
        AssetKind::Ipd => assets::decode_ipd(bytes).map(|map| NativeAsset::Map(Box::new(map))),
        AssetKind::Dms => assets::decode_dms(bytes).map(NativeAsset::Dms),
    }
}
struct Stored {
    file_id: u32,
    kind: AssetKind,
    bytes: Box<[u8]>,
    lm: Option<NativeLm>,
    animation: Option<NativeAnimation>,
}
#[derive(Default)]
pub struct AssetStore {
    entries: Vec<Option<Stored>>,
}
impl AssetStore {
    /// Re-reading an active file returns its existing identity, as with the
    /// original loaded-state guard. Closing invalidates it; IDs are never reused.
    pub fn open<S: psxdisc::SectorReader>(
        &mut self,
        disc: &mut psxdisc::GameDisc<S>,
        file: u32,
    ) -> Result<u32, String> {
        if let Some(index) = self
            .entries
            .iter()
            .position(|entry| entry.as_ref().is_some_and(|entry| entry.file_id == file))
        {
            return Ok(index as u32 + 1);
        }
        let kind = AssetKind::try_from(
            disc.entry(file)
                .map_err(|error| error.to_string())?
                .file_type,
        )?;
        let bytes = disc.read_entry(file).map_err(|error| error.to_string())?;
        self.insert(file, kind, bytes)
    }
    fn insert(&mut self, file_id: u32, kind: AssetKind, bytes: Vec<u8>) -> Result<u32, String> {
        decode(kind, &bytes).map_err(|error| format!("asset {file_id}: {error}"))?;
        let handle = u32::try_from(self.entries.len() + 1)
            .map_err(|_| "native asset handle space exhausted")?;
        self.entries.push(Some(Stored {
            file_id,
            kind,
            bytes: bytes.into_boxed_slice(),
            lm: None,
            animation: None,
        }));
        Ok(handle)
    }
    fn entry(&self, handle: u32) -> Result<&Stored, String> {
        let index = handle.checked_sub(1).ok_or("null asset handle")? as usize;
        self.entries
            .get(index)
            .and_then(Option::as_ref)
            .ok_or_else(|| "invalid or released asset handle".into())
    }
    pub fn get(&self, handle: u32) -> Result<NativeAsset<'_>, String> {
        let entry = self.entry(handle)?;
        decode(entry.kind, &entry.bytes).map_err(|error| error.to_string())
    }
    pub fn close(&mut self, handle: u32) -> Result<(), String> {
        self.entry(handle)?;
        self.entries[(handle - 1) as usize] = None;
        Ok(())
    }
    /// Leaf views contain no serialized pointers. C consumers decode numeric
    /// fields with disk32.h helpers instead of overlaying native structs.
    pub fn span(
        &self,
        handle: u32,
        section: u32,
        model: usize,
        mesh: usize,
    ) -> Result<(&[u8], usize), String> {
        let asset = self.get(handle)?;
        let missing = || "asset section/index outside decoded graph".to_owned();
        if (1..=5).contains(&section) {
            let lm = match &asset {
                NativeAsset::Lm(lm) => lm,
                NativeAsset::Map(map) => &map.lm,
                _ => return Err(missing()),
            };
            let m = lm
                .models
                .get(model)
                .and_then(|m| m.meshes.get(mesh))
                .ok_or_else(missing)?;
            return Ok(match section {
                1 => (m.primitives.bytes(), 20),
                2 => (m.vertices_xy.bytes(), 4),
                3 => (m.vertices_z.bytes(), 2),
                4 => (m.normals.bytes(), 4),
                5 => (m.unknown.bytes(), 1),
                _ => unreachable!(),
            });
        }
        match asset {
            NativeAsset::Map(map) => Ok(match section {
                10 => (map.collision.split_vertices.bytes(), 6),
                11 => (map.collision.surfaces.bytes(), 12),
                12 => (map.collision.subcells.bytes(), 10),
                13 => (map.collision.spheres.bytes(), 10),
                14 => (map.collision.ranges.bytes(), 4),
                15 => (map.collision.indices[0], 1),
                16 => (map.collision.indices[1], 1),
                _ => return Err(missing()),
            }),
            NativeAsset::Dms(dms) => match section {
                20 => Ok((
                    dms.characters
                        .get(model)
                        .ok_or_else(missing)?
                        .keyframes
                        .bytes(),
                    12,
                )),
                21 => Ok((dms.camera.keyframes.bytes(), 16)),
                _ => Err(missing()),
            },
            NativeAsset::Lm(_) | NativeAsset::Anm(_) => Err(missing()),
        }
    }
    pub fn info(&self, handle: u32) -> Result<AssetInfo, String> {
        let entry = self.entry(handle)?;
        let mut info = AssetInfo {
            kind: entry.kind as u32,
            file_id: entry.file_id,
            ..Default::default()
        };
        match self.get(handle)? {
            NativeAsset::Anm(anm) => {
                info.characters = u32::from(anm.bone_count);
                info.segments = u32::from(anm.keyframe_count);
            }
            NativeAsset::Lm(lm) => {
                info.models = lm.models.len() as u32;
                info.materials = lm.materials.len() as u32;
            }
            NativeAsset::Map(map) => {
                info.models = map.lm.models.len() as u32;
                info.materials = map.lm.materials.len() as u32;
                info.map_buffers = map.buffers.len() as u32;
                info.collision_cells = map.collision.subcells.len() as u32;
            }
            NativeAsset::Dms(dms) => {
                info.characters = dms.characters.len() as u32;
                info.segments = dms.segments.len() as u32;
            }
        }
        Ok(info)
    }
    /// Native working graphs have separate owned leaves. Their addresses stay
    /// stable until close; C may mutate them without ever modifying file bytes.
    pub fn native_lm(&mut self, handle: u32) -> Result<*mut NativeLmHeader, String> {
        self.entry(handle)?;
        let entry = self.entries[(handle - 1) as usize].as_mut().unwrap();
        if entry.lm.is_none() {
            let lm = match decode(entry.kind, &entry.bytes).map_err(|e| e.to_string())? {
                NativeAsset::Lm(lm) => lm,
                NativeAsset::Map(map) => map.lm,
                _ => return Err("asset is not a model graph".into()),
            };
            entry.lm = Some(NativeLm::new(&lm));
        }
        Ok(entry.lm.as_mut().unwrap().header.as_mut())
    }
    pub fn native_animation(
        &mut self,
        handle: u32,
    ) -> Result<*mut crate::gameplay::NativeAnmHeader, String> {
        self.entry(handle)?;
        let entry = self.entries[(handle - 1) as usize].as_mut().unwrap();
        if entry.kind != AssetKind::Anm {
            return Err("asset is not an animation".into());
        }
        if entry.animation.is_none() {
            let anm = crate::gameplay::decode_animation(&entry.bytes).map_err(|e| e.to_string())?;
            entry.animation = Some(NativeAnimation::new(&anm));
        }
        Ok(entry.animation.as_mut().unwrap().header.as_mut())
    }
}
#[derive(Default, Debug)]
#[repr(C)]
pub struct AssetInfo {
    pub kind: u32,
    pub file_id: u32,
    pub models: u32,
    pub materials: u32,
    pub map_buffers: u32,
    pub collision_cells: u32,
    pub characters: u32,
    pub segments: u32,
}
const _: () = assert!(std::mem::size_of::<AssetInfo>() == 32);
#[repr(C)]
pub struct NativeSpan {
    pub data: *const u8,
    pub count: usize,
    pub stride: usize,
}
const _: () = assert!(std::mem::size_of::<NativeSpan>() == 3 * std::mem::size_of::<usize>());

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_loading_rejects_in_place_relocation_and_stale_handles() {
        let mut store = AssetStore::default();
        let mut lm = vec![0; 20];
        lm[0] = b'0';
        lm[1] = 6;
        let handle = store.insert(17, AssetKind::Plm, lm.clone()).unwrap();
        assert_eq!(store.info(handle).unwrap().file_id, 17);
        assert!(matches!(
            store.get(handle).unwrap(),
            NativeAsset::Lm(Lm { loaded: true, .. })
        ));
        store.close(handle).unwrap();
        assert!(store.get(handle).is_err());
        let next = store.insert(17, AssetKind::Plm, lm.clone()).unwrap();
        assert_ne!(next, handle);
        lm[2] = 1;
        assert!(store.insert(17, AssetKind::Plm, lm).is_err());
        assert!(store.info(0).is_err());
        assert!(store.info(u32::MAX).is_err());
    }
    #[test]
    fn c_reads_unaligned_wire_fields_through_native_span() {
        unsafe extern "C" {
            fn sh_native_span_read16(
                span: *const NativeSpan,
                index: usize,
                field: usize,
                out: *mut u16,
            ) -> i32;
            fn sh_native_span_read32(
                span: *const NativeSpan,
                index: usize,
                field: usize,
                out: *mut u32,
            ) -> i32;
        }
        let bytes = [0, 1, 2, 3, 4, 5, 6, 7, 8];
        let span = NativeSpan {
            data: bytes[1..].as_ptr(),
            count: 2,
            stride: 4,
        };
        let mut word = 0;
        let mut half = 0;
        // SAFETY: all descriptors/output pointers refer to live correctly sized
        // objects; the C helpers explicitly reject invalid indices and fields.
        unsafe {
            assert_eq!(sh_native_span_read32(&span, 1, 0, &mut word), 1);
            assert_eq!(word, 0x08070605);
            assert_eq!(sh_native_span_read16(&span, 0, 1, &mut half), 1);
            assert_eq!(half, 0x0302);
            assert_eq!(sh_native_span_read32(&span, 2, 0, &mut word), 0);
            assert_eq!(sh_native_span_read32(&span, 0, 1, &mut word), 0);
        }
    }
}
