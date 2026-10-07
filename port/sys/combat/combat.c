/* SPDX-License-Identifier: GPL-3.0-only */
#include "combat.h"
#include <stdlib.h>
bool sh_combat_dark_environment;
s_AnimInfo* sh_combat_harry_map_anims;
u32 sh_combat_lzc_input;
s32 sh_combat_ps1_div(s32 numerator,s32 denominator) {
    /* PORT: MIPS DIV specifies these results and does not trap like host C. */
    if (!denominator) return numerator<0?1:-1;
    if (numerator==(-2147483647-1) && denominator==-1) return numerator;
    return numerator/denominator;
}
u32 sh_combat_lzc(u32 input) {
    u32 count=0;
    if (!input) return 32;
    while (!(input & 0x80000000u)) {input <<= 1; ++count;}
    return count;
}
s32 sh_combat_npc_index(const s_SubCharacter* character) {
    s32 i;
    /* PORT: Equality is defined even for unrelated pointers. Invalid identities
       must fail visibly, rather than shift by a fabricated/negative index. */
    for (i=0;i<NPC_COUNT_MAX;++i) if (character==&g_SysWork.npcs[i]) return i;
    abort();
}
static u16 half(const u8* p) {return (u16)((u16)p[0]|((u16)p[1]<<8));}
static u32 word(const u8* p) {return (u32)half(p)|((u32)half(p+2)<<16);}
bool sh_combat_attack_decode(const u8* p,size_t length,u32 expected_aux,s_800AD4C8* out,u32* aux) {
    s_800AD4C8 record;
    if (!p || !out || !aux || length<24 || word(p+20)!=expected_aux) return false;
    memset(&record,0,sizeof(record));
    record.field_0=(s16)half(p);record.field_2=(s16)half(p+2);record.field_4=half(p+4);
    record.field_6=(s8)p[6];record.unk_7=(s8)p[7];record.field_8=p[8];record.charaId_9=p[9];
    record.field_A=p[10];record.field_B=p[11];record.field_C=(s16)half(p+12);
    record.field_E=p[14];record.field_F=p[15];record.field_10=p[16];record.field_11=p[17];
    record.field_12=p[18];record.__pad_13=(s8)p[19];record.unk_14=aux;
    *out=record;return true;
}
bool sh_combat_keyframe_decode(const u8* bytes,size_t length,s_Keyframe* out) {
    s16 values[10];size_t i;
    if (!bytes || !out || length<20) return false;
    for (i=0;i<10;++i) values[i]=(s16)half(bytes+i*2);
    out->box.top=values[0];out->box.bottom=values[1];out->box.height=values[2];
    out->box.offsetY=values[3];out->box.field_8=values[4];out->box.field_A=values[5];
    out->shapeOffsets.box.vx=values[6];out->shapeOffsets.box.vz=values[7];
    out->shapeOffsets.cylinder.vx=values[8];out->shapeOffsets.cylinder.vz=values[9];return true;
}
bool sh_combat_sqrt_decode(const u8* bytes,size_t length,s16* table,size_t count) {
    size_t i;
    if(!bytes || !table || count!=192 || length!=384) return false;
    for(i=0;i<count;++i) table[i]=(s16)half(bytes+i*2);
    return true;
}
bool sh_combat_collision_surface_height(const s_IpdCollisionData* collision,u32 index,s16* height) {
    if(!collision || !height || index>=collision->surfaceCount || !collision->surfaces) return false;
    *height=collision->surfaces[index].baseGroundHeight;
    return true;
}
s32 sh_combat_model_probe(const s_LmHeader* lm,const s_AnmHeader* anm) {
    s32 i,j,total=0;
    GsCOORDINATE2 coordinates[256]={{0}};
    if (!lm || !anm || !lm->modelHdrs || !anm->bindPoses || !anm->keyframes) return -1;
    sh_combat_Anim_BoneInit((s_AnmHeader*)anm,coordinates);
    for(i=1;i<anm->boneCount;++i) if(coordinates[i].super!=&coordinates[anm->bindPoses[i].parentBone]) return -1;
    for (i=0;i<lm->modelCount;++i) {
        const s_ModelHeader* model=&lm->modelHdrs[i];
        s_Bone bone={0};
        sh_combat_Bone_ModelAssign(&bone,(s_LmHeader*)lm,i);
        if(bone.modelInfo.modelHdr!=model || bone.modelInfo.modelIdx!=i) return -1;
        for (j=0;j<model->meshCount;++j) {
            const s_MeshHeader* mesh=&model->meshHdrs[j];
            total+=mesh->vertexCount;
            if (mesh->vertexCount && (!mesh->verticesXy || !mesh->verticesZ)) return -1;
        }
    }
    return total;
}
