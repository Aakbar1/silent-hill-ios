/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef SH_NPC_STARTUP_H
#define SH_NPC_STARTUP_H
#include "world.h"
// PORT: Unsigned masks preserve bit 31 without a signed-shift overflow.
#define Savegame_EventFlagGet(n) (g_SavegamePtr->eventFlags[(n)>>5] & (1u<<((n)&31)))
#define Savegame_EventFlagClear(n) (g_SavegamePtr->eventFlags[(n)>>5] &= ~(1u<<((n)&31)))
#define Savegame_EventFlagSet(n) (g_SavegamePtr->eventFlags[(n)>>5] |= (1u<<((n)&31)))
#define Savegame_EventFlagSetAlt(n) Savegame_EventFlagSet(n)
void GameBoot_NpcInit(void);
void GameBoot_InGameInit(void);
void Fs_CharaAnimDataAlloc(s32,e_CharaId,s_AnmHeader*,GsCOORDINATE2*);
void Fs_CharaAnimBoneInfoSet(void);
s32 WorldGfx_MapInitCharaLoad(s_MapOverlayHdr*);
void WorldGfx_CharaModelProcessAllLoads(void);
s32 WorldGfx_CharaModelLoad(s32,s32,s_LmHeader*,s_FsImageDesc*);
void WorldGfx_CharaLmBufferAdvance(u8**,s32);
void GameBoot_WolrdEnvInit(s32);
void func_8005E70C(void);
void func_8005E650(s32);
void func_80037124(void);
void func_8007E8C0(void);
void Game_NpcRoomInitSpawn(bool);
void Game_PlayerHeightUpdate(void);
void GameFs_WeaponInfoUpdate(void);
void GameFs_Tim00TIMLoad(void);
void GameFs_MapItemsModelLoad(u32);
s8 AreaLoad_TransitionFlags(void);
bool Sd_BgmInit(void);
bool Sd_BgmActiveSongCheck(s32);
void Sd_BgmSongSet(s32);
void Sd_BgmChannelSet(void);
void Sd_BgmUpdateTrack(void);
s32 Sd_AmbientSfxInit(void);
bool Sd_ActiveAmbientSfxCheck(s32);
void Sd_AmbientSfxSet(s32);
void Bgm_Update(bool);
void Bgm_LayerGlobalVariablesMute(void);
u16 Sd_MidiChannelTaskGet(void);
typedef struct {s16 presetIdx0,presetIdx1;} s_MapEnvPresetIdxs;
typedef s32 e_PrimitiveType;
enum {PrimitiveType_None=0};
void WorldEnv_MapPresetSet(s_MapOverlayHdr*);
void Gfx_MapEnvSet(s32,s32);
void Gfx_MapEnvUpdate(s32,s32,e_PrimitiveType,void*,s32,s32);
void Gfx_MapEnvStepUpdate(const s_MapEffectsInfo*,const s_MapEffectsInfo*,e_PrimitiveType,void*,s32,s32);
void Gfx_FogParametersSet(s_StructUnk3*,const s_MapEffectsInfo*);
bool Math_Distance2dCheck(const VECTOR3*,const VECTOR3*,q19_12);
bool func_8008F914(s32,s32);
void Game_RadioNoiseReset(void);
bool WorldMap_CloseChunkEdgeCheck(s32,s32);
s32 Chara_Spawn(e_CharaId,s32,q19_12,q19_12,q3_12,u32);
#define HAS_FLAG(p,n) ((((const u32*)(p))[(n)>>5]>>((n)&31))&1u)
#define SET_FLAG(p,n) (((u32*)(p))[(n)>>5]|=1u<<((n)&31))
#define SD_TASK_CHANNEL_SET(n) ((n)+0x300)
#endif
