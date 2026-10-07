/* SPDX-License-Identifier: GPL-3.0-only */
#include "render_services.h"
#include "render_gte.h"
#include <stdio.h>
void port_render_model_check(const s_ModelHeader* model) {
    if(!model || (model->meshCount && !model->meshHdrs))port_unimplemented("native render model");
    for(u32 i=0;i<model->meshCount;i++) {
        const s_MeshHeader* mesh=&model->meshHdrs[i];
        if((u32)model->vertexOffset+mesh->vertexCount>256 ||
           (u32)model->normalOffset+mesh->unkCount_3>256 ||
           (u32)model->normalOffset+mesh->normalCount>256) {
            printf("RENDER_CAPACITY mode=%u vertex=%u+%u normal=%u+%u unknown=%u\n",model->field_B_0,
                model->vertexOffset,mesh->vertexCount,model->normalOffset,mesh->normalCount,mesh->unkCount_3);
            port_unimplemented("native render scratch capacity");
        }
    }
}
void port_render_project(s_MeshHeader* mesh,s32 offset,s_GteScratchData* scratch,MATRIX* view) {
    SetRotMatrix(view);SetTransMatrix(view);
    for(u32 base=0;base<mesh->vertexCount;base+=3) {
        SVECTOR points[3]={{0}};s32 xy[3];s32 depth[3];
        u32 count=MIN(3u,(u32)mesh->vertexCount-base);
        for(u32 i=0;i<count;i++)points[i]=(SVECTOR){mesh->verticesXy[base+i].vx,mesh->verticesXy[base+i].vy,mesh->verticesZ[base+i],0};
        gte_ldv3c(points);gte_rtpt();gte_stsxy3(&xy[0],&xy[1],&xy[2]);gte_stsz3(&depth[0],&depth[1],&depth[2]);
        for(u32 i=0;i<count;i++) {
            port_gte_store32(&scratch->screenXy_0[(u32)offset+base+i],(u32)xy[i]);
            scratch->screenZ_168[(u32)offset+base+i]=(u16)depth[i];
        }
    }
}
u8 port_render_normals(s_MeshHeader* mesh,s32 offset,s_GteScratchData2* scratch) {
    CVECTOR color={0};gte_ldrgb(&color);
    for(u32 base=0;base<mesh->normalCount;base+=3) {
        SVECTOR normals[3]={{0}};u32 colors[3];
        u32 count=MIN(3u,(u32)mesh->normalCount-base);
        for(u32 i=0;i<count;i++) {
            const s_Normal* n=&mesh->normals[base+i];
            normals[i]=(SVECTOR){(s16)(n->nx*32),(s16)(n->ny*32),(s16)(n->nz*32),0};
        }
        gte_ldv3c(normals);gte_nct();gte_strgb3(&colors[0],&colors[1],&colors[2]);
        for(u32 i=0;i<count;i++)scratch->field_21C[(u32)offset+base+i]=(s32)colors[i];
    }
    return 0;
}

int port_render_leaf_probe(u32* out) {
    // PORT: Synthetic exact-size leaves exercise final triples without any disc.
    DVECTOR xy[2]={{0,0},{64,-64}};s16 z[2]={0,128};s_Normal normal={127,0,0,1};
    s_MeshHeader mesh={0};mesh.vertexCount=2;mesh.verticesXy=xy;mesh.verticesZ=z;
    mesh.normalCount=1;mesh.normals=&normal;
    s_GteScratchData scratch;memset(&scratch,0xA5,sizeof(scratch));
    s_GteScratchData2 lights;memset(&lights,0xA5,sizeof(lights));
    MATRIX matrix={{{4096,0,0},{0,4096,0},{0,0,4096}},{0,0,128}};
    InitGeom();SetGeomScreen(128);SetGeomOffset(0,0);
    port_render_project(&mesh,0,&scratch,&matrix);
    out[0]=port_gte_load32(&scratch.screenXy_0[0]);out[1]=port_gte_load32(&scratch.screenXy_0[1]);
    out[2]=scratch.screenZ_168[0];out[3]=scratch.screenZ_168[1];
    out[4]=port_gte_load32(&scratch.screenXy_0[2]);out[5]=scratch.screenZ_168[2];
    SetLightMatrix(&matrix);SetColorMatrix(&matrix);SetBackColor(0,0,0);
    port_render_normals(&mesh,0,&lights);out[6]=(u32)lights.field_21C[0];out[7]=(u32)lights.field_21C[1];
    MATRIX custom={{{1,2,3},{4,5,6},{7,8,9}},{0,0,0}};
    port_render_rot_custom(&custom);
    for(u32 i=0;i<5;i++)out[8+i]=port_gte_read_control(i);
    return 1;
}
void NormalColor3(SVECTOR* a,SVECTOR* b,SVECTOR* c,CVECTOR* x,CVECTOR* y,CVECTOR* z) {
    // PORT: Original PsyQ triple lighting operation via the native GTE bridge.
    gte_NormalColor3(a,b,c,x,y,z);
}
void port_render_average_col(const u8* first,const u8* second,s32 a,s32 b,u8* result) {
    // PORT: PsyQ's three-byte vector interpolation has read-only inputs,
    // despite its public declaration lacking const. Retain its IR rounding.
    u8 c0[3]={first[0],first[1],first[2]},c1[3]={second[0],second[1],second[2]};
    gte_LoadAverageCol(c0,c1,a,b,result);
}
void port_render_light_tint(u8 r,u8 g,u8 b,u8 tr,u8 tg,u8 tb) {
    (void)r;(void)g;(void)b;(void)tr;(void)tg;(void)tb;
    port_unimplemented("volumetric light tint parameters");
}
// PORT: Native packet tag uses the token-based ordering-table bridge.
void SetPriority(void* packet,s32 set_mask,s32 check_mask) {
    u32* words=packet;words[0]=0x02000000;
    words[1]=0xE6000000u | (u32)(set_mask!=0) | ((u32)(check_mask!=0)<<1);
    words[2]=0;
}

// PORT: Until native world-object registration is shared with the player lane,
// fail closed for nonempty lists rather than silently dropping geometry.
void port_render_world_objects(s_WorldGfxWork* work) {
    if(work->objectCount)port_unimplemented("native world object registration");
}
void port_render_held_item(void) {
    if(g_WorldGfxWork.heldItem.itemId!=NO_VALUE)
        port_unimplemented("native held-item rendering");
}
void func_80041074(GsOT* ot,q19_12 intensity,const SVECTOR* dir,const VECTOR3* pos) {
    (void)ot;(void)intensity;(void)dir;(void)pos;
    port_unimplemented("volumetric light overlay rendering");
}
void func_8008D470(q3_12 intensity,const SVECTOR* dir,const VECTOR3* pos,s_WaterZone* zones) {
    (void)intensity;(void)dir;(void)pos;(void)zones;
    port_unimplemented("lens-flare rendering");
}
void func_8003E740(void) {port_unimplemented("lighter flame rendering");}

void port_render_first_map(void) {
    // PORT: A static rendering milestone while the player lane owns startup.
    // The original New Game/movie/loading path has already produced the player,
    // bones, textures and camera. Only world streaming/drawing executes here.
    if(g_SavegamePtr->mapIdx!=MapIdx_MAP0_S00 || !g_WorldGfxWork.harryModel.isLoaded)
        port_unimplemented("first map rendering scene identity");
    s_SubCharacter* player=&g_SysWork.playerWork.player;
    // PORT: Use the original first-map camera setup from InGameInit; the
    // loading-screen user camera is a close-up of the running character.
    vcInitCamera((s_MapOverlayHdr*)&g_MapOverlayHdr,&player->position);
    vcSetCameraUseWarp(&player->position,g_SysWork.cameraAngleY);
    vcMoveAndSetCamera(true,false,false,false,false,false,false,false);
    WorldGfx_MapInit((s_MapOverlayHdr*)&g_MapOverlayHdr,player->position.vx,player->position.vz);
    s32 iteration=0;
    for(;iteration<256;iteration++) {
        Fs_QueueUpdate();WorldGfx_CloseRangeChunksInit();
        if(!Fs_QueueGetLength() && WorldMap_ActiveModelsLoadStateCheck())break;
    }
    if(iteration==256)port_unimplemented("first map rendering readiness timeout");
    WorldEnv_MapPresetSet((s_MapOverlayHdr*)&g_MapOverlayHdr);port_render_effects();
    GsClearOt(0,0,&g_OrderingTable0[g_ActiveBufferIdx]);
    GsOUT_PACKET_P=(PACKET*)port_packets[g_ActiveBufferIdx];
    port_draw_env(0,0,320,224,160,112);
    port_clear_vram(0,0,320,224,0,0,0);
    port_render_character(Chara_Harry,g_SysWork.playerBoneCoords,1,0,0);
    WorldGfx_Draw(1);
    GsDrawOt(&g_OrderingTable0[g_ActiveBufferIdx]);
    printf("FIRST_MAP_VIEW original world draw map=%d fog=%d near=%d far=%d rgb=(%u,%u,%u) queue=%d startup_guard_retained=1\n",
        g_SavegamePtr->mapIdx,g_WorldEnvWork.isFogEnabled,g_WorldEnvWork.fog.nearDistance,
        g_WorldEnvWork.fog.farDistance,g_WorldEnvWork.fog.color.r,g_WorldEnvWork.fog.color.g,
        g_WorldEnvWork.fog.color.b,Fs_QueueGetLength());
    if(port_present(0,0,320,224,g_GameWork.gameState,g_GameWork.gameStateSteps[0],0))
        port_unimplemented("first map rendering presentation");
}
