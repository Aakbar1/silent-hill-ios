// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 shdecompilations. Schemas from pinned GPL headers.
//! Stable native working graphs consumed by C. Serialized offsets stay in the
//! immutable file; native pointers refer only to separate owned allocations.
use crate::assets::{DecodeError, Lm};
use std::ffi::c_void;
pub(crate) type SharedNativeLm = std::rc::Rc<std::cell::RefCell<NativeLm>>;

#[repr(C)]
pub struct NativeMaterial {
    name: [u8; 8],
    texture: *mut c_void,
    fields: [u8; 4],
    values: [u16; 4],
}
#[repr(C)]
struct NativePrimitive {
    uv0: [u8; 2],
    bits0: u16,
    uv1: [u8; 2],
    bits1: u16,
    uv23: [u8; 4],
    faces: [u8; 4],
    normals: [u8; 4],
}
#[repr(C)]
pub struct NativeMesh {
    counts: [u8; 4],
    primitives: *mut NativePrimitive,
    xy: *mut [i16; 2],
    z: *mut i16,
    normals: *mut [u8; 4],
    unknown: *mut u8,
}
#[repr(C)]
pub struct NativeModel {
    name: [u8; 8],
    mesh_count: u8,
    vertex_offset: u8,
    normal_offset: u8,
    pub(crate) flags: u8,
    meshes: *mut NativeMesh,
}
#[repr(C)]
pub struct NativeLmHeader {
    magic: u8,
    version: u8,
    loaded: u8,
    material_count: u8,
    materials: *mut NativeMaterial,
    model_count: u8,
    models: *mut NativeModel,
    order: *mut u8,
}
impl NativeLm {
    pub(crate) fn find_model(&mut self, name: &[u8; 8]) -> Option<*mut NativeModel> {
        self._models
            .iter_mut()
            .find(|model| &model.name == name)
            .map(|model| model as *mut _)
    }
}
struct MeshStorage {
    _primitives: Box<[NativePrimitive]>,
    _xy: Box<[[i16; 2]]>,
    _z: Box<[i16]>,
    _normals: Box<[[u8; 4]]>,
    _unknown: Box<[u8]>,
}
pub struct NativeLm {
    pub header: Box<NativeLmHeader>,
    _materials: Box<[NativeMaterial]>,
    _models: Box<[NativeModel]>,
    _meshes: Vec<Box<[NativeMesh]>>,
    _leaves: Vec<MeshStorage>,
    _order: Box<[u8]>,
}
impl NativeLm {
    pub fn new(lm: &Lm<'_>) -> Self {
        let mut materials: Box<[_]> = lm
            .materials
            .iter()
            .map(|m| NativeMaterial {
                name: m.name,
                texture: std::ptr::null_mut(),
                fields: m.fields,
                values: m.values,
            })
            .collect();
        let mut leaves = Vec::new();
        let mut meshes = Vec::new();
        let mut models: Box<[_]> = lm
            .models
            .iter()
            .map(|model| {
                let mut native_meshes: Box<[_]> = model
                    .meshes
                    .iter()
                    .map(|mesh| {
                        let mut primitives: Box<[_]> = mesh
                            .primitives
                            .bytes()
                            .as_chunks::<20>()
                            .0
                            .iter()
                            .map(|p| NativePrimitive {
                                uv0: p[0..2].try_into().unwrap(),
                                bits0: u16::from_le_bytes(p[2..4].try_into().unwrap()),
                                uv1: p[4..6].try_into().unwrap(),
                                bits1: u16::from_le_bytes(p[6..8].try_into().unwrap()),
                                uv23: p[8..12].try_into().unwrap(),
                                faces: p[12..16].try_into().unwrap(),
                                normals: p[16..20].try_into().unwrap(),
                            })
                            .collect();
                        let mut xy: Box<[_]> = mesh
                            .vertices_xy
                            .bytes()
                            .as_chunks::<4>()
                            .0
                            .iter()
                            .map(|p| {
                                [
                                    i16::from_le_bytes(p[..2].try_into().unwrap()),
                                    i16::from_le_bytes(p[2..].try_into().unwrap()),
                                ]
                            })
                            .collect();
                        let mut z: Box<[_]> = mesh
                            .vertices_z
                            .bytes()
                            .as_chunks::<2>()
                            .0
                            .iter()
                            .map(|p| i16::from_le_bytes(*p))
                            .collect();
                        let mut normals = mesh
                            .normals
                            .bytes()
                            .as_chunks::<4>()
                            .0
                            .to_vec()
                            .into_boxed_slice();
                        let mut unknown = mesh.unknown.bytes().to_vec().into_boxed_slice();
                        let native = NativeMesh {
                            counts: [
                                primitives.len() as u8,
                                xy.len() as u8,
                                normals.len() as u8,
                                unknown.len() as u8,
                            ],
                            primitives: primitives.as_mut_ptr(),
                            xy: xy.as_mut_ptr(),
                            z: z.as_mut_ptr(),
                            normals: normals.as_mut_ptr(),
                            unknown: unknown.as_mut_ptr(),
                        };
                        leaves.push(MeshStorage {
                            _primitives: primitives,
                            _xy: xy,
                            _z: z,
                            _normals: normals,
                            _unknown: unknown,
                        });
                        native
                    })
                    .collect();
                let native = NativeModel {
                    name: model.name,
                    mesh_count: native_meshes.len() as u8,
                    vertex_offset: model.vertex_offset,
                    normal_offset: model.normal_offset,
                    flags: model.flags,
                    meshes: native_meshes.as_mut_ptr(),
                };
                meshes.push(native_meshes);
                native
            })
            .collect();
        let mut order = lm.model_order.to_vec().into_boxed_slice();
        let header = Box::new(NativeLmHeader {
            magic: b'0',
            version: 6,
            loaded: 1,
            material_count: materials.len() as u8,
            materials: materials.as_mut_ptr(),
            model_count: models.len() as u8,
            models: models.as_mut_ptr(),
            order: order.as_mut_ptr(),
        });
        Self {
            header,
            _materials: materials,
            _models: models,
            _meshes: meshes,
            _leaves: leaves,
            _order: order,
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BindPose {
    pub parent: i8,
    pub rotation: i8,
    pub translation: i8,
    pub initial: [i8; 3],
}
pub struct Animation<'a> {
    pub data_offset: u16,
    pub rotation_count: u8,
    pub translation_count: u8,
    pub frame_size: u16,
    pub bone_count: u8,
    pub active_bones: u32,
    pub file_size: u32,
    pub keyframe_count: u16,
    pub scale: u8,
    pub root_y: u8,
    pub poses: Vec<BindPose>,
    pub frames: &'a [u8],
}
pub fn decode_animation(bytes: &[u8]) -> Result<Animation<'_>, DecodeError> {
    let bad = |s: &str| DecodeError(s.into());
    let header = bytes.get(..20).ok_or_else(|| bad("truncated ANM header"))?;
    let half = |i| u16::from_le_bytes(header[i..i + 2].try_into().unwrap());
    let word = |i| u32::from_le_bytes(header[i..i + 4].try_into().unwrap());
    let data_offset = half(0);
    let frame_size = half(4);
    let keyframe_count = half(16);
    let bone_count = header[6];
    let file_size = word(12);
    if bone_count == 0
        || bone_count > 32
        || header[18] > 30
        || file_size as usize > bytes.len()
        || file_size < 20
    {
        return Err(bad("invalid ANM header counts/scale/size"));
    }
    let pose_end = 20 + usize::from(bone_count) * 6;
    if usize::from(data_offset) < pose_end
        || (frame_size as usize) < usize::from(header[2]) * 9 + usize::from(header[3]) * 3
    {
        return Err(bad("ANM poses overlap frames or frame stride is too short"));
    }
    let poses = bytes
        .get(20..pose_end)
        .ok_or_else(|| bad("truncated ANM poses"))?
        .as_chunks::<6>()
        .0
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let pose = BindPose {
                parent: p[0] as i8,
                rotation: p[1] as i8,
                translation: p[2] as i8,
                initial: [p[3] as i8, p[4] as i8, p[5] as i8],
            };
            if (i > 0 && (pose.parent < 0 || pose.parent as usize >= i))
                || pose.rotation >= header[2] as i8
                || pose.translation >= header[3] as i8
            {
                return Err(bad("ANM parent/index outside hierarchy"));
            }
            Ok(pose)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let end = usize::from(data_offset) + usize::from(frame_size) * usize::from(keyframe_count);
    if end > file_size as usize {
        return Err(bad("ANM keyframes exceed declared size"));
    }
    let frames = bytes
        .get(usize::from(data_offset)..end)
        .ok_or_else(|| bad("truncated ANM keyframes"))?;
    Ok(Animation {
        data_offset,
        rotation_count: header[2],
        translation_count: header[3],
        frame_size,
        bone_count,
        active_bones: word(8),
        file_size,
        keyframe_count,
        scale: header[18],
        root_y: header[19],
        poses,
        frames,
    })
}
#[repr(C)]
pub struct NativeAnmHeader {
    data_offset: u16,
    rotation_count: u8,
    translation_count: u8,
    frame_size: u16,
    bone_count: u8,
    pad: u8,
    active_bones: u32,
    file_size: u32,
    keyframe_count: u16,
    scale: u8,
    root_y: u8,
    poses: *mut BindPose,
    pub(crate) frames: *const u8,
}
pub struct NativeAnimation {
    pub header: Box<NativeAnmHeader>,
    _poses: Box<[BindPose]>,
    _frames: Box<[u8]>,
}
impl NativeAnimation {
    pub fn new(a: &Animation<'_>) -> Self {
        let mut poses = a.poses.clone().into_boxed_slice();
        // PORT: Reserve the original player arena up front. Header copies can
        // retain this pointer across map patches; no leaf allocation moves.
        let capacity = if a.file_size <= 0x2e630 {
            0x2e630 - usize::from(a.data_offset)
        } else {
            a.frames.len()
        };
        let mut frames = vec![0; capacity].into_boxed_slice();
        frames[..a.frames.len()].copy_from_slice(a.frames);
        let header = Box::new(NativeAnmHeader {
            data_offset: a.data_offset,
            rotation_count: a.rotation_count,
            translation_count: a.translation_count,
            frame_size: a.frame_size,
            bone_count: a.bone_count,
            pad: 0,
            active_bones: a.active_bones,
            file_size: a.file_size,
            keyframe_count: a.keyframe_count,
            scale: a.scale,
            root_y: a.root_y,
            poses: poses.as_mut_ptr(),
            frames: frames.as_ptr(),
        });
        Self {
            header,
            _poses: poses,
            _frames: frames,
        }
    }
    /// Map ANMs are headerless frame blocks written at FS_BUFFER_4, within
    /// HB_BASE's virtual frame arena. This is distinct from decoding a header.
    pub(crate) fn patch_map_frames(&mut self, bytes: &[u8]) -> Result<(), String> {
        const MAP_OFFSET: usize = 0x19d84; // FS_BUFFER_4 - FS_BUFFER_0, pinned fsqueue.h.
        const PLAYER_BUDGET: usize = 0x2e630; // GameBoot_WorldInit's original arena.
        let start = MAP_OFFSET
            .checked_sub(usize::from(self.header.data_offset))
            .ok_or("map animation starts before frame data")?;
        let stride = usize::from(self.header.frame_size);
        let capacity = PLAYER_BUDGET - usize::from(self.header.data_offset);
        if stride == 0
            || start % stride != 0
            || bytes.len() < stride
            || bytes.len() > capacity - start
        {
            return Err(
                "map animation frame block exceeds player arena or has incompatible stride".into(),
            );
        }
        let complete = bytes.len() / stride * stride;
        if self._frames.len() != capacity {
            return Err("player animation arena was not reserved".into());
        }
        // PORT: Replacing a map clears its previous tail so no prior overlay's
        // frame data can survive a shorter load. Weapon frames precede start.
        self._frames[start..].fill(0);
        self._frames[start..start + complete].copy_from_slice(&bytes[..complete]);
        self.header.frames = self._frames.as_ptr();
        Ok(())
    }
}
const _: () = {
    assert!(std::mem::size_of::<NativeAnmHeader>() == 40);
    assert!(std::mem::size_of::<NativeLmHeader>() == 40);
    assert!(std::mem::size_of::<NativeMesh>() == 48);
    assert!(std::mem::size_of::<NativeModel>() == 24);
    assert!(std::mem::size_of::<NativeMaterial>() == 32);
    assert!(std::mem::offset_of!(NativeLmHeader, materials) == 8);
    assert!(std::mem::offset_of!(NativeLmHeader, models) == 24);
    assert!(std::mem::offset_of!(NativeAnmHeader, poses) == 24);
    assert!(std::mem::offset_of!(NativeAnmHeader, frames) == 32);
    assert!(std::mem::offset_of!(NativeModel, meshes) == 16);
    assert!(std::mem::offset_of!(NativeMesh, primitives) == 8);
};

#[cfg(test)]
mod tests {
    use super::*;
    static CONSUMERS: std::sync::Mutex<()> = std::sync::Mutex::new(());
    unsafe extern "C" {
        fn port_native_graph_probe(
            lm: *mut NativeLmHeader,
            anm: *mut NativeAnmHeader,
            out: *mut u32,
        ) -> i32;
    }
    fn synthetic_anm() -> Vec<u8> {
        let mut bytes = vec![0; 32];
        bytes[..2].copy_from_slice(&32u16.to_le_bytes());
        bytes[6] = 2;
        bytes[12..16].copy_from_slice(&32u32.to_le_bytes());
        bytes[18] = 4;
        bytes[20..26].copy_from_slice(&[255, 255, 255, 0, 0, 0]);
        bytes[26..32].copy_from_slice(&[0, 255, 255, 254, 3, 252]);
        bytes
    }
    #[test]
    fn native_graphs_run_original_bone_and_skeleton_consumers() {
        let _lock = CONSUMERS.lock().unwrap();
        let mut lm = vec![0; 67];
        lm[0] = b'0';
        lm[1] = 6;
        lm[8] = 1;
        lm[12..16].copy_from_slice(&20u32.to_le_bytes());
        lm[16..20].copy_from_slice(&66u32.to_le_bytes());
        lm[20..28].copy_from_slice(b"01BODY\0\0");
        lm[28] = 1;
        lm[32..36].copy_from_slice(&36u32.to_le_bytes());
        lm[37] = 1;
        lm[44..48].copy_from_slice(&60u32.to_le_bytes());
        lm[48..52].copy_from_slice(&64u32.to_le_bytes());
        lm[60..62].copy_from_slice(&(-123i16).to_le_bytes());
        let before = lm.clone();
        let mut native = NativeLm::new(&crate::assets::decode_lm(&lm).unwrap());
        let anm = synthetic_anm();
        let mut animation = NativeAnimation::new(&decode_animation(&anm).unwrap());
        let mut out = [0; 8];
        // SAFETY: Owned graphs keep every C leaf live for this synchronous call.
        assert_eq!(
            unsafe {
                port_native_graph_probe(
                    native.header.as_mut(),
                    animation.header.as_mut(),
                    out.as_mut_ptr(),
                )
            },
            1
        );
        assert_eq!(out, [2, 1, 1, 1, (-32i32) as u32, 48, (-64i32) as u32, 1]);
        assert_eq!(lm, before);
        // SAFETY: These graph fields refer to allocations owned by native.
        unsafe {
            assert_eq!((*(*native.header.models).meshes).xy.read()[0], -123);
        }
    }
    #[test]
    fn animation_rejects_bad_hierarchy_stride_and_ranges() {
        let valid = synthetic_anm();
        assert!(decode_animation(&valid).is_ok());
        for length in 0..valid.len() {
            assert!(decode_animation(&valid[..length]).is_err());
        }
        let mut bad = valid.clone();
        bad[26] = 1;
        assert!(decode_animation(&bad).is_err());
        bad = valid.clone();
        bad[27] = 0;
        assert!(decode_animation(&bad).is_err());
        bad = valid.clone();
        bad[18] = 31;
        assert!(decode_animation(&bad).is_err());
        bad = valid.clone();
        bad[2] = 1;
        assert!(decode_animation(&bad).is_err());
        bad = valid.clone();
        bad[4] = 12;
        bad[16] = 1;
        assert!(decode_animation(&bad).is_err());
    }
    #[test]
    fn owned_player_assets_decode_and_run_original_consumers() {
        let _lock = CONSUMERS.lock().unwrap();
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../private/disc/Silent Hill (USA).bin");
        if !path.exists() {
            return;
        }
        let mut disc = crate::disc::GameDisc::open(path).unwrap();
        let find = |name: &str, kind| {
            disc.entries()
                .iter()
                .position(|e| e.file_type == kind && e.name == name)
                .unwrap() as u32
        };
        let model = find("HERO.ILM", 7);
        let animation = find("HB_BASE.ANM", 4);
        let mut store = crate::asset_store::AssetStore::default();
        let lm = store.open(&mut disc, model).unwrap();
        let anm = store.open(&mut disc, animation).unwrap();
        let mut out = [0; 8];
        // SAFETY: Both graph identities remain active for the whole C probe.
        assert_eq!(
            unsafe {
                port_native_graph_probe(
                    store.native_lm(lm).unwrap(),
                    store.native_animation(anm).unwrap(),
                    out.as_mut_ptr(),
                )
            },
            1
        );
        assert_eq!(out[0], 18);
        assert!(out[1] > 0);
        assert_eq!(out[1], out[2]);
        assert_eq!(out[1], out[3]);
    }
}
