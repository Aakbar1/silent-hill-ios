/* Copyright (C) 2026 shdecompilations; SPDX-License-Identifier: GPL-3.0-only */
#ifndef SH_COMBAT_AI_H
#define SH_COMBAT_AI_H
#include "combat.h"
#include "combat_ai_namespace.h"
#include "maps/characters/stalker.h"
#include "maps/characters/cat.h"
#include "maps/characters/parasite.h"
#include "maps/characters/flauros.h"
/* PORT: Clear the whole native property union, including nurse host pointers. */
#define Chara_PropsClear(chara) memset(&(chara)->properties,0,sizeof((chara)->properties))
#define Chara_DamageClear(chara) memset(&(chara)->damage,0,sizeof((chara)->damage))
#define ModelAnim_AnimInfoSet(anim,infos) ((anim)->baseAnimInfos=(infos),(anim)->mapAnimInfos=NULL)
static inline void Chara_AnimSet(s_SubCharacter* chara,s32 status,s32 keyframe) {
    chara->model.anim.status=(u8)status;chara->model.anim.time=Q12(keyframe);chara->model.anim.keyframeIdx=(u16)keyframe;
}
extern u32 sh_combat_chara_group_flags[4];
s32 Math_Vector2MagCalcSafeQ6(s32 x,s32 z);
s32 Math_AngleNormalizeSigned(s32 angle);
s32 Math_AngleBetweenPositionsGet(VECTOR3 from,VECTOR3 to);
void Math_MatrixTransform(VECTOR3* position,SVECTOR3* rotation,GsCOORDINATE2* coordinate);
void Sfx_WithPitchPlay(u16 sound,VECTOR3* pos,q23_8 volume,s32 pitch);
void Stalker_Init(s_SubCharacter* chara);
void sharedFunc_800D3308_0_s00(s_SubCharacter* chara);
void Stalker_Control_13(s_SubCharacter* chara);
void sharedFunc_800D7E04_0_s00(s_SubCharacter* chara,s32 sound);
#endif
