/* SPDX-License-Identifier: GPL-3.0-only */
#include "native_player_loop.h"
#include <stdio.h>
VECTOR3 D_800C45B0;
void World_NearbyPlayerCollisionTriggersGet(void) {port_unimplemented("World_NearbyPlayerCollisionTriggersGet/native nearby trigger classification");}
// Missing game logic terminates with its exact seam; these never simulate success.
void func_800892A4(s32 value) {
    // PORT: This is a PS1 motor/vibration request, not game logic. The native
    // host has no PS1 motors; keep the request visible until touch haptics exist.
    printf("HAPTIC request=%d (native motor backend unavailable)\n",value);
}
void Game_FlashlightToggle(void) {port_unimplemented("Game_FlashlightToggle");}
void Player_CombatUpdate(s_SubCharacter* player,GsCOORDINATE2* coords) {
    (void)player;(void)coords;port_unimplemented("Player_CombatUpdate/native combat services");
}
void func_8008A3AC(s_SubCharacter* player) {(void)player;port_unimplemented("func_8008A3AC/native combat slice");}
void Game_NpcUpdate(void) {port_unimplemented("Game_NpcUpdate/native NPC animation/collision scheduling");}
void func_8005E89C(void) {port_unimplemented("func_8005E89C/map effects state");}
void WorldGfx_Draw(s32 pass) {(void)pass;} // PORT: No-draw bridge for the world lane.
void Player_ReceiveDamage(s_SubCharacter* player,s_PlayerExtra* extra) {
    (void)player;(void)extra;port_unimplemented("Player_ReceiveDamage");
}
void Player_LogicUpdate(s_SubCharacter* player,s_PlayerExtra* extra,GsCOORDINATE2* coords) {
    (void)player;(void)extra;(void)coords;port_unimplemented("Player_LogicUpdate/original movement state machine");
}
void Player_PositionUpdate(s_SubCharacter* player,s_PlayerExtra* extra,GsCOORDINATE2* coords) {
    (void)player;(void)extra;(void)coords;port_unimplemented("Player_PositionUpdate/native wall collision");
}
void Player_AnimUpdate(s_SubCharacter* player,s_PlayerExtra* extra,s_AnmHeader* anm,GsCOORDINATE2* coords) {
    (void)player;(void)extra;(void)anm;(void)coords;port_unimplemented("Player_AnimUpdate/native upper/lower body animation");
}
void func_8007D090(s_SubCharacter* player,s_PlayerExtra* extra,GsCOORDINATE2* coords) {
    (void)player;(void)extra;(void)coords;port_unimplemented("func_8007D090/player bone transforms");
}
