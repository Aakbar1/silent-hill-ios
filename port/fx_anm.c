/* SPDX-License-Identifier: GPL-3.0-only */
#include "sys/combat/anm_decode.c"
#include <stdlib.h>
#include <stdio.h>
#if defined(__STDC_HOSTED__) && !__STDC_HOSTED__
void* malloc(size_t);
void free(void*);
#endif
// PORT: Stable owned leaves for forward-parent bird animations. Allocation is
// per file identity and lasts for the sole worker, like the host AssetStore.
typedef struct {s32 file;u8* bytes;s_AnmBindPose poses[64];s_AnmHeader header;} FightAnm;
static FightAnm fight_bird[2];
int port_fight_asset_load(u32 file,u8* destination,u32 kind) {
    if(kind!=FileType_Anm ||
       (file!=(u32)CHARA_FILE_INFOS[Chara_AirScreamer].animFileIdx &&
        file!=(u32)CHARA_FILE_INFOS[Chara_NightFlutter].animFileIdx)) {
        int result=port_asset_load_native(file,destination,kind);
        // PORT: Native graph publication precedes the original held-item
        // material and Bone_ModelAssign step, just as for the global PLM.
        if(!result && kind==FileType_Plm && destination==(u8*)g_WorldGfxWork.heldItem.lmHdr)
            ((s_LmHeader*)destination)->isLoaded=false;
        return result;
    }
    for(s32 i=0;i<2;i++) {
        FightAnm* owner=&fight_bird[i];
        if(owner->bytes && owner->file!=(s32)file)continue;
        if(!owner->bytes) {
            u32 size=(u32)Fs_GetFileSize((s32)file);
            if(size<20 || size>0x40000)return 1;
            u8* bytes=malloc(size);
            if(!bytes)return 1;
            // PORT: Read numeric ANM leaves, then validate and explicitly build
            // native pointers. Never publish raw serialized header addresses.
            if(port_map_data_read(file,0,size,bytes) || !sh_fight_anm_decode(bytes,size,&owner->header,owner->poses,64)) {free(bytes);return 1;}
            owner->bytes=bytes;owner->file=(s32)file;
            printf("FIGHT_ANM file=%u bones=%u frames=%u forward_parent=validated\n",file,owner->header.boneCount,owner->header.keyframeCount);
        }
        memcpy(destination,&owner->header,sizeof(owner->header));return 0;
    }
    return 1;
}
