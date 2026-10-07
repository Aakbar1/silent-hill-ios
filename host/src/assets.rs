// SPDX-License-Identifier: GPL-3.0-only
// Wire schemas derived from silent-hill-decomp d9e28f8, GPL-3.0-only.
// Copyright (C) 2026 shdecompilations. Upstream notices retained in game/decomp/LICENSE.
//! Decode disk32 into separate native objects. No casts of file bytes to C/Rust
//! structs, in-place relocation, recovered host addresses, or implicit bitfields.
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError(pub String);
impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl std::error::Error for DecodeError {}
type Result<T> = std::result::Result<T, DecodeError>;
fn invalid(text: &str) -> DecodeError {
    DecodeError(text.into())
}

#[derive(Clone, Copy)]
struct Bytes<'a>(&'a [u8]);
impl<'a> Bytes<'a> {
    fn range(self, offset: usize, count: usize, stride: usize) -> Result<&'a [u8]> {
        let len = count
            .checked_mul(stride)
            .ok_or_else(|| invalid("array size overflow"))?;
        let end = offset
            .checked_add(len)
            .ok_or_else(|| invalid("offset overflow"))?;
        self.0.get(offset..end).ok_or_else(|| {
            DecodeError(format!(
                "wire range {offset}..{end} outside {}-byte file (count={count}, stride={stride})",
                self.0.len()
            ))
        })
    }
    fn array(self, offset: u32, count: usize, stride: usize) -> Result<&'a [u8]> {
        self.range(offset as usize, count, stride)
    }
    fn byte(self, offset: usize) -> Result<u8> {
        Ok(self.range(offset, 1, 1)?[0])
    }
    fn u16(self, offset: usize) -> Result<u16> {
        Ok(u16::from_le_bytes(
            self.range(offset, 1, 2)?.try_into().unwrap(),
        ))
    }
    fn i16(self, offset: usize) -> Result<i16> {
        Ok(self.u16(offset)? as i16)
    }
    fn u32(self, offset: usize) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.range(offset, 1, 4)?.try_into().unwrap(),
        ))
    }
    fn i32(self, offset: usize) -> Result<i32> {
        Ok(self.u32(offset)? as i32)
    }
    fn name<const N: usize>(self, offset: usize) -> Result<[u8; N]> {
        Ok(self.range(offset, N, 1)?.try_into().unwrap())
    }
}

/// Retained byte ranges have explicit strides and bounds. Multibyte fields are
/// read little-endian on demand; native alignment is never required of the source.
#[derive(Clone, Copy, Debug)]
pub struct WireArray<'a, const STRIDE: usize> {
    bytes: &'a [u8],
}
impl<'a, const STRIDE: usize> WireArray<'a, STRIDE> {
    fn from_blob(blob: Bytes<'a>, offset: u32, count: usize) -> Result<Self> {
        Ok(Self {
            bytes: blob.array(offset, count, STRIDE)?,
        })
    }
    pub fn len(&self) -> usize {
        self.bytes.len() / STRIDE
    }
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
    pub fn record(&self, index: usize) -> Option<&'a [u8]> {
        let offset = index.checked_mul(STRIDE)?;
        self.bytes.get(offset..offset.checked_add(STRIDE)?)
    }
    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
}

#[derive(Debug)]
pub struct Mesh<'a> {
    pub primitives: WireArray<'a, 20>,
    pub vertices_xy: WireArray<'a, 4>,
    pub vertices_z: WireArray<'a, 2>,
    pub normals: WireArray<'a, 4>,
    pub unknown: WireArray<'a, 1>,
}
#[derive(Debug)]
pub struct Model<'a> {
    pub name: [u8; 8],
    pub vertex_offset: u8,
    pub normal_offset: u8,
    pub flags: u8,
    pub meshes: Vec<Mesh<'a>>,
}
#[derive(Debug)]
pub struct Material {
    pub name: [u8; 8],
    /// Initial texture slot is an encoded identity, not a file offset. Texture
    /// cache linkage must replace it with a native reference outside the file.
    pub texture_token: u32,
    pub fields: [u8; 4],
    pub values: [u16; 4],
}
#[derive(Debug)]
pub struct Lm<'a> {
    pub materials: Vec<Material>,
    pub models: Vec<Model<'a>>,
    pub model_order: &'a [u8],
    /// Native decoded state. Original wire loaded byte is never overwritten.
    pub loaded: bool,
}

fn decode_model<'a>(blob: Bytes<'a>, header: Bytes<'_>) -> Result<Model<'a>> {
    let mesh_headers = blob.array(header.u32(12)?, usize::from(header.byte(8)?), 24)?;
    let mut meshes = Vec::new();
    for mesh in mesh_headers.as_chunks::<24>().0.iter() {
        let mesh = Bytes(mesh);
        let primitive_count = usize::from(mesh.byte(0)?);
        let vertex_count = usize::from(mesh.byte(1)?);
        let normal_count = usize::from(mesh.byte(2)?);
        let result = Mesh {
            primitives: WireArray::from_blob(blob, mesh.u32(4)?, primitive_count)?,
            vertices_xy: WireArray::from_blob(blob, mesh.u32(8)?, vertex_count)?,
            vertices_z: WireArray::from_blob(blob, mesh.u32(12)?, vertex_count)?,
            normals: WireArray::from_blob(blob, mesh.u32(16)?, normal_count)?,
            unknown: WireArray::from_blob(blob, mesh.u32(20)?, usize::from(mesh.byte(3)?))?,
        };
        // Renderer indexes transformed scratch vertices with the model offset.
        // The fourth face index 0xff denotes a triangle (Gfx_MeshDraw); it is
        // not a fourth vertex. Normal indices also retain their original encoding.
        let vertex_end = usize::from(header.byte(9)?) + vertex_count;
        for primitive in result.primitives.bytes.as_chunks::<20>().0.iter() {
            if primitive[12..16].iter().enumerate().any(|(slot, &index)| {
                !(slot == 3 && index == 255) && usize::from(index) >= vertex_end
            }) {
                return Err(invalid("mesh face vertex index outside array"));
            }
        }
        meshes.push(result);
    }
    Ok(Model {
        name: header.name(0)?,
        vertex_offset: header.byte(9)?,
        normal_offset: header.byte(10)?,
        flags: header.byte(11)?,
        meshes,
    })
}
pub fn decode_lm(data: &[u8]) -> Result<Lm<'_>> {
    let blob = Bytes(data);
    blob.range(0, 1, 20)?;
    if blob.byte(0)? != b'0' || blob.byte(1)? != 6 {
        return Err(invalid("LM magic/version differs from USA schema"));
    }
    if blob.byte(2)? != 0 {
        return Err(invalid(
            "LM already relocated: expected immutable disk32 image",
        ));
    }
    let mut materials = Vec::new();
    for material in blob
        .array(blob.u32(4)?, usize::from(blob.byte(3)?), 24)?
        .as_chunks::<24>()
        .0
        .iter()
    {
        let m = Bytes(material);
        materials.push(Material {
            name: m.name(0)?,
            texture_token: m.u32(8)?,
            fields: m.name(12)?,
            values: [m.u16(16)?, m.u16(18)?, m.u16(20)?, m.u16(22)?],
        });
    }
    let count = usize::from(blob.byte(8)?);
    let mut models = Vec::new();
    for header in blob
        .array(blob.u32(12)?, count, 16)?
        .as_chunks::<16>()
        .0
        .iter()
    {
        models.push(decode_model(blob, Bytes(header))?);
    }
    let model_order = blob.array(blob.u32(16)?, count, 1)?;
    if model_order.iter().any(|&index| usize::from(index) >= count) {
        return Err(invalid("LM model order outside model array"));
    }
    Ok(Lm {
        materials,
        models,
        model_order,
        loaded: true,
    })
}

#[derive(Debug)]
pub struct Collision<'a> {
    pub position: [i32; 2],
    pub split_vertices: WireArray<'a, 6>,
    pub surfaces: WireArray<'a, 12>,
    pub subcells: WireArray<'a, 10>,
    pub spheres: WireArray<'a, 10>,
    pub cell_size: i16,
    pub grid: [u8; 2],
    pub ranges: WireArray<'a, 4>,
    pub indices: [&'a [u8]; 2],
    pub check_count: u8,
    /// Mutable native working state, copied rather than aliasing the wire file.
    pub checks: [u8; 256],
}
pub fn decode_collision(data: &[u8]) -> Result<Collision<'_>> {
    let b = Bytes(data);
    b.range(0, 1, 308)?;
    let grid = [b.byte(30)?, b.byte(31)?];
    let result = Collision {
        position: [b.i32(0)?, b.i32(4)?],
        split_vertices: WireArray::from_blob(b, b.u32(12)?, usize::from(b.byte(8)?))?,
        surfaces: WireArray::from_blob(b, b.u32(16)?, usize::from(b.byte(9)?))?,
        subcells: WireArray::from_blob(b, b.u32(20)?, usize::from(b.byte(10)?))?,
        spheres: WireArray::from_blob(b, b.u32(24)?, usize::from(b.byte(11)?))?,
        cell_size: b.i16(28)?,
        grid,
        ranges: WireArray::from_blob(b, b.u32(32)?, usize::from(grid[0]) * usize::from(grid[1]))?,
        indices: [
            b.array(b.u32(40)?, usize::from(b.u16(36)?), 1)?,
            b.array(b.u32(44)?, usize::from(b.u16(38)?), 1)?,
        ],
        check_count: b.byte(48)?,
        checks: b.name(52)?,
    };
    for cell in result.subcells.bytes.as_chunks::<10>().0.iter() {
        // 0xff means no split/surface. Preserve that sentinel; real indices are bounded.
        for index in &cell[6..8] {
            if *index != 255 && usize::from(*index) >= result.split_vertices.len() {
                return Err(invalid("collision split vertex index outside array"));
            }
        }
        for index in &cell[8..10] {
            if *index != 255 && usize::from(*index) >= result.surfaces.len() {
                return Err(invalid("collision surface index outside array"));
            }
        }
    }
    Ok(result)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelIdentity {
    pub global: bool,
    pub name: [u8; 8],
    pub wire_token: u32,
}
#[derive(Debug)]
pub struct MapInstance {
    pub model_index: usize,
    pub rotation: [i16; 9],
    pub translation: [i32; 3],
    pub matrix_padding: i16,
}
#[derive(Debug)]
pub struct MapBuffer<'a> {
    pub flags: u8,
    pub bounds: [i16; 4],
    pub instances: Vec<MapInstance>,
    pub unknown: WireArray<'a, 8>,
    pub positions: WireArray<'a, 8>,
}
#[derive(Debug)]
pub struct Map<'a> {
    pub chunk: [i8; 2],
    pub lm: Lm<'a>,
    pub model_identities: Vec<ModelIdentity>,
    pub buffers: Vec<MapBuffer<'a>>,
    pub cell_ranges: [u8; 52],
    pub order: &'a [u8],
    pub collision: Collision<'a>,
    pub loaded: bool,
}
pub fn decode_ipd(data: &[u8]) -> Result<Map<'_>> {
    let b = Bytes(data);
    b.range(0, 1, 392)?;
    if b.byte(0)? != 20 {
        return Err(invalid("IPD magic differs from USA schema"));
    }
    if b.byte(1)? != 0 {
        return Err(invalid("IPD already relocated"));
    }
    let lm_offset = b.u32(4)? as usize;
    let lm = decode_lm(
        b.range(
            lm_offset,
            data.len()
                .checked_sub(lm_offset)
                .ok_or_else(|| invalid("IPD LM outside file"))?,
            1,
        )?,
    )?;
    let mut identities = Vec::new();
    for info in b
        .array(b.u32(20)?, usize::from(b.byte(8)?), 16)?
        .as_chunks::<16>()
        .0
        .iter()
    {
        let info = Bytes(info);
        identities.push(ModelIdentity {
            global: info.byte(0)? != 0,
            name: info.name(4)?,
            wire_token: info.u32(12)?,
        });
    }
    let mut buffers = Vec::new();
    for buffer in b
        .array(b.u32(24)?, usize::from(b.byte(9)?), 24)?
        .as_chunks::<24>()
        .0
        .iter()
    {
        let buffer = Bytes(buffer);
        let mut instances = Vec::new();
        for instance in b
            .array(buffer.u32(12)?, usize::from(buffer.byte(0)?), 36)?
            .as_chunks::<36>()
            .0
            .iter()
        {
            let instance = Bytes(instance);
            let model_index = instance.u32(0)? as usize;
            if model_index >= identities.len() {
                return Err(invalid("IPD instance model index outside model identities"));
            }
            let mut rotation = [0; 9];
            for (i, value) in rotation.iter_mut().enumerate() {
                *value = instance.i16(4 + i * 2)?;
            }
            instances.push(MapInstance {
                model_index,
                rotation,
                matrix_padding: instance.i16(22)?,
                translation: [instance.i32(24)?, instance.i32(28)?, instance.i32(32)?],
            });
        }
        let cells = usize::from(buffer.byte(2)?);
        buffers.push(MapBuffer {
            flags: buffer.byte(1)?,
            bounds: [
                buffer.i16(4)?,
                buffer.i16(6)?,
                buffer.i16(8)?,
                buffer.i16(10)?,
            ],
            instances,
            unknown: WireArray::from_blob(b, buffer.u32(16)?, cells)?,
            positions: WireArray::from_blob(b, buffer.u32(20)?, cells)?,
        });
    }
    let order = b.array(b.u32(80)?, usize::from(b.byte(10)?), 1)?;
    if order
        .iter()
        .any(|&index| usize::from(index) >= buffers.len())
    {
        return Err(invalid("IPD buffer order outside buffers"));
    }
    let cell_ranges = b.name::<52>(28)?;
    for range in cell_ranges[..50].as_chunks::<2>().0.iter() {
        if usize::from(range[0]) + usize::from(range[1]) > order.len() {
            return Err(invalid("IPD cell draw range outside order"));
        }
    }
    Ok(Map {
        chunk: [b.byte(2)? as i8, b.byte(3)? as i8],
        lm,
        model_identities: identities,
        buffers,
        cell_ranges,
        order,
        collision: decode_collision(&data[84..])?,
        loaded: true,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoldRange {
    pub start: i16,
    pub end: i16,
    pub keyframe: usize,
}
#[derive(Debug)]
pub struct DmsEntry<'a, const STRIDE: usize> {
    pub name: [u8; 4],
    pub holds: Vec<HoldRange>,
    pub keyframes: WireArray<'a, STRIDE>,
}
#[derive(Debug)]
pub struct Dms<'a> {
    pub origin: [i32; 3],
    pub unknown: u32,
    pub segments: Vec<[i16; 2]>,
    pub characters: Vec<DmsEntry<'a, 12>>,
    pub camera: DmsEntry<'a, 16>,
    pub loaded: bool,
}
fn decode_dms_entry<'a, const STRIDE: usize>(
    b: Bytes<'a>,
    header: Bytes<'_>,
) -> Result<DmsEntry<'a, STRIDE>> {
    let count =
        usize::try_from(header.i16(0)?).map_err(|_| invalid("negative DMS keyframe count"))?;
    let mut holds = Vec::new();
    for hold in b
        .array(header.u32(8)?, usize::from(header.byte(2)?), 6)?
        .as_chunks::<6>()
        .0
        .iter()
    {
        let hold = Bytes(hold);
        let start = hold.i16(0)?;
        let end = hold.i16(2)?;
        let keyframe =
            usize::try_from(hold.i16(4)?).map_err(|_| invalid("negative DMS hold keyframe"))?;
        if start < 0 || end < start || keyframe >= count {
            return Err(invalid("invalid DMS hold range/keyframe"));
        }
        holds.push(HoldRange {
            start,
            end,
            keyframe,
        });
    }
    Ok(DmsEntry {
        name: header.name(4)?,
        holds,
        keyframes: WireArray::from_blob(b, header.u32(12)?, count)?,
    })
}
pub fn decode_dms(data: &[u8]) -> Result<Dms<'_>> {
    let b = Bytes(data);
    b.range(0, 1, 44)?;
    if b.byte(0)? != 0 {
        return Err(invalid("DMS already relocated"));
    }
    let mut segments = Vec::new();
    for segment in b
        .array(b.u32(8)?, usize::from(b.byte(2)?), 4)?
        .as_chunks::<4>()
        .0
        .iter()
    {
        let s = Bytes(segment);
        segments.push([s.i16(0)?, s.i16(2)?]);
    }
    let mut characters = Vec::new();
    for header in b
        .array(b.u32(24)?, usize::from(b.byte(1)?), 16)?
        .as_chunks::<16>()
        .0
        .iter()
    {
        characters.push(decode_dms_entry(b, Bytes(header))?);
    }
    Ok(Dms {
        origin: [b.i32(12)?, b.i32(16)?, b.i32(20)?],
        unknown: b.u32(4)?,
        segments,
        characters,
        camera: decode_dms_entry(b, Bytes(&data[28..44]))?,
        loaded: true,
    })
}

/// Savegame is pointer-free on disc (636 bytes). Keep reserved/unknown bytes for
/// exact roundtrip; native gameplay fields are decoded explicitly, not overlaid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Savegame {
    original: [u8; 636],
    pub items: [[u8; 4]; 40],
    pub map_index: i8,
    pub room_index: i8,
    pub save_count: i16,
    pub location: i8,
    pub paper_map: u8,
    pub weapon: u8,
    pub inventory_slots: u8,
    pub toggles: u32,
    pub enemy_states: [i32; 45],
    pub paper_map_flags: i32,
    pub event_flags: [u32; 52],
    pub health_saturation: i32,
    pub picked_up_count: i16,
    pub inventory_flags: u8,
    pub health: i32,
    pub x: i32,
    pub rotation_y: i16,
    pub clear_count: u8,
    pub endings: u8,
    pub z: i32,
    pub timers: [u32; 3],
    pub progress_flags: u8,
    pub kill_counts: [u8; 3],
    pub difficulty_word: u32,
    pub shot_counters: [u16; 11],
    pub current_ending: i8,
    pub continues: u8,
}
pub fn decode_save(data: &[u8]) -> Result<Savegame> {
    if data.len() != 636 {
        return Err(invalid(
            "savegame must be exactly 636 bytes (not the card container)",
        ));
    }
    let b = Bytes(data);
    let mut items = [[0; 4]; 40];
    for (item, bytes) in items.iter_mut().zip(data[..160].as_chunks::<4>().0.iter()) {
        item.copy_from_slice(bytes);
    }
    let mut enemy_states = [0; 45];
    for (i, state) in enemy_states.iter_mut().enumerate() {
        *state = b.i32(176 + i * 4)?;
    }
    let mut event_flags = [0; 52];
    for (i, flags) in event_flags.iter_mut().enumerate() {
        *flags = b.u32(360 + i * 4)?;
    }
    let mut shot_counters = [0; 11];
    for (i, count) in shot_counters.iter_mut().enumerate() {
        *count = b.u16(612 + i * 2)?;
    }
    Ok(Savegame {
        original: data.try_into().unwrap(),
        items,
        map_index: b.byte(164)? as i8,
        room_index: b.byte(165)? as i8,
        save_count: b.i16(166)?,
        location: b.byte(168)? as i8,
        paper_map: b.byte(169)?,
        weapon: b.byte(170)?,
        inventory_slots: b.byte(171)?,
        toggles: b.u32(172)?,
        enemy_states,
        paper_map_flags: b.i32(356)?,
        event_flags,
        health_saturation: b.i32(568)?,
        picked_up_count: b.i16(572)?,
        inventory_flags: b.byte(575)?,
        health: b.i32(576)?,
        x: b.i32(580)?,
        rotation_y: b.i16(584)?,
        clear_count: b.byte(586)?,
        endings: b.byte(587)?,
        z: b.i32(588)?,
        timers: [b.u32(592)?, b.u32(596)?, b.u32(600)?],
        progress_flags: b.byte(604)?,
        kill_counts: b.name(605)?,
        difficulty_word: b.u32(608)?,
        shot_counters,
        current_ending: b.byte(634)? as i8,
        continues: b.byte(635)?,
    })
}
impl Savegame {
    pub fn encode(&self) -> [u8; 636] {
        let mut out = self.original;
        for (bytes, item) in out[..160].as_chunks_mut::<4>().0.iter_mut().zip(self.items) {
            bytes.copy_from_slice(&item);
        }
        out[164] = self.map_index as u8;
        out[165] = self.room_index as u8;
        out[166..168].copy_from_slice(&self.save_count.to_le_bytes());
        out[168..172].copy_from_slice(&[
            self.location as u8,
            self.paper_map,
            self.weapon,
            self.inventory_slots,
        ]);
        for (offset, value) in [
            (172, self.toggles),
            (356, self.paper_map_flags as u32),
            (568, self.health_saturation as u32),
            (576, self.health as u32),
            (580, self.x as u32),
            (588, self.z as u32),
            (608, self.difficulty_word),
        ] {
            out[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        for (i, state) in self.enemy_states.iter().enumerate() {
            out[176 + i * 4..180 + i * 4].copy_from_slice(&state.to_le_bytes());
        }
        for (i, flags) in self.event_flags.iter().enumerate() {
            out[360 + i * 4..364 + i * 4].copy_from_slice(&flags.to_le_bytes());
        }
        out[572..574].copy_from_slice(&self.picked_up_count.to_le_bytes());
        out[575] = self.inventory_flags;
        out[584..586].copy_from_slice(&self.rotation_y.to_le_bytes());
        out[586] = self.clear_count;
        out[587] = self.endings;
        for (i, time) in self.timers.iter().enumerate() {
            out[592 + i * 4..596 + i * 4].copy_from_slice(&time.to_le_bytes());
        }
        out[604] = self.progress_flags;
        out[605..608].copy_from_slice(&self.kill_counts);
        for (i, count) in self.shot_counters.iter().enumerate() {
            out[612 + i * 2..614 + i * 2].copy_from_slice(&count.to_le_bytes());
        }
        out[634] = self.current_ending as u8;
        out[635] = self.continues;
        out
    }
}

/// Read-only audit through the shared archive reader. No asset bytes are written.
pub fn inspect_disc<S: psxdisc::SectorReader>(
    disc: &mut psxdisc::GameDisc<S>,
) -> std::result::Result<(), String> {
    let entries: Vec<_> = disc
        .entries()
        .iter()
        .filter(|entry| matches!(entry.file_type, 3 | 5 | 6 | 7))
        .cloned()
        .collect();
    let mut passed = [0_usize; 4];
    let mut failed = 0;
    for entry in entries {
        let bytes = disc
            .read_entry(entry.id)
            .map_err(|error| error.to_string())?;
        let (slot, result) = match entry.file_type {
            3 => (0, decode_dms(&bytes).map(|_| ())),
            5 => (1, decode_lm(&bytes).map(|_| ())),
            6 => (2, decode_ipd(&bytes).map(|_| ())),
            7 => (3, decode_lm(&bytes).map(|_| ())),
            _ => unreachable!(),
        };
        match result {
            Ok(()) => passed[slot] += 1,
            Err(error) => {
                failed += 1;
                println!("ASSET FAIL id={} {}: {}", entry.id, entry.path, error);
            }
        }
    }
    println!(
        "ASSET AUDIT DMS={} PLM={} IPD={} ILM={} failures={failed}",
        passed[0], passed[1], passed[2], passed[3]
    );
    if failed == 0 {
        Ok(())
    } else {
        Err(format!("{failed} asset layouts require investigation"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn put(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    fn lm() -> Vec<u8> {
        let mut data = vec![0; 104];
        data[0] = b'0';
        data[1] = 6;
        data[8] = 1;
        put(&mut data, 12, 20);
        put(&mut data, 16, 100);
        data[20..28].copy_from_slice(b"MODEL123");
        data[28] = 1;
        put(&mut data, 32, 36);
        data[36] = 1;
        data[37] = 3;
        data[38] = 1;
        data[39] = 2;
        for (offset, value) in [(40, 60), (44, 80), (48, 92), (52, 98), (56, 102)] {
            put(&mut data, offset, value);
        }
        data[72..76].copy_from_slice(&[0, 1, 2, 0]);
        data
    }
    #[test]
    fn model_wire_strides_bounds_immutable_and_unaligned_input() {
        let data = lm();
        let original = data.clone();
        let model = decode_lm(&data).unwrap();
        assert_eq!(model.models[0].name, *b"MODEL123");
        assert_eq!(model.models[0].meshes[0].vertices_z.len(), 3);
        assert_eq!(data, original);
        let mut unaligned = vec![0];
        unaligned.extend(&data);
        assert_eq!(decode_lm(&unaligned[1..]).unwrap().models.len(), 1);
        let mut bad = data.clone();
        bad[75] = 255;
        assert!(decode_lm(&bad).is_ok());
        bad = data.clone();
        put(&mut bad, 40, u32::MAX);
        assert!(decode_lm(&bad).is_err());
        bad = data.clone();
        bad[72] = 3;
        assert!(decode_lm(&bad).is_err());
        bad = data.clone();
        bad[2] = 1;
        assert!(decode_lm(&bad).is_err());
        for size in 0..20 {
            assert!(decode_lm(&data[..size]).is_err());
        }
    }
    #[test]
    fn collision_relative_offsets_native_working_state_and_truncation() {
        let mut data = vec![0; 340];
        data[8] = 2;
        data[9] = 1;
        data[10] = 1;
        put(&mut data, 12, 308);
        put(&mut data, 16, 320);
        put(&mut data, 20, 332);
        data.resize(342, 0);
        data[338..342].copy_from_slice(&[0, 1, 0, 255]);
        let mut collision = decode_collision(&data).unwrap();
        assert_eq!(collision.split_vertices.len(), 2);
        collision.checks[0] = 9;
        assert_eq!(collision.checks[0], 9);
        assert_eq!(data[52], 0);
        data[338] = 2;
        assert!(decode_collision(&data).is_err());
        put(&mut data, 12, u32::MAX);
        assert!(decode_collision(&data).is_err());
    }
    #[test]
    fn ipd_nested_base_and_model_indices_are_not_pointers() {
        let mut data = vec![0; 392];
        data[0] = 20;
        put(&mut data, 4, 392);
        data.extend(lm());
        data[8] = 1;
        put(&mut data, 20, 496);
        data.resize(512, 0);
        data[9] = 1;
        put(&mut data, 24, 512);
        data.resize(536, 0);
        data[512] = 1;
        put(&mut data, 524, 536);
        data.resize(572, 0);
        let map = decode_ipd(&data).unwrap();
        assert_eq!(map.lm.models[0].meshes[0].vertices_xy.len(), 3);
        assert_eq!(map.buffers[0].instances[0].model_index, 0);
        put(&mut data, 536, 1);
        assert!(decode_ipd(&data).is_err());
    }
    #[test]
    fn dms_distinct_camera_character_strides_and_hold_indices() {
        let mut data = vec![0; 96];
        data[1] = 1;
        data[2] = 1;
        put(&mut data, 8, 44);
        put(&mut data, 24, 48);
        data[28] = 1;
        put(&mut data, 40, 64);
        data[48] = 1;
        data[50] = 1;
        data[52..56].copy_from_slice(b"HARR");
        put(&mut data, 56, 92);
        put(&mut data, 60, 80);
        data.resize(98, 0);
        let dms = decode_dms(&data).unwrap();
        assert_eq!(dms.camera.keyframes.bytes().len(), 16);
        assert_eq!(dms.characters[0].keyframes.bytes().len(), 12);
        assert_eq!(dms.characters[0].name, *b"HARR");
        data[96] = 1;
        assert!(decode_dms(&data).is_err());
        data[96] = 0;
        data[48] = 255;
        data[49] = 255;
        assert!(decode_dms(&data).is_err());
    }
    #[test]
    fn save_roundtrip_preserves_unknown_bits_and_signed_values() {
        let data: Vec<u8> = (0..636).map(|i| (i * 73) as u8).collect();
        let mut save = decode_save(&data).unwrap();
        assert_eq!(save.encode().as_slice(), data);
        save.x = -12345;
        save.rotation_y = -17;
        save.map_index = -1;
        save.event_flags[51] = 0xaabbccdd;
        let encoded = save.encode();
        let decoded = decode_save(&encoded).unwrap();
        assert_eq!(decoded.x, save.x);
        assert_eq!(decoded.event_flags, save.event_flags);
        assert_eq!(decoded.encode(), encoded);
        assert_eq!(encoded[161], data[161]);
        assert!(decode_save(&data[..635]).is_err());
    }
    #[test]
    fn malformed_decoders_never_panic() {
        for length in 0..800 {
            let data: Vec<u8> = (0..length).map(|i| (i * 47 + length) as u8).collect();
            let _ = decode_lm(&data);
            let _ = decode_ipd(&data);
            let _ = decode_collision(&data);
            let _ = decode_dms(&data);
            let _ = decode_save(&data);
        }
        assert!(Bytes(&[0; 20]).range(usize::MAX, 2, usize::MAX).is_err());
    }
    #[test]
    fn c_wire_and_native_layout_gate_links() {
        unsafe extern "C" {
            fn sh_disk32_layout_check() -> i32;
        }
        // SAFETY: compile/link probe has no parameters or memory effects.
        assert_eq!(
            unsafe { sh_disk32_layout_check() },
            (std::mem::size_of::<usize>() * 3) as i32
        );
    }
}
