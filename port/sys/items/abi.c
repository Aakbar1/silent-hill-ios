/* SPDX-License-Identifier: GPL-3.0-only */
#include "abi.h"
#include "game.h"
#include "bodyprog/bodyprog.h"
#include "bodyprog/memcard.h"
#include "bodyprog/item_screens.h"
#include "screens/credits/credits.h"
#include <stdlib.h>
_Static_assert(sizeof(s_InventoryItem)==4,"original inventory");
_Static_assert(sizeof(s_Savegame)==636,"original save payload");
_Static_assert(offsetof(s_Savegame,eventFlags)==0x168,"event flags");
_Static_assert(offsetof(s_Savegame,playerHealth)==0x240,"health");
_Static_assert(offsetof(s_Savegame,continueCount)==0x27b,"save tail");
_Static_assert(sizeof(s_MemCard_SaveMetadata)==12,"metadata");
_Static_assert(sizeof(s_Savegame_Container)==640,"PS1 container, never native slot");
_Static_assert(sizeof(s_Savegame_OptionsConfig)==128,"PS1 options container");
_Static_assert(sizeof(s_MemCard_SaveHeader)==256,"PS1 header");
_Static_assert(sizeof(s_SaveScreenElement)==24,"native save row");
_Static_assert(offsetof(s_SaveScreenElement,saveMetadata)==16,"native metadata pointer");
_Static_assert(sizeof(s_800AEDBC)==24,"native marking record");
_Static_assert(sizeof(s_CreditTextState)==40,"native credit state");
_Static_assert(sizeof(s_CreditText3dState)==104,"native 3d credit state");
_Static_assert(offsetof(s_CreditTextState,widthTable)==16,"native width pointer");
_Static_assert(offsetof(s_CreditTextState,colorTable)==24,"native color pointer");
_Static_assert(sizeof(s_TmdFile)==24,"native TMD header");
_Static_assert(sizeof(struct TMD_STRUCT)==48,"native TMD object");
_Static_assert(offsetof(struct TMD_STRUCT,nortop)==16,"TMD normal pointer");
_Static_assert(offsetof(struct TMD_STRUCT,primtop)==32,"TMD primitive pointer");
void sh_items_tmd_prepared(s_TmdFile* header) {
    /* PORT: TMD wire offsets already decode to owned native pointers. Repeating
     * PsyQ's in-place 32-bit relocation would overwrite the native graph. */
    if(!header || header->id!=0x41 || header->flags!=1 || header->modelCount<0 || header->modelCount>1024 || (header->modelCount && !header->models))abort();
}
static uint16_t le16(const uint8_t* p) { return (uint16_t)((uint16_t)p[0]|((uint16_t)p[1]<<8)); }
static uint32_t le32(const uint8_t* p) { return (uint32_t)le16(p)|((uint32_t)le16(p+2)<<16); }
int sh_items_piano_decode(const uint8_t* bytes,size_t length,ShItemsPianoNative* out) {
    ShItemsPianoNative temp;
    if (!bytes || !out || length!=17) return 2;
    for (int i=0;i<11;i++) {
        temp.thresholds[i]=(int8_t)bytes[i];
        if (i && temp.thresholds[i]<=temp.thresholds[i-1]) return 2;
    }
    for (int i=0;i<5;i++) {
        uint8_t key=bytes[12+i];
        if (key!=1 && key!=2 && key!=7 && key!=9 && key!=10) return 2;
        temp.sequence[i]=(int8_t)key;
    }
    *out=temp;
    return 0;
}
int sh_items_point_decode(const uint8_t* bytes,size_t length,ShItemsPointNative* out) {
    if (!bytes || !out || length!=4) return 2;
    out->x=(int16_t)le16(bytes); out->y=(int16_t)le16(bytes+2); return 0;
}
int sh_items_save_layout_probe(const uint8_t* bytes,size_t length) {
    s_Savegame native;
    uint8_t encoded[636];
    if (!bytes || length!=636) return 2;
    /* PORT: all supported hosts are little endian; probe original bitfield
     * allocation before publishing native save pointers to core. */
    memcpy(&native,bytes,sizeof(native));
    if ((uint32_t)native.playerHealth!=le32(bytes+0x240) || native.items[39].field_3!=bytes[159] ||
        native.isNextFearMode!=(bytes[0x25c]&1) || native.add290Hours!=((bytes[0x25c]>>1)&3) ||
        native.pickedUpSpecialItemCount!=(bytes[0x25c]>>3) || native.field_260!=(le32(bytes+0x260)&0xfffffff)) return 2;
    { int expected=(int)(le32(bytes+0x260)>>28); if (expected&8) expected-=16; if (native.gameDifficulty!=expected) return 2; }
    memcpy(encoded,&native,sizeof(native));
    return memcmp(encoded,bytes,sizeof(encoded)) ? 2 : 0;
}
int sh_items_slot_id(int device,int file,int save,uint32_t* out) {
    if (!out || (device!=0 && device!=4) || file<0 || file>=15 || save<0 || save>=11) return 2;
    *out=(uint32_t)((device/4)*165+file*11+save); return 0;
}
