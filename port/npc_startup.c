/* SPDX-License-Identifier: GPL-3.0-only */
#include "npc_startup.h"

// PORT: Each animation group has a stable native descriptor, independent of
// PS1 arena offsets. AssetStore owns its decoded leaves until the worker exits.
static s_AnmHeader npc_animations[CHARA_GROUP_COUNT];
s_LmHeader port_npc_models[CHARA_GROUP_COUNT];
void Fs_CharaAnimDataAlloc(s32 slot,e_CharaId id,s_AnmHeader* anm,GsCOORDINATE2* coords) {
    if(id==Chara_None)return;
    if(slot<=0 || slot>=CHARA_GROUP_COUNT || id<=Chara_None || id>=Chara_Count)
        port_unimplemented("NPC animation allocation identity");
    if(!CHARA_FILE_INFOS[id].animFileIdx)
        port_unimplemented("NPC character file table not migrated");
    PortAnimSlot* dst=&g_CharaModelAnimsData[slot];
    if(dst->activeCharaId>Chara_None && dst->activeCharaId<Chara_Count)g_CharaAnimDataIdxs[dst->activeCharaId]=NO_VALUE;
    dst->activeCharaId=id;g_CharaAnimDataIdxs[id]=(s8)slot;
    dst->allocAddr=anm?anm:&npc_animations[slot];
    dst->activeAnmHdr=dst->allocAddr;
    dst->boneCoords=coords?coords:g_SysWork.npcBoneCoordBuffer;
    dst->allocSize=Fs_GetFileSize(CHARA_FILE_INFOS[id].animFileIdx);
    dst->activeSize=dst->allocSize;
    Fs_QueueStartRead(CHARA_FILE_INFOS[id].animFileIdx,dst->activeAnmHdr);
}
void WorldGfx_CharaLmBufferAdvance(u8** buffer,s32 id) {
    (void)id;
    // PORT: Advance typed native descriptor slots instead of serialized file lengths.
    for(s32 slot=0;slot<CHARA_GROUP_COUNT;slot++)if(*buffer==(u8*)&port_npc_models[slot]) {
        *buffer=(u8*)&port_npc_models[slot+1];return;
    }
    port_unimplemented("NPC native model slot capacity");
}
void port_move_npc_animation_ready(s_AnmHeader* animation) {
    for(s32 slot=1;slot<CHARA_GROUP_COUNT;slot++)if(g_CharaModelAnimsData[slot].activeAnmHdr==animation) {
        GsCOORDINATE2* coords=g_CharaModelAnimsData[slot].boneCoords;
        s32 available=0;
        for(s32 i=0;i<NPC_BONE_COUNT_MAX;i++)if(coords==&g_SysWork.npcBoneCoordBuffer[i])available=NPC_BONE_COUNT_MAX-i;
        if(!available || animation->boneCount>available)port_unimplemented("NPC native coordinate capacity");
        Anim_BoneInit(animation,coords);return;
    }
}
// These guards are logic dependencies, never silent substitutes for game code.
void Bgm_Update(bool update) {(void)update;port_unimplemented("Bgm_Update/original layer controller");}
// PORT: The existing native libsd task sink is silent and has no MIDI channels.
// Report its actual idle state; this does not establish BGM playback.
u16 Sd_MidiChannelTaskGet(void) {return 0;}
