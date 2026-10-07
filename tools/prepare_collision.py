"""Original ground/collision query closure over checked native IPD leaves.

SPDX-License-Identifier: GPL-3.0-only. Every inclusion is pinned GPL C source.
"""
import re
from prepare_gameplay import between,function
from prepare_camera import narrow

NOTICE='/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations; derived from silent-hill-decomp. */\n'


def generate(decomp,out):
    text=(decomp/'src/bodyprog/collision/collision.c').read_text()
    original=(decomp/'include/bodyprog/collision/collision.h').read_text()
    cylinder=between((decomp/'include/bodyprog/collision/trigger.h').read_text(),'typedef struct _CollisionCylinder','typedef struct\n')
    records=between(original,'typedef enum _CollisionType','// emoose:')
    records=records.replace(between(original,'/** @brief Collision point surface data.','/** @brief Collision point data.'),'')
    records=re.sub(r'(\b\w+)\s*:\s*\d+;',r'\1;',records)
    records=records.replace('STATIC_ASSERT_SIZEOF(s_CollisionCellPoint, 56);','// PORT: The collision point has a native IPD pointer.\nSTATIC_ASSERT_SIZEOF(s_CollisionCellPoint, 64);')
    records+='\n// PORT: Native work only; these records never overlay serialized bytes.\nSTATIC_ASSERT_SIZEOF(s_CollisionState_44,88);\nSTATIC_ASSERT_SIZEOF(s_CollisionState,328);\n_Static_assert(offsetof(s_CollisionState,field_40)==72,"native collision buffer");\n_Static_assert(offsetof(s_CollisionState,field_A0)==208,"native collision subcells");\n_Static_assert(offsetof(s_CollisionState,point)==264,"native collision point");\n'
    defs={name:function(text,name).removeprefix('static ') for name in re.findall(r'^(?:static\s+)?(?:inline\s+)?\w+\s*\*?\s+(\w+)\([^;{}]*\)[^{;]*\{',text,re.M)}
    wanted={'Collision_SurfaceGet'}
    while True:
        expanded=wanted|{name for caller in wanted for name in re.findall(r'\b(\w+)\s*\(',defs[caller]) if name in defs}
        if expanded==wanted:break
        wanted=expanded
    assert len(wanted)==29,'review pinned surface query closure'
    header=NOTICE+'#ifndef SH_COLLISION_H\n#define SH_COLLISION_H\n#include "world.h"\n#define DEFAULT_CEILING_HEIGHT -16.0f\n'+cylinder+records
    # PORT: These original project macros store two shorts, not two words.
    header+='''
#define gte_ldR11R12(v) do {port_gte_write_control(0,(u32)(v));port_gte_write_control(2,(u32)(v));} while(0)
#define gte_ldR13R21(v) port_gte_write_control(1,0u-(u32)(v))
#define gte_ldvxy0(v) port_gte_write_data(0,(u32)(v))
#define gte_ldvz0() port_gte_write_data(1,0)
#define gte_stMAC12(v) do {port_gte_store16(v,port_gte_read_data(25));port_gte_store16((u8*)(v)+2,port_gte_read_data(26));} while(0)
'''
    sources=[]
    for name,code in defs.items():
        if name not in wanted:continue
        code=narrow(code,'')
        if name=='Collision_SurfaceGet':
            code=code.replace('s_CollisionCylinder cylinder;', 's_CollisionCylinder cylinder={0}; // PORT: Ground-only probes do not classify a character; avoid copying an uninitialized classification byte.')
        if name=='func_8006B6E8':code=code.replace('{\n','{\n    (void)subcellRanges;\n',1)
        if name=='Collision_SubcellChecksReset':
            code=code.replace('= &collData->subcellCheckIdxs[0]', '= (s32*)&collData->subcellCheckIdxs[0]')
            code=code.replace('< &collData->subcellCheckIdxs[sizeof(collData->subcellCheckIdxs)]','< (s32*)&collData->subcellCheckIdxs[sizeof(collData->subcellCheckIdxs)]')
        if name=='func_8006B318':code=code.replace('const s_IpdCollisionData* collData','s_IpdCollisionData* collData')
        def store(match):
            lhs,op,rhs=match.groups();tail=re.split(r'->|\.',lhs)[-1];typ=None
            if tail in ['surfaceIdx','surfaceIdx0','surfaceIdx1','subcellIdx','field_E','field_F','closestXSubcellIdx','closestZSubcellIdx','closeFarXSubcellIdxDiff','closeFarZSubcellIdxDiff','disableSurface0Height','disableSurface1Height']:typ='u8'
            if lhs=='state->point.heightDisabled':typ='u8'
            if re.search(r'(?:offset|direction|splitVertex[01]|field_6|charaVertDiff|charaMoveVertDiff|charaMoveOffset)(?:\.|->)v[xyz]$',lhs):typ='s16'
            if tail in ['field_3A','field_3C','field_3E','field_38','collDiffDist']:typ='s16'
            if lhs.startswith('charaState->') and tail in ['radius','top','bottom']:typ='s16'
            if lhs.startswith('surface->') and tail in ['groundType','tiltAngleX','tiltAngleZ']:typ='s8' if tail=='groundType' else 's16'
            if lhs=='temp_s0->subChunkTransDir':typ='u8'
            if not typ:return match[0]
            expr=rhs if op=='=' else lhs+' '+op[0]+' ('+rhs+')'
            return lhs+' = ('+typ+')('+expr+');'
        code=re.sub(r'(\b\w+(?:(?:->|\.)\w+|\[[^]\n]+\])*)\s*(\+=|-=|=(?!=))\s*([^;{}]+);',store,code)
        code=code.replace('&state->point.ipdCollisionData->subcellCheckIdxs[state->point.subcellIdx]', '(s8*)&state->point.ipdCollisionData->subcellCheckIdxs[state->point.subcellIdx]')
        code=code.replace('state->field_34 != arg0','(u32)state->field_34 != arg0')
        code=code.replace('func_8006C0C8(state, var_a1, var_a2)', 'func_8006C0C8(state, (s16)var_a1, (q7_8)var_a2)')
        code=code.replace('state->charaState.direction, state->charaState.distance,', 'state->charaState.direction, (q3_12)state->charaState.distance,')
        code=code.replace('arg1, charaCollDistX, charaCollDistZ, temp2)', 'arg1, (q7_8)charaCollDistX, (q7_8)charaCollDistZ, temp2)')
        code=code.replace('state->charaPositionFrom.offset.vx - state->point.field_6.vx,', '(q7_8)(state->charaPositionFrom.offset.vx - state->point.field_6.vx),')
        code=code.replace('state->charaPositionFrom.offset.vz - state->point.field_6.vz,', '(q7_8)(state->charaPositionFrom.offset.vz - state->point.field_6.vz),')
        code=code.replace('(state->charaState.radius + state->point.field_C.radiusOffset) - dist)', '(q7_8)((state->charaState.radius + state->point.field_C.radiusOffset) - dist))')
        prototype=re.sub(r'//[^\n]*','',code.split('{',1)[0]).strip()+';\n'
        header+=prototype;sources.append(code)
    header+='#endif\n'
    (out/'collision.h').write_text(header)
    (out/'collision_consumers.c').write_text(NOTICE+'#include "collision.h"\n'+''.join(sources))
