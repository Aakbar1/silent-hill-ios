/* SPDX-License-Identifier: GPL-3.0-only */
#include "native_player_loop.h"
#include "native_player_movement.h"
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

// PORT: Regression fixtures for native word coordinates, SDK product rounding
// and the original unsigned gameplay timer. All live save state is restored.
u32 port_move_native_probe(void) {
    u32 checks=0;
    s_CollisionResult collision={0};
    Collision_DefaultResultSet(&collision,-100000,900000,655360,-131072);
    if(collision.offset.vx==-100000 && collision.offset.vy==900000 && collision.offset.vz==655360 && collision.surface.groundHeight==-131072)checks|=1;
    MATRIX matrix={{{-65,123,45},{17,-234,67},{111,222,333}},{123456,-234567,345678}};
    Math_RotMatrixZ(256,&matrix);
    // Pinned SDK sine/cosine at 22.5 degrees are 1567/3784. Individual
    // arithmetic product shifts yield -67/-10 for the first column.
    if(matrix.m[0][0]==-67 && matrix.m[1][0]==-10)checks|=2;
    if(matrix.m[2][0]==111 && matrix.m[2][1]==222 && matrix.m[2][2]==333 && matrix.t[0]==123456 && matrix.t[1]==-234567 && matrix.t[2]==345678)checks|=4;
    if(SquareRoot12(0)==0 && SquareRoot12(4096)==4096 && SquareRoot12(16384)==8192)checks|=8;
    s_Savegame saved=*g_SavegamePtr;s32 saved_raw=g_DeltaTimeRaw;
    g_SavegamePtr->gameplayTimer=4276224000u-5;g_SavegamePtr->add290Hours=0;g_DeltaTimeRaw=10;
    port_move_game_timer_update();
    if(g_SavegamePtr->gameplayTimer==5 && g_SavegamePtr->add290Hours==1)checks|=16;
    g_SavegamePtr->gameplayTimer=4276224001u;g_SavegamePtr->add290Hours=3;g_DeltaTimeRaw=0;
    port_move_game_timer_update();
    if(g_SavegamePtr->gameplayTimer==1916928000u && g_SavegamePtr->add290Hours==3)checks|=32;
    g_SavegamePtr->gameplayTimer=0;port_move_game_timer_update();
    if(g_SavegamePtr->gameplayTimer==1)checks|=64;
    *g_SavegamePtr=saved;g_DeltaTimeRaw=saved_raw;
    return checks;
}
