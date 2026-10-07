/* SPDX-License-Identifier: GPL-3.0-only */
#include "camera.h"
#include <stdio.h>
#include <stdlib.h>
#if defined(__STDC_HOSTED__) && !__STDC_HOSTED__
// PORT: Compile-only CRT has no environment declaration; runtime uses its CRT.
char* getenv(const char* name);
#endif
void port_player_trace(u32 frame) {
    const char* enabled=getenv("SH_PLAYER_TRACE");
    if(!enabled || strcmp(enabled,"1") || g_GameWork.gameState < GameState_MainLoadScreen)return;
    const s_SubCharacter* player=&g_SysWork.playerWork.player;
    printf("PLAYER_FRAME tick=%u state=%d sys=%d x=%d y=%d z=%d heading=%d camera_x=%d camera_z=%d look_x=%d look_z=%d lock_x=%d lock_z=%d pad=%u\n",
        frame,g_GameWork.gameState,g_SysWork.sysState,
        player->position.vx,player->position.vy,player->position.vz,player->rotation.vy,
        vcWork.cam_tgt_pos.vx,vcWork.cam_tgt_pos.vz,
        vcWork.watch_tgt_pos.vx,vcWork.watch_tgt_pos.vz,
        vcWork.chara_pos.vx,vcWork.chara_pos.vz,g_Controller0->buttonFlags.held);
}
