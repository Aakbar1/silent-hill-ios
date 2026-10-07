/* SPDX-License-Identifier: GPL-3.0-only */
#include "native_player_dms.h"
#include <stdlib.h>
#if defined(__STDC_HOSTED__) && !__STDC_HOSTED__
// PORT: Compile-only CRT declarations; the platform CRT owns allocation.
void* calloc(size_t count,size_t size);
void free(void* pointer);
#endif

// PORT: One live cutscene graph owns typed numeric leaves. Serialized offsets
// are bounds checked and decoded; no pointer ever refers into the file image.
static s_DmsHeader owner;
static u16 half(const u8* bytes) {return (u16)((u16)bytes[0]|((u16)bytes[1]<<8));}
static u32 word(const u8* bytes) {return (u32)half(bytes)|((u32)half(bytes+2)<<16);}
static const u8* span(const u8* bytes,size_t size,u32 offset,size_t count) {
    if(offset>size || count>size-offset)return NULL;
    return bytes+offset;
}
static void release_entry(s_DmsEntry* entry) {
    free(entry->holdRanges);free(entry->keyframes.character);
}
static void release(s_DmsHeader* header) {
    if(header->characterEntries)for(size_t i=0;i<header->characterEntryCount;i++)release_entry(&header->characterEntries[i]);
    release_entry(&header->cameraEntry);
    free(header->characterEntries);free(header->segments);
    memset(header,0,sizeof(*header));
}
static bool decode_entry(s_DmsEntry* entry,const u8* header,const u8* bytes,size_t size,bool camera) {
    s16 count=(s16)half(header);size_t stride=camera?16:12;
    if(count<0)return false;
    entry->keyframeCount=count;entry->holdRangeCount=header[2];entry->__pad_3=header[3];
    memcpy(entry->name,header+4,4);
    const u8* holds=span(bytes,size,word(header+8),(size_t)entry->holdRangeCount*6);
    const u8* frames=span(bytes,size,word(header+12),(size_t)count*stride);
    if(!holds || !frames)return false;
    if(entry->holdRangeCount) {
        entry->holdRanges=calloc(entry->holdRangeCount,sizeof(*entry->holdRanges));
        if(!entry->holdRanges)return false;
        for(size_t i=0;i<entry->holdRangeCount;i++) {
            s_DmsHoldRange* range=&entry->holdRanges[i];
            range->startFrameIdx=(s16)half(holds+i*6);
            range->endFrameIdx=(s16)half(holds+i*6+2);
            range->keyframeIdx=(s16)half(holds+i*6+4);
            if(range->startFrameIdx>range->endFrameIdx || range->keyframeIdx<0 || range->keyframeIdx>=count)return false;
        }
    }
    if(!count)return true;
    if(camera)entry->keyframes.camera=calloc((size_t)count,sizeof(*entry->keyframes.camera));
    else entry->keyframes.character=calloc((size_t)count,sizeof(*entry->keyframes.character));
    if(!entry->keyframes.character)return false;
    for(size_t i=0;i<(size_t)count;i++) {
        s16 values[8];
        for(size_t j=0;j<stride/2;j++)values[j]=(s16)half(frames+i*stride+j*2);
        if(camera) {
            s_DmsKeyframeCamera* frame=&entry->keyframes.camera[i];
            frame->positionTarget=(SVECTOR3){values[0],values[1],values[2]};
            frame->lookAtTarget=(SVECTOR3){values[3],values[4],values[5]};
            frame->unusedAngle=values[6];frame->projectionDistance=values[7];
        } else {
            s_DmsKeyframeCharacter* frame=&entry->keyframes.character[i];
            frame->position=(SVECTOR3){values[0],values[1],values[2]};
            frame->rotation=(SVECTOR3){values[3],values[4],values[5]};
        }
    }
    return true;
}
int port_move_dms_publish(void* destination,const u8* bytes,size_t size) {
    s_DmsHeader decoded={0};
    if(!destination || !bytes || size<44 || bytes[0])return 1;
    decoded.characterEntryCount=bytes[1];decoded.segmentCount=bytes[2];decoded.field_3=bytes[3];
    decoded.field_4=word(bytes+4);
    decoded.origin=(VECTOR3){(s32)word(bytes+12),(s32)word(bytes+16),(s32)word(bytes+20)};
    const u8* segments=span(bytes,size,word(bytes+8),(size_t)decoded.segmentCount*4);
    const u8* entries=span(bytes,size,word(bytes+24),(size_t)decoded.characterEntryCount*16);
    if(!segments || !entries)return 1;
    if(decoded.segmentCount) {
        decoded.segments=calloc(decoded.segmentCount,sizeof(*decoded.segments));
        if(!decoded.segments)goto failed;
        for(size_t i=0;i<decoded.segmentCount;i++) {
            decoded.segments[i].startFrameIdx=(s16)half(segments+i*4);
            decoded.segments[i].frameCount=(s16)half(segments+i*4+2);
        }
    }
    if(decoded.characterEntryCount) {
        decoded.characterEntries=calloc(decoded.characterEntryCount,sizeof(*decoded.characterEntries));
        if(!decoded.characterEntries)goto failed;
        for(size_t i=0;i<decoded.characterEntryCount;i++)if(!decode_entry(&decoded.characterEntries[i],entries+i*16,bytes,size,false))goto failed;
    }
    if(!decode_entry(&decoded.cameraEntry,bytes+28,bytes,size,true))goto failed;
    decoded.isLoaded=true;
    release(&owner);owner=decoded;
    memcpy(destination,&decoded,sizeof(decoded));
    return 0;
failed:
    release(&decoded);return 1;
}
