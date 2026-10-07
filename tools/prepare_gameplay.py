"""Generate explicitly native gameplay work records from pinned GPL source.

SPDX-License-Identifier: GPL-3.0-only. These are never serialized asset views.
Only named pointer-bearing records gain native layout assertions; all other
upstream size assertions remain intact. The broad upstream ABI gate is separate.
"""
from pathlib import Path
import argparse
import re


def between(text, start, end):
    first = text.index(start)
    return text[first:text.index(end, first)]


def function(text, name):
    match = re.search(r'^(?:static\s+)?(?:inline\s+)?(?:void|s32|u32|s8|s16|bool)\s+' + re.escape(name) + r'\([^;{}]*\)[^{;]*\{', text, re.M)
    if not match:
        raise ValueError(f'missing pinned function {name}')
    # Ignore braces in comments/literals while preserving their source positions.
    masked = re.sub(r'//[^\n]*|/\*.*?\*/|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'',
                    lambda m: ' ' * len(m[0]), text, flags=re.S)
    start = text.index('{', match.start())
    depth = 0
    for i in range(start, len(text)):
        depth += (masked[i] == '{') - (masked[i] == '}')
        if depth == 0:
            return text[match.start():i+1] + '\n'
    raise ValueError(f'unterminated function {name}')


def generate(decomp, out):
    def read(path):
        return (decomp / path).read_text()
    def no_includes(text):
        return re.sub(r'^#include[^\n]*', '', text, flags=re.M)
    data = ['/* SPDX-License-Identifier: GPL-3.0-only; derived from silent-hill-decomp. */\n',
            '// PORT: Native work records with host pointers; never disk views.\n',
            '#ifndef SH_NATIVE_GAMEPLAY_RECORDS_H\n#define SH_NATIVE_GAMEPLAY_RECORDS_H\n',
            'typedef s16 q4_12; typedef s16 q7_8; typedef s32 q23_8;\n',
            'typedef struct {u32 flg;MATRIX coord,workm;void* param;struct _GsCOORDINATE2* super;struct _GsCOORDINATE2* sub;} ShCoordinatePlaceholder;\n']
    # Preserve the original coordinate tag used by animation callbacks.
    data[-1] = 'typedef struct _GsCOORDINATE2 {u32 flg;MATRIX coord,workm;void* param;struct _GsCOORDINATE2* super;struct _GsCOORDINATE2* sub;} GsCOORDINATE2;\nSTATIC_ASSERT_SIZEOF(GsCOORDINATE2,96);\n'
    vectors = between(read('include/decomp/types.h'), 'typedef struct _VECTOR3', '#endif')
    data.append(re.sub(r'\blong\b', 's32', vectors))
    anm = no_includes(read('include/bodyprog/formats/anm.h')).replace('s_AnmBindPose bindPoses[0];', 's_AnmBindPose* bindPoses; const u8* keyframes;')
    anm = anm.replace('STATIC_ASSERT_SIZEOF(s_AnmHeader, 20);', '// PORT: ANM offsets decode to separate native bind-pose/keyframe storage.\nSTATIC_ASSERT_SIZEOF(s_AnmHeader, 40);')
    data.append(anm)
    data.append('struct _Model; struct _AnmHeader;\n')
    data.append(no_includes(read('include/bodyprog/anim.h')))
    data.append(no_includes(read('include/bodyprog/model.h')).replace('model->anim.status = ANIM_STATUS', 'model->anim.status = (u8)ANIM_STATUS'))
    chara = between(read('include/bodyprog/chara/chara.h'), '#define NPC_COUNT_MAX', 'typedef struct _CharaFileInfo')
    chara = chara[:chara.rindex('/**')]
    data.append(chara.replace('Chara_Count,', 'PortChara_Count,'))
    game = read('include/game.h')
    player = between(game, 'typedef struct _PlayerExtra', '/** @brief Map effects info.')
    data.append(player.replace('e_InvItemId', 's32'))
    native_sizes = {'s_AnimInfo':(16,32), 's_ModelAnim':(20,32), 's_Model':(24,40),
                    's_800D5710':(52,64), 's_PropsPuppetNurse':(64,72),
                    's_SubCharacter':(296,328), 's_PlayerExtra':(44,64), 's_PlayerWork':(340,392)}
    text = ''.join(data)
    for name, (old, new) in native_sizes.items():
        original = f'STATIC_ASSERT_SIZEOF({name}, {old});'
        if text.count(original) != 1:
            raise ValueError(f'upstream native layout declaration drift: {name}')
        text = text.replace(original, f'// PORT: {name} owns native pointers; PS1 size {old} is not its native ABI.\nSTATIC_ASSERT_SIZEOF({name}, {new});')
    text += '#endif\n'
    (out / 'native_gameplay_records.h').write_text(text)
    assets = ['/* SPDX-License-Identifier: GPL-3.0-only; derived from silent-hill-decomp. */\n',
              '// PORT: Decoded native graphs, never relocated file storage.\n',
              '#ifndef SH_NATIVE_ASSET_RECORDS_H\n#define SH_NATIVE_ASSET_RECORDS_H\n']
    assets.append(no_includes(read('include/bodyprog/formats/model.h')))
    assets.append(no_includes(read('include/bodyprog/formats/texture.h')))
    assets.append(no_includes(read('include/bodyprog/formats/lm.h')))
    collision = no_includes(read('include/bodyprog/formats/ipd.h'))
    assets.append(collision[:collision.index('/** @brief IPD file model info.')])
    assets.append('#endif\n')
    assets.append(no_includes(read('include/bodyprog/chara/chara_model.h')))
    text = ''.join(assets)
    for name, old, new in [('s_MeshHeader',24,48),('s_ModelHeader',16,24),('s_Material',24,32),
                           ('s_IpdCollisionData',308,352),('s_Bone',20,40),('s_BoneNode',24,48),
                           ('s_Skeleton',1356,2712),('s_CharaModel',1376,2736)]:
        original = f'STATIC_ASSERT_SIZEOF({name}, {old});'
        if text.count(original) != 1:
            raise ValueError(f'upstream native asset declaration drift: {name}')
        text = text.replace(original, f'// PORT: {name} holds decoded native pointers (PS1 size {old}).\nSTATIC_ASSERT_SIZEOF({name}, {new});')
    text += 'STATIC_ASSERT_SIZEOF(s_LmHeader,40);\nSTATIC_ASSERT_SIZEOF(s_ModelInfo,32);\n#endif\n'
    text += '_Static_assert(offsetof(s_LmHeader,materials)==8,"native LM materials");\n_Static_assert(offsetof(s_LmHeader,modelHdrs)==24,"native LM models");\n_Static_assert(offsetof(s_ModelHeader,meshHdrs)==16,"native model meshes");\n_Static_assert(offsetof(s_MeshHeader,primitives)==8,"native mesh primitives");\n_Static_assert(offsetof(s_AnmHeader,bindPoses)==24,"native ANM poses");\n_Static_assert(offsetof(s_AnmHeader,keyframes)==32,"native ANM frames");\n'
    (out / 'native_asset_records.h').write_text(text)
    env = between(read('include/bodyprog/gfx/world.h'), 'typedef struct _Fog', '#endif')
    (out / 'native_environment.h').write_text('/* SPDX-License-Identifier: GPL-3.0-only; derived from silent-hill-decomp. */\ntypedef struct _WaterZone s_WaterZone;\n'+env)
    source = ['/* SPDX-License-Identifier: GPL-3.0-only; derived from silent-hill-decomp. */\n',
              '#include "gameplay.h"\n']
    selections = {
        'src/bodyprog/world/bodyprog_anim_800445A4.c':['Anim_BoneInit'],
        'src/bodyprog/world/bodyprog_bone_80044F14.c':['Bone_ModelIdxGet','Skeleton_Init','func_80045014',
            'func_8004506C','func_80045108','Skeleton_BoneModelAssign','func_80045258','func_800452EC','func_800453E8'],
        'src/bodyprog/gfx/materials.c':['Lm_MaterialFileIdxApply','Lm_MaterialFsImageApply','Material_FsImageApply',
            'Lm_MaterialFlagsApply','Model_MaterialFlagsApply','LmHeader_ModelCountGet','Bone_ModelAssign'],
        'src/bodyprog/world/world_draw.c':['WorldGfx_HarryCharaLoad','Chara_FsImageCalc','WorldGfx_PlayerModelProcessLoad','WorldGfx_CharaModelProcessLoad',
            'World_Init','WorldGfx_HeldItemModelFree','WorldGfx_CharaModelsFree','Chara_ModelFree','WorldObjects_Clear'],
        'src/bodyprog/world/world_effects.c':['Game_FlashlightAttributesFix','Game_TurnFlashlightOn','Game_TurnFlashlightOff'],
        'src/bodyprog/events/game_sys_states.c':['SysWork_SavegameReadPlayer'],
        'src/bodyprog/player_control.c':['Game_PlayerInfoInit'],
        'src/bodyprog/game_boot/game_boot.c':['GameBoot_WorldInit'],
        'src/bodyprog/gfx/bodyprog_80055028.c':['WorldEnv_Init','WorldEnv_FogDistanceSet'],
        'src/bodyprog/world/bodyprog_80040B74.c':['func_80040BAC'],
        'src/bodyprog/gfx/billboard_draw.c':['func_8005B55C'],
        'src/bodyprog/collision/collision.c':['Collision_Init','Collision_FlagsSet'],
    }
    source.append('static PACKET g_Map_GfxPackets[2][0xA10];\nstatic GsCOORDINATE2* g_ViewCoord;\n')
    source.append(between(read('src/bodyprog/gfx/billboard_draw.c'), 's_800AE204 D_800AE204', '// Used in `Gfx_BillboardDraw`'))
    source.append(between(read('src/bodyprog/gfx/bodyprog_80055028.c'), 's32 D_800AE1C0[]', '// ========================================'))
    for path, names in selections.items():
        original = read(path)
        source.append(f'#line 1 "{path}"\n')
        for name in names:
            code = function(original, name)
            # PORT: Native-width pointers and explicit PS1 field narrowing.
            replacements = {
                'translationInitial[i] << anmHdr->scaleLog2':'translationInitial[i] * (s32)(1u << anmHdr->scaleLog2)',
                'boneMeshIdx D_800C15B4':'boneMeshIdx port_bone_mesh_idx',
                'modelCount = LmHeader_ModelCountGet':'modelCount = LmHeader_ModelCountGet',
                'sp10[2] = LmHeader_ModelCountGet(lmHdr) - 1':'sp10[2] = (u8)(LmHeader_ModelCountGet(lmHdr) - 1)',
                '= BoneHierarchy_End;':'= (u8)BoneHierarchy_End;',
                '= BoneHierarchy_MultiModel;':'= (u8)BoneHierarchy_MultiModel;',
                'curBoneNode->bone.idx = boneIdx':'curBoneNode->bone.idx = (s8)boneIdx',
                'boneIdxOnes = modelHdr->name.str[1]':'boneIdxOnes = (u32)modelHdr->name.str[1]',
                '1 << 31':'(s32)0x80000000u',
                'Lm_HeaderPtrsInit(model->lmHdr)':'port_lm_native_check(model->lmHdr)',
                'model->lmHdr    = MAP_CHARA_LM_BUFFER':'model->lmHdr    = (s_LmHeader*)MAP_CHARA_LM_BUFFER',
                'clutX = (modelIdx * 16) + 704':'clutX = (s16)((modelIdx * 16) + 704)',
                'playerCombat.weaponInventoryIdx = i':'playerCombat.weaponInventoryIdx = (s8)i',
                'playerCombat.totalWeaponAmmo = (s8)g_SavegamePtr->items[i].count':'playerCombat.totalWeaponAmmo = g_SavegamePtr->items[i].count',
                'posTable[i].vx =':'posTable[i].vx = (s16)',
                'posTable[i].vy =':'posTable[i].vy = (s16)',
                'ptr = &posTable[0]':'ptr = (s32*)&posTable[0]',
                'packet = &g_Map_GfxPackets':'packet = (PACKET*)g_Map_GfxPackets',
                'poly_g3 = packet +':'poly_g3 = (POLY_G3*)(packet +',
                'poly_f4 = packet +':'poly_f4 = (POLY_F4*)(packet +',
                'poly_g4 = packet +':'poly_g4 = (POLY_G4*)(packet +',
                'v     = modelIdx << 7':'v     = (s8)(modelIdx << 7)',
                'v     = (modelIdx - 2) << 7':'v     = (s8)((modelIdx - 2) << 7)',
                'image->tPage[1] = tPage':'image->tPage[1] = (u8)tPage',
                'image->u        = u':'image->u        = (u8)u',
                'image->v        = v':'image->v        = (u8)v',
                'mat->field_14.u8[0] = image->u * coeff':'mat->field_14.u8[0] = (u8)(image->u * coeff)',
                'mat->field_E  = ((image':'mat->field_E  = (u8)(((image',
                '(image->tPage[1] & 0xF);':'(image->tPage[1] & 0xF));',
                'mat->field_10 = (image->clutY << 6) |':'mat->field_10 = (u16)((image->clutY << 6) |',
                '((image->clutX >> 4) & 0x3F);':'((image->clutX >> 4) & 0x3F));',
                'curPrim->bits0 = mat->field_10 + (curPrim->bits0 - mat->field_12)':'curPrim->bits0 = (u16)(mat->field_10 + (curPrim->bits0 - mat->field_12))',
            }
            for before, after in replacements.items():
                code = code.replace(before, after)
            if name == 'func_80040BAC':
                code = re.sub(r'(posTable\[i\]\.v[xy]\s*=) \(s16\) (.*?);', r'\1 (s16)(\2);',code)
                code = code.replace('sizeof(DR_TPAGE) * 2;', 'sizeof(DR_TPAGE) * 2);').replace('((sizeof(POLY_G4) * 16) * 3) +\n                           (sizeof(POLY_G3) * 16);', '((sizeof(POLY_G4) * 16) * 3) +\n                           (sizeof(POLY_G3) * 16));').replace('(sizeof(POLY_G3) * 16);\n\n        for (k', '(sizeof(POLY_G3) * 16));\n\n        for (k')
            if name == 'func_8005B55C':
                code = re.sub(r'(curPtr->(?:field_C|position)\.v[xyz]\s*=) (.*?);',r'\1 (s16)(\2);',code)
            if name == 'GameBoot_WorldInit':
                code = code[:code.rindex('}')] + '\n    port_world_boot_note();\n}\n'
            code = re.sub(r'(\*\(u16\*\)&curPrim->u[0-3]) = (field_14 \+ .*?);', r'\1 = (u16)(\2);', code)
            source.append(code)
    (out / 'gameplay_consumers.c').write_text(''.join(source))


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--decomp', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    generate(args.decomp, args.out)
