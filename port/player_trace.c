/* SPDX-License-Identifier: GPL-3.0-only */
#include "camera.h"
#include "native_player_events.h"
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
    fflush(stdout);
    const char* events=getenv("SH_EVENTS_TRACE");
    if(events && !strcmp(events,"1")) {
        printf("TRANSIT_FRAME tick=%u map=%d room=%d game=%d step=%d sys=%d flags=%d fade=%d control=%d speed=%d lower=%d upper=%d\n",
            frame,g_SavegamePtr->mapIdx,g_SavegamePtr->mapRoomIdx,g_GameWork.gameState,g_GameWork.gameStateSteps[0],
            g_SysWork.sysState,g_SysWork.sysFlags,g_ScreenFade_Status,(int)g_Player_DisableControl,
            player->properties.player.moveSpeed,g_SysWork.playerWork.extra.lowerBodyState,g_SysWork.playerWork.extra.upperBodyState);
        printf("EVENTS_FRAME tick=%u map=%d room=%d sys=%d event=%u step0=%d step1=%d step2=%d flags=%d health=%d npc0=%d npc0_health=%d\n",
            frame,g_SavegamePtr->mapIdx,g_SavegamePtr->mapRoomIdx,g_SysWork.sysState,g_MapEventParam,
            g_SysWork.sysStateSteps[0],g_SysWork.sysStateSteps[1],g_SysWork.sysStateSteps[2],g_SysWork.sysFlags,
            player->health,g_SysWork.npcs[0].model.charaId,g_SysWork.npcs[0].health);
        printf("COMBAT_PLAYER tick=%u health=%d damage=%d weapon=%d ammo=%u pad=%u\n",frame,player->health,
            player->damage.amount,g_SysWork.playerCombat.weaponAttack,g_SysWork.playerCombat.totalWeaponAmmo,g_Controller0->buttonFlags.held);
        for(s32 i=0;i<NPC_COUNT_MAX;i++) {
            const s_SubCharacter* npc=&g_SysWork.npcs[i];
            if(npc->model.charaId)printf("COMBAT_NPC tick=%u slot=%d id=%u health=%d damage=%d state=%u\n",
                frame,i,npc->model.charaId,npc->health,npc->damage.amount,npc->model.controlState);
        }
    }
}

// PORT: Optional native-stage diagnostics retain the original update order.
void port_move_stage(const char* name) {
    const char* enabled=getenv("SH_MOVE_STAGE_TRACE");
    if(!enabled || strcmp(enabled,"1") || VSync(-1)<3478)return;
    printf("MOVE_STAGE tick=%d name=%s\n",VSync(-1),name);fflush(stdout);
}

// PORT: Numeric rollout evidence contains no owned message strings or glyph pixels.
void port_events_message_trace(s32 message,s32 length,s32 rollout,s32 selection) {
    const char* enabled=getenv("SH_EVENTS_TRACE");
    if(!enabled || strcmp(enabled,"1"))return;
    printf("MESSAGE_FRAME tick=%d id=%d length=%d rollout=%d selection=%d timer=%d audio=%u\n",
        VSync(-1),message,length,rollout,selection,g_SysWork.mapMsgTimer,g_MapMsg_AudioType);
}

void port_events_page_trace(s32 message) {
    const char* enabled=getenv("SH_EVENTS_TRACE");
    if(enabled && !strcmp(enabled,"1"))printf("MESSAGE_PAGE tick=%d id=%d\n",VSync(-1),message);
}

void port_events_hit_trace(const s_SubCharacter* attacker,const s_SubCharacter* target,s32 damage) {
    const char* enabled=getenv("SH_EVENTS_TRACE");
    if(!enabled || strcmp(enabled,"1"))return;
    s32 attack_slot=-1,target_slot=-1;
    if(attacker==&g_SysWork.playerWork.player)attack_slot=NPC_COUNT_MAX;
    if(target==&g_SysWork.playerWork.player)target_slot=NPC_COUNT_MAX;
    for(s32 i=0;i<NPC_COUNT_MAX;i++) {
        if(attacker==&g_SysWork.npcs[i])attack_slot=i;
        if(target==&g_SysWork.npcs[i])target_slot=i;
    }
    printf("COMBAT_HIT tick=%d attacker=%d target=%d damage=%d\n",VSync(-1),attack_slot,target_slot,damage);
}
