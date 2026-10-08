// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 shdecompilations. Schemas from pinned GPL headers.
//! Owned IPD graphs. Wire indices are resolved before publication; no disk
//! pointer is widened, relocated or used as a native address.
use crate::{
    assets::{Collision, DecodeError, Map},
    gameplay::{NativeLmHeader, NativeModel, SharedNativeLm},
};

#[repr(C)]
pub struct NativeCollision {
    position: [i32; 2],
    counts: [u8; 4],
    split_vertices: *mut [i16; 3],
    surfaces: *mut [u16; 6],
    subcells: *mut [u16; 5],
    spheres: *mut [u16; 5],
    cell_size: i16,
    grid: [u8; 2],
    ranges: *mut [i16; 2],
    index_counts: [u16; 2],
    indices0: *mut u8,
    indices1: *mut u8,
    check_count: u8,
    padding: [i8; 3],
    checks: [u8; 256],
}
struct CollisionStorage {
    vertices: Box<[[i16; 3]]>,
    surfaces: Box<[[u16; 6]]>,
    subcells: Box<[[u16; 5]]>,
    spheres: Box<[[u16; 5]]>,
    ranges: Box<[[i16; 2]]>,
    indices: [Box<[u8]>; 2],
}
fn halves<const N: usize>(bytes: &[u8]) -> Box<[[u16; N]]> {
    bytes
        .chunks_exact(N * 2)
        .map(|row| std::array::from_fn(|i| u16::from_le_bytes([row[i * 2], row[i * 2 + 1]])))
        .collect()
}
impl CollisionStorage {
    fn new(collision: &Collision<'_>) -> Self {
        Self {
            vertices: halves::<3>(collision.split_vertices.bytes())
                .iter()
                .map(|v| v.map(|x| x as i16))
                .collect(),
            surfaces: halves(collision.surfaces.bytes()),
            subcells: halves(collision.subcells.bytes()),
            spheres: halves(collision.spheres.bytes()),
            ranges: halves::<2>(collision.ranges.bytes())
                .iter()
                .map(|v| v.map(|x| x as i16))
                .collect(),
            indices: collision
                .indices
                .map(|bytes| bytes.to_vec().into_boxed_slice()),
        }
    }
    fn header(&mut self, c: &Collision<'_>) -> NativeCollision {
        NativeCollision {
            position: c.position,
            counts: [
                self.vertices.len() as u8,
                self.surfaces.len() as u8,
                self.subcells.len() as u8,
                self.spheres.len() as u8,
            ],
            split_vertices: self.vertices.as_mut_ptr(),
            surfaces: self.surfaces.as_mut_ptr(),
            subcells: self.subcells.as_mut_ptr(),
            spheres: self.spheres.as_mut_ptr(),
            cell_size: c.cell_size,
            grid: c.grid,
            ranges: self.ranges.as_mut_ptr(),
            index_counts: [self.indices[0].len() as u16, self.indices[1].len() as u16],
            indices0: self.indices[0].as_mut_ptr(),
            indices1: self.indices[1].as_mut_ptr(),
            check_count: c.check_count,
            padding: [0; 3],
            checks: c.checks,
        }
    }
}
#[repr(C)]
struct NativeModelIdentity {
    global: u8,
    padding: [i8; 3],
    name: [u8; 8],
    model: *mut NativeModel,
}
#[repr(C)]
struct NativeMatrix {
    rotation: [i16; 9],
    padding: i16,
    translation: [i32; 3],
}
#[repr(C)]
struct NativeInstance {
    model: *mut NativeModel,
    matrix: NativeMatrix,
}
#[repr(C)]
struct NativeBuffer {
    instance_count: u8,
    billboard_count: u8,
    cell_count: u8,
    padding: i8,
    bounds: [i16; 4],
    instances: *mut NativeInstance,
    unknown: *mut [u16; 4],
    positions: *mut [u16; 4],
}
struct BufferStorage {
    _instances: Box<[NativeInstance]>,
    _unknown: Box<[[u16; 4]]>,
    _positions: Box<[[u16; 4]]>,
}
#[repr(C)]
pub struct NativeMapHeader {
    magic: u8,
    loaded: u8,
    chunk: [i8; 2],
    lm: *mut NativeLmHeader,
    model_count: u8,
    buffer_count: u8,
    order_count: u8,
    padding: [i8; 9],
    identities: *mut NativeModelIdentity,
    buffers: *mut NativeBuffer,
    cell_ranges: [u8; 52],
    order: *mut u8,
    collision: NativeCollision,
}
pub struct NativeMap {
    pub header: Box<NativeMapHeader>,
    _lm: SharedNativeLm,
    // PORT: Share model allocations with their active file identity, retaining
    // original material mutations and keeping them live after handle closure.
    _globals: Vec<SharedNativeLm>,
    _identities: Box<[NativeModelIdentity]>,
    _buffers: Box<[NativeBuffer]>,
    _leaves: Vec<BufferStorage>,
    _collision: CollisionStorage,
    _order: Box<[u8]>,
}
impl NativeMap {
    pub(crate) fn new(
        map: &Map<'_>,
        lm: SharedNativeLm,
        globals: Vec<SharedNativeLm>,
    ) -> Result<Self, DecodeError> {
        if globals.is_empty() && map.model_identities.iter().any(|model| model.global) {
            return Err(DecodeError(
                "IPD global model sources have not been supplied".into(),
            ));
        }
        let mut identities: Box<[_]> = map
            .model_identities
            .iter()
            .map(|identity| {
                let model = if identity.global {
                    globals
                        .iter()
                        .find_map(|global| global.borrow_mut().find_model(&identity.name))
                } else {
                    lm.borrow_mut().find_model(&identity.name)
                };
                // PORT: The original name search returns NULL when absent, and
                // WorldMap_Draw explicitly skips that instance. Retain this path.
                let model = model.unwrap_or(std::ptr::null_mut());
                Ok(NativeModelIdentity {
                    global: u8::from(identity.global),
                    padding: [0; 3],
                    name: identity.name,
                    model,
                })
            })
            .collect::<Result<_, DecodeError>>()?;
        let mut leaves = Vec::new();
        let mut buffers: Box<[_]> = map
            .buffers
            .iter()
            .map(|buffer| {
                let mut instances: Box<[_]> = buffer
                    .instances
                    .iter()
                    .map(|instance| NativeInstance {
                        model: identities[instance.model_index].model,
                        matrix: NativeMatrix {
                            rotation: instance.rotation,
                            padding: instance.matrix_padding,
                            translation: instance.translation,
                        },
                    })
                    .collect();
                let mut unknown = halves(buffer.unknown.bytes());
                let mut positions = halves(buffer.positions.bytes());
                let result = NativeBuffer {
                    instance_count: instances.len() as u8,
                    billboard_count: buffer.billboard_count,
                    cell_count: positions.len() as u8,
                    padding: 0,
                    bounds: buffer.bounds,
                    instances: instances.as_mut_ptr(),
                    unknown: unknown.as_mut_ptr(),
                    positions: positions.as_mut_ptr(),
                };
                leaves.push(BufferStorage {
                    _instances: instances,
                    _unknown: unknown,
                    _positions: positions,
                });
                result
            })
            .collect();
        let mut collision = CollisionStorage::new(&map.collision);
        let mut order = map.order.to_vec().into_boxed_slice();
        let header = Box::new(NativeMapHeader {
            magic: 20,
            loaded: 1,
            chunk: map.chunk,
            lm: lm.borrow_mut().header.as_mut(),
            model_count: identities.len() as u8,
            buffer_count: buffers.len() as u8,
            order_count: order.len() as u8,
            padding: [0; 9],
            identities: identities.as_mut_ptr(),
            buffers: buffers.as_mut_ptr(),
            cell_ranges: map.cell_ranges,
            order: order.as_mut_ptr(),
            collision: collision.header(&map.collision),
        });
        Ok(Self {
            header,
            _lm: lm,
            _globals: globals,
            _identities: identities,
            _buffers: buffers,
            _leaves: leaves,
            _collision: collision,
            _order: order,
        })
    }
}
const _: () = {
    assert!(std::mem::size_of::<NativeCollision>() == 352);
    assert!(std::mem::offset_of!(NativeCollision, checks) == 92);
    assert!(std::mem::size_of::<NativeMapHeader>() == 464);
    assert!(std::mem::offset_of!(NativeMapHeader, collision) == 112);
    assert!(std::mem::size_of::<NativeModelIdentity>() == 24);
    assert!(std::mem::size_of::<NativeInstance>() == 40);
    assert!(std::mem::size_of::<NativeBuffer>() == 40);
};

#[cfg(test)]
mod tests {
    use super::*;
    unsafe extern "C" {
        fn port_native_map_probe(map: *mut NativeMapHeader, output: *mut u32) -> i32;
        fn port_map_layout_probe(output: *mut u32);
        fn sh_map0_s00_reset_probe() -> i32;
    }
    fn put(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    fn model() -> Vec<u8> {
        let mut bytes = vec![0; 37];
        bytes[0] = b'0';
        bytes[1] = 6;
        bytes[8] = 1;
        put(&mut bytes, 12, 20);
        put(&mut bytes, 16, 36);
        bytes[20..28].copy_from_slice(b"MODEL123");
        bytes
    }
    fn ipd(global: bool) -> Vec<u8> {
        let mut bytes = vec![0; 392];
        bytes[0] = 20;
        put(&mut bytes, 4, 392);
        bytes.extend(model());
        bytes.resize(432, 0);
        bytes[8] = 1;
        put(&mut bytes, 20, 432);
        bytes.resize(448, 0);
        bytes[432] = u8::from(global);
        bytes[436..444].copy_from_slice(b"MODEL123");
        put(&mut bytes, 444, 0xdeadbeef); // Identity token must never be interpreted as a pointer.
        bytes[9] = 1;
        put(&mut bytes, 24, 448);
        bytes.resize(472, 0);
        bytes[448] = 1;
        put(&mut bytes, 460, 472);
        bytes.resize(508, 0);
        put(&mut bytes, 496, 1234);
        bytes
    }
    fn shared(lm: &crate::assets::Lm<'_>) -> SharedNativeLm {
        std::rc::Rc::new(std::cell::RefCell::new(crate::gameplay::NativeLm::new(lm)))
    }
    #[test]
    fn native_instances_resolve_names_and_keep_global_allocations_live() {
        let bytes = ipd(true);
        let before = bytes.clone();
        let decoded = crate::assets::decode_ipd(&bytes).unwrap();
        let global = shared(&decoded.lm);
        let model_ptr = global.borrow_mut().find_model(b"MODEL123").unwrap();
        let mut native =
            NativeMap::new(&decoded, shared(&decoded.lm), vec![global.clone()]).unwrap();
        assert_eq!(native._identities[0].model, model_ptr);
        drop(global);
        // SAFETY: Every native leaf is owned by the graph throughout the call.
        unsafe {
            (*model_ptr).flags = 77;
            assert_eq!((*native._leaves[0]._instances[0].model).flags, 77);
            let mut out = [0; 9];
            assert_eq!(
                port_native_map_probe(native.header.as_mut(), out.as_mut_ptr()),
                1
            );
            assert_eq!(out, [1, 1, 1, 1234, 0, 0, 0, 73, 1]);
        }
        assert_eq!(bytes, before);
        assert!(NativeMap::new(&decoded, shared(&decoded.lm), vec![]).is_err());
        let mut local = ipd(false);
        local[436] = b'!';
        let decoded = crate::assets::decode_ipd(&local).unwrap();
        let missing = NativeMap::new(&decoded, shared(&decoded.lm), vec![]).unwrap();
        assert!(missing._identities[0].model.is_null());
    }
    #[test]
    fn descriptor_restores_all_linked_mutable_data() {
        // SAFETY: The probe dirties/resets only this map's private namespace.
        unsafe {
            assert_eq!(sh_map0_s00_reset_probe(), 1);
            unsafe extern "C" {
                fn port_maps_reset_probe() -> i32;
            }
            assert_eq!(port_maps_reset_probe(), 0, "all 43 map data images restore");
        }
        let mut layout = [0; 5];
        // SAFETY: C writes exactly five u32 layout results.
        unsafe {
            port_map_layout_probe(layout.as_mut_ptr());
        }
        eprintln!("native map layout: {layout:?}");
    }
    #[test]
    fn owned_opening_chunks_run_native_c_graph_consumer() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../private/disc/Silent Hill (USA).bin");
        if !path.exists() {
            return;
        }
        let mut disc = crate::disc::GameDisc::open(path).unwrap();
        let global = disc
            .entries()
            .iter()
            .position(|entry| entry.name == "THR_GLB.PLM")
            .unwrap() as u32;
        let chunks: Vec<_> = disc
            .entries()
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.file_type == 6 && entry.name.starts_with("THR"))
            .map(|(i, _)| i as u32)
            .collect();
        let mut store = crate::asset_store::AssetStore::default();
        let global = store.open(&mut disc, global).unwrap();
        let mut instances = 0;
        let mut resolved = 0;
        for file in &chunks {
            let handle = store.open(&mut disc, *file).unwrap();
            let graph = store.native_map(handle, &[global]).unwrap();
            let mut output = [0; 9];
            // SAFETY: AssetStore owns the graph until this handle is closed.
            unsafe {
                assert_eq!(port_native_map_probe(graph, output.as_mut_ptr()), 1);
            }
            instances += output[2];
            resolved += output[8];
            store.close(handle).unwrap();
            assert!(store.native_map(handle, &[global]).is_err());
        }
        assert!(instances > 0 && resolved > 0 && !chunks.is_empty());
        eprintln!(
            "owned opening IPDs: {} chunks, {instances} instances, {resolved} resolved (original NULL skip retained)",
            chunks.len()
        );
    }
    #[test]
    fn owned_map_frame_blocks_patch_a_stable_player_arena() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../private/disc/Silent Hill (USA).bin");
        if !path.exists() {
            return;
        }
        let mut disc = crate::disc::GameDisc::open(path).unwrap();
        let base = disc
            .entries()
            .iter()
            .position(|entry| entry.name == "HB_BASE.ANM")
            .unwrap() as u32;
        let base_bytes = disc.read_entry(base).unwrap();
        let base_animation = crate::gameplay::decode_animation(&base_bytes).unwrap();
        let mut native = crate::gameplay::NativeAnimation::new(&base_animation);
        let pointer = native.header.frames;
        let start = 0x19d84 - usize::from(base_animation.data_offset);
        let capacity = 0x2e630 - usize::from(base_animation.data_offset);
        let stride = usize::from(base_animation.frame_size);
        let mut previous_end = 0;
        for name in ["HB_M0S01.ANM", "HB_M0S00.ANM"] {
            let file = disc
                .entries()
                .iter()
                .position(|entry| entry.name == name)
                .unwrap() as u32;
            let bytes = disc.read_entry(file).unwrap();
            assert!(
                crate::gameplay::decode_animation(&bytes).is_err(),
                "a frame block must not decode as an ANM header"
            );
            native.patch_map_frames(&bytes).unwrap();
            assert_eq!(native.header.frames, pointer);
            // SAFETY: The native animation owns this complete fixed arena.
            let arena = unsafe { std::slice::from_raw_parts(pointer, capacity) };
            let complete = bytes.len() / stride * stride;
            assert_eq!(&arena[..base_animation.frames.len()], base_animation.frames);
            assert_eq!(&arena[start..start + complete], &bytes[..complete]);
            if previous_end > start + complete {
                assert!(
                    arena[start + complete..previous_end]
                        .iter()
                        .all(|&byte| byte == 0)
                );
            }
            previous_end = start + complete;
        }
        assert!(native.patch_map_frames(&vec![0; capacity + 1]).is_err());
        assert!(native.patch_map_frames(&[]).is_err());
    }
}

include!(concat!(env!("OUT_DIR"), "/native-source/maps_catalog.rs"));

/// PORT: Explicit test-only spawn selection; the production loader is unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DebugWarp {
    pub map: u32,
    pub spawn: u32,
}
impl DebugWarp {
    pub fn parse(text: &str) -> Result<Self, String> {
        let (name, spawn) = text.split_once(':').unwrap_or((text, "0"));
        let map = MAP_NAMES
            .iter()
            .position(|candidate| *candidate == name)
            .ok_or_else(|| format!("unknown map {name}"))?;
        let spawn = spawn
            .parse::<u32>()
            .map_err(|_| "invalid warp spawn".to_owned())?;
        if spawn > 255 {
            return Err("warp spawn exceeds descriptor index range".into());
        }
        Ok(Self {
            map: map as u32,
            spawn,
        })
    }
}

#[cfg(test)]
mod warp_tests {
    use super::*;
    #[test]
    fn warp_catalog_and_arguments() {
        assert_eq!(MAP_NAMES.len(), 43);
        for (index, name) in MAP_NAMES.iter().enumerate() {
            assert_eq!(
                DebugWarp::parse(name).unwrap(),
                DebugWarp {
                    map: index as u32,
                    spawn: 0
                }
            );
        }
        assert_eq!(
            DebugWarp::parse("MAP7_S03:255").unwrap(),
            DebugWarp {
                map: 42,
                spawn: 255
            }
        );
        for bad in [
            "map0_s00",
            "MAP9_S00",
            "MAP0_S00:",
            "MAP0_S00:-1",
            "MAP0_S00:256",
            "MAP0_S00:1:2",
        ] {
            assert!(DebugWarp::parse(bad).is_err(), "{bad}");
        }
    }
    #[test]
    #[ignore = "requires owned disc; run through tools/prepare_maps.py --warp"]
    fn debug_map_warp() {
        let warp = DebugWarp::parse(&std::env::var("SH_MAP_WARP").expect("SH_MAP_WARP")).unwrap();
        let disc =
            crate::disc::GameDisc::open(std::env::var_os("SH_MAP_DISC").expect("SH_MAP_DISC"))
                .unwrap();
        crate::spu_cpal::configure(crate::spu_cpal::AudioMode::parse("off").unwrap()).unwrap();
        crate::gpu_wgpu::configure(crate::gpu_wgpu::Options::default()).unwrap();
        let result = crate::native::run_headless(
            disc,
            16,
            None,
            crate::pad::ReplayPad::parse("0 0000").unwrap(),
            crate::native::ReplayCheck {
                warp: Some(warp),
                state: Some(11),
                step: Some(0),
                min_lit_pixels: 1,
                ..Default::default()
            },
        );
        assert!(result.is_ok(), "warp failed: {result:?}");
    }
}
