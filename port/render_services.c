/* SPDX-License-Identifier: GPL-3.0-only */
#include "render_services.h"
#include "render_gte.h"
#include <stdio.h>
u32 port_render_world_models;
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
static void project_world(s_MeshHeader* mesh,s32 offset,s_GteScratchData* scratch,MATRIX* view,const s_WorldEnvWork* env) {
    SetRotMatrix(view);SetTransMatrix(view);
    for(u32 base=0;base<mesh->vertexCount;base+=3) {
        SVECTOR points[3]={{0}};s32 xy[3],depth[3];
        u32 count=MIN(3u,(u32)mesh->vertexCount-base);
        for(u32 i=0;i<count;i++) {
            u32 index=(u32)offset+base+i;
            points[i]=(SVECTOR){scratch->screenXy_0[index].vx,scratch->screenXy_0[index].vy,scratch->field_18C[index],0};
        }
        gte_ldv3c(points);gte_rtpt();gte_stsxy3(&xy[0],&xy[1],&xy[2]);gte_stsz3(&depth[0],&depth[1],&depth[2]);
        for(u32 i=0;i<count;i++) {
            u32 index=(u32)offset+base+i;
            port_gte_store32(&scratch->screenXy_0[index],(u32)xy[i]);
            scratch->field_18C[index]=(s16)(u16)depth[i];
            if(env->isFogEnabled) {
                u32 limit=1u << env->fog.depthShift;
                scratch->field_252[index]=(u8)((u32)depth[i]<limit ?
                    env->fogRamp[((u32)depth[i]*128)/limit] : 255);
            }
        }
    }
}
void port_render_project_world(s_MeshHeader* mesh,s32 offset,s_GteScratchData* scratch,MATRIX* view) {
    project_world(mesh,offset,scratch,view,&g_WorldEnvWork);
}
int port_render_world_projection_probe(void) {
    // PORT: Two live vertices in the final triple, nonzero arena offset,
    // behind-camera and far depths, and untouched adjacent slots.
    s_GteScratchData scratch;memset(&scratch,0xA5,sizeof(scratch));
    s_WorldEnvWork env={0};env.isFogEnabled=1;env.fog.depthShift=12;
    for(u32 i=0;i<128;i++)env.fogRamp[i]=(u8)i;
    s_MeshHeader mesh={0};mesh.vertexCount=5;
    const SVECTOR points[5]={{0,0,0,0},{64,-64,128,0},{0,0,4096,0},{0,0,-256,0},{0,0,32700,0}};
    for(u32 i=0;i<5;i++) {
        scratch.screenXy_0[250+i]=(DVECTOR){points[i].vx,points[i].vy};
        scratch.field_18C[250+i]=points[i].vz;
    }
    MATRIX matrix={{{4096,0,0},{0,4096,0},{0,0,4096}},{0,0,128}};
    InitGeom();SetGeomScreen(128);SetGeomOffset(0,0);
    project_world(&mesh,250,&scratch,&matrix,&env);
    return port_gte_load32(&scratch.screenXy_0[250])==0 &&
        port_gte_load32(&scratch.screenXy_0[251])==0xffe00020u &&
        scratch.field_18C[250]==128 && scratch.field_18C[251]==256 &&
        scratch.field_252[250]==4 && scratch.field_252[251]==8 &&
        scratch.field_252[252]==255 && scratch.field_18C[253]==0 &&
        scratch.field_252[253]==0 && scratch.field_252[254]==255 &&
        port_gte_load32(&scratch.screenXy_0[255])==0xa5a5a5a5u &&
        (u16)scratch.field_18C[255]==0xa5a5 && scratch.field_252[255]==0xa5;
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
void OuterProduct12(VECTOR* a,VECTOR* b,VECTOR* result) {
    // PORT: Original SDK outer product retains the GTE's fixed-point rounding.
    gte_OuterProduct12(a,b,result);
}
// PORT: Native packet tag uses the token-based ordering-table bridge.
void SetPriority(void* packet,s32 check_mask,s32 set_mask) {
    u32* words=packet;words[0]=0x02000000;
    words[1]=0xE6000000u | (u32)(set_mask!=0) | ((u32)(check_mask!=0)<<1);
    words[2]=0;
}
void port_render_priority_probe(u32* words) {
    SetPriority(words,0,1);
    SetPriority(words+3,1,1);
    SetPriority(words+6,0,0);
}
void port_render_street_snapshot(s32* out) {
    out[0]=g_SavegamePtr->mapIdx;
    out[1]=g_SysWork.playerWork.player.moveSpeed;
    out[2]=(s32)g_Controller0->buttonFlags.held;
    out[3]=g_SysWork.playerWork.player.rotation.vy;
    out[4]=g_WorldEnvWork.fog.color.r;out[5]=g_WorldEnvWork.fog.color.g;
    out[6]=g_WorldEnvWork.fog.color.b;out[7]=(s32)port_render_world_models;
}

void port_render_world_objects(s_WorldGfxWork* work) {
    WorldObjects_DrawAllObjects(work);
}
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
    // PORT: Retain the original framebuffer gap so the static capture preserves CLUT rows.
    port_draw_env(0,32,320,224,160,144);
    port_clear_vram(0,32,320,224,0,0,0);
    port_render_character(Chara_Harry,g_SysWork.playerBoneCoords,1,0,0);
    WorldGfx_Draw(1);
    GsDrawOt(&g_OrderingTable0[g_ActiveBufferIdx]);
    printf("FIRST_MAP_VIEW original world draw map=%d fog=%d near=%d far=%d rgb=(%u,%u,%u) queue=%d startup_guard_retained=1\n",
        g_SavegamePtr->mapIdx,g_WorldEnvWork.isFogEnabled,g_WorldEnvWork.fog.nearDistance,
        g_WorldEnvWork.fog.farDistance,g_WorldEnvWork.fog.color.r,g_WorldEnvWork.fog.color.g,
        g_WorldEnvWork.fog.color.b,Fs_QueueGetLength());
    if(port_present(0,32,320,224,g_GameWork.gameState,g_GameWork.gameStateSteps[0],0))
        port_unimplemented("first map rendering presentation");
}

void port_render_draw_move(DR_MOVE* packet,const RECT* rect,s32 x,s32 y) {
    // PORT: SDK VRAM-copy packet, four GP0 words after the ordering-table tag.
    packet->tag=0x04000000;
    packet->code[0]=0x80000000;
    packet->code[1]=(u32)(u16)rect->x | ((u32)(u16)rect->y<<16);
    packet->code[2]=(u32)(u16)x | ((u32)(u16)y<<16);
    packet->code[3]=(u32)(u16)rect->w | ((u32)(u16)rect->h<<16);
}
