/* SPDX-License-Identifier: GPL-3.0-only */
#include "gameplay.h"
#include <stdio.h>
s_WorldGfxWork g_WorldGfxWork;
s_LmHeader port_harry_lm;
s8* D_800C15B0;
s8 port_bone_mesh_idx;
s_WorldEnvWork g_WorldEnvWork;
s_LmHeader port_global_lm,port_held_lm,port_npc_lm;
u8 port_map_chunk_storage[0x2C000];
PortActiveCollision g_ActiveCollisionTriggers;
PortAnimSlot g_CharaModelAnimsData[CHARA_GROUP_COUNT];
s32 g_Inventory_EquippedItem,g_Player_CutsceneState,g_Player_LastWeaponSelected;
s16 D_800C4588;
q19_12 g_Player_GrabReleaseInputTimer,D_800C45EC;
bool g_Player_DisableControl;
void SetDrawTPage(DR_TPAGE* packet,int dfe,int dtd,int page) {
    setDrawTPage(packet,dfe,dtd,page);
}
s32 Lzc(s32 value) {u32 result;gte_ldlzc(value);gte_stlzc(&result);return (s32)result;}
static struct {u8 field_0,field_1,field_2,field_3;s16 field_A;} water_work;
void func_8008D41C(void) {water_work.field_0=0;water_work.field_1=0;water_work.field_2=0;water_work.field_A=0;}
// PORT: The boot slice consumes Harry only. Other identities remain unlinked.
const PortCharaFileInfo CHARA_FILE_INFOS[Chara_Count] = {
    [Chara_Harry]={FILE_ANIM_HB_BASE_ANM,FILE_CHARA_HERO_ILM,FILE_CHARA_HERO_TIM,1,0,Q8(-0.7f)}
};
void GsInitCoordinate2(GsCOORDINATE2* parent,GsCOORDINATE2* coord) {
    memset(coord,0,sizeof(*coord));coord->super=parent;
    for(s32 i=0;i<3;i++) coord->coord.m[i][i]=4096;
}
void port_lm_native_check(const s_LmHeader* lm) {
    if(!lm || lm->magic!=LM_HEADER_MAGIC || lm->version!=LM_VERSION || !lm->isLoaded || lm->modelCount>BONE_NODE_COUNT_MAX)
        port_unimplemented("invalid native skeletal model descriptor");
}
// PORT: The fixed-width file-table names expand into bounded native strings.
void Fs_GetFileName(char* destination,s32 file) {
    u32 parts[2]={g_FileTable[file].name0123,g_FileTable[file].name4567};
    u32 i=0;
    for(;i<8;i++) {u32 c=(parts[i/4]>>((i%4)*6))&63;if(!c)break;destination[i]=(char)(0x20+c);}
    destination[i]='.';destination[i+1]=0;
}
int port_native_graph_probe(s_LmHeader* lm,s_AnmHeader* anm,u32* out) {
    GsCOORDINATE2 coords[32]={0};s_Skeleton skel={0};
    if(!lm || !anm || anm->boneCount>32 || lm->modelCount>BONE_NODE_COUNT_MAX) return 0;
    Anim_BoneInit(anm,coords);
    Skeleton_Init(&skel,skel.boneNodes,BONE_NODE_COUNT_MAX);
    func_8004506C(&skel,lm);func_800452EC(&skel);func_800453E8(&skel,true);
    out[0]=anm->boneCount;out[1]=lm->modelCount;out[2]=skel.boneIdx;
    out[3]=0;for(s_BoneNode* n=skel.bones_4;n;n=n->next) out[3]++;
    out[4]=(u32)coords[1].coord.t[0];out[5]=(u32)coords[1].coord.t[1];out[6]=(u32)coords[1].coord.t[2];
    out[7]=(u32)(coords[1].super==&coords[0]);
    return 1;
}
void port_world_boot_note(void) {
    const s_CharaModel* player=&g_WorldGfxWork.harryModel;
    const s_AnmHeader* animation=(const s_AnmHeader*)FS_BUFFER_0;
    if(!player->isLoaded || !player->lmHdr || !player->lmHdr->modelCount ||
       player->skeleton.boneIdx!=player->lmHdr->modelCount || animation->boneCount!=18 ||
       g_ActiveCollisionTriggers.flags!=CollisionTriggerFlag_Map)
        port_unimplemented("world bootstrap native graph validation");
    g_CharaModelAnimsData[0].activeAnmHdr=(s_AnmHeader*)FS_BUFFER_0;
    g_CharaModelAnimsData[0].boneCoords=g_SysWork.playerBoneCoords;
    printf("WORLD_INIT native models=%u bones=%u skeleton_nodes=%u health=%d collision_flags=%u fog_near=%d fog_far=%d\n",
        player->lmHdr->modelCount,animation->boneCount,player->skeleton.boneIdx,
        g_SysWork.playerWork.player.health,g_ActiveCollisionTriggers.flags,
        g_WorldEnvWork.fog.nearDistance,g_WorldEnvWork.fog.farDistance);
}
void port_harry_empty_hand(void) {
    WorldGfx_HarryMeshSwap(&g_WorldGfxWork.harryModel.skeleton,MESH_SWAP_STATUS(HarrySwappableMesh_None,HarryVariantMesh_RightHandEmpty));
}
void port_player_spawn_note(void) {
    const s_SubCharacter* player=&g_SysWork.playerWork.player;
    printf("PLAYER_SPAWN map=%d xyz=(%d,%d,%d) heading=%d camera_heading=%d loading=%d\n",
        g_SavegamePtr->mapIdx,player->position.vx,player->position.vy,player->position.vz,
        player->rotation.vy,g_SysWork.cameraAngleY,g_SysWork.loadingScreenIdx);
}
// PORT: Startup stops the four weapon gas SFX in the original func_8008B398.
// The native sound task bridge already accepts these stop commands.
void func_8008B398(void) {
    g_SysWork.field_275C=0;g_SysWork.field_2760=0;g_SysWork.field_2764=0;
    for(u16 sound=1300;sound<=1303;sound++) Sd_SfxStop(sound);
}
// PORT: The current native sound task bridge logs SFX requests. Keep stop
// requests separate from SD_Call play commands until SFX voice linkage exists.
void Sd_SfxStop(u16 sound) { printf("SFX_STOP %u (native SFX voice linkage pending)\n",sound); }
// PORT: No-draw bridge while the parallel world lane links these render services.
void Screen_BackgroundMotionBlur(s32 mode) {(void)mode;}
void AreaLoad_UpdatePlayerPosition(void) {port_unimplemented("AreaLoad_UpdatePlayerPosition");}
void Gfx_LoadScreenMapEffectsUpdate(s32 first,s32 second) {
    // PORT: Loading render effects do not block the original startup dispatcher.
    (void)first;(void)second;
}
// PORT: No-draw bridge; environment/gameplay initialization remains guarded separately.
void Gfx_EffectsUpdate(void) {}
void WorldGfx_CharaDraw(e_CharaId id,GsCOORDINATE2* coords,s32 shift,q3_12 timer,s32 palette) {
    // PORT: No-draw bridge for the world lane; animation still runs in original C.
    (void)id;(void)coords;(void)shift;(void)timer;(void)palette;
}
int port_animation_sample(s_AnmHeader* anm,s32 first,s32 second,q19_12 alpha,s32 bone,s32* out) {
    GsCOORDINATE2 coords[32]={0};
    if(!anm || bone<=0 || bone>=anm->boneCount || anm->boneCount>32)return 0;
    Anim_BoneInit(anm,coords);Anim_BoneUpdate(anm,coords,first,second,alpha);
    for(s32 i=0;i<3;i++)out[i]=coords[bone].coord.t[i];
    for(s32 i=0;i<9;i++)out[i+3]=coords[bone].coord.m[i/3][i%3];
    return 1;
}
