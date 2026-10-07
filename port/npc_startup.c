/* SPDX-License-Identifier: GPL-3.0-only */
#include "npc_startup.h"

// PORT: Each animation group has a stable native descriptor, independent of
// PS1 arena offsets. AssetStore owns its decoded leaves until the worker exits.
static s_AnmHeader npc_animations[CHARA_GROUP_COUNT];
void Fs_CharaAnimDataAlloc(s32 slot,e_CharaId id,s_AnmHeader* anm,GsCOORDINATE2* coords) {
    if(id==Chara_None)return;
    if(slot<=0 || slot>=CHARA_GROUP_COUNT || id<=Chara_None || id>=Chara_Count)
        port_unimplemented("NPC animation allocation identity");
    if(!CHARA_FILE_INFOS[id].animFileIdx)
        port_unimplemented("NPC character file table not migrated");
    PortAnimSlot* dst=&g_CharaModelAnimsData[slot];
    dst->allocAddr=anm?anm:&npc_animations[slot];
    dst->activeAnmHdr=dst->allocAddr;
    dst->boneCoords=coords?coords:g_SysWork.npcBoneCoordBuffer;
    Fs_QueueStartRead(CHARA_FILE_INFOS[id].animFileIdx,dst->activeAnmHdr);
}
s32 WorldGfx_CharaModelLoad(s32 id,s32 slot,s_LmHeader* lm,s_FsImageDesc* image) {
    (void)id;(void)slot;(void)lm;(void)image;
    port_unimplemented("NPC native model ownership not migrated");return 0;
}
void WorldGfx_CharaLmBufferAdvance(u8** buffer,s32 id) {
    (void)buffer;(void)id;port_unimplemented("NPC native model slot selection");
}
// These guards are logic dependencies, never silent substitutes for game code.
void Bgm_Update(bool update) {(void)update;port_unimplemented("Bgm_Update/original layer controller");}
// PORT: The existing native libsd task sink is silent and has no MIDI channels.
// Report its actual idle state; this does not establish BGM playback.
u16 Sd_MidiChannelTaskGet(void) {return 0;}
