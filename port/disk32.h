/* SPDX-License-Identifier: GPL-3.0-only
 * Layouts derived from silent-hill-decomp d9e28f8, GPL-3.0-only.
 * Copyright (C) 2026 shdecompilations. See game/decomp/LICENSE.
 * These are wire descriptions, never native game objects.
 */
#ifndef SH_DISK32_H
#define SH_DISK32_H
#include <stdint.h>
#include <stddef.h>

// PORT: File offsets and encoded indices remain 32 bits on every host ABI.
typedef uint32_t ShDiskOffset;
typedef struct { uint8_t counts[4]; ShDiskOffset arrays[5]; } ShDiskMesh;
typedef struct { uint8_t name[8], counts_flags[4]; ShDiskOffset meshes; } ShDiskModel;
typedef struct { uint8_t name[8]; uint32_t texture_token; uint8_t fields[4]; uint16_t values[4]; } ShDiskMaterial;
typedef struct { uint8_t magic, version, loaded, material_count; ShDiskOffset materials;
    uint8_t model_count, padding[3]; ShDiskOffset models, order; } ShDiskLm;
typedef struct { int32_t x,z; uint8_t counts[4]; ShDiskOffset arrays[4];
    int16_t cell_size; uint8_t cells_x,cells_z; ShDiskOffset ranges;
    uint16_t counts_24[2]; ShDiskOffset indices[2]; uint8_t check_count,padding[3],checks[256]; } ShDiskCollision;
typedef struct { uint8_t global,padding[3],name[8]; uint32_t model_token; } ShDiskMapModel;
typedef struct { uint32_t model_index; int16_t rotation[9],padding; int32_t translation[3]; } ShDiskMapInstance;
typedef struct { uint8_t counts[4]; int16_t bounds[4]; ShDiskOffset instances,unknown,positions; } ShDiskMapBuffer;
typedef struct { uint8_t magic,loaded; int8_t x,z; ShDiskOffset lm;
    uint8_t counts[3],padding[9]; ShDiskOffset model_infos,buffers;
    uint8_t cell_ranges[52]; ShDiskOffset order; ShDiskCollision collision; } ShDiskIpd;
typedef struct { int16_t keyframe_count; uint8_t hold_count,padding; char name[4]; ShDiskOffset holds,keyframes; } ShDiskDmsEntry;
typedef struct { uint8_t loaded,characters,segments,padding; uint32_t unknown;
    ShDiskOffset segment_array; int32_t origin[3]; ShDiskOffset character_array; ShDiskDmsEntry camera; } ShDiskDms;
typedef struct { uint8_t bytes[636]; } ShDiskSave;

// PORT: Native views are separate allocations; pointer/size_t never overwrite file slots.
typedef struct { const uint8_t* data; size_t count,stride; } ShNativeSpan;
typedef struct { uint32_t kind,file_id,models,materials,map_buffers,collision_cells,characters,segments; } ShNativeAssetInfo;
uint32_t port_asset_open(uint32_t file_id);
int port_asset_info(uint32_t handle,ShNativeAssetInfo* info);
int port_asset_close(uint32_t handle);
// Borrowed until close; section constants below select pointer-free leaf data.
int port_asset_span(uint32_t handle,uint32_t section,uint32_t model,uint32_t mesh,ShNativeSpan* span);
enum { ShAsset_Primitives=1,ShAsset_VertexXy=2,ShAsset_VertexZ=3,ShAsset_Normals=4,ShAsset_Unknown=5,
    ShAsset_CollisionVertices=10,ShAsset_CollisionSurfaces=11,ShAsset_CollisionSubcells=12,
    ShAsset_CollisionSpheres=13,ShAsset_CollisionRanges=14,ShAsset_CollisionIndices0=15,
    ShAsset_CollisionIndices1=16,ShAsset_CharacterFrames=20,ShAsset_CameraFrames=21 };
static inline int sh_span_u16(const ShNativeSpan* span,size_t index,size_t field,uint16_t* out) {
    if (!span || !out || !span->data || index>=span->count || span->stride<2 || field>span->stride-2 || index>(((size_t)-1)-field)/span->stride) return 0;
    const uint8_t* p=span->data+index*span->stride+field;
    *out=(uint16_t)((uint16_t)p[0]|((uint16_t)p[1]<<8)); return 1;
}
static inline int sh_span_u32(const ShNativeSpan* span,size_t index,size_t field,uint32_t* out) {
    if (!span || !out || !span->data || index>=span->count || span->stride<4 || field>span->stride-4 || index>(((size_t)-1)-field)/span->stride) return 0;
    const uint8_t* p=span->data+index*span->stride+field;
    *out=(uint32_t)p[0]|((uint32_t)p[1]<<8)|((uint32_t)p[2]<<16)|((uint32_t)p[3]<<24); return 1;
}
#define SH_WIRE_SIZE(T,N) _Static_assert(sizeof(T)==(N), #T " wire size")
int sh_native_span_read16(const ShNativeSpan* span,size_t index,size_t field,uint16_t* out);
int sh_native_span_read32(const ShNativeSpan* span,size_t index,size_t field,uint32_t* out);
#define SH_WIRE_OFFSET(T,F,N) _Static_assert(offsetof(T,F)==(N), #T "." #F " wire offset")
SH_WIRE_SIZE(ShDiskOffset,4);
SH_WIRE_SIZE(ShDiskMesh,24); SH_WIRE_OFFSET(ShDiskMesh,arrays,4);
SH_WIRE_SIZE(ShDiskModel,16); SH_WIRE_OFFSET(ShDiskModel,meshes,12);
SH_WIRE_SIZE(ShDiskMaterial,24); SH_WIRE_OFFSET(ShDiskMaterial,values,16);
SH_WIRE_SIZE(ShDiskLm,20); SH_WIRE_OFFSET(ShDiskLm,models,12);
SH_WIRE_SIZE(ShDiskCollision,308); SH_WIRE_OFFSET(ShDiskCollision,checks,52);
SH_WIRE_SIZE(ShDiskMapModel,16); SH_WIRE_SIZE(ShDiskMapInstance,36);
SH_WIRE_OFFSET(ShDiskMapInstance,translation,24);
SH_WIRE_SIZE(ShDiskMapBuffer,24); SH_WIRE_OFFSET(ShDiskMapBuffer,instances,12);
SH_WIRE_SIZE(ShDiskIpd,392); SH_WIRE_OFFSET(ShDiskIpd,collision,84);
SH_WIRE_SIZE(ShDiskDmsEntry,16); SH_WIRE_OFFSET(ShDiskDmsEntry,holds,8);
SH_WIRE_SIZE(ShDiskDms,44); SH_WIRE_OFFSET(ShDiskDms,camera,28);
SH_WIRE_SIZE(ShDiskSave,636);
SH_WIRE_SIZE(ShNativeAssetInfo,32);
_Static_assert(sizeof(ShNativeSpan)==3*sizeof(void*), "native span ABI");
#endif
