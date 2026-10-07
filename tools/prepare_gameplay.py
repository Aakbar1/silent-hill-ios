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
    match = re.search(r'^(?:static\s+)?(?:inline\s+)?\w+\s*\*?\s+' + re.escape(name) + r'\([^;{}]*\)[^{;]*\{', text, re.M)
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
            'typedef s16 q4_12; typedef s16 q7_8; typedef s32 q23_8; typedef s16 q11_4; typedef s32 q27_4; typedef u32 q24_8; typedef s8 q0_7;\n',
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
    assets.append(collision)
    assets.append(no_includes(read('include/bodyprog/chara/chara_model.h')))
    text = ''.join(assets)
    for name, old, new in [('s_MeshHeader',24,48),('s_ModelHeader',16,24),('s_Material',24,32),
                           ('s_IpdCollisionData',308,352),('s_Bone',20,40),('s_BoneNode',24,48),
                           ('s_Skeleton',1356,2712),('s_CharaModel',1376,2736),
                           ('s_IpdModelInfo',16,24),('s_IpdModelInstance',36,40),('s_IpdModelBuffer',24,40)]:
        original = f'STATIC_ASSERT_SIZEOF({name}, {old});'
        if text.count(original) != 1:
            raise ValueError(f'upstream native asset declaration drift: {name}')
        text = text.replace(original, f'// PORT: {name} holds decoded native pointers (PS1 size {old}).\nSTATIC_ASSERT_SIZEOF({name}, {new});')
    text += 'STATIC_ASSERT_SIZEOF(s_LmHeader,40);\nSTATIC_ASSERT_SIZEOF(s_ModelInfo,32);\n#endif\n'
    text += 'STATIC_ASSERT_SIZEOF(s_IpdHeader,464);\n_Static_assert(offsetof(s_IpdHeader,collisionData)==112,"native IPD collision");\n_Static_assert(offsetof(s_IpdModelInfo,modelHdr)==16,"native IPD model");\n_Static_assert(offsetof(s_IpdModelBuffer,modelInstances)==16,"native IPD instances");\n_Static_assert(offsetof(s_IpdCollisionData,subcellCheckIdxs)==92,"native collision checks");\n'
    text += '_Static_assert(offsetof(s_LmHeader,materials)==8,"native LM materials");\n_Static_assert(offsetof(s_LmHeader,modelHdrs)==24,"native LM models");\n_Static_assert(offsetof(s_ModelHeader,meshHdrs)==16,"native model meshes");\n_Static_assert(offsetof(s_MeshHeader,primitives)==8,"native mesh primitives");\n_Static_assert(offsetof(s_AnmHeader,bindPoses)==24,"native ANM poses");\n_Static_assert(offsetof(s_AnmHeader,keyframes)==32,"native ANM frames");\n'
    (out / 'native_asset_records.h').write_text(text)
    env = between(read('include/bodyprog/gfx/world.h'), 'typedef struct _Fog', '#endif')
    (out / 'native_environment.h').write_text('/* SPDX-License-Identifier: GPL-3.0-only; derived from silent-hill-decomp. */\ntypedef struct _WaterZone s_WaterZone;\n'+env)
    source = ['/* SPDX-License-Identifier: GPL-3.0-only; derived from silent-hill-decomp. */\n',
              '#include "world.h"\n']
    selections = {
        'src/bodyprog/world/bodyprog_anim_800445A4.c':['Anim_TimestepGet','Anim_BoneInit','Anim_BoneUpdate',
            'Anim_DurationGet','Anim_PlaybackOnce','Anim_PlaybackLoop','Anim_BlendLinear','Anim_BlendEaseOut'],
        'src/bodyprog/world/bodyprog_bone_80044F14.c':['Bone_ModelIdxGet','Skeleton_Init','func_80045014',
            'func_8004506C','func_80045108','Skeleton_BoneModelAssign','func_80045258','func_800452EC','func_800453E8','func_80045468'],
        'src/bodyprog/gfx/materials.c':['Lm_MaterialFileIdxApply','Lm_MaterialFsImageApply','Material_FsImageApply',
            'Lm_MaterialFlagsApply','Model_MaterialFlagsApply','LmHeader_ModelCountGet','Bone_ModelAssign'],
        'src/bodyprog/world/world_draw.c':['WorldGfx_HarryCharaLoad','Chara_FsImageCalc','WorldGfx_PlayerModelProcessLoad','WorldGfx_CharaModelProcessLoad',
            'World_Init','WorldGfx_HeldItemModelFree','WorldGfx_CharaModelsFree','Chara_ModelFree','WorldObjects_Clear','WorldGfx_HarryMeshSwap'],
        'src/bodyprog/world/world_effects.c':['Game_FlashlightAttributesFix','Game_TurnFlashlightOn','Game_TurnFlashlightOff','Game_SpotlightLoadScreenAttribsFix'],
        'src/bodyprog/game_boot/load_screen.c':['Math_MatrixTransform','GameBoot_LoadScreen_PlayerRun'],
        'src/bodyprog/events/game_sys_states.c':['SysWork_SavegameReadPlayer'],
        'src/bodyprog/game_boot/game_boot.c':['GameBoot_WorldInit','GameBoot_MapLoad','GameState_LoadScreen_Update','GameBoot_LoadingScreen','GameBoot_InGameStartup'],
        'src/bodyprog/gfx/bodyprog_80055028.c':['WorldEnv_Init','WorldEnv_FogDistanceSet'],
        'src/bodyprog/world/bodyprog_80040B74.c':['func_80040BAC'],
        'src/bodyprog/gfx/billboard_draw.c':['func_8005B55C'],
        'src/bodyprog/collision/collision.c':['Collision_Init','Collision_FlagsSet'],
        'src/bodyprog/player_control.c':['Game_PlayerInfoInit','GameFs_PlayerMapAnimLoad'],
    }
    source.append('static PACKET g_Map_GfxPackets[2][0xA10];\nstatic GsCOORDINATE2* g_ViewCoord;\n')
    source.append('static void GameBoot_LoadingScreen(void);\nstatic s32 g_MapAreaLoadCounter;\n')
    from prepare_maps import initializer
    source.append(initializer(read('src/bodyprog/game_boot/fs_chara_anim.c'),'D_800A998C'))
    # PORT: Retain the entire original startup dispatcher. Unavailable leaves
    # fail with their own names rather than replacing or skipping its states.
    guards = ['Demo_DemoFileSavegameUpdate','Demo_PlayFileBufferSetup','Demo_PlayDataRead','Demo_Start',
              'Fs_CharaAnimDataAlloc','WorldGfx_MapInitCharaLoad','AreaLoad_TransitionFlags',
              'AreaLoad_TransitionSound','Sd_BgmInit','Sd_AmbientSfxInit','GameBoot_InGameInit']
    headers='\n'.join(read(path) for path in ['include/bodyprog/bodyprog.h','include/bodyprog/demo.h','include/bodyprog/game_boot/background_sound_init.h','include/bodyprog/game_boot/fs_chara_anim.h','include/bodyprog/game_boot/game_boot.h'])
    for name in guards:
        signature=re.search(r'^(?:void|bool|s32|u32|u16|s16|s8|u8)\s+'+name+r'\([^;{}]*\);',headers,re.M)
        if not signature:raise ValueError('missing pinned startup declaration '+name)
        signature=signature[0][:-1]
        params=signature.split('(',1)[1].rsplit(')',1)[0].split(',')
        unused=''.join('(void)'+re.findall(r'\b\w+',param)[-1]+';' for param in params if param.strip()!='void')
        source.append(signature+' {'+unused+'port_unimplemented("'+name+'/native startup dependency");'+('return 0;' if not signature.startswith('void ') else '')+'}\n')
    source.append('static void GameBoot_NpcInit(void) {port_unimplemented("GameBoot_NpcInit/native room transition");}\n')
    source.append(between(read('src/bodyprog/gfx/billboard_draw.c'), 's_800AE204 D_800AE204', '// Used in `Gfx_BillboardDraw`'))
    source.append(between(read('src/bodyprog/gfx/bodyprog_80055028.c'), 's32 D_800AE1C0[]', '// ========================================'))
    for path, names in selections.items():
        original = read(path)
        source.append(f'#line 1 "{path}"\n')
        for name in names:
            code = function(original, name)
            if name=='GameBoot_InGameStartup':
                code=code.replace('WorldGfx_MapInit(&g_MapOverlayHdr,','WorldGfx_MapInit((s_MapOverlayHdr*)&g_MapOverlayHdr,')
                code=code.replace('WorldGfx_MapInitCharaLoad(&g_MapOverlayHdr)', 'WorldGfx_MapInitCharaLoad((s_MapOverlayHdr*)&g_MapOverlayHdr)')
            if name=='Math_MatrixTransform':
                # PORT: SubCharacter owns a six-byte SVECTOR3. Copy named values
                # to the SDK's padded eight-byte rotation argument.
                code=code.replace('SVECTOR* rot,','const SVECTOR3* rot,')
                code=code.replace('Math_RotMatrixZxyNegGte(rot,', 'SVECTOR rotation={rot->vx,rot->vy,rot->vz,0};\n    Math_RotMatrixZxyNegGte(&rotation,')
            if name=='GameBoot_LoadScreen_PlayerRun':
                # PORT: The upstream call casts the ANM header to an unrelated
                # skeleton type. Native playback requires its actual descriptor.
                code=code.replace('(s_Skeleton*)FS_BUFFER_0','(s_AnmHeader*)FS_BUFFER_0')
                code=code.replace('vcInitCamera(&g_MapOverlayHdr,','vcInitCamera((s_MapOverlayHdr*)&g_MapOverlayHdr,')
                code=code.replace('World_CollisionTriggersSet(&g_MapOverlayHdr)', 'World_CollisionTriggersSet((s_MapOverlayHdr*)&g_MapOverlayHdr)')
            if name == 'Anim_DurationGet':
                code = code.replace('{', '{\n    (void)unused;', 1)
            if name == 'Anim_BoneUpdate':
                # PORT: ANM frames are separate owned allocations, not header-relative bytes.
                code = code.replace('void*          frame', 'const u8*      frame')
                code = code.replace('s8*            frame', 'const s8*      frame')
                code = code.replace('((u8*)anmHdr + anmHdr->dataOffset)', 'anmHdr->keyframes')
                for target in ['frame0TranslationData', 'frame1TranslationData', 'frame0RotationData', 'frame1RotationData']:
                    code = re.sub(r'(' + target + r'\s*=) (frame[01](?:Rot)?Data)', r'\1 (const s8*)\2', code)
                # PORT: Retain signed scaling without C's undefined negative left shift.
                code = code.replace('*frame0TranslationData << scaleLog2', '(s32)((u32)(s32)*frame0TranslationData * (1u << scaleLog2))')
                # PORT: MIPS masks variable shift counts; keep legal high-scale
                # ANMs defined on native C as well as the real player's scale.
                code = code.replace('>> (Q12_SHIFT - scaleLog2)', '>> ((Q12_SHIFT - scaleLog2) & 31)')
                code = code.replace('*frame0RotationData << 5', '*frame0RotationData * 32')
                code = re.sub(r'(curBoneCoord->coord\.m\[i\]\[j\]\s*=) (.*?);', r'\1 (s16)(\2);', code, flags=re.S)
                code = code.replace('boneCount     = anmHdr->boneCount;', '''// PORT: Reject invalid frames before indexing native keyframe storage.
    if (keyframe0 < 0 || keyframe1 < 0 || keyframe0 >= anmHdr->keyframeCount || keyframe1 >= anmHdr->keyframeCount)
        port_unimplemented("animation keyframe bounds");
    boneCount     = anmHdr->boneCount;''')
            if name in ['Anim_PlaybackOnce','Anim_PlaybackLoop','Anim_BlendLinear','Anim_BlendEaseOut']:
                code = re.sub(r'(model->anim\.(?:keyframeIdx|alpha)\s*=) ([^;]+);', r'\1 (s16)(\2);', code)
            if name == 'func_80045468':
                code = code.replace('Bone_ModelIdxGet(arg1,', 'Bone_ModelIdxGet((s8*)arg1,').replace('1 << 31', '1u << 31')
            if name == 'GameFs_PlayerMapAnimLoad':
                code = code.replace('g_GameWork.mapAnimIdx = mapIdx', 'g_GameWork.mapAnimIdx = (s8)mapIdx')
            if name == 'GameBoot_MapLoad':
                code = code.replace('g_OvlDynamic', 'port_overlay_dynamic')
            if name == 'GameState_LoadScreen_Update':
                for sfx in (1501,1502):
                    assert f'Sfx_Unk{sfx} = {sfx}' in read('include/bodyprog/sound/sfx_id_enum.h')
                    code=code.replace(f'Sfx_Unk{sfx}',str(sfx))
            # PORT: Native-width pointers and explicit PS1 field narrowing.
            replacements = {
                'anmHdr->bindPoses[boneIdx].translationInitial[i] << anmHdr->scaleLog2':'(s32)((u32)(s32)anmHdr->bindPoses[boneIdx].translationInitial[i] * (1u << anmHdr->scaleLog2))',
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
            # PORT: Upstream source-local aliases must not rewrite qualified
            # member names in unrelated generated functions.
            aliases = [name for name in ('playerExtra', 'playerCombat') if re.search(r'(?<!\.)\b' + name + r'\b', code)]
            for alias in aliases:
                source.append(f'#define {alias} g_SysWork.' + ('playerWork.extra' if alias == 'playerExtra' else 'playerCombat') + '\n')
            source.append(code)
            source.extend(f'#undef {alias}\n' for alias in aliases)
    (out / 'gameplay_consumers.c').write_text(''.join(source))
    from prepare_camera import generate as prepare_camera
    prepare_camera(decomp, out)
    from prepare_world import generate as prepare_world
    prepare_world(decomp, out)
    from prepare_collision import generate as prepare_collision
    prepare_collision(decomp,out)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--decomp', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    generate(args.decomp, args.out)
