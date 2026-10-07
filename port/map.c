/* SPDX-License-Identifier: GPL-3.0-only */
#include "map.h"
#include "gameplay.h"
#include <stdio.h>
static const s_MapOverlayHdr* active_map;
static s32 active_map_index=-1;
int port_map_activate(u32 file) {
    if(file!=FILE_VIN_MAP0_S00_BIN) return 1;
    sh_map0_s00_reset();
    if(sh_map0_s00_load_data()) return 1;
    active_map=sh_map0_s00_descriptor();active_map_index=0;
    printf("NATIVE MAP descriptor=%d tag=%.4s events=72 spawn=(%d,%d) reset=16 callbacks=guarded\n",
        active_map_index,active_map->mapInfo->tag,active_map->mapPoints[0].positionX,active_map->mapPoints[0].positionZ);
    return 0;
}
const s_MapOverlayHdr* port_map_active(void) {
    if(!active_map) port_unimplemented("map descriptor not active");
    return active_map;
}
int port_map_zero(const void* data,size_t size) {
    const u8* bytes=data;
    for(size_t i=0;i<size;i++) if(bytes[i]) return 0;
    return 1;
}
// PORT: Preserve the original held-item no-weapon path, guarding actual model
// attachment until that native consumer is linked.
s32 WorldGfx_PlayerPrevHeldItem(s_PlayerCombat* combat) {
    if(combat->weaponAttack!=NO_VALUE || g_WorldGfxWork.heldItem.itemId!=NO_VALUE)
        port_unimplemented("WorldGfx_PlayerPrevHeldItem/held-item change");
    return 0;
}
void Gfx_PlayerHeldItemAttach(u8 attack) {
    if(attack!=255) port_unimplemented("Gfx_PlayerHeldItemAttach/equipped weapon");
    port_harry_empty_hand();
}
void Map_EffectTexturesLoad(s32 map) {
    if(map==NO_VALUE) {
        // PORT: The original atlas destination is below a fixed FONT24 address.
        // A native file buffer owns those bytes while retaining its VRAM image.
        s_FsImageDesc blood={{0,11},0,0,304,0};
        Fs_QueueStartReadTim(FILE_TIM_BLD_TIM,port_fs_buffers[2],&blood);
        return;
    }
    if(map!=MapIdx_MAP0_S00) port_unimplemented("Map_EffectTexturesLoad/native effect texture records");
    // PORT: MAP0_S00 selects EffectTextureFlag_None in the original switch.
}
// PORT: Probe the same named native pointers used by chunk/collision consumers.
// Serialized bytes never participate in these mutations.
int port_native_map_probe(s_IpdHeader* map,u32* output) {
    if(!map || map->magic!=IPD_HEADER_MAGIC || !map->isLoaded || !map->lmHdr) return 0;
    output[0]=map->modelCount;output[1]=map->modelBufferCount;
    output[2]=0;output[3]=0;output[8]=0;
    for(u32 i=0;i<map->modelBufferCount;i++) {
        const s_IpdModelBuffer* buffer=&map->modelBuffers[i];
        output[2]+=buffer->modelInstanceCount;
        for(u32 j=0;j<buffer->modelInstanceCount;j++) {
            const s_IpdModelInstance* instance=&buffer->modelInstances[j];
            if(!instance->modelHdr) continue;
            output[8]++;
            output[3]+=(u32)instance->mat.t[0];
        }
    }
    output[4]=map->collisionData.surfaceCount;output[5]=map->collisionData.subcellCount;
    output[6]=(u32)map->collisionData.positionX;
    map->collisionData.subcellCheckIdxs[0]=73;
    output[7]=map->collisionData.subcellCheckIdxs[0];
    return 1;
}
void port_map_layout_probe(u32* output) {
    output[0]=sizeof(s_MapOverlayHdr);output[1]=offsetof(s_MapOverlayHdr,mapPoints);
    output[2]=offsetof(s_MapOverlayHdr,charaUpdateFuncs);output[3]=offsetof(s_MapOverlayHdr,cameraPaths);
    output[4]=offsetof(s_MapOverlayHdr,collisionTriggers);
}
