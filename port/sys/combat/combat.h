/* Copyright (C) 2026 shdecompilations; SPDX-License-Identifier: GPL-3.0-only */
#ifndef SH_COMBAT_H
#define SH_COMBAT_H
#include "boot.h"
#include "combat_namespace.h"
#include "combat_records.h"
#include "disk32.h"
extern s_800AD4C8 sh_combat_attacks[70];
extern u32 sh_combat_attack_aux;
extern bool sh_combat_dark_environment;
extern s_AnimInfo* sh_combat_harry_map_anims;
extern s_AnimInfo D_800297B8[];
extern s16 SQRT[];
extern u32 sh_combat_lzc_input;
u32 sh_combat_lzc(u32 input);
s32 sh_combat_ps1_div(s32 numerator,s32 denominator);
/* PORT: Local compile-only CRT lacks abort; runtime uses the native CRT. */
#ifdef SH_COMBAT_FREESTANDING_CRT
_Noreturn void abort(void);
#endif
s32 sh_combat_npc_index(const s_SubCharacter* character);
u8 sh_combat_gun_capacity(u8 attack);
enum {SfxFlag_None=0,GameDifficulty_Normal=0};
#define WEAPON_ATTACK(weapon,input) ((weapon)+((input)*10))
q19_12 Rng_RandQ12(void);
q19_12 Math_Distance2dGet(const VECTOR3* from,const VECTOR3* to);
q19_12 Math_Vector2MagCalc(s32 x,s32 y);
s_AnimInfo* func_80044918(s_ModelAnim* anim);
q19_12 func_8007FD2C(void);
s32 func_80080540(q19_12 x,q19_12 y,q19_12 z);
void func_8005F6B0(s_SubCharacter* target,VECTOR3* pos,s32 kind,s32 group);
void func_800892A4(s32 index);
void func_80089314(s32 value);
void func_8009151C(s32 index,s32 hit,q19_12 value);
s32 func_8009146C(s32 kind);
void func_800914C4(s32 kind,s32 count);
s32 Game_HyperBlasterBeamColorGet(void);
void Sfx_WithFlagsPlay(u16 id,VECTOR3* pos,q23_8 volume,s32 flags);
void Sfx_WithFlagsAndPitchPlay(u16 id,VECTOR3* pos,q23_8 volume,s32 flags,s32 pitch);
void Sd_SfxStop(u16 id);
bool Ray_TraceQuery(s_RayTrace* trace,const VECTOR3* from,const VECTOR3* to);
bool Ray_CharaTraceQuery(s_RayTrace* trace,const VECTOR3* from,VECTOR3* offset,s_SubCharacter* exclude);
/* PORT: Fixed-width wire records are decoded numerically; never cast to native. */
typedef struct {
    s16 range,offset;u16 damage;
    s8 interval,interval_high;u8 count,character,spread,spread_high;
    s16 force;u8 start,duration,type,response,effect; s8 reserved;
    u32 auxiliary_address;
} ShCombatAttackWire;
STATIC_ASSERT_SIZEOF(ShCombatAttackWire,24);
_Static_assert(offsetof(ShCombatAttackWire,auxiliary_address)==20,"wire attack identity");
typedef struct {s16 top,bottom,height,offset_y,radius,character_radius,box_x,box_z,cylinder_x,cylinder_z;} ShCombatKeyframeWire;
STATIC_ASSERT_SIZEOF(ShCombatKeyframeWire,20);
STATIC_ASSERT_SIZEOF(s_Keyframe,20);
STATIC_ASSERT_SIZEOF(s_CharaCollision,40);
_Static_assert(offsetof(s_SubCharacter,health)==192,"native character health");
_Static_assert(offsetof(s_SubCharacter,properties)==256,"native character properties");
_Static_assert(offsetof(s_PlayerWork,extra)==328,"native player upper-body state");
bool sh_combat_attack_decode(const u8* bytes,size_t length,u32 expected_aux,s_800AD4C8* out,u32* aux);
bool sh_combat_keyframe_decode(const u8* bytes,size_t length,s_Keyframe* out);
bool sh_combat_sqrt_decode(const u8* bytes,size_t length,s16* table,size_t count);
s32 sh_combat_model_probe(const s_LmHeader* model,const s_AnmHeader* animation);
void GsInitCoordinate2(GsCOORDINATE2* parent,GsCOORDINATE2* coordinate);
void sh_combat_Anim_BoneInit(s_AnmHeader* animation,GsCOORDINATE2* coordinates);
void sh_combat_Bone_ModelAssign(s_Bone* bone,s_LmHeader* model,s32 index);
void sh_combat_reset_scratch(void);
bool sh_combat_collision_surface_height(const s_IpdCollisionData* collision,u32 index,s16* height);
#endif
