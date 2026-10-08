"""Original world chunk/texture scheduling over owned native descriptors.

SPDX-License-Identifier: GPL-3.0-only. No disc reads during generation.
"""
import re
from prepare_gameplay import between, function
from prepare_camera import narrow

NOTICE='/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations; derived from silent-hill-decomp. */\n'


def generate(decomp,out):
    def read(path):return (decomp/path).read_text()
    def no_includes(text):return re.sub(r'^#include[^\n]*','',text,flags=re.M)
    records=no_includes(read('include/bodyprog/map/terrain.h'))
    for name,old,new in [('s_MapChunk',28,32),('s_ChunkTextures',328,416),('s_WorldMapWork',1420,1592)]:
        records=records.replace(f'STATIC_ASSERT_SIZEOF({name}, {old});',f'// PORT: {name} contains native pointers.\nSTATIC_ASSERT_SIZEOF({name}, {new});')
    header=NOTICE+'#ifndef SH_WORLD_H\n#define SH_WORLD_H\n#include "camera.h"\n#define bzero(p,n) memset(p,0,n)\n#define NAME_PART_CHARS 4\n#define NAME_CHAR_BITS 6\n#define NAME_CHAR_OFFSET 0x20\nenum {BlendMode_Additive=1};\n'+records
    header+='''// PORT: Object storage is lane-owned; preceding shared workspace offsets stay intact.
typedef struct {s_WorldObjectModel* model;s32 positionX:18;s32 positionY:14;s32 positionZ:18;s32 pad:14;s32 rotationX:10;s32 rotationY:12;s32 rotationZ:10;} PortWorldObject;
extern PortWorldObject port_world_objects[29];
extern s_LmHeader port_world_item_lm;extern s32 port_world_item_queue;
'''
    header+='void port_world_common_items_load(void);void WorldObjects_Clear(s_WorldGfxWork*);\n'
    header+='void port_world_ipd_check(s_IpdHeader*);void port_world_lm_check(s_LmHeader*);\ns16* port_world_grid(s_WorldMapWork*,s32,s32);\n'
    header+='void port_camera_loading_probe(void);void func_8008E4EC(s_LmHeader*);void Fs_EncodeFileName(s32*,s32*,const char*);s32 Fs_FindNextFile(const char*,s32,s32);\n'
    sources=[]
    selections={
        'src/bodyprog/world/world_map.c': ['WorldMap_LoadStateGet','WorldMap_ChunkLoadStateGet','WorldMap_GlobalLmLoadStateGet',
            'WorldMap_Init','WorldMap_GlobalLmInit','WorldMap_GlobalLmHeaderInit','WorldMap_ChunkQueueClear',
            'WorldMap_TexturesInit','WorldMap_CollisionDataReset','WorldMap_ChunkSet','WorldMap_ActiveChunksClear',
            'WorldMap_TexturesRefClear','WorldMap_Reset','WorldMap_GlobalLmReset','WorldMap_ActiveTextureInfoGet',
            'WorldMap_InfoSet','WorldMap_ChunksClear','WorldMap_MakeGrid','ConvertHexToS8','WorldMap_CollisionDataGet',
            'WorldMap_ActiveChunkLoadedCheck','WorldMap_ChunkInit','WorldMap_PaddedDistanceToChunkEdgeGet',
            'WorldMap_DistanceToChunkEdgeGet','WorldMap_ChunkLoad','WorldMap_ActiveChunksSample','WorldMap_DistanceToEdgeCalc',
            'WorldMap_ChunkMaterialsApply','WorldMap_IpdChunkFileIdxGet','WorldMap_IsChunkPresentCheck',
            'WorldMap_FreeChunkSpaceFind','WorldMap_ChunkLoadStart','WorldMap_ActiveModelsLoadStateCheck',
            'WorldMap_NextChunkLoadCheck','WorldMap_CloseChunkEdgeCheck','WorldMap_ChunkPositionMatchCheck',
            'WorldMap_TextureLoadedCheck','WorldMap_HeaderCollisionDataGet','WorldMap_ChunkPropertiesSet',
            'WorldMap_ChunkMaterialsLoad','WorldMap_ChunkHalfPageMaterialCountGet','LmFilter_IsFullPage','LmFilter_IsHalfPage','func_80044044',
            'WorldMap_ChunksDraw','WorldMap_Draw','WorldMap_SubcellVisibleCheck','WorldMap_ObjectModelLocationGet'],
        'src/bodyprog/gfx/materials.c':['Lm_MaterialCountGet','Lm_MaterialsLoadWithFilter','Lm_IsTextureLoaded',
            'Lm_MaterialRefCountDec','Lm_MaterialFsImageApply1','Lm_ModelFind','StringCopy'],
        'src/bodyprog/gfx/texture_utils.c':['Texture_Init','Texture_Get','Texture_RefCountReset','func_8005B378',
            'Texture_RefClear','Material_TimFileNameGet','Textures_ActiveTex_CountReset','Textures_ActiveTex_PutTextures','Textures_ActiveTex_FindTexture'],
        'src/bodyprog/world/world_draw.c':['WorldGfx_MapInit','WorldGfx_MapReset','WorldGfx_CloseRangeChunksInit',
            'WorldGfx_ChunkInitCheck','WorldGfx_IpdSamplePointStore','WorldGfx_IpdSamplePointReset','WorldGfx_Draw',
            'GameFs_CommonItemsTextureLoad','WorldObject_ModelNameSet','WorldObjects_Add',
            'WorldObjects_DrawAllObjects','WorldObjects_Draw','WorldObjects_DrawStep'],
        'src/main/fileinfo.c':['Fs_EncodeFileName','Fs_FindNextFile'],
        'src/bodyprog/world/collision_trigger.c':['World_CollisionTriggersSet'],
    }
    for path,names in selections.items():
        for name in names:
            code=function(read(path),name).removeprefix('static ')
            code=code.replace('s_WorldObject*','PortWorldObject*').replace('q21_10','s32')
            code=code.replace('model->metadata.modelLocation = modelLoc;', 'model->metadata.modelLocation = (s8)modelLoc;')
            code=code.replace('g_WorldGfxWork.objects','port_world_objects').replace('worldGfxWork->objects','port_world_objects')
            code=code.replace('g_WorldGfxWork.itemLmHdr','port_world_item_lm').replace('g_WorldGfxWork.itemLmQueueIdx','port_world_item_queue')
            if name=='GameFs_CommonItemsTextureLoad':code=code.replace('Lm_HeaderPtrsInit(', 'port_world_lm_check(')
            if name=='WorldObjects_Draw':
                for f in ['vx','vy','vz']:code=re.sub(r'(rot\.'+f+r'\s*=) ([^;]+);',r'\1 (s16)(\2);',code)
            if name=='WorldMap_ObjectModelLocationGet':code=code.replace('return (curChunk - g_WorldMapWork.activeChunks)', 'return (s32)(curChunk - g_WorldMapWork.activeChunks)')
            code=code.replace('WorldMap_Init(s_LmHeader* lmHdr, s_IpdHeader* ipdBuf, s32 ipdBufSize)', 'WorldMap_Init(s_LmHeader* lmHdr, void* ipdBuf, s32 ipdBufSize)')
            if name=='WorldMap_Init':code=code.replace('= ipdBuf;', '= native_chunks;\n    (void)ipdBuf;')
            if name=='WorldMap_TexturesInit':code=code.replace('x += 16','x = (s16)(x + 16)').replace('NULL, 0, y,','NULL, 0, (u8)y,')
            if name=='WorldMap_ChunksClear':
                # PORT: Native descriptors have separate owned leaves; no packed byte arena.
                code=code.replace('    s32          chunkSize;\n','').replace('    chunkSize   = (terrain->chunkBufferSize / activeChunkCount) & ~0x3;\n','')
                code=code.replace('i++, *(u8**)&chunkBuffer += chunkSize', 'i++, chunkBuffer++')
                code=code.replace('chunkBuffer = terrain->chunkBuffer;', 'chunkBuffer = native_chunks;')
            if name=='WorldMap_MakeGrid':
                code=code.replace('{\n','{\n    (void)mapTag;\n',1)
                code=code.replace('    s_ChunkColumn* col;\n','')
                code=re.sub(r'col\s*=\s*&terrain->chunksGridCenter\[z\];\s*col->idx\[x\] = i;', '*port_world_grid(terrain,x,z) = (s16)i;',code)
            if name=='WorldMap_InfoSet':
                code=code.replace('strcpy(g_WorldMapWork.mapTag, mapTag);', 'if(strlen(mapTag)>=sizeof(g_WorldMapWork.mapTag))port_unimplemented("world map tag bounds");\n        memcpy(g_WorldMapWork.mapTag,mapTag,strlen(mapTag)+1);')
                code=code.replace('g_WorldMapWork.mapTagSize = strlen(mapTag)', 'g_WorldMapWork.mapTagSize = (s32)strlen(mapTag)')
            code=re.sub(r'\(\(s16\*\)&(\w+(?:->|\.)\w+)\[([^]]+)\]\)\[([^]]+)\]',lambda m:'*port_world_grid('+('terrain' if m[1].startswith('terrain') else '&g_WorldMapWork')+','+m[3]+','+m[2]+')',code)
            code=code.replace('((s16*)(&g_WorldMapWork.chunksGridCenter[chunkZ]))[chunkX]', '*port_world_grid(&g_WorldMapWork,chunkX,chunkZ)')
            code=code.replace('Lm_HeaderPtrsInit(', 'port_world_lm_check(')
            code=code.replace('WorldMap_ChunkHeaderPtrsInit(ipdHdr);', 'port_world_ipd_check(ipdHdr);')
            code=code.replace('    Collision_ChunkHeaderPtrsInit(&ipdHdr->collisionData);\n','')
            code=code.replace('    WorldMap_ModelLinkObjectLists(ipdHdr, lmHdrs, lmHdrCount);\n','    (void)lmHdrs;(void)lmHdrCount; // PORT: AssetStore already resolves original name/index identities.\n')
            code=code.replace('    WorldMap_ModelBufferLinkObjectLists(ipdHdr, ipdHdr->modelInfos);\n','')
            if name=='Texture_Init':code=code.replace('StringCopy(tex->name.str, texName);','StringCopy(tex->name.str, texName ? texName : ""); // PORT: Empty unallocated SDK texture name has no native address-zero string.')
            if name=='Texture_Get':
                code=code.replace('s8         filename[12];','char       filename[12];').replace('s8         debugStr[12];','char       debugStr[13];')
                code=code.replace('Material_TimFileNameGet(&filename,', 'Material_TimFileNameGet(filename,').replace('Fs_FindNextFile(&filename,','Fs_FindNextFile(filename,').replace('strncpy(&debugStr, &filename,', 'strncpy(debugStr, filename,')
                code=code.replace('strncpy(debugStr, filename, 12);','memcpy(debugStr,filename,12);')
                code=code.replace('{\n','{\n    (void)arg4;\n',1)
                # PORT: Invalid files must fail before indexing the native queue/file table.
                code=code.replace('    foundTex->queueIdx =', '    if(fileId==NO_VALUE)port_unimplemented("world texture file absent");\n    foundTex->queueIdx =')
            if name=='World_CollisionTriggersSet':code=code.replace('&overlayHeader->collisionTriggers','overlayHeader->collisionTriggers')
            if name=='StringCopy':code=code.replace('strncpy(prevStr, newStr, 8);','for(s32 i=0;i<8 && newStr[i];i++)prevStr[i]=newStr[i]; // PORT: Bounded copy into zeroed filename.')
            if name=='Material_TimFileNameGet':
                # PORT: Eight basename bytes plus .TIM plus its terminator need
                # thirteen local bytes; the original twelve-byte output stays unchanged.
                code=code.replace('char filenameCpy[12];','char filenameCpy[13];')
                code=code.replace('strcat(filenameCpy, ".TIM");','size_t end=0;while(end<8 && filenameCpy[end])end++;memcpy(filenameCpy+end,".TIM",5);')
            if name=='Fs_FindNextFile':
                for field in ['name4567','name0123','type']:code=code.replace('fileEntry->'+field+' ==', '(s32)fileEntry->'+field+' ==')
                code=code.replace('fileEntry->type    ==', '(s32)fileEntry->type    ==')
            code=code.replace('largestOutsideCount < curChunk->outsideCount','largestOutsideCount < (u32)curChunk->outsideCount')
            code=code.replace('tex->queueIdx = NO_VALUE', 'tex->queueIdx = (u32)NO_VALUE')
            if name=='WorldGfx_MapInit':code=code.replace('s_MapInfo* mapInfo;', 'const s_MapInfo* mapInfo;').replace('mapInfo->tag,', '(char*)mapInfo->tag,')
            if name=='WorldGfx_Draw':
                code=code.replace('{\n','{\n    port_render_world_models=0; // PORT: Read-only milestone draw accounting.\n',1)
                code=code.replace('WorldObjects_DrawAllObjects(', 'port_render_world_objects(')
            if name=='WorldMap_Draw':
                # PORT: Subcell orders are a named native array, not PS1 header adjacency.
                code=code.replace('&ipdHdr->textureCount + (subcellZ * 10) + (subcellX * 2)',
                    '(u8*)ipdHdr + offsetof(s_IpdHeader,textureCount) + subcellZ*10 + subcellX*2')
                code=code.replace('modelBuffer->field_10','((SVECTOR*)modelBuffer->field_10)')
                code=code.replace('WorldMap_SubcellVisibleCheck(modelBuffer, geomX - chunkBoundX, geomZ - chunkBoundZ,',
                    'WorldMap_SubcellVisibleCheck(modelBuffer, (q7_8)(geomX - chunkBoundX), (q7_8)(geomZ - chunkBoundZ),')
            if name=='WorldMap_SubcellVisibleCheck':
                code=code.replace('modelBuf->subcellPositions','((SVECTOR*)modelBuf->subcellPositions)')
                code=code.replace('Q8(25.0f), g_GameWork.gsScreenHeight)', 'Q8(25.0f), (u16)g_GameWork.gsScreenHeight)')
            code=narrow(code,records+'\n'+read('include/bodyprog/formats/texture.h')+'\n'+read('include/bodyprog/gfx/world.h'))
            for lhs in ['tex->imageDesc.u','tex->imageDesc.v']:
                code=re.sub(r'('+re.escape(lhs)+r'\s*=) ([^;]+);',r'\1 (u8)(\2);',code)
            if name=='func_80044044':code=code.replace('ipd->chunkX = (s16)', 'ipd->chunkX = (s8)').replace('ipd->chunkZ = (s16)', 'ipd->chunkZ = (s8)')
            if name=='ConvertHexToS8':
                code=re.sub(r'\b(high|low|letterIdx|hexVal) (\|=|<<=|=) ([^;]+);',lambda m:m[1]+' = (char)('+ (m[3] if m[2]=='=' else m[1]+' '+m[2][:-1]+' ('+m[3]+')')+');',code)
                code=code.replace('    *out = (hexVal << 24) >> 24;', '    *out = (s8)hexVal; // PORT: Explicit signed byte extension avoids negative signed shifts.')
            # PORT: Namespace registration bridges so map-owned local helpers can compile independently.
            for old,new in [('WorldObjects_Add','port_world_object_add'),('WorldObject_ModelNameSet','port_world_object_name_set')]:
                code=re.sub(r'\b'+old+r'\b',new,code)
            prototype=re.sub(r'//[^\n]*','',code.split('{',1)[0]).strip()+';\n'
            header+=prototype
            sources.append(code)
    header+='#endif\n'
    (out/'world.h').write_text(header)
    probe='''
// PORT: A separate streaming diagnostic; this never substitutes for gameplay.
void port_world_probe(void) {
    Fs_QueueInitialize();WorldMap_Init(GLOBAL_LM_BUFFER,IPD_BUFFER,0x2C000);
    if(port_map_activate(FILE_VIN_MAP0_S00_BIN))port_unimplemented("world probe map descriptor");
    q19_12 x=g_MapOverlayHdr.mapPoints[0].positionX,z=g_MapOverlayHdr.mapPoints[0].positionZ;
    WorldGfx_MapInit((s_MapOverlayHdr*)&g_MapOverlayHdr,x,z);
    for(s32 leg=0;leg<3;leg++) {
        q19_12 sampleX=x,sampleZ=z-(leg==1?Q12(CHUNK_SIZE):0);
        s32 iteration=0;
        for(;iteration<256;iteration++) {
            Fs_QueueUpdate();WorldMap_ChunkInit(sampleX,sampleZ,sampleX,sampleZ);
            if(!Fs_QueueGetLength() && WorldMap_ActiveModelsLoadStateCheck())break;
        }
        if(iteration==256)port_unimplemented("world probe texture/chunk readiness timeout");
        s_IpdCollisionData* collision=WorldMap_CollisionDataGet(sampleX,sampleZ);
        if(!collision || collision==&g_WorldMapWork.collisionData)port_unimplemented("world probe real collision data absent");
        Collision_FlagsSet(CollisionTriggerFlag_Map);
        s_CollisionSurface surface;Collision_SurfaceGet(&surface,sampleX,sampleZ);
        if(surface.groundHeight==Q12(8.0f))port_unimplemented("world probe collision fallback ground");
        printf("WORLD_PROBE surface=(height=%d type=%d tilt=%d,%d)\\n",surface.groundHeight,surface.groundType,surface.tiltAngleX,surface.tiltAngleZ);
        printf("WORLD_PROBE leg=%d sample=(%d,%d) iterations=%d active=",leg,sampleX,sampleZ,iteration);
        for(s32 i=0;i<g_WorldMapWork.activeChunkCount;i++) {
            s_MapChunk* chunk=&g_WorldMapWork.activeChunks[i];
            if(WorldMap_LoadStateGet(chunk->queueIdx)==WorldMapLoadState_Loaded)
                printf(" (%d,%d ready=%u)",chunk->chunkX,chunk->chunkZ,WorldMap_ChunkLoadStateGet(chunk));
        }
        printf(" collision_surfaces=%d queue=%d\\n",collision->surfaceCount,Fs_QueueGetLength());
    }
    g_SysWork.playerWork.player.position=(VECTOR3){x,0,z};
    g_GameWork.gsScreenWidth=320;g_GameWork.gsScreenHeight=224;g_DeltaTime=TIMESTEP_60_FPS;
    port_camera_loading_probe();
    printf("WORLD_PROBE original loading camera targets/view PASS\\n");
    WorldMap_Reset();
    for(s32 i=0;i<4;i++)if(g_WorldMapWork.activeChunks[i].queueIdx!=NO_VALUE)port_unimplemented("world probe reset queue");
    printf("WORLD_PROBE PASS reset=4 textures=10; player update/render not exercised\\n");
}
'''
    (out/'world_consumers.c').write_text(NOTICE+'#include "world.h"\n#include "render_services.h"\n#include <stdio.h>\nstatic s_WorldMapWork g_WorldMapWork;\nstatic s_IpdHeader native_chunks[4];\n'+''.join(sources)+probe)
    # PORT: Standalone SDK/layout preparation must expose the same rendering
    # declarations and bindings as Cargo, without editing the player generator.
    from prepare_render import generate as prepare_render
    prepare_render(decomp,out)
