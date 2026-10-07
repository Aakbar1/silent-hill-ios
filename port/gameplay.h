/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef SH_GAMEPLAY_H
#define SH_GAMEPLAY_H
#include "boot.h"
#define ARRAY_SIZE(a) (sizeof(a)/sizeof((a)[0]))
enum {BoneHierarchy_End=-2,BoneHierarchy_MultiModel=-3,MaterialFlag_None=0,MaterialFlag_0=1,MaterialFlag_1=2,MaterialFlag_2=4};
typedef struct {s32 animFileIdx,modelFileIdx,textureFileIdx,materialBlendMode;} PortCharaFileInfo;
extern const PortCharaFileInfo CHARA_FILE_INFOS[Chara_Count];
typedef struct {s32 itemId,queueIdx;char* textureName;s_FsImageDesc imageDesc;s_LmHeader* lmHdr;s_Bone bone;} s_HeldItem;
// PORT: Bootstrap's native work contains owned model slots and empty map caches.
// Draw/chunk/trigger consumers remain guarded until their integration is complete.
typedef struct {s_CharaModel harryModel; s_CharaModel* registeredCharaModels[Chara_Count];
    s_CharaModel charaModels[CHARA_GROUP_COUNT];s_HeldItem heldItem;
    u8* charaLmBuffer;bool useStoredPoint;s32 objectCount;} s_WorldGfxWork;
#include "native_environment.h"
extern s_WorldEnvWork g_WorldEnvWork;
extern s_WorldGfxWork g_WorldGfxWork;
extern s_LmHeader port_harry_lm;
extern s_LmHeader port_global_lm,port_held_lm,port_npc_lm;
extern u8 port_map_chunk_storage[0x2C000];
#define GLOBAL_LM_BUFFER (&port_global_lm)
#define HELD_ITEM_LM_BUFFER (&port_held_lm)
#define MAP_CHARA_LM_BUFFER ((u8*)&port_npc_lm)
#define IPD_BUFFER ((void*)port_map_chunk_storage)
#define HARRY_LM_BUFFER (&port_harry_lm)
extern s8* D_800C15B0;
extern s8 port_bone_mesh_idx;
void GsInitCoordinate2(GsCOORDINATE2* parent,GsCOORDINATE2* coord);
void port_lm_native_check(const s_LmHeader* lm);
void Anim_BoneInit(s_AnmHeader* anm,GsCOORDINATE2* coords);
void Chara_FsImageCalc(s_FsImageDesc* image,s32 charaId,s32 modelIdx);
void WorldGfx_PlayerModelProcessLoad(void);
void WorldGfx_CharaModelProcessLoad(s_CharaModel* model);
void Skeleton_Init(s_Skeleton* skel,s_BoneNode* nodes,u8 count);
void func_80045014(s_Skeleton* skel);
void func_8004506C(s_Skeleton* skel,s_LmHeader* lm);
void func_80045108(s_Skeleton* skel,s_LmHeader* lm,s8* hierarchy,bool append);
void Skeleton_BoneModelAssign(s_Skeleton* skel,s_LmHeader* lm,s8* hierarchy);
void func_80045258(s_BoneNode** order,s_BoneNode* nodes,s32 count,s_LmHeader* lm);
void func_800452EC(s_Skeleton* skel);
void func_800453E8(s_Skeleton* skel,bool visible);
s8 Bone_ModelIdxGet(s8* hierarchy,bool reset);
void Bone_ModelAssign(s_Bone* bone,s_LmHeader* lm,s32 modelIdx);
s32 LmHeader_ModelCountGet(s_LmHeader* lm);
void Fs_GetFileName(char* destination,s32 file);
void Lm_MaterialFileIdxApply(s_LmHeader* lm,e_FsFile file,s_FsImageDesc* image,s32 blend);
bool Lm_MaterialFsImageApply(s_LmHeader* lm,char* name,s_FsImageDesc* image,s32 blend);
void Material_FsImageApply(s_Material* mat,s_FsImageDesc* image,s32 blend);
void Lm_MaterialFlagsApply(s_LmHeader* lm);
void Model_MaterialFlagsApply(s_ModelHeader* model,s32 index,const s_Material* mat,s32 flags);
enum {UnkGfxEnum_0=0,CollisionTriggerFlag_Map=1,HarryBone_Root=0,HarryBone_Torso=1,
    ItemToggleFlag_FlashlightOff=2,InvItemGroup_MeleeWeapons=4,InvItemGroup_GunWeapons=5,
    PlayerCutsceneState_RunForward=0,
    GameDifficulty_Normal=0};
#define DEFAULT_PLAYER_CYLINDER_FIELD_2 Q12(0.23f)
#define INV_ITEM_GROUP(id) ((id)>>5)
#define Math_Vector3Set(p,x,y,z) ((p)->vx=(x),(p)->vy=(y),(p)->vz=(z))
#define Math_SVectorSet(p,x,y,z) ((p)->vx=(s16)(x),(p)->vy=(s16)(y),(p)->vz=(s16)(z))
#define SVECTOR(x,y,z) {Q12_ANGLE(x),Q12_ANGLE(y),Q12_ANGLE(z),0}
#define SetPolyG3 setPolyG3
#define SetPolyG4 setPolyG4
#define SetPolyF4 setPolyF4
typedef struct {q3_12 positionY,field_2,field_4;s16 field_6,field_8,pad;SVECTOR field_C,position;} s_800AE204;
typedef struct {s_AnmHeader* allocAddr,*activeAnmHdr;s32 allocSize,activeSize;GsCOORDINATE2* boneCoords;} PortAnimSlot;
extern PortAnimSlot g_CharaModelAnimsData[CHARA_GROUP_COUNT];
extern s32 g_Inventory_EquippedItem,g_Player_CutsceneState,g_Player_LastWeaponSelected;
extern s16 D_800C4588;
extern q19_12 g_Player_GrabReleaseInputTimer,D_800C45EC;
extern bool g_Player_DisableControl;
typedef struct {u16 flags;s32 collisionTriggerCount;} PortActiveCollision;
extern PortActiveCollision g_ActiveCollisionTriggers;
void Collision_Init(void);
void Collision_FlagsSet(u16 flags);
void World_Init(void);
void WorldEnv_Init(void);
void WorldEnv_FogDistanceSet(q19_12 near_distance,q19_12 far_distance);
void func_80040BAC(void);
void func_8008D41C(void);
void func_8005B55C(GsCOORDINATE2* coord);
void Game_FlashlightAttributesFix(void);
void WorldObjects_Clear(s_WorldGfxWork* work);
void WorldGfx_HeldItemModelFree(void);
void WorldGfx_CharaModelsFree(void);
void Chara_ModelFree(s_CharaModel* model);
void WorldMap_Init(s_LmHeader* lm,void* storage,s32 budget);
void Game_TurnFlashlightOn(void);
void Game_TurnFlashlightOff(void);
void Game_PlayerInfoInit(void);
void SysWork_SavegameReadPlayer(void);
GsCOORDINATE2* vwGetViewCoord(void);
s32 Lzc(s32 value);
void port_world_boot_note(void);
void port_harry_empty_hand(void);
void WorldGfx_HarryMeshSwap(s_Skeleton* skeleton,s32 status);
void func_80045468(s_Skeleton* skeleton,s32* indices,bool visible);
void func_8007E9C4(void);
void func_8004C564(u8 arg0,s8 attack);
void func_8008B398(void);
void port_player_spawn_note(void);
void Sd_SfxStop(u16 sound);
void Screen_BackgroundMotionBlur(s32 mode);
#endif
