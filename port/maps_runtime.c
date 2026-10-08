/* SPDX-License-Identifier: GPL-3.0-only */
#include "maps_registry.h"
#include "world.h"
#include "render_services.h"
#include <stdio.h>
static const s_MapOverlayHdr* active;
_Static_assert(FILE_VIN_MAP7_S03_BIN-FILE_VIN_MAP0_S00_BIN+1==43,"native map file inventory");
int port_map_activate(u32 file) {
    if(file<FILE_VIN_MAP0_S00_BIN || file>FILE_VIN_MAP7_S03_BIN)return 1;
    u32 index=file-FILE_VIN_MAP0_S00_BIN;
    const PortMapEntry* entry=&port_maps[index];
    // PORT: Invalidate first so a failed data load cannot expose stale state.
    active=NULL;
    port_maps_ground_reset();
    entry->reset();
    if(entry->load_data())return 1;
    active=entry->descriptor();
    printf("NATIVE MAP name=%s index=%u tag=%.4s reset=complete-slice\n",entry->name,index,active->mapInfo->tag);
    return 0;
}
const s_MapOverlayHdr* port_map_active(void) {
    if(!active)port_unimplemented("map descriptor not active");
    return active;
}
int port_maps_reset_probe(void) {
    if(!port_maps_ground_reset_probe())return 200;
    for(u32 i=0;i<43;i++) {
        if(!port_maps[i].reset_probe() || !port_maps[i].point_count())return (int)i+1;
        const s_MapOverlayHdr* descriptor=port_maps[i].descriptor();
        for(u32 j=0;j<i;j++) {
            const s_MapOverlayHdr* previous=port_maps[j].descriptor();
            if(descriptor==previous || descriptor->mapPoints==previous->mapPoints ||
               descriptor->mapEvents==previous->mapEvents ||
               descriptor->harryMapAnimInfos==previous->harryMapAnimInfos)return 100+(int)i;
        }
    }
    active=NULL;
    return 0;
}
void port_maps_warp(u32 map,u32 spawn) {
    // PORT: Test-only warp exercises queued activation and the real map init.
    if(map>=43 || spawn>255)port_unimplemented("debug warp arguments");
    // PORT: Direct test entry establishes the same nonzero virtual timestep
    // normally supplied by MainLoop before camera integration.
    g_DeltaTime=g_DeltaTimeRaw=TIMESTEP_60_FPS;
    GameBoot_SavegameInitialize((s8)map,GameDifficulty_Normal);
    InitGeom();Screen_Refresh(320,false);vwInitViewInfo();
    World_Init();
    Fs_QueueInitialize();
    Fs_QueueStartRead((s32)(FILE_VIN_MAP0_S00_BIN+map),port_overlay_dynamic);
    Fs_QueueUpdate();
    const s_MapOverlayHdr* descriptor=port_map_active();
    printf("MAP_WARP selected=%s spawn=%u\n",port_maps[map].name,spawn);
    fflush(stdout);
    if(spawn>=port_maps[map].point_count())port_unimplemented("debug warp spawn outside map points");
    descriptor->initWorldObjects();
    printf("MAP_WARP init=%s\n",port_maps[map].name);fflush(stdout);
    Chara_PositionSet(&descriptor->mapPoints[spawn]);
    q19_12 x=g_SysWork.playerWork.player.position.vx,z=g_SysWork.playerWork.player.position.vz;
    WorldGfx_MapInit((s_MapOverlayHdr*)descriptor,x,z);
    printf("MAP_WARP world=%s\n",port_maps[map].name);fflush(stdout);
    s32 iteration=0;
    for(;iteration<256;iteration++) {
        Fs_QueueUpdate();WorldMap_ChunkInit(x,z,x,z);
        if(!Fs_QueueGetLength() && WorldMap_ActiveModelsLoadStateCheck())break;
    }
    if(iteration==256)port_unimplemented("debug warp streaming readiness timeout");
    vcInitCamera((s_MapOverlayHdr*)descriptor,&g_SysWork.playerWork.player.position);
    vcSetCameraUseWarp(&g_SysWork.playerWork.player.position,g_SysWork.cameraAngleY);
    vcMoveAndSetCamera(true,false,false,false,false,false,false,false);
    WorldEnv_MapPresetSet((s_MapOverlayHdr*)descriptor);port_render_effects();
    GsClearOt(0,0,&g_OrderingTable0[g_ActiveBufferIdx]);
    GsOUT_PACKET_P=(PACKET*)port_packets[g_ActiveBufferIdx];
    port_draw_env(0,0,320,224,160,112);port_clear_vram(0,0,320,224,0,0,0);
    WorldGfx_Draw(1);GsDrawOt(&g_OrderingTable0[g_ActiveBufferIdx]);
    // PORT: Setup may present a screen-clear frame. Rust verifies this final
    // world frame separately and retains any backend error or cancellation.
    (void)port_present(0,0,320,224,11,0,0);
    printf("MAP_WARP rendered=%s queue=%d\n",port_maps[map].name,Fs_QueueGetLength());
}
