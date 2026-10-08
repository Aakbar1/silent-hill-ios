/* SPDX-License-Identifier: GPL-3.0-only */
#include "fight_fx.h"
#include <stdio.h>
#include <stdlib.h>
#if defined(__STDC_HOSTED__) && !__STDC_HOSTED__
char* getenv(const char*);
#endif
u32 port_fight_held_draws;
u32 port_fight_effect_bytes;
void port_fight_frame(u32 frame) {
    const char* mode=getenv("SH_FIGHT_DIAGNOSTIC");
    if(!mode)return;
    // PORT: Explicit isolated rendering/equip fixture, never a combat-play
    // pass. This grants a test weapon and emits test blood without damage.
    if(frame==3500) {
        s32 weapon=NO_VALUE;
        if(!strcmp(mode,"handgun"))weapon=EquippedWeaponId_Handgun;
        else if(!strcmp(mode,"pipe"))weapon=EquippedWeaponId_SteelPipe;
        else port_unimplemented("unknown fight diagnostic weapon");
        g_SysWork.playerCombat.weaponAttack=(s8)weapon;
        g_SysWork.playerCombat.totalWeaponAmmo=30;
        g_SysWork.playerCombat.currentWeaponAmmo=15;
        g_SysWork.playerCombat.weaponInventoryIdx=0;
        g_SavegamePtr->items[0].id=(u8)(weapon+InvItemId_KitchenKnife);
        g_SavegamePtr->items[0].count=15;
        g_SavegamePtr->equippedWeapon=(u8)(weapon+InvItemId_KitchenKnife);
        GameFs_WeaponInfoUpdate();
        port_render_previous_item(&g_SysWork.playerCombat);
        port_render_item_attach((u8)weapon);
        // PORT: This fixture runs at a VBlank boundary. The normal file queue
        // completes held-item loads on following ticks; never nest VSync here.
        printf("FIGHT_FIXTURE tick=%u weapon=%d injected_equip=1 injected_effect=1\n",frame,weapon);
    }
    if(frame==3600) {
        s_SubCharacter* player=&g_SysWork.playerWork.player;
        VECTOR3 pos=player->position;pos.vy-=Q12(0.8f);
        func_8005F6B0(player,&pos,4,1);
        func_800622B8(0,player,0,1);
    }
    if(frame>=3500) {
        const s_SubCharacter* player=&g_SysWork.playerWork.player;
        printf("FIGHT_FRAME tick=%u weapon=%d upper=%d lower=%d anim=%u keyframe=%d extra_anim=%u extra_keyframe=%d pad=%u aim=%d attacks=%d weapon_draws=%u fx_bytes=%u health=%d\n",
            frame,g_SysWork.playerCombat.weaponAttack,g_SysWork.playerWork.extra.upperBodyState,g_SysWork.playerWork.extra.lowerBodyState,
            player->model.anim.status,player->model.anim.keyframeIdx,g_SysWork.playerWork.extra.model.anim.status,
            g_SysWork.playerWork.extra.model.anim.keyframeIdx,g_Controller0->buttonFlags.held,g_SysWork.playerCombat.isAiming,
            Player_IsAttacking(),port_fight_held_draws,port_fight_effect_bytes,player->health);
    }
}
