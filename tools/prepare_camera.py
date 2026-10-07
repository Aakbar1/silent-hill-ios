"""Native original camera source, with migrated pointer-bearing work records.

SPDX-License-Identifier: GPL-3.0-only. Reads only pinned reference source.
"""
import re
from prepare_gameplay import between, function
from prepare_math import generate as prepare_math

NOTICE = '/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations; derived from silent-hill-decomp. */\n'


def narrow(source, view):
    """Make the original small-field stores explicit, without widening them."""
    fields = dict((name, typ) for typ,name in re.findall(r'\b(q3_12|q7_8|q11_4|u8|s8|s16|u16)\s+(\w+)\s*;',view))
    fields['deathTimer'] = 'u16'
    fields['cameraAngleY'] = fields['cameraAngleZ'] = 'q3_12'
    for name in re.findall(r'^(?:static\s+)?(?:inline\s+)?\w+\s*\*?\s+(\w+)\([^;{}]*\)[^{;]*\{',source,re.M):
        code = function(source,name)
        original = code
        scalars = dict((var,typ) for typ,var in re.findall(r'\b(q3_12|q7_8|q11_4|u8|s8|s16|u16)\s+(\w+)\s*[;,=]',code))
        vectors = set(re.findall(r'\bSVECTOR(?:3)?\s*\*?\s+(\w+)',code))
        def store(match):
            lhs,op,rhs = match.groups()
            typ = scalars.get(lhs)
            if '->' in lhs or '.' in lhs:
                typ = fields.get(re.split(r'->|\.',lhs)[-1],typ)
            if re.search(r'\.m\[[^]]+\]\[[^]]+\]$',lhs):typ='s16'
            if re.search(r'(?:\.|->)v[xyz]$',lhs):
                root = re.split(r'->|\.|\[',lhs)[0]
                if root in vectors or re.search(r'(?:cam_mat_ang|ofs_cam_ang|ofs_cam_ang_spd|base_cam_ang|rotation|field_20\[[^]]+\]|field_C4\[[^]]+\])(?:\.|->)',lhs):typ='s16'
            if not typ:return match[0]
            expression = rhs if op=='=' else f'{lhs} {op[0]} ({rhs})'
            return f'{lhs} = ({typ})({expression});'
        code = re.sub(r'(\b\w+(?:(?:->|\.)\w+|\[[^]\n]+\])*)\s*(\+=|-=|=(?!=))\s*([^;{}]+);',store,code)
        ret = re.match(r'(?:static\s+)?(?:inline\s+)?(s8|q3_12|q7_8|s16|u16)\b',code)
        if ret:code=re.sub(r'\breturn ([^;]+);',lambda m:'return ('+ret[1]+')('+m[1]+');',code)
        for unused in ['unused','far_watch_rate','cur_rd_area_size']:
            if unused=='cur_rd_area_size' and name!='vcMakeIdealCamPosByHeadPos':continue
            if re.search(r'\b'+unused+r'\b',code.split('{',1)[0]):code=code.replace('{','{\n    (void)'+unused+';',1)
        source=source.replace(original,code)
    return source


def generate(decomp, out):
    def read(path): return (decomp/path).read_text()
    def no_includes(text): return re.sub(r'^#include[^\n]*','',text,flags=re.M)
    prepare_math(decomp,out)
    from prepare_maps import initializer
    globals_source = initializer(read('src/bodyprog/screen/screen_data.c'),'vcNullRoadArray')
    globals_source += function(read('src/bodyprog/player_control.c'),'Math_MagnitudeShiftGet')
    (out/'camera_globals.c').write_text(NOTICE+'#include "camera.h"\n'+globals_source)
    math = no_includes(read('include/bodyprog/math/math.h'))
    # PORT: Keep the native bootstrap's explicit field narrowing macros.
    for name in ['SVECTOR','COLOR_RGBC','Math_Vector3Set','Math_SVectorSet','Math_SetDVectorFast']:
        math = re.sub(r'^#define '+name+r'\([^\n]*\)(?:[^\n]*\\\n)*[^\n]*\n','',math,flags=re.M)
    math=math.replace('return Q12_ANGLE_NORM_S(angle);','return (q3_12)Q12_ANGLE_NORM_S(angle);')
    math=math.replace('Q12_ANGLE(180.0f)', '(q19_12)Q12_ANGLE(180.0f)')
    math=math.replace('> (range * 2)', '> (u32)(range * 2)')
    header = NOTICE+'#ifndef SH_CAMERA_H\n#define SH_CAMERA_H\n#include "gameplay.h"\n#include <stdlib.h>\n#include <limits.h>\ntypedef PortSysWork s_SysWork;\n'+math
    header += 's32 Map_TypeGet(void);s32 Math_MagnitudeShiftGet(s32);extern MATRIX GsIDMATRIX,GsIDMATRIX2,GsWSMATRIX;\n#define BOX_VERT_COUNT 8\nvoid GsSetLsMatrix(MATRIX*);\n'
    header += between(read('include/bodyprog/collision/collision.h'),'/** @brief Collision point surface data.', '/** @brief Collision point data.')
    header += 'void Collision_SurfaceGet(s_CollisionSurface*,q19_12,q19_12);\n'
    for name in ['vc_main','vc_util','vw_main','vw_calc','vw_system']:
        header += no_includes(read('include/bodyprog/view/'+name+'.h'))
    header += '#endif\n'
    (out/'camera.h').write_text(header)
    for name in ['vc_main','vc_util','vw_main','vw_calc']:
        source = no_includes(read('src/bodyprog/view/'+name+'.c'))
        source = source.replace('s_SysWork*','PortSysWork*')
        if name=='vw_calc':
            project = function(source,'Vw_TransformAndProjectPoint')
            source = source.replace(project, '''// PORT: Exact COP2 save/add/project/restore from the original inline assembly.
s32 Vw_TransformAndProjectPoint(VECTOR* worldPos,DVECTOR* screenPos) {
    VECTOR translated;u32 saved[3];
    ApplyRotMatrixLV(worldPos,&translated);
    for(u32 i=0;i<3;i++) {
        saved[i]=port_gte_read_control(5+i);
        port_gte_write_control(5+i,saved[i]+((u32*)&translated)[i]);
    }
    port_gte_write_data(0,0);port_gte_write_data(1,0);
    gte_rtps();gte_stsxy(screenPos);
    for(u32 i=0;i<3;i++)port_gte_write_control(5+i,saved[i]);
    return (s32)port_gte_read_data(19)>>2;
}
''')
            source=source.replace('INT_MAX + 1','INT_MIN')
            source=source.replace('&screenPos,','(s32*)&screenPos,')
            source=source.replace('screenPoints,','(s32*)screenPoints,')
            source=source.replace('&cullData->field_60[i],','(VECTOR*)&cullData->field_60[i],')
            source=source.replace('(u32)var_t1_2 < ((u32)D_800AD480 + 24)', 'var_t1_2 < D_800AD480 + 24')
            source=source.replace('vwLimitOverLimVector(velo_x, velo_z, to_tgt_dist, to_tgt_ang_y)', 'vwLimitOverLimVector(velo_x, velo_z, to_tgt_dist, (q3_12)to_tgt_ang_y)')
            source=source.replace('to_tgt_dist >> 1, to_tgt_ang_y)', 'to_tgt_dist >> 1, (q3_12)to_tgt_ang_y)')
        source = source.replace('hr_p->moveSpeed, hr_p->headingAngle,', '(q3_12)hr_p->moveSpeed, hr_p->headingAngle,')
        source = source.replace('switch (cam_mv_type == VC_MV_SELF_VIEW)', 's32 is_self_view = (cam_mv_type == VC_MV_SELF_VIEW);\n    switch (is_self_view)')
        source = source.replace('&w_p->watch_tgt_ang_z, self_view_eff_rate,', '&w_p->watch_tgt_ang_z, (q3_12)self_view_eff_rate,')
        source = re.sub(r'\*watch_tgt_ang_z_p \+= ([^;]+);',r'*watch_tgt_ang_z_p = (q3_12)(*watch_tgt_ang_z_p + (\1));',source)
        source = source.replace('case true:', 'default:')
        source = source.replace('q19_12 vcSelfViewTimer)', 'q19_12 selfViewTimer)')
        if name=='vc_main':
            start=source.index('s32 vcCamMatNoise(')
            stop=source.index('q19_12 Vc_VectorMagnitudeCalc',start)
            source=source[:start]+source[start:stop].replace('vcSelfViewTimer','selfViewTimer')+source[stop:]
        source = narrow(source,read('include/bodyprog/view/structs.h'))
        (out/(name+'.c')).write_text(NOTICE+'#include "camera.h"\n'+source)
