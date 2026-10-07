/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef SH_MAP_H
#define SH_MAP_H
#include "boot.h"
#ifndef ARRAY_SIZE
#define ARRAY_SIZE(a) (sizeof(a)/sizeof((a)[0]))
#endif
#include "native_map_records.h"
STATIC_ASSERT_SIZEOF(s_MapOverlayHdr,11688);
STATIC_ASSERT_SIZEOF(s_MapInfo,24);
_Static_assert(offsetof(s_MapOverlayHdr,mapPoints)==56,"native map points");
_Static_assert(offsetof(s_MapOverlayHdr,charaUpdateFuncs)==808,"native map character callbacks");
_Static_assert(offsetof(s_MapOverlayHdr,cameraPaths)==1684,"native map camera paths");
_Static_assert(offsetof(s_MapOverlayHdr,collisionTriggers)==6884,"native map collision triggers");
// PORT: Active native descriptor survives STREAM's temporary overlay selection.
// Disc overlay addresses are never used as callback or data pointers.
const s_MapOverlayHdr* port_map_active(void);
#define g_MapOverlayHdr (*port_map_active())
const s_MapOverlayHdr* sh_map0_s00_descriptor(void);
void sh_map0_s00_reset(void);
int sh_map0_s00_reset_probe(void);
int sh_map0_s00_load_data(void);
int port_map_data_read(u32 file,u32 offset,u32 size,u8* destination);
int port_map_activate(u32 file);
int port_map_zero(const void* data,size_t size);
void Chara_PositionSet(s_MapPoint2d* point);
void Game_MapRoomIdxUpdate(void);
void GameFs_PlayerMapAnimLoad(s32 map);
void Map_EffectTexturesLoad(s32 map);
s32 WorldGfx_PlayerPrevHeldItem(s_PlayerCombat* combat);
void Gfx_PlayerHeldItemAttach(u8 attack);
int port_native_map_probe(s_IpdHeader* map,u32* output);
void port_map_layout_probe(u32* output);
int port_asset_load_ipd(u32 file,s_IpdHeader* destination,s32 global_file);
#endif
