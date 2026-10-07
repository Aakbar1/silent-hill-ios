/* SPDX-License-Identifier: GPL-3.0-only; startup subset derived from silent-hill-decomp. */
#include "boot.h"
s32 g_MemCard_SavegameCount;
struct PortMapHeader g_MapOverlayHdr;
void SysWork_StateSetNext(s32 state) {g_SysWork.sysState=state;g_SysWork.sysStateCounter=0;}
void Fs_QueueReset(void) {Fs_QueueInitialize();}
void MemCard_SysDisable(void) {}
// PORT: Retain the original demo startup's first two states. Fail explicitly at
// the first unported gameplay dependency, never pretend that a world was loaded.
void GameBoot_InGameStartup(void) {
    switch (g_GameWork.gameStateSteps[0]) {
        case 0:
            g_IntervalVBlanks=1;
            g_GameWork.background2dColor.r=0;g_GameWork.background2dColor.g=0;g_GameWork.background2dColor.b=0;
            if (g_SysWork.processFlags==ProcessFlag_BootDemo) {g_GameWork.gameStateSteps[0]=1;g_SysWork.gameStateStepCounter=1;}
            else {port_unimplemented("GameBoot_InGameStartup gameplay records");}
            SD_Call(19);break;
        case 1:
            if (g_SysWork.gameStateStepCounter>SECONDS_60_FPS(20) && Fs_QueueGetLength()==0 && Sd_AudioStreamingCheck()==AudioStreamingState_None)
                port_unimplemented("Demo_DemoFileSavegameUpdate/native map linkage");
            break;
        default: port_unimplemented("GameBoot_InGameStartup/native world loading");
    }
}
void GameBoot_WorldInit(void) {port_unimplemented("GameBoot_WorldInit/native model, animation and collision consumers");}
void GameBoot_MapLoad(s32 map) {(void)map;port_unimplemented("GameBoot_MapLoad/native map descriptor");}
void Chara_PositionSet(const PortMapPoint* point) {(void)point;port_unimplemented("Chara_PositionSet/native player records");}
int sh_save_init_probe(void) {
    GameBoot_SavegameInitialize(0,GameDifficulty_Easy);
    u32 difficulty; memcpy(&difficulty,(const u8*)g_SavegamePtr+0x260,4);
    if (difficulty!=0xf0000000u || g_SavegamePtr->playerHealth!=Q12(100.0f) || g_SavegamePtr->invSlotCount!=8) return 0;
    for (s32 i=0;i<45;i++) if (g_SavegamePtr->mapEnemyStates[i]!=-1) return 0;
    for (s32 i=0;i<40;i++) if (g_SavegamePtr->items[i].id!=255) return 0;
    GameBoot_SavegameInitialize(0,GameDifficulty_Hard);
    memcpy(&difficulty,(const u8*)g_SavegamePtr+0x260,4);
    return difficulty==0x10000000u;
}
