/* SPDX-License-Identifier: GPL-3.0-only */
/* PORT: Test-only services. This file MUST NOT be linked by core. */
#include "combat_ai.h"
#include <math.h>
PortGameWork g_GameWork;
PortSysWork g_SysWork;
q19_12 g_DeltaTime;
s_AnimInfo D_800297B8[100];
s16 SQRT[192];
u32 sh_combat_chara_group_flags[4];
s_AnimInfo STALKER_ANIM_INFOS[100], CAT_ANIM_INFOS[10],PARASITE_ANIM_INFOS[10],FLAUROS_ANIM_INFOS[10];
q19_12 sharedData_800E3A20_0_s00,sharedData_800E3A24_0_s00,sharedData_800E3A28_0_s00,sharedData_800E3A2C_0_s00;
static s32 random_value=2048,ray_slot,ray_calls,damage_effects,kill_counts[2],sound_calls,stat_calls;
q19_12 Rng_RandQ12(void) {return random_value;}
s32 Rng_Rand16(void) {return 0;}
/* PORT: libm atan/sqrt are test doubles, never production dependencies. Tests
   requiring approximation fidelity must use the real SDK provider in core. */
s32 SquareRoot0(s32 x) {return x>0?(s32)sqrt((double)x):0;}
s32 SquareRoot12(s32 x) {return x>0?(s32)sqrt((double)x*4096.0):0;}
s32 ratan2(s32 y,s32 x) {return (s32)(atan2((double)y,(double)x)*4096.0/6.283185307179586);}
s32 Math_Vector2MagCalc(s32 x,s32 y) {return SquareRoot0(x*x+y*y);}
s32 Math_Vector2MagCalcSafeQ6(s32 x,s32 z) {return SquareRoot0((x>>6)*(x>>6)+(z>>6)*(z>>6))*64;}
s32 Math_AngleNormalizeSigned(s32 angle) {return ((angle+2048)&4095)-2048;}
s32 Math_AngleBetweenPositionsGet(VECTOR3 from,VECTOR3 to) {return ratan2(to.vx-from.vx,to.vz-from.vz);}
q19_12 Math_Distance2dGet(const VECTOR3* from,const VECTOR3* to) {return Math_Vector2MagCalcSafeQ6(to->vx-from->vx,to->vz-from->vz);}
s32 func_80080540(s32 x,s32 y,s32 z) {return (s32)(((s64)x*x+(s64)y*y+(s64)z*z)>>12);}
q19_12 func_8007FD2C(void) {return Q12(1.0f);}
s_AnimInfo* func_80044918(s_ModelAnim* anim) {
    if (anim->mapAnimInfos && anim->status>=anim->mapAnimStatusStart) return &anim->mapAnimInfos[anim->status-anim->mapAnimStatusStart];
    return &anim->baseAnimInfos[anim->status];
}
s32 Game_HyperBlasterBeamColorGet(void) {return 0;}
void func_8005F6B0(s_SubCharacter* target,VECTOR3* pos,s32 kind,s32 group) {(void)target;(void)pos;(void)kind;(void)group;++damage_effects;}
void func_800892A4(s32 index) {(void)index;++stat_calls;}
void func_80089314(s32 value) {(void)value;}
void func_8009151C(s32 index,s32 hit,q19_12 value) {(void)index;(void)hit;(void)value;}
s32 func_8009146C(s32 kind) {return kill_counts[kind];}
void func_800914C4(s32 kind,s32 count) {kill_counts[kind]=count;}
void Sfx_WithFlagsPlay(u16 id,VECTOR3* pos,q23_8 volume,s32 flags) {(void)id;(void)pos;(void)volume;(void)flags;++sound_calls;}
void Sfx_WithFlagsAndPitchPlay(u16 id,VECTOR3* pos,q23_8 volume,s32 flags,s32 pitch) {(void)pitch;Sfx_WithFlagsPlay(id,pos,volume,flags);}
void Sfx_WithPitchPlay(u16 id,VECTOR3* pos,q23_8 volume,s32 pitch) {Sfx_WithFlagsPlay(id,pos,volume,pitch);}
void Sd_SfxStop(u16 id) {(void)id;}
bool Ray_TraceQuery(s_RayTrace* trace,const VECTOR3* from,const VECTOR3* to) {(void)trace;(void)from;(void)to;return false;}
bool Ray_CharaTraceQuery(s_RayTrace* trace,const VECTOR3* from,VECTOR3* offset,s_SubCharacter* exclude) {
    (void)from;(void)offset;(void)exclude;++ray_calls;
    if (ray_slot<0) {memset(trace,0,sizeof(*trace));return false;}
    memset(trace,0,sizeof(*trace));trace->hasHit=1;trace->character=&g_SysWork.npcs[ray_slot];
    trace->target=trace->character->position;return true;
}
void Math_MatrixTransform(VECTOR3* position,SVECTOR3* rotation,GsCOORDINATE2* coordinate) {(void)position;(void)rotation;(void)coordinate;}
void GsInitCoordinate2(GsCOORDINATE2* parent,GsCOORDINATE2* coordinate) {memset(coordinate,0,sizeof(*coordinate));coordinate->super=parent;}
static void playback(s_Model* model,s_AnmHeader* anm,GsCOORDINATE2* coord,s_AnimInfo* info) {(void)model;(void)anm;(void)coord;(void)info;}
static void reset(void) {
    size_t i;
    memset(&g_GameWork,0,sizeof(g_GameWork));memset(&g_SysWork,0,sizeof(g_SysWork));
    sh_combat_reset_scratch();
    g_DeltaTime=Q12(1.0f/30.0f);random_value=2048;ray_slot=0;ray_calls=0;damage_effects=0;stat_calls=0;sound_calls=0;
    memset(kill_counts,0,sizeof(kill_counts));memset(sh_combat_chara_group_flags,0,sizeof(sh_combat_chara_group_flags));
    memset(STALKER_ANIM_INFOS,0,sizeof(STALKER_ANIM_INFOS));
    for(i=0;i<10;++i) {CAT_ANIM_INFOS[i].playbackFunc=playback;PARASITE_ANIM_INFOS[i].playbackFunc=playback;FLAUROS_ANIM_INFOS[i].playbackFunc=playback;}
    sharedData_800E3A20_0_s00=Q12(350.0f);sharedData_800E3A24_0_s00=Q12(100.0f);
    sharedData_800E3A28_0_s00=0;sharedData_800E3A2C_0_s00=Q12(1000.0f);
    g_SysWork.playerWork.player.model.charaId=Chara_Harry;g_SysWork.playerWork.player.health=Q12(100.0f);
}
/* Each case returns a failure line. Rust serializes access to C globals. */
#define CHECK(x) do {if (!(x)) return __LINE__;} while(0)
s32 sh_combat_test_case(s32 test) {
    s_SubCharacter* player;s_SubCharacter* npc;VECTOR3 pos={0};
    reset();player=&g_SysWork.playerWork.player;npc=&g_SysWork.npcs[0];
    npc->model.charaId=Chara_Stalker;npc->health=Q12(350.0f);
    switch(test) {
        case 0: /* All six native NPC slots, player attacking -> hit mask. */
            player->field_44.field_2=32;
            for(s32 i=0;i<6;++i) {
                s_SubCharacter* target=&g_SysWork.npcs[i];target->model.charaId=Chara_Stalker;
                CHECK(func_8008B714(player,target,&pos,4096)==(1<<i));
                CHECK(target->damage.amount==Q12(100.0f));CHECK(target->attackReceived==32);
            }
            CHECK(player->field_44.field_8==63);break;
        case 1: /* Original enemy damage handling changes health and maximum. */
            Stalker_Init(npc);CHECK(npc->health==Q12(350.0f));
            player->field_44.field_2=32;func_8008B714(player,npc,&pos,4096);
            sharedFunc_800D3308_0_s00(npc);CHECK(npc->health==Q12(250.0f));
            CHECK(npc->properties.stalker.health_110==Q12(325.0f));break;
        case 2: /* Enemy -> player identity and one-time death counter. */
            npc=&g_SysWork.npcs[5];npc->field_44.field_2=32;
            CHECK(func_8008B714(npc,player,&pos,4096)==-1);CHECK(player->field_40==5);
            player->health=0;Chara_DamagedFlagUpdate(player);func_80037E78(player);func_80037E78(player);
            CHECK(kill_counts[1]==1);CHECK(player->flags&CharaFlag_Dead);break;
        case 3: { /* Original reload, full/partial/empty reserve. */
            s32 loaded=2,reserve=20;Items_AmmoReloadCompute(&loaded,&reserve,32);CHECK(loaded==15&&reserve==7);
            loaded=1;reserve=3;Items_AmmoReloadCompute(&loaded,&reserve,34);CHECK(loaded==4&&reserve==0);
            loaded=6;reserve=9;Items_AmmoReloadCompute(&loaded,&reserve,33);CHECK(loaded==6&&reserve==9);break;
        }
        case 4: { /* ANM-derived collision leaf interpolation, including negatives. */
            u8 bytes[20]={0};s_Keyframe a={0},b={0};
            bytes[0]=0;bytes[1]=0xe0;bytes[4]=0;bytes[5]=0x10;bytes[10]=0;bytes[11]=4;
            CHECK(sh_combat_keyframe_decode(bytes,20,&a));b=a;b.box.top=0;b.box.height=Q12(2.0f);
            npc->model.anim.status=1;npc->model.anim.time=Q12(0.5f);
            Collision_CharaCollisionSet(npc,&a,&b);CHECK(npc->collision.box.top==-4096);CHECK(npc->collision.box.height==6144);
            CHECK(npc->collision.cylinder.field_2==1024);CHECK(!sh_combat_keyframe_decode(bytes,19,&a));break;
        }
        case 5: { /* Original firearm attack invokes the injected ray and damage. */
            s_AnimInfo infos[2]={{0}};player->field_44.field_2=32;player->field_44.field_0=1;player->field_44.field_3=100;
            player->field_44.field_C=0;player->field_44.field_E=1024;
            g_SysWork.playerWork.extra.model.anim.status=1;g_SysWork.playerWork.extra.model.anim.time=Q12(1.0f);
            g_SysWork.playerWork.extra.model.anim.baseAnimInfos=infos;
            CHECK(func_8008A3E0(player)==1);CHECK(ray_calls==1);CHECK(npc->damage.amount==Q12(100.0f));
            func_8008A3E0(player);CHECK(ray_calls==1);break;
        }
        case 6: /* Original scripted enemy state machine advances and dies. */
            npc->model.stateStep=1;Stalker_Control_13(npc);CHECK(npc->model.stateStep==2&&npc->model.anim.status==30);
            npc->model.anim.time=Q12(212.0f);Stalker_Control_13(npc);CHECK(npc->health==-1);
            npc->model.stateStep=3;Stalker_Control_13(npc);CHECK(npc->model.stateStep==4&&npc->model.anim.status==95);break;
        case 7: { /* Complete Cat/Parasite modules dispatch native callback pointers. */
            s_AnmHeader anm={0};GsCOORDINATE2 coords[12]={{0}};npc->model.controlState=0;
            Cat_Update(npc,&anm,coords);CHECK(npc->model.controlState==1&&npc->model.stateStep==1&&npc->model.anim.keyframeIdx==7);
            npc->model.controlState=0;Parasite_Update(npc,&anm,coords);CHECK(npc->model.controlState==1&&npc->model.anim.status==3);break;
        }
        case 8: /* Pause leaves the original attack gate inactive. */
            g_DeltaTime=0;CHECK(func_8008A0E4(1,32,npc,&pos,NULL,0,0)==-1);CHECK(npc->field_44.field_0==0);break;
        case 9: /* Original enemy persistence updates only its map/spawn bit. */
            g_GameWork.savegame.mapIdx=2;g_GameWork.savegame.mapEnemyStates[2]=-1;npc->field_40=5;
            Savegame_EnemyStateUpdate(npc);CHECK((u32)g_GameWork.savegame.mapEnemyStates[2]==0xffffffdfu);break;
        case 10: /* Native division keeps specified non-trapping PS1 edge cases. */
            CHECK(sh_combat_ps1_div(0,0)==-1);CHECK(sh_combat_ps1_div(-123,0)==1);
            CHECK(sh_combat_ps1_div((-2147483647-1),-1)==(-2147483647-1));
            CHECK(sh_combat_ps1_div(-7,2)==-3);break;
        case 11: { /* Unaligned wire reads, pointer identity, failure atomicity. */
            u8 storage[25]={0};u8* bytes=storage+1;u32 aux=42;s_800AD4C8 record={0},saved;
            bytes[0]=255;bytes[1]=255;bytes[4]=100;bytes[20]=0xc4;bytes[21]=0xd4;bytes[22]=0x0a;bytes[23]=0x80;
            CHECK(sh_combat_attack_decode(bytes,24,0x800AD4C4u,&record,&aux));
            CHECK(record.field_0==-1&&record.field_4==100&&record.unk_14==&aux);saved=record;
            CHECK(!sh_combat_attack_decode(bytes,23,0x800AD4C4u,&record,&aux));CHECK(!memcmp(&record,&saved,sizeof(record)));
            CHECK(!sh_combat_attack_decode(bytes,24,0x800AD4C0u,&record,&aux));CHECK(!memcmp(&record,&saved,sizeof(record)));
            CHECK(!sh_combat_attack_decode(NULL,24,0x800AD4C4u,&record,&aux));break;
        }
        default:return -1;
    }
    return 0;
}
s32 sh_combat_attack_compare(const u8* p,size_t size) {
    s_800AD4C8 native;u32 aux=0;s32 i;
    if(size!=70*24) return -1;
    for(i=0;i<70;++i) {
        if(!sh_combat_attack_decode(p+(size_t)i*24,24,0x800AD4C4u,&native,&aux)) return i+1;
        if(memcmp(&native,&sh_combat_attacks[i],20) || native.unk_14!=&aux) return i+1;
    }
    return 0;
}
s32 sh_combat_sqrt_disc_probe(const u8* bytes,size_t length) {
    if(!sh_combat_sqrt_decode(bytes,length,SQRT,192)) return -1;
    if(func_8008A058(0)!=0 || func_8008A058(4096)!=4096 || func_8008A058(16384)!=8192 || func_8008A058(262144)!=32768) return 1;
    return 0;
}
