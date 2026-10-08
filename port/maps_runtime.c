/* SPDX-License-Identifier: GPL-3.0-only */
#include "maps_registry.h"
#include "world.h"
#include "render_services.h"
#include "npc_startup.h"
#ifdef SH_NATIVE_AUDIO
#include "audio_records.h"
#endif
#include <stdio.h>
// PORT: Original shared numeric boss state; never a serialized pointer view.
s_800C4418 D_800C4418;
// Original screen_data.c ground-image VRAM destination, not texture bytes.
s_FsImageDesc g_LoadingScreenImg={{0,5},0,16,288,0};
// PORT: Original finale's shared scalar; no credits/cutscene consumer is substituted.
u8 g_EndingIdx;
static const s_MapOverlayHdr* active;
static bool debug_light;
static bool debug_warp;
static bool debug_walk,debug_walk_started;
static u32 debug_walk_map,debug_walk_spawn;
extern int sh_main(void);
void port_maps_debug_walk_set(u32 enabled) {debug_walk=enabled!=0;debug_walk_started=false;}
s32 port_maps_boot_map(s32 map) {
    if(debug_walk && !debug_walk_started) {
        g_SavegamePtr->mapIdx=(u8)debug_walk_map;
        return (s32)debug_walk_map;
    }
    return map;
}
extern s_MapPoint2d g_MapPoint;
void port_maps_boot_spawn(void) {
    if(!debug_walk || debug_walk_started)return;
    const PortMapEntry* entry=&port_maps[debug_walk_map];
    if(port_map_active()!=entry->descriptor() || debug_walk_spawn>=entry->point_count())
        port_unimplemented("walking replay initial map/point identity");
    // PORT: One explicit diagnostic warp before first gameplay; original
    // Chara_PositionSet establishes pose/room/camera. Nothing is injected later.
    g_MapPoint=entry->descriptor()->mapPoints[debug_walk_spawn];
    AreaLoad_UpdatePlayerPosition();debug_walk_started=true;
    printf("TRANSIT_WALK_START tick=%d map=%s spawn=%u\n",VSync(-1),entry->name,debug_walk_spawn);
    fflush(stdout);
}
void port_maps_init_note(const char* stage) {if(debug_warp){printf("MAP_INIT %s\n",stage);fflush(stdout);}}
void port_maps_object_sfx(s32 task) {
    // PORT: Original init SFX requires a published VAB header. Never treat
    // a missing bank as silent success or dereference its native null pointer.
    if(task>=1281 && task<=1791) {
#ifdef SH_NATIVE_AUDIO
        s32 bank=g_Vab_InfoTable[task-1280].vabProgIdx>>8;
        if(bank<0 || bank>=SD_VAB_SLOTS || !vab_h[bank].vh_addr_4)
            port_unimplemented("map object SFX/VAB header not loaded");
#else
        port_unimplemented("map object SFX/native audio bridge not configured");
#endif
    }
    SD_Call(task);
}
extern u32 port_maps_frame_detail(void);
// PORT: Explicit opt-in lighting fixture for isolated warp diagnostics only.
void port_maps_debug_light_set(u32 enabled) {debug_light=enabled!=0;}
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
static const PortMapEntry* active_entry(void) {
    for(u32 i=0;i<43;i++)if(active==port_maps[i].descriptor())return &port_maps[i];
    port_unimplemented("active map registry identity");return NULL;
}
u32 port_maps_event_count(void) {return active_entry()->event_count();}
void port_maps_callback_validate(u32 index) {
    if(index>=active_entry()->callback_count() || !active->mapEventFuncs[index])
        port_unimplemented("map event callback index outside source bounds");
}
void port_maps_transition_validate(const s_EventData* event) {
    const PortMapEntry* entry=active_entry();
    bool found=false;
    for(u32 i=0;i<entry->event_count();i++)if(event==&active->mapEvents[i])found=true;
    if(!found || event->eventParam>=entry->point_count() || event->mapPointIdx>=entry->point_count() || event->sfxPairIdx_8_19>=25)
        port_unimplemented("area transition source event/point/SFX bounds");
    if(g_SysWork.sysState==SysState_LoadOverlay && event->mapIdx>=43)
        port_unimplemented("area transition destination map bounds");
    printf("AREA_TRANSITION tick=%d map=%s room=%d sys=%d destination=%u point=%u trigger=%u flags=%u\n",
        VSync(-1),entry->name,g_SavegamePtr->mapRoomIdx,g_SysWork.sysState,event->mapIdx,event->eventParam,event->mapPointIdx,event->transitionFlags);
    fflush(stdout);
}
int port_maps_reset_probe(void) {
    if(!port_maps_ground_reset_probe())return 200;
    for(u32 i=0;i<43;i++) {
        if(!port_maps[i].reset_probe() || !port_maps[i].point_count() || !port_maps[i].event_count() || !port_maps[i].callback_count())return (int)i+1;
        const s_MapOverlayHdr* descriptor=port_maps[i].descriptor();
        if(descriptor->mapEvents[port_maps[i].event_count()-1].triggerType!=TriggerType_EndOfArray)return 300+(int)i;
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
    if(debug_walk) {
        if(map>=43 || spawn>255)port_unimplemented("walking replay initial arguments");
        debug_walk_map=map;debug_walk_spawn=spawn;
        // PORT: The original boot/main loop owns assets, banks, input and all
        // startup services; the only test override is the initial map/point.
        (void)sh_main();return;
    }
    debug_warp=true;
    // PORT: Test-only warp exercises queued activation and the real map init.
    if(map>=43 || spawn>255)port_unimplemented("debug warp arguments");
    // PORT: Direct test entry establishes the same nonzero virtual timestep
    // normally supplied by MainLoop before camera integration.
    g_DeltaTime=g_DeltaTimeRaw=TIMESTEP_60_FPS;
    GameBoot_SavegameInitialize((s8)map,GameDifficulty_Normal);
    InitGeom();Screen_Refresh(320,false);
    // PORT: Original init callbacks can issue SFX tasks even with host audio off.
    // Establish the real sound driver before settings or object initialization.
    SpuInit();SD_Init();Settings_RestoreDefaults();vwInitViewInfo();
    World_Init();
    Fs_QueueInitialize();
    Fs_QueueStartRead((s32)(FILE_VIN_MAP0_S00_BIN+map),port_overlay_dynamic);
    Fs_QueueUpdate();
    const s_MapOverlayHdr* descriptor=port_map_active();
    printf("MAP_WARP selected=%s spawn=%u\n",port_maps[map].name,spawn);
    fflush(stdout);
    if(spawn>=port_maps[map].point_count())port_unimplemented("debug warp spawn outside map points");
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
    printf("MAP_WARP ready=%s\n",port_maps[map].name);fflush(stdout);
    vcInitCamera((s_MapOverlayHdr*)descriptor,&g_SysWork.playerWork.player.position);
    vcSetCameraUseWarp(&g_SysWork.playerWork.player.position,g_SysWork.cameraAngleY);
    vcMoveAndSetCamera(true,false,false,false,false,false,false,false);
    printf("MAP_WARP camera=%s\n",port_maps[map].name);fflush(stdout);
    // PORT: Match production setup order: camera and environment exist before
    // init callbacks can invoke Gfx_MapEnvSet. Some original descriptors have
    // no init callback; the original func_8005E650 also checks this pointer.
    {
        // PORT: A diagnostic loading pose supplies the spotlight coordinate.
        // This does not load Harry, grant an item, or certify gameplay lighting.
        GsCOORDINATE2* root=&g_SysWork.playerBoneCoords[HarryBone_Root];
        GsCOORDINATE2* torso=&g_SysWork.playerBoneCoords[HarryBone_Torso];
        GsInitCoordinate2(NULL,root);GsInitCoordinate2(root,torso);
        SVECTOR heading;Math_SVectorSet(&heading,0,-g_SysWork.cameraAngleY,0);
        Math_RotMatrixZxyNeg(&heading,&root->coord);
        root->coord.t[0]=Q12_TO_Q8(x);root->coord.t[1]=0;root->coord.t[2]=Q12_TO_Q8(z);
        Game_FlashlightAttributesFix();
    }
    WorldEnv_MapPresetSet((s_MapOverlayHdr*)descriptor);
    printf("MAP_WARP preset=%s\n",port_maps[map].name);fflush(stdout);
    if(debug_light)Game_TurnFlashlightOn();
    func_8005E650((s32)map);
    Fs_QueueWaitForEmpty();
    printf("MAP_WARP init=%s\n",port_maps[map].name);fflush(stdout);
    for(s32 i=0;i<(debug_light?16:1);i++)port_render_effects();
    printf("MAP_WARP lighting=on:%d fade:%d intensity:%d position:%d,%d,%d mode:%d ambient:%d brightness:%d\n",
        (int)g_SysWork.field_2388.isFlashlightOn,(int)g_SysWork.gameplayEnvironment.flashlightIntensity,
        g_WorldEnvWork.light.intensity,g_WorldEnvWork.light.position.vx,g_WorldEnvWork.light.position.vy,
        g_WorldEnvWork.light.position.vz,(int)g_WorldEnvWork.field_0,g_WorldEnvWork.field_20,g_WorldEnvWork.screenBrightness);
    GsClearOt(0,0,&g_OrderingTable0[g_ActiveBufferIdx]);
    GsOUT_PACKET_P=(PACKET*)port_packets[g_ActiveBufferIdx];
    port_draw_env(0,0,320,224,160,112);port_clear_vram(0,0,320,224,0,0,0);
    WorldGfx_Draw(1);GsDrawOt(&g_OrderingTable0[g_ActiveBufferIdx]);
    // PORT: Setup may present a screen-clear frame. Rust verifies this final
    // world frame separately and retains any backend error or cancellation.
    (void)port_present(0,0,320,224,11,0,0);
    u32 detail=port_maps_frame_detail();
    printf("MAP_WARP geometry=%u detail=%u flashlight_fixture=%d\n",port_render_world_models,detail,(int)debug_light);
    printf("MAP_WARP rendered=%s queue=%d\n",port_maps[map].name,Fs_QueueGetLength());
    if(!port_render_world_models)port_unimplemented("debug warp/no world mesh at original spawn");
    // PORT: At least 64 native pixels must differ from the dominant color;
    // a screen rectangle plus a single lens-flare pixel is not a visible world.
    if(detail<64)port_unimplemented("debug warp/under 64 varied pixels despite world meshes");
}

int port_maps_fixed_coord_probe(void) {
    MATRIX matrix;memset(&matrix,0xa5,sizeof(matrix));
    Vw_CoordHierarchyMatrixCompute(NULL,&matrix);
    return memcmp(&matrix,&GsIDMATRIX,sizeof(matrix))==0 && matrix.m[0][0]==4096 && matrix.t[2]==0;
}
