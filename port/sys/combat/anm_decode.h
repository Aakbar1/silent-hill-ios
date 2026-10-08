/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef SH_COMBAT_ANM_DECODE_H
#define SH_COMBAT_ANM_DECODE_H
#include "gameplay.h"
bool sh_fight_anm_decode(const u8* bytes,size_t size,s_AnmHeader* output,s_AnmBindPose* poses,size_t capacity);
#endif
