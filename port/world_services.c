/* SPDX-License-Identifier: GPL-3.0-only */
#include "world.h"
s16* port_world_grid(s_WorldMapWork* work,s32 x,s32 z) {
    // PORT: Original center-relative negative indexing becomes a checked native
    // 19x16 array index; it never steps outside a C subobject.
    if(x < -8 || x >= 8 || z < -8 || z >= 11)port_unimplemented("world chunk grid bounds");
    return &work->chunksGrid[z+8].idx[x+8];
}
void port_world_lm_check(s_LmHeader* lm) {
    if(!lm || lm->magic!=LM_HEADER_MAGIC || lm->version!=LM_VERSION ||
       (lm->modelCount && !lm->modelHdrs) || (lm->materialCount && !lm->materials))
        port_unimplemented("native world LM descriptor");
    // PORT: AssetStore already resolves every leaf. Complete original logical
    // initialization without relocating or mutating immutable serialized bytes.
    lm->isLoaded=true;
}
void port_world_ipd_check(s_IpdHeader* ipd) {
    if(!ipd || !ipd->lmHdr || (ipd->modelCount && !ipd->modelInfos) ||
       (ipd->modelBufferCount && !ipd->modelBuffers))port_unimplemented("native world IPD descriptor");
}
extern int port_water_texture_name(char* destination);
void func_8008E4EC(s_LmHeader* lm) {
    char name[8];s_FsImageDesc image={{0,13},0,0,0,0};
    // PORT: Upstream's INCLUDE_RODATA filename is bounded scalar data, decoded
    // at runtime from the player's encrypted BODYPROG. No game bytes in Git.
    if(port_water_texture_name(name))port_unimplemented("water texture filename read");
    Lm_MaterialFsImageApply1(lm,name,&image,1);
}
