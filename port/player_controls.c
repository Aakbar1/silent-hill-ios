/* SPDX-License-Identifier: GPL-3.0-only */
#include "native_player_loop.h"
s32 Inventory_HyperBlasterFunctionalTest(void) {
    port_unimplemented("Inventory_HyperBlasterFunctionalTest/original inventory service");return 0;
}
// PORT: Isolated original-controller fixture. It restores all touched globals;
// this checks input interpretation and turn arithmetic, not map movement.
u32 port_player_controls_probe(void) {
    PortGameWork saved_game=g_GameWork;PortSysWork saved_sys=g_SysWork;
    PortController* saved_controller=g_Controller0;PortController controller={0};
    s32 saved_delta=g_DeltaTime,saved_move=g_Player_HasMoveInput,saved_action=g_Player_HasActionInput,saved_rotation=D_800C454C;
    u8 saved_transition=g_Player_IsInWalkToRunTransition;
    u8 saved_stick=g_Player_MoveStickMag;
    u16* histories[]={&g_Player_IsMovingForward,&g_Player_IsMovingBackward,&g_Player_IsSteppingLeftTap,&g_Player_IsSteppingRightTap,&g_Player_IsSteppingLeftHold,&g_Player_IsSteppingRightHold,&g_Player_IsTurningLeft,&g_Player_IsTurningRight,&g_Player_IsRunning,&g_Player_IsAiming,&g_Player_IsShooting,&g_Player_IsAttacking,&g_Player_IsHoldAttack};
    u16 saved_histories[ARRAY_SIZE(histories)];
    for(size_t i=0;i<ARRAY_SIZE(histories);i++)saved_histories[i]=*histories[i];
    memset(&g_GameWork.config,0,sizeof(g_GameWork.config));
    g_GameWork.config.controllerConfig.run=ControllerFlag_Square;
    g_SysWork.sysState=SysState_Gameplay;
    g_SysWork.playerCombat.weaponAttack=NO_VALUE;
    g_SysWork.playerCombat.isAiming=false;
    g_Controller0=&controller;Game_PlayerMovementsReset();
    u32 checks=0;
    controller.buttonFlags.held=ControllerFlag_LStickHighUp;
    Player_Controller();if(g_Player_IsMovingForward==1 && !g_Player_IsRunning)checks|=1;
    Player_Controller();if(g_Player_IsMovingForward==3)checks|=2;
    controller.buttonFlags.held=0;Player_Controller();
    if(g_Player_IsMovingForward==2)checks|=4;
    Player_Controller();if(!g_Player_IsMovingForward)checks|=8;
    controller.buttonFlags.held=ControllerFlag_LStickHighUp|ControllerFlag_Square;
    Player_Controller();if(g_Player_IsMovingForward && g_Player_IsRunning)checks|=16;
    controller.buttonFlags.held=ControllerFlag_LStickHighRight;
    Player_Controller();g_DeltaTime=136;g_SysWork.playerWork.extra.lowerBodyState=PlayerLowerBodyState_None;
    Player_CharaRotate(8);if(g_Player_IsTurningRight==64 && D_800C454C==1088)checks|=32;
    g_SysWork.playerWork.extra.lowerBodyState=PlayerLowerBodyState_WalkBackward;
    Player_CharaRotate(8);if(D_800C454C==-1088)checks|=64;
    s_SubCharacter character={0};character.flags=CharaFlag_Dead|CharaFlag_Unk8;
    sh_combat_Chara_Flag8Clear(&character);character.damage.amount=1;
    sh_combat_Chara_DamagedFlagUpdate(&character);
    bool damage_ok=character.flags==(CharaFlag_Dead|CharaFlag_Damaged);
    character.damage.amount=0;sh_combat_Chara_DamagedFlagUpdate(&character);
    if(damage_ok && character.flags==CharaFlag_Dead)checks|=128;
    s_Model model={0};model.anim.status=(u8)ANIM_STATUS(HarryAnim_WalkForward,true);
    g_SysWork.playerWork.extra.state=PlayerState_None;g_Player_MoveStickMag=0;
    if(Anim_DurationGet(&model,&HARRY_BASE_ANIM_INFOS[model.anim.status])==Q12(22.0f))checks|=256;
    g_GameWork=saved_game;g_SysWork=saved_sys;g_Controller0=saved_controller;g_DeltaTime=saved_delta;
    g_Player_HasMoveInput=saved_move;g_Player_HasActionInput=saved_action;D_800C454C=saved_rotation;
    g_Player_IsInWalkToRunTransition=saved_transition;
    g_Player_MoveStickMag=saved_stick;
    for(size_t i=0;i<ARRAY_SIZE(histories);i++)*histories[i]=saved_histories[i];
    return checks;
}
