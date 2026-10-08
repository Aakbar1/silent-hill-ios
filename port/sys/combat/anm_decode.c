/* SPDX-License-Identifier: GPL-3.0-only */
#include "anm_decode.h"
static u16 fight_half(const u8* p) {return (u16)((u16)p[0]|((u16)p[1]<<8));}
static u32 fight_word(const u8* p) {return (u32)p[0]|((u32)p[1]<<8)|((u32)p[2]<<16)|((u32)p[3]<<24);}
bool sh_fight_anm_decode(const u8* bytes,size_t size,s_AnmHeader* output,s_AnmBindPose* poses,size_t capacity) {
    if(!bytes || !output || !poses || size<20)return false;
    u32 bones=bytes[6],offset=fight_half(bytes),stride=fight_half(bytes+4);
    u32 count=fight_half(bytes+16),file=fight_word(bytes+12);
    if(!bones || bones>capacity || bytes[18]>30 || file>size || file<20 ||
       20+bones*6>offset || stride<(u32)bytes[2]*9+(u32)bytes[3]*3 ||
       !stride || offset>file || count>(file-offset)/stride)return false;
    // PORT: BIRD bones refer forward to bone 16. Bounds and a cycle walk
    // validate that graph without imposing an incorrect serialized ordering.
    for(u32 i=1;i<bones;i++) {
        const u8* p=bytes+20+i*6;
        if((s8)p[0]<0 || p[0]>=bones ||
           ((s8)p[1]>=0 && p[1]>=bytes[2]) ||
           ((s8)p[2]>=0 && p[2]>=bytes[3]))return false;
        u32 current=i;
        for(u32 steps=0;current!=0;steps++) {
            if(steps>=bones || current>=bones)return false;
            s8 parent=(s8)bytes[20+current*6];
            if(parent<0)return false;
            current=(u32)parent;
        }
    }
    s_AnmHeader decoded={0};
    decoded.dataOffset=(u16)offset;decoded.rotationBoneCount=bytes[2];decoded.translationBoneCount=bytes[3];
    decoded.keyframeDataSize=(u16)stride;decoded.boneCount=(u8)bones;decoded.activeBones=fight_word(bytes+8);
    decoded.fileSize=file;decoded.keyframeCount=(u16)count;decoded.scaleLog2=bytes[18];decoded.rootYOffset=bytes[19];
    // Publish only after complete validation; failed reads leave both outputs intact.
    memcpy(poses,bytes+20,bones*6);decoded.bindPoses=poses;decoded.keyframes=bytes+offset;*output=decoded;
    return true;
}
