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

// PORT: Native sidecars for the original bounded object list and common item LM.
PortWorldObject port_world_objects[29];
s_LmHeader port_world_item_lm;
s32 port_world_item_queue=-1;
void port_world_common_items_load(void) {
    port_world_item_queue=Fs_QueueStartRead(FILE_BG_BG_ITEM_PLM,&port_world_item_lm);
}
int port_world_registration_probe(void) {
    // PORT: Exercise the original nonempty registration path without assets.
    // This runs inside the single existing native-global fixture and restores it.
    PortWorldObject saved[29];memcpy(saved,port_world_objects,sizeof(saved));
    s32 savedCount=g_WorldGfxWork.objectCount;
    s_WorldObjectModel model={0};
    port_world_object_name_set(&model,"PROBEOBJ");
    model.metadata.modelLocation=WorldModelLocation_Global;
    VECTOR3 pos={-25395,0,657408};SVECTOR3 rot={-20,1000,12};
    WorldObjects_Clear(&g_WorldGfxWork);
    port_world_object_add(&model,&pos,&rot);
    port_world_object_add(&model,&pos,&rot);
    int ok=g_WorldGfxWork.objectCount==1 &&
        port_world_objects[0].model==&model && port_world_objects[0].positionX==-1588 &&
        port_world_objects[0].positionZ==41088 && port_world_objects[0].rotationX==-5 &&
        port_world_objects[0].rotationY==1000 && port_world_objects[0].rotationZ==3;
    for(s32 i=1;i<=29;i++) {pos.vx=i*4096;port_world_object_add(&model,&pos,&rot);}
    ok=ok && g_WorldGfxWork.objectCount==29;
    WorldObjects_Clear(&g_WorldGfxWork);ok=ok && g_WorldGfxWork.objectCount==0;
    memcpy(port_world_objects,saved,sizeof(saved));g_WorldGfxWork.objectCount=savedCount;
    return ok;
}
