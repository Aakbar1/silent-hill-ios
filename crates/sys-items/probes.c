/* SPDX-License-Identifier: GPL-3.0-only */
/* Test fixture only. Core must provide live identities/services; these mocks
 * never ship in the production library. Unexpected paths fail immediately. */
#include "game.h"
#include "bodyprog/bodyprog.h"
#include "bodyprog/item_screens.h"
#include "bodyprog/memcard.h"
#include "bodyprog/math/math.h"
#include "bodyprog/ranking.h"
#include "stf_roll_namespace.h"
#include "screens/credits/credits.h"
#include "saveload_namespace.h"
#include "screens/saveload.h"
#include "map1_s01_items_namespace.h"
#include "maps/map1/map1_s01.h"
#include "abi.h"
extern void sh_saveload_reset(void);
extern int sh_saveload_reset_probe(void), sh_stf_roll_reset_probe(void);
s_GameWork g_GameWork;
s_GameWork* const g_GameWorkPtr=&g_GameWork;
s_GameWork* const g_GameWorkConst=&g_GameWork;
s_Savegame* const g_SavegamePtr=&g_GameWork.savegame;
s_SysWork g_SysWork;
s_ControllerData* const g_Controller0=&g_GameWork.controllers[0];
s32 g_DeltaTimeRaw, g_DeltaTime;
s8 g_PianoKeys[11],g_PianoKeySequence[5];
q19_12 g_PianoCursorX,g_PianoCursorY;
s32 g_PianoKeyCounter,D_800DD594;
u32 g_Inventory_SelectionId;
s32 g_Inventory_SelectedItemIdx;
u8 g_Inventory_EquippedItem;
u8 D_800C3E40;
s32 g_Inventory_SelectionBordersDraw;
/* Imported state supplied by the original event dispatcher. */
s32 g_ItemTriggerItemIds[5];
static int flashOn,cursorCalls,noteCalls;
void Game_TurnFlashlightOn(void) { flashOn++; }
bool Math_Distance2dCheck(const VECTOR3* a,const VECTOR3* b,q19_12 distance) { (void)a;(void)b;(void)distance; return true; }
void Event_WaitTimer(q19_12 time,bool unused) { (void)time;(void)unused; g_SysWork.sysStateSteps[1]++; }
void SD_Call(u32 id) { (void)id; noteCalls++; }
void Gfx_CursorDraw(s32 x,s16 y,s32 x1,s16 y1,s16 u,s16 v,s16 width,s32 height,s32 tint,u32 clutX,s16 clutY,s32 page) {
    (void)x;(void)y;(void)x1;(void)y1;(void)u;(void)v;(void)width;(void)height;(void)tint;(void)clutX;(void)clutY;(void)page;cursorCalls++;
}
void sh_items_probe_reset(void) {
    memset(&g_GameWork,0,sizeof(g_GameWork)); memset(&g_SysWork,0,sizeof(g_SysWork));
    for (int i=0;i<40;i++) g_SavegamePtr->items[i].id=255;
    g_SavegamePtr->invSlotCount=8;
    g_GameWork.config.controllerConfig.enter=1;
    g_GameWork.config.controllerConfig.cancel=2;
    g_ItemTriggerItemIds[0]=InvItemId_HouseKey;g_ItemTriggerItemIds[1]=-1;
    flashOn=0;cursorCalls=0;noteCalls=0;
    sh_items_save_service_reset();sh_saveload_reset();
}
int sh_items_probe_inventory(void) {
    sh_items_probe_reset();
    Inventory_AddSpecialItem(InvItemId_HouseKey,1);
    if(g_SavegamePtr->items[0].id!=InvItemId_HouseKey || g_SavegamePtr->items[0].count!=1) return 1;
    Inventory_AddSpecialItem(InvItemId_HealthDrink,60);Inventory_AddSpecialItem(InvItemId_HealthDrink,60);
    if(g_SavegamePtr->items[0].id!=InvItemId_HealthDrink || g_SavegamePtr->items[0].count!=100) return 2;
    for(int i=0;i<40;i++)if(g_SavegamePtr->items[i].id==InvItemId_HouseKey) {
        Inventory_ItemUse(i);
        if(g_SysWork.playerWork.extra.lastUsedItem!=InvItemId_HouseKey || flashOn!=1 || g_GameWork.gameStateSteps[1]!=11) return 3;
    }
    if(!Player_ItemRemove(InvItemId_HouseKey,1) || Player_ItemRemove(InvItemId_HouseKey,1)) return 4;
    return 0;
}
int sh_items_probe_piano(const ShItemsPianoNative* tables,int wrong) {
    sh_items_probe_reset();memcpy(g_PianoKeys,tables->thresholds,11);memcpy(g_PianoKeySequence,tables->sequence,5);
    PianoPuzzle_Control(false);
    for(int i=0;i<5;i++) {
        int key=wrong ? 0 : tables->sequence[i];
        int x=key==0 ? tables->thresholds[0]-1 : (tables->thresholds[key-1]+tables->thresholds[key])/2;
        g_PianoCursorX=x*4096;g_PianoCursorY=0;g_SysWork.sysStateSteps[1]=0;
        g_Controller0->buttonFlags.clicked=1;PianoPuzzle_Control(true);
    }
    g_Controller0->buttonFlags.clicked=0;g_SysWork.sysStateSteps[1]=0;PianoPuzzle_Control(true);
    return ((Savegame_EventFlagGet(EventFlag_M1S01_PianoPuzzleSolved)!=0)==!wrong && cursorCalls==(wrong ? 6 : 5) && noteCalls==5) ? 0 : 1;
}
int sh_items_probe_save(uint8_t* bytes,size_t length,int device,int file,int save,int fail) {
    if(length!=636)return 2;
    sh_items_probe_reset();memcpy(g_SavegamePtr,bytes,636);
    g_SelectedDeviceId=(s8)device;g_SelectedFileIdx=(s8)file;g_Savegame_SelectedElementIdx=(u8)save;
    SaveScreen_SaveGame(); /* queue data */
    sh_items_save_service_tick();
    if(fail) {
        SaveScreen_SaveGame();
        return MemCard_LastMemCardResultGet()==MemCardWorkResult_FileIoError && g_GameWork.gameStateSteps[0]==1 ? 0 : 1;
    }
    SaveScreen_SaveGame(); /* queue settings */
    sh_items_save_service_tick();SaveScreen_SaveGame();
    if(MemCard_LastMemCardResultGet()!=MemCardWorkResult_FileIoComplete)return 3;
    memcpy(bytes,g_SavegamePtr,636);
    if(memcmp(&g_GameWork.autosave,g_SavegamePtr,636))return 4;
    memset(g_SavegamePtr,0,636);g_GameWork.gameStateSteps[1]=0;
    SaveScreen_LoadSave();sh_items_save_service_tick();SaveScreen_LoadSave();
    sh_items_save_service_tick();SaveScreen_LoadSave();
    if(MemCard_LastMemCardResultGet()!=MemCardWorkResult_FileIoComplete || memcmp(bytes,g_SavegamePtr,636))return 5;
    g_GameWork.gameState=GameState_LoadSavegameScreen;MemCard_SysDisable();
    if(!MemCard_ElementsUpdate())return 6;
    { int found=0; s_SaveScreenElement* rows=sh_items_save_elements((uint32_t)(device/4));
      for(int i=0;i<g_Savegame_ElementCount1[device/4];i++)if(rows[i].deviceId==device && rows[i].fileIdx==file && rows[i].elementIdx==save && rows[i].type==SavegameEntryType_Save) {
          if(rows[i].saveMetadata!=MemCard_SaveMetadataGet(device,file,save))return 7;
          found=1;
      }
      if(!found)return 8;
    }
    return 0;
}
int sh_items_probe_overlays(void) { return sh_saveload_reset_probe() && sh_stf_roll_reset_probe() ? 0 : 1; }
int sh_items_probe_aliases(void) {
    memset(sh_items_lights,0,sizeof(sh_items_lights));memset(sh_items_models,0,sizeof(sh_items_models));memset(sh_items_slot_indices,0,sizeof(sh_items_slot_indices));
    D_800C3E18[7]=123;D_800C3E18[9]=456;
    if(g_Inventory_EquippedItemIdx!=123 || __pad_bss_800C3E38[1]!=456)return 1;
    g_Items_Lights[9][1].r=99;if(D_800C3AC8[1].r!=99)return 2;
    D_800C3A88[3].g=88;if(g_Items_Lights[8][1].g!=88)return 3;
    g_Items_ItemsModelData[9].id=77;if(D_800C3E08.id!=77)return 4;
    return 0;
}
int sh_items_probe_radio_notes(void) {
    sh_items_probe_reset();g_SavegamePtr->items[0].id=InvItemId_PocketRadio;g_SavegamePtr->items[1].id=InvItemId_Flashlight;g_SavegamePtr->items[2].id=InvItemId_NoteToSchool;
    g_SavegamePtr->mapIdx=MapIdx_MAP2_S00;g_SavegamePtr->itemToggleFlags=0xfedcba98;
    func_8004EF48();
    if(g_SavegamePtr->items[0].command!=InvCmdId_OnOff || g_SavegamePtr->items[1].command!=InvCmdId_OnOff || g_SavegamePtr->items[2].command!=InvCmdId_Look)return 1;
    g_SavegamePtr->mapIdx=MapIdx_MAP5_S00;func_8004EF48();if(g_SavegamePtr->items[0].command!=InvCmdId_Unk10)return 2;
    g_SavegamePtr->mapIdx=MapIdx_MAP6_S03;func_8004EF48();if(g_SavegamePtr->items[0].command!=InvCmdId_Unk10 || g_SavegamePtr->itemToggleFlags!=0xfedcba98)return 3;
    return 0;
}
int sh_items_probe_endings(void) {
    sh_items_probe_reset();g_SavegamePtr->clearGameCount=99;g_SavegamePtr->clearGameEndings=GameEndingFlag_Good|GameEndingFlag_Bad;g_SavegamePtr->currentEndingFlags=GameEndingFlag_Bad;
    Ranking_EvaluateScore();Ranking_PrepareSavegame();
    return g_SavegamePtr->clearGameCount==99 && g_SavegamePtr->clearGameEndings==(GameEndingFlag_Good|GameEndingFlag_Bad) && g_SavegamePtr->isNextFearMode==1 ? 0 : 1;
}
int sh_items_probe_ending_table(const uint8_t* bytes,size_t length) {
    return length==36 && memcmp(bytes,D_801E5558,36)==0 ? 0 : 1;
}
int sh_items_probe_tmd(s_TmdFile* header) {
    sh_items_tmd_prepared(header);
    for(int i=0;i<header->modelCount;i++) {
        struct TMD_STRUCT* model=&header->models[i];
        if(model->vern && !model->vertop)return 1;
        if(model->norn && !model->nortop)return 2;
        if(model->primn && !model->primtop)return 3;
        if(model->vern){volatile u32 word=model->vertop[(model->vern*2)-1];(void)word;}
        if(model->norn){volatile u32 word=model->nortop[(model->norn*2)-1];(void)word;}
    }
    return 0;
}
