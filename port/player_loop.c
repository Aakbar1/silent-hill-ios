/* SPDX-License-Identifier: GPL-3.0-only */
#include "native_player_loop.h"
#include <stdio.h>
VECTOR3 D_800C45B0;
// Missing game logic terminates with its exact seam; these never simulate success.
void func_800892A4(s32 value) {
    // PORT: This is a PS1 motor/vibration request, not game logic. The native
    // host has no PS1 motors; keep the request visible until touch haptics exist.
    printf("HAPTIC request=%d (native motor backend unavailable)\n",value);
}
void Game_FlashlightToggle(void) {port_unimplemented("Game_FlashlightToggle");}
