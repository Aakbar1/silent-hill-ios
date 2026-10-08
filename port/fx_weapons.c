/* SPDX-License-Identifier: GPL-3.0-only */
#include "fx_weapons.h"
#include <stdio.h>

void port_fight_weapon_frames(s32 file) {
    // PORT: Headerless weapon fragments occupy FS_BUFFER_12..FS_BUFFER_4
    // inside the existing stable HB_BASE native frame allocation. Never decode
    // a fragment as an ANM header or overwrite a map's later frame block.
    enum { WEAPON_OFFSET=0x15bb4, MAP_OFFSET=0x19d84 };
    s_AnmHeader* anm=g_CharaModelAnimsData[0].activeAnmHdr;
    u32 bytes=(u32)Fs_GetFileSize(file);
    if(!anm || !anm->keyframes || !anm->keyframeDataSize ||
       WEAPON_OFFSET<anm->dataOffset || bytes<anm->keyframeDataSize ||
       bytes>MAP_OFFSET-WEAPON_OFFSET ||
       (WEAPON_OFFSET-anm->dataOffset)%anm->keyframeDataSize)
        port_unimplemented("weapon fragment native arena bounds");
    u8 fragment[MAP_OFFSET-WEAPON_OFFSET];
    // PORT: Use the bounded numeric-leaf reader. This is headerless frame data,
    // not raw publication of a serialized pointer-bearing asset.
    if(port_map_data_read((u32)file,0,bytes,fragment))port_unimplemented("weapon fragment owned read");
    u32 complete=bytes/anm->keyframeDataSize*anm->keyframeDataSize;
    u32 start=WEAPON_OFFSET-anm->dataOffset;
    // PORT: Rust reserves the entire original player arena before publishing
    // this pointer; C owns fragment writes on the sole game worker.
    u8* destination=(u8*)anm->keyframes+start;
    memset(destination,0,MAP_OFFSET-WEAPON_OFFSET);
    memcpy(destination,fragment,complete);
    u32 end=(start+complete)/anm->keyframeDataSize;
    if(end>anm->keyframeCount)anm->keyframeCount=(u16)end;
    printf("FIGHT_WEAPON file=%d frames=%u start=%u end=%u\n",file,complete/anm->keyframeDataSize,start/anm->keyframeDataSize,end);
}
