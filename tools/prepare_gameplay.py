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
    match = re.search(r'^(?:static\s+)?(?:inline\s+)?\w+\s*\**\s+' + re.escape(name) + r'\([^;{}]*\)[^{;]*\{', text, re.M)
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


def prepare_shared_types(decomp, out):
    """One declaration owner for the world/player environment ABI."""
    game = (decomp / 'include/game.h').read_text()
    body = (decomp / 'include/bodyprog/bodyprog.h').read_text()
    text = '/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations; derived from silent-hill-decomp. */\n'
    text += '#ifndef SH_SHARED_TYPES_H\n#define SH_SHARED_TYPES_H\n'
    text += between(game, 'typedef enum _SpecialEnvEventFlags', '/** @brief Game workspace.')
    text += between(body, 'typedef enum _PrimitiveType', 'typedef enum _LoadingScreenId')
    text += between(body, 'typedef struct _MapEnvPresetIdxs', 'typedef struct\n{\n    /* 0x0   */ DVECTOR')
    text += 'STATIC_ASSERT_SIZEOF(s_MapEnvPresetIdxs, 2);\n_Static_assert(offsetof(s_MapEnvPresetIdxs,presetIdx1)==1,"environment preset byte indices");\n'
    effects = between(game, '/** @brief Map effects info.', '/** @brief Main system workspace.')
    original = 'STATIC_ASSERT_SIZEOF(s_SysWork_2388, 392);'
    if effects.count(original) != 1:
        raise ValueError('upstream environment layout declaration drift')
    text += effects.replace(original, '// PORT: Native primitive-data pointer and alignment; PS1 size is 392.\nSTATIC_ASSERT_SIZEOF(s_SysWork_2388, 400);')
    text += '_Static_assert(offsetof(s_SysWork_2388,field_4)==8,"native environment pointer");\n'
    text += '_Static_assert(offsetof(s_SysWork_2388,field_1C)==36,"native environment presets");\n'
    text += '_Static_assert(offsetof(s_SysWork_2388,field_154)==348,"native live environment");\n#endif\n'
    (out / 'shared_types.h').write_text(text)


def generate(decomp, out):
    prepare_shared_types(decomp, out)
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
    # PORT: Duration callbacks actually consume the model; retain its native type.
    data.append(no_includes(read('include/bodyprog/anim.h')).replace('(*variableFunc)(void)', '(*variableFunc)(struct _Model*)'))
    data.append(no_includes(read('include/bodyprog/model.h')).replace('model->anim.status = ANIM_STATUS', 'model->anim.status = (u8)ANIM_STATUS'))
    chara = between(read('include/bodyprog/chara/chara.h'), '#define NPC_COUNT_MAX', 'typedef struct _CharaFileInfo')
    chara = chara[:chara.rindex('/**')]
    data.append(chara.replace('Chara_Count,', 'PortChara_Count,'))
    game = read('include/game.h')
    player = between(game, 'typedef struct _PlayerExtra', '/** @brief Map effects info.')
    data.append(player.replace('e_InvItemId', 's32'))
    data.append('#include "shared_types.h"\n')
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
    objects=between(read('include/bodyprog/gfx/world_object.h'), '/** @brief World object metadata.', '/** @brief Geometry-space world object to draw.')
    objects=objects.replace('STATIC_ASSERT_SIZEOF(s_WorldObjectModel, 28);','// PORT: Native model-info pointer alignment.\nSTATIC_ASSERT_SIZEOF(s_WorldObjectModel, 48);')
    assets.append(objects)
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
              '#include "npc_startup.h"\n']
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
    source.append('static void GameBoot_LoadingScreen(void);\ns32 g_MapAreaLoadCounter;\n')
    from prepare_maps import initializer
    source.append(initializer(read('src/bodyprog/game_boot/fs_chara_anim.c'),'D_800A998C'))
    # PORT: Retain the entire original startup dispatcher. Unavailable leaves
    # fail with their own names rather than replacing or skipping its states.
    guards = ['Demo_DemoFileSavegameUpdate','Demo_PlayFileBufferSetup','Demo_PlayDataRead','Demo_Start']
    headers='\n'.join(read(path) for path in ['include/bodyprog/bodyprog.h','include/bodyprog/demo.h','include/bodyprog/game_boot/background_sound_init.h','include/bodyprog/game_boot/fs_chara_anim.h','include/bodyprog/game_boot/game_boot.h'])
    for name in guards:
        signature=re.search(r'^(?:void|bool|s32|u32|u16|s16|s8|u8)\s+'+name+r'\([^;{}]*\);',headers,re.M)
        if not signature:raise ValueError('missing pinned startup declaration '+name)
        signature=signature[0][:-1]
        params=signature.split('(',1)[1].rsplit(')',1)[0].split(',')
        unused=''.join('(void)'+re.findall(r'\b\w+',param)[-1]+';' for param in params if param.strip()!='void')
        source.append(signature+' {'+unused+'port_unimplemented("'+name+'/native startup dependency");'+('return 0;' if not signature.startswith('void ') else '')+'}\n')
    source.append('void GameBoot_NpcInit(void);\n')
    source.append(between(read('src/bodyprog/gfx/billboard_draw.c'), 's_800AE204 D_800AE204', '// Used in `Gfx_BillboardDraw`'))
    source.append(between(read('src/bodyprog/gfx/bodyprog_80055028.c'), 's32 D_800AE1C0[]', '// ========================================'))
    for path, names in selections.items():
        original = read(path)
        source.append(f'#line 1 "{path}"\n')
        for name in names:
            code = function(original, name)
            if name=='GameBoot_InGameStartup':
                code=code.replace('{','{\n    if(g_GameWork.gameStateSteps[0]==4)port_maps_boot_spawn();',1)
                code=code.replace('WorldGfx_MapInit(&g_MapOverlayHdr,','WorldGfx_MapInit((s_MapOverlayHdr*)&g_MapOverlayHdr,')
                code=code.replace('WorldGfx_MapInitCharaLoad(&g_MapOverlayHdr)', 'WorldGfx_MapInitCharaLoad((s_MapOverlayHdr*)&g_MapOverlayHdr)')
            if name=='GameBoot_MapLoad':
                code=code.replace('{','{\n    mapIdx=port_maps_boot_map(mapIdx);',1)
            if name=='Math_MatrixTransform':
                code=code.replace('static void Math_MatrixTransform','void Math_MatrixTransform')
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
                # PORT: MIPS retained the model in a0 across the indirect call.
                # Native C must pass that model explicitly to the typed callback.
                code = code.replace('duration.variableFunc()', 'duration.variableFunc(unused)')
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
    prepare_player_startup(decomp, out)
    prepare_player_controls(decomp, out)
    prepare_player_loop(decomp, out)
    prepare_player_collision(decomp, out)
    prepare_player_movement(decomp, out)
    prepare_transit_services(decomp, out)


def prepare_transit_services(decomp, out):
    """Original effect atlas streaming and inventory removal on native storage."""
    from prepare_maps import initializer, enumeration
    from prepare_camera import narrow
    def read(path): return (decomp/path).read_text(encoding='utf-8')
    path=out/'player_loop.c'
    code=path.read_text(encoding='utf-8')
    effects=read('src/bodyprog/gfx/bodyprog_effects_8005E0DC.c')
    code+='\n/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations. */\n'
    code+=enumeration(read('include/bodyprog/bodyprog.h'),'EffectTextureFlags')
    # PORT: fight's player_effects.c (tools/prepare_fight.py) owns D_800A9084/8C/94 and
    # Map_EffectTexturesLoad (with a bounds-checked native buffer); don't emit a second copy.
    body=function(effects,'Map_EffectTexturesLoad')
    body=re.sub(r'    static s16 __pad_bss_800C42DA\[7\];\n','',body)
    # PORT: Queue consumers own decoded TIMs; no fixed FONT24 arena arithmetic.
    body=re.sub(r'\(s32\)FONT24_BUFFER - ALIGN\(Fs_GetFileSize\(\w+\), 0x800\)', 'port_fs_buffers[2]',body)
    body=body.replace('if (gte_IsDisabled())', 'if (false) // PORT: Native GTE has no disabled coprocessor state.')
    body=body.replace('loadedEffectTextureFlags |= 1 << i;', 'loadedEffectTextureFlags = (u16)(loadedEffectTextureFlags | (1u << i));')
    body=body[:-2]+'    (void)loadedEffectTextureFlags;\n}\n'
    code+='extern s_FsImageDesc g_LoadingScreenImg;\nvoid Screen_BackgroundImgDraw(s_FsImageDesc*);\n'
    code+=function(read('src/bodyprog/game_boot/load_screen.c'),'GameBoot_LoadScreen_BackgroundImg')
    items=read('src/bodyprog/items/item_screens_2.c')
    code+='\n#define INV_WEAPON_AMMO_ID(id) ((id)+32)\nu8 port_player_inv_item_selected;\n'
    groups=enumeration(read('include/bodyprog/items.h'),'InvItemGroup')
    # PORT: The shared gameplay header already owns these two group constants.
    groups=groups.replace('InvItemGroup_MeleeWeapons','PortTransit_MeleeWeapons').replace('InvItemGroup_GunWeapons','PortTransit_GunWeapons')
    code+=groups+initializer(items,'D_80025EB0')
    sort=function(items,'func_8004F190').replace('func_8004F190','port_transit_inventory_sort')
    # PORT: Copy the original four inventory bytes through their actual type.
    sort=re.sub(r'\*\(s32\*\)&tempItem\s*=.*?;', 'tempItem = savePtr->items[i];',sort,flags=re.S)
    sort=re.sub(r'\*\(s32\*\)&savePtr->items\[i\]\.id\s*=.*?;', 'savePtr->items[i] = savePtr->items[j];',sort,flags=re.S)
    sort=re.sub(r'\*\(s32\*\)&savePtr->items\[j\]\.id\s*=.*?;', 'savePtr->items[j] = tempItem;',sort,flags=re.S)
    sort=sort.replace('D_80025EB0[savePtr->items[i].id - InvItemId_HealthDrink]', 'port_transit_item_order(savePtr->items[i].id)')
    sort=sort.replace('g_SysWork.invItemSelectedIdx','port_player_inv_item_selected')
    sort=sort.replace('= InvItemId_Empty;', '= (u8)InvItemId_Empty;').replace('= id;', '= (u8)id;')
    sort=sort.replace('weaponInventoryIdx = i;', 'weaponInventoryIdx = (s8)i;').replace('port_player_inv_item_selected = count;', 'port_player_inv_item_selected = (u8)count;')
    sort=re.sub(r'(savePtr->items\[[^]]+\]\.count)\s*\+=\s*([^;]+);',r'\1 = (u8)(\1 + (\2));',sort)
    code+='static u8 port_transit_item_order(u8 id){if(id<InvItemId_HealthDrink || (u32)(id-InvItemId_HealthDrink)>=ARRAY_SIZE(D_80025EB0))port_unimplemented("inventory sort item ID bounds");return (u8)D_80025EB0[id-InvItemId_HealthDrink];}\n'
    code+=narrow(sort,read('include/bodyprog/savegame.h'))
    remove=function(items,'Player_ItemRemove').replace('func_8004F190(g_SavegamePtr)', '(u8)port_transit_inventory_sort(g_SavegamePtr)')
    remove=remove.replace('= InvItemId_Empty;', '= (u8)InvItemId_Empty;')
    code+=remove
    math=function(read('src/bodyprog/bodyprog_math_8005BF38.c'),'func_8005C478')
    math=math.replace('Math_AngleNormalizeSigned(ratan2(', 'Math_AngleNormalizeSigned((q3_12)ratan2(')
    math=math.replace('*arg0 = ((var_s1 << Q12_SHIFT) / temp);', '*arg0 = (s16)(((s32)((u32)var_s1 << Q12_SHIFT)) / temp); // PORT: Preserve PS1 wrapping shift.')
    math=math.replace('return ABS(', 'return (u32)ABS(')
    code+=math
    wave=function(read('src/bodyprog/events/bgm_update.c'),'func_800364BC')
    wave=wave.replace('g_DeltaTimeRaw * (Q12(64.0f) + 1)', '(u32)g_DeltaTimeRaw * (u32)(Q12(64.0f) + 1)')
    wave=wave.replace('Math_Sin(D_800BCD58 >> 18)', 'Math_Sin((s32)(D_800BCD58 >> 18))').replace('Math_Sin((D_800BCD58 & 0xFFFF) / 16)', 'Math_Sin((s32)((D_800BCD58 & 0xFFFF) / 16))')
    wave=wave.replace('var0 += Math_Sin', 'var0 += (u32)Math_Sin').replace('var1  = Math_Sin', 'var1  = (u32)Math_Sin')
    code+=wave
    path.write_text(code,encoding='utf-8')


def prepare_player_startup(decomp, out):
    """Retain startup control flow; unresolved leaves remain named guards."""
    from prepare_maps import initializer
    def read(path):
        return (decomp / path).read_text(encoding='utf-8')
    selections = {
        'src/bodyprog/game_boot/game_boot.c': ['GameBoot_NpcClear', 'GameBoot_NpcInit', 'GameBoot_InGameInit'],
        'src/bodyprog/game_boot/fs_chara_anim.c': ['Fs_CharaAnimBoneInfoSet'],
        'src/bodyprog/world/world_draw.c': ['WorldGfx_MapInitCharaLoad', 'WorldGfx_CharaModelProcessAllLoads'],
        'src/bodyprog/events/game_sys_states.c': ['AreaLoad_TransitionFlags'],
        'src/bodyprog/game_boot/background_sound_init.c': ['Sd_BgmInit', 'Sd_BgmActiveSongCheck', 'Sd_BgmSongSet', 'Sd_BgmChannelSet', 'Sd_BgmUpdateTrack', 'Sd_AmbientSfxInit', 'Sd_ActiveAmbientSfxCheck', 'Sd_AmbientSfxSet'],
        'src/bodyprog/events/player_pos_update.c': ['Game_PlayerHeightUpdate'],
        'src/bodyprog/events/bgm_update.c': ['Bgm_LayerGlobalVariablesMute'],
        'src/bodyprog/game_boot/load_screen.c': ['GameBoot_WolrdEnvInit'],
        'src/bodyprog/gfx/bodyprog_effects_8005E0DC.c': ['func_8005E650','func_8005E70C'],
        'src/bodyprog/events/npc_main.c': ['Game_NpcRoomInitSpawn','Math_Distance2dCheck'],
        'src/bodyprog/events/chara_spawn.c': ['Chara_Spawn'],
        'src/bodyprog/demo.c': ['func_8008F914'],
        'src/bodyprog/events/radio.c': ['Game_RadioNoiseReset'],
        'src/bodyprog/text/bodyprog_8004B45C.c': ['func_80037124'],
        'src/bodyprog/text/bodyprog_8003652C.c': ['func_8003652C'],
        'src/bodyprog/items/item_screens_3.c': ['GameFs_Tim00TIMLoad','GameFs_MapItemsModelLoad'],
    }
    source = ['/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations. */\n#include "npc_startup.h"\n#include "collision.h"\n']
    radio_header=read('include/bodyprog/bodyprog.h')
    radio_start=radio_header.index('typedef struct _RadioNoise')
    radio_end=radio_header.index('STATIC_ASSERT_SIZEOF(s_RadioNoise, 4);',radio_start)+len('STATIC_ASSERT_SIZEOF(s_RadioNoise, 4);')
    source.append(radio_header[radio_start:radio_end]+'\n')
    source.append('s_RadioNoise g_RadioNoise[2];\n')
    msg_header=read('include/bodyprog/events/map_msg.h')
    source.append(between(msg_header,'typedef struct','extern s_MapMsgSelect'))
    source.append('s_MapMsgSelect g_MapMsg_Select;\nvoid func_8003652C(void);\n')
    for name in ['g_FirstAidKitItemTextureImg','g_InventoryKeyItemTextureImg','D_800A9074']:
        source.append(initializer(read('src/bodyprog/screen/screen_data.c'),name))
    sound = read('src/bodyprog/game_boot/background_sound_init.c')
    for name in ['g_BgmTaskLoad', 'g_BgmChannelSetTask', 'g_AmbientVabTaskLoad']:
        source.append(initializer(sound, name))
    # PORT: The shared environment functions/tables are owned by render_consumers.c.
    source.append('s16 D_800C4408;\ns8 D_800C4414;\n')
    for path, names in selections.items():
        for name in names:
            code = function(read(path), name)
            code=code.replace('g_SysWork.field_2388.', 'g_SysWork.gameplayEnvironment.')
            code=code.replace('g_SysWork.gameplayEnvironment.isFlashlightOn','g_SysWork.field_2388.isFlashlightOn')
            code = code.replace('static void GameBoot_NpcInit', 'void GameBoot_NpcInit')
            code = code.replace('&g_MapOverlayHdr,', '(s_MapOverlayHdr*)&g_MapOverlayHdr,')
            code = code.replace('World_CollisionTriggersSet(&g_MapOverlayHdr)', 'World_CollisionTriggersSet((s_MapOverlayHdr*)&g_MapOverlayHdr)')
            code = code.replace('WorldEnv_MapPresetSet(&g_MapOverlayHdr)', 'WorldEnv_MapPresetSet((s_MapOverlayHdr*)&g_MapOverlayHdr)')
            if name == 'GameBoot_WolrdEnvInit':
                code = code.replace('{', '{\n    (void)unused;', 1)
            if name in ['func_8005E650','func_8005E70C']:
                code=code.replace('count = g_MapOverlayHdr.', 'count = (s16)g_MapOverlayHdr.')
            if name == 'func_8005E650':
                code=code.replace('{','{\n    (void)mapId;',1)
            if name == 'Game_NpcRoomInitSpawn':
                # PORT: Native spawn fields are explicit scalars, not a packed VECTOR3 alias.
                code=code.replace('VECTOR3*           pos;', 'VECTOR3 spawnPos;\n    VECTOR3* pos=&spawnPos;')
                code=code.replace('pos = (VECTOR3*)curCharaSpawn;', 'spawnPos=(VECTOR3){curCharaSpawn->positionX,0,curCharaSpawn->positionZ};')
                code=code.replace('s_SpawnInfo*       curCharaSpawn;', 'const s_SpawnInfo* curCharaSpawn;')
            if name in ['Game_NpcRoomInitSpawn','Chara_Spawn']:
                for field,typ in [('field_40','s8'),('model.charaId','s8'),('model.stateStep','u8'),('rotation.vy','s16')]:
                    code=re.sub(r'(g_SysWork\.npcs\[[^\]]+\]\.'+re.escape(field)+r'\s*=(?!=))\s*([^;{}]+);',r'\1 ('+typ+r')(\2);',code)
            if name == 'func_8003652C':
                code=code.replace('LoadImage(&rect, VALS)', 'LoadImage(&rect, (u_long*)VALS)')
            code = code.replace('g_GameWork.bgmIdx = bgmIdx', 'g_GameWork.bgmIdx = (s8)bgmIdx')
            code = code.replace('g_GameWork.ambientIdx = bgmIdx', 'g_GameWork.ambientIdx = (s8)bgmIdx')
            if name == 'Sd_AmbientSfxInit':
                # PORT: Descriptor data is mutable native state, reset on activation.
                code = code.replace('g_MapOverlayHdr.ambientAudioIdx =', '((s_MapOverlayHdr*)port_map_active())->ambientAudioIdx =')
            source.append(code)
    (out / 'player_startup.c').write_text(''.join(source), encoding='utf-8')


def prepare_player_controls(decomp, out):
    """Original input/turning and animation initialization, with checked tables."""
    from prepare_maps import initializer, enumeration
    from prepare_camera import narrow
    original = (decomp / 'src/bodyprog/player_control.c').read_text(encoding='utf-8')
    player_header = (decomp / 'include/bodyprog/player.h').read_text(encoding='utf-8')
    names = ['Player_Controller','Player_CharaRotate','Player_CharaTurn_0','Player_CharaTurn_1',
             'Player_MovementStateReset','Player_VariableAnimDurationGet','func_8007E8C0',
             'Game_PlayerMovementsReset','Player_DisableDamage','Player_IsAttacking','Player_IsBusy',
             'Player_FlexRotationYReset']
    codes = {name:function(original,name) for name in names}
    used = set(re.findall(r'\b(?:g_Player_\w+|D_800\w+)\b', ''.join(codes.values())))
    provided = set(re.findall(r'\bg_Player_\w+',function(original,'func_8007E9C4')))
    provided |= {'g_Player_DisableControl','g_Player_CutsceneState','g_Player_LastWeaponSelected','g_Player_GrabReleaseInputTimer','D_800C4588','D_800C45EC'}
    header = ['/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations. */\n#ifndef SH_PLAYER_CONTROLS_H\n#define SH_PLAYER_CONTROLS_H\n#include "npc_startup.h"\n']
    header.append(enumeration(player_header,'RockDrillAttackType'))
    header.append(between(player_header,'typedef struct _800AFBF4','// Used in player lower body state handling.'))
    header.append('extern s_800AFBF4 g_Player_EquippedWeaponInfo;\n')
    enums = (decomp/'include/game.h').read_text(encoding='utf-8') + (decomp/'include/bodyprog/items.h').read_text(encoding='utf-8')
    for tag in ['AttackInputType','EquippedWeaponId']:
        header.append(enumeration(enums,tag))
    header += ['#define WEAPON_ATTACK(w,i) ((w)+((i)*10))\n#define WEAPON_ATTACK_ID_GET(w) ((w)%10)\n',
               '#define DEFAULT_PLAYER_BOX_TOP Q12(-1.6f)\n#define DEFAULT_PLAYER_BOX_OFFSET_Y Q12(-1.1f)\n',
               function(player_header,'Player_CollisionReset')]
    globals_code=[]
    types={}
    for name in sorted(used):
        if name == 'D_800AFBF4': raise ValueError('weapon data needs a separate native migration')
        match = re.search(r'(?:extern\s+)?(\w+)\s+'+re.escape(name)+r'\s*(?:=[^;]*|);',player_header+'\n'+original)
        if not match: raise ValueError('missing input global '+name)
        typ={'g_Player_DisableControl':'bool','g_Player_CutsceneState':'s32','D_800C4588':'s16'}.get(name,match[1])
        header.append(f'extern {typ} {name};\n');types[name]=typ
        if name not in provided: globals_code.append(f'{typ} {name};\n')
    for name, code in codes.items():
        header.append(re.sub(r'//[^\n]*','',code.split('{',1)[0]).strip()+';\n')
    header.append('s32 Inventory_HyperBlasterFunctionalTest(void);\nextern s_AnimInfo HARRY_BASE_ANIM_INFOS[256];\n#endif\n')
    (out/'native_player_controls.h').write_text(''.join(header),encoding='utf-8')
    table=initializer(original,'HARRY_BASE_ANIM_INFOS').replace('[57]','[256]')
    # PORT: Select the pointer member explicitly rather than initializing a word
    # with a function address (the PS1 union members had identical sizes).
    table=re.sub(r'\{\s*(Player_VariableAnimDurationGet)\s*}',r'{ .variableFunc = \1 }',table)
    table=re.sub(r'(false|true),\s+NO_VALUE,',r'\1, (u8)NO_VALUE,',table)
    source=['/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations. */\n#include "native_player_controls.h"\n', ''.join(globals_code),
            '// PORT: Explicit native animation storage replaces the original base-table/BSS adjacency.\n',table]
    source.append(initializer(original,'D_800AFBF4').replace('{}','{0}'))
    source.append('s_800AFBF4 g_Player_EquippedWeaponInfo;\n')
    # PORT: Preserve the exact empty-hand boot branch. Equipped weapon streaming
    # still needs a native ANM fragment consumer and remains explicitly guarded.
    source.append('void GameFs_WeaponInfoUpdate(void) {g_SysWork.targetNpcIdx=NO_VALUE;\nif(g_SysWork.playerCombat.weaponAttack!=NO_VALUE)port_unimplemented("GameFs_WeaponInfoUpdate/native weapon ANM fragments");\n')
    source.append(between(function(original,'GameFs_WeaponInfoUpdate'),'g_Player_EquippedWeaponInfo = D_800AFBF4[0];','return;')+'}\n')
    for name, code in codes.items():
        if name=='func_8007E8C0':
            code=code.replace('HARRY_BASE_ANIM_INFOS[i] =','// PORT: Never overrun native animation storage on a malformed descriptor.\n        if(i >= 256)port_unimplemented("Harry map animation count");\n        HARRY_BASE_ANIM_INFOS[i] =')
        if name in ['Player_CharaTurn_0','Player_CharaTurn_1']:
            code=code.replace('{','{\n    (void)player;',1)
        # PORT: Explicitly retain the original 8/16-bit input-history stores.
        for symbol, typ in types.items():
            if typ in ['u8','s8','s16','u16','q3_12']:
                code=re.sub(r'\b'+re.escape(symbol)+r'\s*=(?!=)\s*([^;{}]+);',lambda m:symbol+' = ('+typ+')('+m[1]+');',code)
        code=narrow(code,'')
        if name in ['Player_CharaTurn_0','Player_CharaTurn_1']:
            code=code.replace('playerExtra.lowerBodyState = curState +', 'playerExtra.lowerBodyState = (u8)(curState +').replace('PlayerLowerBodyState_QuickTurnLeft;', 'PlayerLowerBodyState_QuickTurnLeft);').replace('PlayerLowerBodyState_QuickTurnRight;', 'PlayerLowerBodyState_QuickTurnRight);')
        # PORT: Keep upstream source-local aliases from rewriting qualified members.
        aliases={ 'playerChara':'g_SysWork.playerWork.player','playerProps':'g_SysWork.playerWork.player.properties.player','playerExtra':'g_SysWork.playerWork.extra','playerCombat':'g_SysWork.playerCombat'}
        for alias,target in aliases.items():
            code=re.sub(r'(?<!\.)\b'+alias+r'\b',target,code)
        source.append(code)
    (out/'player_controls.c').write_text(''.join(source),encoding='utf-8')


def prepare_player_loop(decomp,out):
    """Compile the original dispatcher/update order, guarding missing logic leaves."""
    def read(path): return (decomp/path).read_text(encoding='utf-8')
    from prepare_maps import enumeration
    states=read('src/bodyprog/events/game_sys_states.c')
    player=read('src/bodyprog/player_control.c')
    header=['/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations. */\n#ifndef SH_PLAYER_LOOP_H\n#define SH_PLAYER_LOOP_H\n#include "native_player_controls.h"\n']
    header.append(enumeration(read('include/bodyprog/screen/cutscene_border.h'),'CutsceneBorderState'))
    header.append('void CutsceneBorder_Reset(void);\n')
    guarded_states=['SysState_OptionsMenu_Update','SysState_StatusMenu_Update','SysState_MapScreen_Update','SysState_Fmv_Update','SysState_LoadArea_Update','SysState_ReadMessage_Update','SysState_SaveMenu_Update','SysState_EventSetFlag_Update','SysState_EventPlaySound_Update','SysState_GameOver_Update','SysState_GamePaused_Update']
    source=['/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations. */\n#include "native_player_loop.h"\n']
    for name in guarded_states:
        header.append(f'void {name}(void);\n')
        if name not in ['SysState_Fmv_Update','SysState_ReadMessage_Update','SysState_EventSetFlag_Update','SysState_EventPlaySound_Update','SysState_LoadArea_Update']:
            source.append(f'void {name}(void) {{port_unimplemented("{name}/native system state");}}\n')
    names=['GameState_InGame_Update','SysState_Gameplay_Update','SysState_EventCallback_Update','SysState_Fmv_Update','SysState_ReadMessage_Update','SysState_EventSetFlag_Update','SysState_EventPlaySound_Update','SysState_LoadArea_Update','AreaLoad_UpdatePlayerPosition','AreaLoad_TransitionSound']
    header += [f'void {name}(void);\n' for name in names]
    header += ['void Player_Update(s_SubCharacter*,s_AnmHeader*,GsCOORDINATE2*);\n',
               'void Event_Update(bool);\nvoid func_800892A4(s32);\nvoid Game_FlashlightToggle(void);\n',
               'void Screen_CutsceneCameraStateUpdate(void);\nvoid Player_CombatUpdate(s_SubCharacter*,GsCOORDINATE2*);\nvoid func_8008A3AC(s_SubCharacter*);\nvoid Game_NpcUpdate(void);\nvoid func_8005E89C(void);\nvoid WorldGfx_Draw(s32);\n',
               'void Demo_DemoRandSeedBackup(void);\nvoid Demo_DemoRandSeedRestore(void);\nvoid Demo_DemoRandSeedAdvance(void);\n',
               'void Player_ReceiveDamage(s_SubCharacter*,s_PlayerExtra*);\nvoid Player_LogicUpdate(s_SubCharacter*,s_PlayerExtra*,GsCOORDINATE2*);\nvoid Player_PositionUpdate(s_SubCharacter*,s_PlayerExtra*,GsCOORDINATE2*);\nvoid Player_AnimUpdate(s_SubCharacter*,s_PlayerExtra*,s_AnmHeader*,GsCOORDINATE2*);\nvoid func_8007D090(s_SubCharacter*,s_PlayerExtra*,GsCOORDINATE2*);\n',
               'extern s32 g_MapEventSysState;\nextern u32 g_MapEventParam;\nextern s_EventData* g_MapEventData;\n#define BgmStatusFlag_6 (1<<6)\n#endif\n']
    header[-1]=header[-1].replace('#endif', 'extern VECTOR3 D_800C45B0;\nbool Fs_QueueChunksLoad(void);\nvoid World_NearbyPlayerCollisionTriggersGet(void);\nvoid sh_combat_Chara_Flag8Clear(s_SubCharacter*);\nvoid sh_combat_Chara_DamagedFlagUpdate(s_SubCharacter*);\n#endif')
    events=read('src/bodyprog/events/events_main.c')
    event_names=['Event_Update','Event_CollideFacingCheck','Event_CollideObbFacingCheck','Event_CollideObbCheck']
    for name in event_names[1:]:
        code=function(events,name)
        header[-1]=header[-1].replace('#endif', re.sub(r'//[^\n]*','',code.split('{',1)[0]).strip()+';\n#endif')
    source.append(between(states,'static void (*g_SysStateFuncs[])(void)', '/** Used to store'))
    source.append('static q19_12 g_DeltaTimeCpy;\nbool g_IsLoadingFinished;\ns32 g_MapEventSysState;\nu32 g_MapEventParam;\ns_EventData* g_MapEventData;\nstatic u32 g_Demo_RandSeedBackup;\n')
    # PORT: Original movie snapshot uses owned native storage, never PS1 fixed-address bytes.
    source.append('static u32 port_events_fmv_image[160*240/2];\n#define IMAGE_BUFFER_0 ((u8*)port_events_fmv_image)\n')
    from prepare_maps import initializer, enumeration
    source.append('#include "maps_registry.h"\n#include "native_player_movement.h"\n')
    source.append('// PORT: Previously absent transition fields own native storage without changing the shared work ABI.\ns_MapPoint2d g_MapPoint;\nstatic s8 port_transit_sfx_pair;\ntypedef struct {u16 sfx_0,sfx_2;} s_AreaLoadSfx;\nextern s32 g_MapAreaLoadCounter;\nvoid Bgm_SongChange(s32);\n#define BgmStatusFlag_Pause (1<<0)\n')
    # PORT: These door task IDs are GPL source constants, never owned sound bytes.
    source.append(initializer(read('src/bodyprog/events/bodyprog_data_800A99B4.c'),'SFX_PAIRS'))
    for name in names:
        code=function(states,name)
        if name=='SysState_EventCallback_Update':
            code=code.replace('g_MapOverlayHdr.mapEventFuncs[g_MapEventParam]();','port_maps_callback_validate(g_MapEventParam);\n    g_MapOverlayHdr.mapEventFuncs[g_MapEventParam]();')
        if name=='SysState_LoadArea_Update':
            # PORT: Validate source-overlay indices before queuing replacement.
            code=code.replace('{','{\n    port_maps_transition_validate(g_MapEventData);',1)
            code=code.replace('offsetZ               = g_SysWork.playerWork.player.position.vz - mapPoint->positionZ;', 'offsetZ               = (u32)g_SysWork.playerWork.player.position.vz - (u32)mapPoint->positionZ;')
            code=code.replace('g_MapPoint.positionZ += offsetZ;', 'g_MapPoint.positionZ = (s32)((u32)g_MapPoint.positionZ + offsetZ);')
            code=code.replace('g_SysWork.field_2349 = g_MapOverlayHdr.mapPoints[g_MapEventData->eventParam].field_4_5 - 1;', 'g_SysWork.field_2349 = (s8)(g_MapOverlayHdr.mapPoints[g_MapEventData->eventParam].field_4_5 - 1);')
            code=code.replace('= g_MapEventData->sfxPairIdx_8_19;', '= (s8)g_MapEventData->sfxPairIdx_8_19;').replace('= g_MapEventData->transitionFlags;', '= (s8)g_MapEventData->transitionFlags;').replace('= g_MapEventData->mapIdx;', '= (u8)g_MapEventData->mapIdx;')
            code=code.replace('SyncMode_Immediate','1 /* SyncMode_Immediate */')
        if name=='AreaLoad_TransitionSound':
            code=code.replace('{','{\n    if(g_SysWork.sfxPairIdx<0 || g_SysWork.sfxPairIdx>=25)port_unimplemented("area transition SFX index");',1)
        code=code.replace('g_SysWork.sfxPairIdx','port_transit_sfx_pair')
        code=code.replace('g_SysWork.field_2388.', 'g_SysWork.gameplayEnvironment.')
        code=code.replace('g_SysWork.gameplayEnvironment.isFlashlightOn','g_SysWork.field_2388.isFlashlightOn')
        code=code.replace('Player_Update(player, FS_BUFFER_0,','Player_Update(player, (s_AnmHeader*)FS_BUFFER_0,')
        # PORT: First member is the player, but native C calls it by its actual type.
        code=code.replace('Player_CombatUpdate(&g_SysWork.playerWork,','Player_CombatUpdate(&g_SysWork.playerWork.player,')
        code=code.replace('Chara_Flag8Clear(', 'sh_combat_Chara_Flag8Clear(')
        code=code.replace('i < ARRAY_SIZE(g_SysWork.npcs)', 'i < (s32)ARRAY_SIZE(g_SysWork.npcs)')
        code=code.replace('i == ARRAY_SIZE(g_SysWork.npcs)', 'i == (s32)ARRAY_SIZE(g_SysWork.npcs)')
        code=code.replace('Gfx_MapMsg_Draw(g_MapEventParam)', 'Gfx_MapMsg_Draw((s32)g_MapEventParam)')
        code=code.replace('void (**unfreezePlayerFunc)(bool);','void (*unfreezePlayerFunc)(bool);').replace('unfreezePlayerFunc = &g_MapOverlayHdr.playerControlUnfreeze;','unfreezePlayerFunc = g_MapOverlayHdr.playerControlUnfreeze;')
        code=code.replace('open_main(BASE_AUDIO_FILE_IDX - g_MapEventParam, g_FileTable[BASE_AUDIO_FILE_IDX - g_MapEventParam].blockCount);','open_main((s32)(BASE_AUDIO_FILE_IDX - g_MapEventParam), (s16)g_FileTable[BASE_AUDIO_FILE_IDX - g_MapEventParam].blockCount);')
        source.append(code)
    for name in ['Demo_DemoRandSeedBackup','Demo_DemoRandSeedRestore','Demo_DemoRandSeedAdvance']:
        source.append(function(read('src/bodyprog/demo.c'),name))
    code=function(player,'Player_Update').replace('extra = &playerExtra;', 'extra = &g_SysWork.playerWork.extra;')
    source.append(code)
    source.append('static s_EventData* g_ItemTriggerEvents[5];\nstatic s32 g_ItemTriggerItemIds[5],g_MapEventLastUsedItem;\n')
    from prepare_camera import narrow
    for name in event_names:
        code=function(events,name)
        code=code.replace('g_SysWork.field_2388.', 'g_SysWork.gameplayEnvironment.')
        code=code.replace('g_SysWork.gameplayEnvironment.isFlashlightOn','g_SysWork.field_2388.isFlashlightOn')
        if name=='Event_Update':
            # PORT: Lift the upstream nested function into ISO C file scope.
            code=code.replace('    void Event_ItemTriggersClear()', 'void Event_ItemTriggersClear(void)')
            inner=function(code,'Event_ItemTriggersClear')
            code=code.replace(inner.rstrip(),'')
            source.append('static '+inner)
            # PORT: Iterate from element zero rather than forming events[-1].
            code=code.replace('mapEvent = &g_MapOverlayHdr.mapEvents[-1];','mapEvent = g_MapOverlayHdr.mapEvents;')
            code=code.replace('while (true)','for (;; mapEvent++)').replace('        mapEvent++;','        if(mapEvent >= g_MapOverlayHdr.mapEvents+port_maps_event_count())port_unimplemented("map event terminator outside source bounds");')
            code=code.replace('completeEventFlag_temp = mapEvent->completeEventFlag;', 'completeEventFlag_temp = mapEvent->completeEventFlag;')
            code=code.replace('completeEventFlag      = completeEventFlag_temp;', 'completeEventFlag      = (s16)completeEventFlag_temp;')
            # PORT: Preserve the five-entry inventory trigger contract with bounds.
            code=code.replace('for (i = 0; g_SysWork.playerWork.extra.lastUsedItem != g_ItemTriggerItemIds[i]; i++);','for (i = 0; i < 5 && g_SysWork.playerWork.extra.lastUsedItem != g_ItemTriggerItemIds[i]; i++);\n        if(i==5)port_unimplemented("unmatched inventory event item");')
            code=code.replace('for (i = 0; g_ItemTriggerItemIds[i] != NO_VALUE; i++);','for (i = 0; i < 5 && g_ItemTriggerItemIds[i] != NO_VALUE; i++);\n            if(i==5)port_unimplemented("inventory trigger count");')
        code=narrow(code,'')
        if name=='Event_CollideObbCheck':
            # PORT: Preserve PS1 wrapping negation followed by signed angle shift.
            code=code.replace('mapPoint->triggerParam1 << 9 < ABS(deltaX)', '(s32)(mapPoint->triggerParam1 << 9) < ABS(deltaX)')
            code=code.replace('-(mapPoint->triggerParam0 << 20) >> 16', '(s32)(0u - (mapPoint->triggerParam0 << 20)) >> 16')
        source.append(code)
    source.append('bool g_Demo_IsLoadingChunks;\n'+function(read('src/main/fsqueue.c'),'Fs_QueueChunksLoad'))
    border=read('src/bodyprog/screen/cutscene_border.c')
    code=function(border,'Screen_CutsceneCameraStateUpdate')
    code=code.replace('    void Screen_BlackBorderDraw(', 'void Screen_BlackBorderDraw(')
    inner=function(code,'Screen_BlackBorderDraw')
    code=code.replace(inner.rstrip(),'')
    code=re.sub(r'^    (?:GsOT|POLY_G4|DR_MODE)\*\s+(?:ot|poly|drMode);\n','',code,flags=re.M)
    code=re.sub(r'^    (?:drMode|poly|ot)\s*=.*;\n','',code,flags=re.M)
    code=re.sub(r'^    AddPrim[^\n]*\n','',code,flags=re.M)
    code=code.replace('Screen_BlackBorderDraw(poly, ', 'Screen_BlackBorderDraw(')
    code=code.replace('vcSetEvCamRate(g_BlackBorderShade)', 'vcSetEvCamRate((q3_12)g_BlackBorderShade)')
    code=code.replace('vcChangeProjectionValue(g_GameWork.gsScreenHeight + Q12_MULT(377 - g_GameWork.gsScreenHeight, g_BlackBorderShade))', 'vcChangeProjectionValue((q3_12)(g_GameWork.gsScreenHeight + Q12_MULT(377 - g_GameWork.gsScreenHeight, g_BlackBorderShade)))')
    source.append('static q19_12 g_BlackBorderShade;\n// PORT: No-draw border bridge; retain original shade timing and camera projection.\nstatic void Screen_BlackBorderDraw(s32 color) {(void)color;}\n')
    source.append('void CutsceneBorder_Reset(void) {g_SysWork.cutsceneBorderState=CutsceneBorderState_Reset;}\n')
    source.append(code)
    (out/'native_player_loop.h').write_text(''.join(header),encoding='utf-8')
    (out/'player_loop.c').write_text(''.join(source),encoding='utf-8')
    # PORT: Reuse combat's pinned extractor and namespace for the two independent
    # production helpers reached by character scheduling. No harness doubles.
    from prepare_combat import function as combat_function
    combat_code= combat_function(read('src/bodyprog/bodyprog_combat_8008A058.c'),'Chara_Flag8Clear') + combat_function(read('src/bodyprog/events/npc_main.c'),'Chara_DamagedFlagUpdate')
    for name in ['Chara_Flag8Clear','Chara_DamagedFlagUpdate']:
        combat_code=combat_code.replace(name+'(', 'sh_combat_'+name+'(')
    (out/'player_combat.c').write_text('/* Copyright (C) 2026 shdecompilations; SPDX-License-Identifier: GPL-3.0-only. */\n#include "native_player_loop.h"\n'+combat_code,encoding='utf-8')
    combat=read('src/bodyprog/bodyprog_combat_8008A058.c')
    # PORT: Link the original empty-attack cleanup path. Active attacks retain
    # an explicit boundary until the entire migrated weapon slice has providers.
    idle='s32 func_8008A3E0(s_SubCharacter* chara) {s32 sp10,sp14,sp30;\n'+between(function(combat,'func_8008A3E0'),'    sp30 = NO_VALUE;','    sp18 = 0;')
    idle+='(void)sp14;port_unimplemented("func_8008A3E0/active combat providers");return 0;}\n'
    combat_code+=function(combat,'Chara_Flag8Set')+idle+function(combat,'func_8008A3AC')
    setup=function(combat,'func_8008A0E4').replace('D_800297B8','HARRY_BASE_ANIM_INFOS')
    setup=setup.replace('{','{\n    (void)unused;',1).replace('chara->field_44.field_2 = weaponAttack;', 'chara->field_44.field_2 = (s8)weaponAttack;')
    combat_code+=setup
    (out/'player_combat.c').write_text('/* Copyright (C) 2026 shdecompilations; SPDX-License-Identifier: GPL-3.0-only. */\n#include "native_player_movement.h"\n'+combat_code,encoding='utf-8')


def prepare_player_collision(decomp,out):
    """Complete original wall-response closure over the existing native IPD graphs."""
    from prepare_camera import narrow
    from prepare_maps import enumeration
    def read(path):return (decomp/path).read_text(encoding='utf-8')
    texts=[read('src/bodyprog/collision/collision.c'),read('src/bodyprog/collision/trigger.c')]
    defs={name:function(text,name).removeprefix('static ') for text in texts for name in re.findall(r'^(?:static\s+)?(?:inline\s+)?\w+\s*\**\s+(\w+)\([^;{}]*\)[^{;]*\{',text,re.M)}
    wanted={'Collision_WallDetect','Collision_NearbyTriggersGet','Collision_FlagsGet'}
    while True:
        expanded=wanted|{n for caller in wanted for n in re.findall(r'\b(\w+)\s*\(',defs[caller]) if n in defs}
        if expanded==wanted:break
        wanted=expanded
    existing=(out/'collision_consumers.c').read_text(encoding='utf-8')
    existing_names=set(re.findall(r'^\w+\s*\**\s+(\w+)\(',existing,re.M))
    header=(out/'collision.h').read_text()
    trigger=read('include/bodyprog/collision/trigger.h')
    work=between(trigger,'typedef struct\n','/** @brief World-space collision trigger')
    work+='STATIC_ASSERT_SIZEOF(s_func_8006F338,48);\n'
    flags=enumeration(read('include/bodyprog/collision/collision.h'),'CollisionTriggerFlags').replace('CollisionTriggerFlag_Map','PortUnused_CollisionTriggerFlag_Map')
    addition=flags+work+'#define INTERSECTION_BUFFER Q12(0.1f)\nextern s_WorldMapWork g_WorldMapWork;\ns_IpdCollisionData** WorldMap_ActiveChunksCollisionDataGet(s32*);\n'
    source='/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations. */\n#include "collision.h"\n#define TRIGGER_HEIGHT_GET(steps) ((-Q12(steps)>>1)-Q12(1.5f))\n'
    for name,code in defs.items():
        if name not in wanted or name in existing_names:continue
        # PORT: Native stack frames replace PS1 scratch-stack switching.
        code=re.sub(r'^\s*s32\s+stackPtr;\n','\n',code,flags=re.M)
        code=re.sub(r'^\s*stackPtr = SetSp\([^\n]+\n','\n    // PORT: Retain the native stack; never load a PS1 stack address.\n',code,flags=re.M)
        code=re.sub(r'^\s*SetSp\(stackPtr\);\n','\n',code,flags=re.M)
        code=code.replace('return &collCharas;','return collCharas;').replace('curCollChara    = &collCharas;','curCollChara    = collCharas;')
        if name in ['Collision_CharaCollisionSetup','func_8006A42C']:
            # PORT: Sequence original output-parameter providers before reading
            # their counts; native C argument evaluation order is unspecified.
            prefix='s_IpdCollisionData** activeData=WorldMap_ActiveChunksCollisionDataGet(&collDataIdx);\n    '
            code=code.replace('WorldMap_ActiveChunksCollisionDataGet(&collDataIdx)', 'activeData')
            if name=='Collision_CharaCollisionSetup':
                prefix+='s_SubCharacter** activeCharas=Collision_CollidableCharasGet(&charaCount,chara,true);\n    '
                code=code.replace('Collision_CollidableCharasGet(&charaCount, chara, true)', 'activeCharas')
            code=code.replace('return func_8006A4A8',prefix+'return func_8006A4A8')
        if name=='func_8006F250':code=code.replace('ptr = PSX_SCRATCH;','s_func_8006F338 nativeWork;\n    ptr = &nativeWork; // PORT: Typed native scratch, no overlap with other live work.')
        if name=='Collision_NearbyTriggersGet':
            code=code.replace('g_ActiveCollisionTriggers.collisionTriggers[g_ActiveCollisionTriggers.collisionTriggerCount] = curTrigger;', 'if(g_ActiveCollisionTriggers.collisionTriggerCount>=20)port_unimplemented("nearby collision trigger capacity");\n            g_ActiveCollisionTriggers.collisionTriggers[g_ActiveCollisionTriggers.collisionTriggerCount] = curTrigger;')
        if name=='Collision_WallResponse':code=code.replace('{','{\n    (void)moveOffset;',1)
        if name=='Collision_WallResponse':code=code.replace('q19_12             groundHeight;', 'q19_12             groundHeight=0; // PORT: Used only after its paired groundType is set.')
        if name=='func_8006D2B4':code=code.replace('q3_12  rotY1;','q3_12  rotY1=0; // PORT: Only consumed when the matching range is populated.').replace('q3_12  rotX1;','q3_12  rotX1=0;')
        view=re.sub(r'\b(?:s16|s32|q3_12|q7_8|s8|u8)\s+(?:v[xyz]|field_0|groundHeight|ceilingHeight)\s*;', '',header+work)
        code=narrow(code,view)
        code=code.replace('arg1->field_0 = (s8)(arg1->field_30);','arg1->field_0 = arg1->field_30;').replace('arg1->field_0 = (s8)(arg1->field_8);','arg1->field_0 = arg1->field_8;')
        code=code.replace('chara->collision.state < (u32)state.charaState.collisionState','chara->collision.state < state.charaState.collisionState')
        code=code.replace('curArg1->collisionState < (u32)state->charaState.collisionState','curArg1->collisionState < state->charaState.collisionState')
        code=code.replace('state->field_A0.s_1.field_8','(s8*)state->field_A0.s_1.field_8')
        code=re.sub(r'\(s8\*\)(state->field_A0.s_1.field_8\s*=)',r'\1',code)
        code=code.replace('state->charaState.distance,','(q3_12)state->charaState.distance,')
        code=code.replace('state->charaPositionFrom.vx - state->charaState.positionFromX,','(q3_12)(state->charaPositionFrom.vx - state->charaState.positionFromX),')
        code=code.replace('state->charaPositionTo.vz   - state->charaState.positionFromZ,','(q7_8)(state->charaPositionTo.vz - state->charaState.positionFromZ),')
        code=code.replace('2, deltaX, deltaZ, temp3)', '2, (q7_8)deltaX, (q7_8)deltaZ, (q7_8)temp3)')
        code=code.replace('func_8006C1B8(1, temp_v0, state)', 'func_8006C1B8(1, (q3_12)temp_v0, state)')
        for target,typ in [(r'collResult->surface.groundType','s8'),(r'collResult->surface.tiltAngle[ZX]','s16'),(r'charaState->direction.v[xz]','s16')]:
            code=re.sub(r'('+target+r')\s*=(?!=)\s*([^;{}]+);',lambda m:m[1]+' = ('+typ+')('+m[2]+');',code)
        addition+=re.sub(r'//[^\n]*','',code.split('{',1)[0]).strip()+';\n'
        source+=code
    # The original wrapper consumes the current descriptor's live trigger array.
    source+='void World_NearbyPlayerCollisionTriggersGet(void) {Collision_NearbyTriggersGet(g_SysWork.playerWork.player.position.vx,g_SysWork.playerWork.player.position.vz,g_WorldGfxWork.collisionTriggers);}\n'
    active=function(read('src/bodyprog/world/world_map.c'),'WorldMap_ActiveChunksCollisionDataGet')
    active=active.replace('activeChunksCollData[(*collDataIdx)++] = collData;', 'if(*collDataIdx>=4)port_unimplemented("active collision chunk capacity");\n                    activeChunksCollData[(*collDataIdx)++] = collData;')
    # PORT: Keep the world's original private workspace private; append this
    # original accessor in its owning generated translation unit.
    world=out/'world_consumers.c'
    world.write_text(world.read_text(encoding='utf-8')+'\n'+active,encoding='utf-8')
    # PORT: The same pinned SDK normalization table/kernel returns Q12 sqrt
    # by keeping six more fractional result bits than SquareRoot0.
    math=out/'native_math.c'
    math_source=math.read_text(encoding='utf-8')
    root12=function(math_source,'SquareRoot0').replace('SquareRoot0(','SquareRoot12(').replace(')>>12)',')>>6)')
    math.write_text(math_source+'\n'+root12,encoding='utf-8')
    (out/'collision.h').write_text(header.replace('#endif',addition+'#endif'))
    (out/'player_collision.c').write_text(source)


def prepare_player_movement(decomp, out):
    """Original player delegate closure; no replacement movement algorithms."""
    from prepare_maps import initializer, enumeration
    from prepare_camera import narrow
    def read(path): return (decomp/path).read_text(encoding='utf-8')
    original = read('src/bodyprog/player_control.c')
    player_header = read('include/bodyprog/player.h')
    function_names = re.findall(r'^(?:static\s+)?(?:inline\s+)?\w+\s*\*?\s+(\w+)\([^;{}]*\)[^{;]*\{', original, re.M)
    provided = {'Game_SavegameResetPlayer','func_8007E9C4'}
    for source in list(out.glob('*.c')) + list((Path(__file__).resolve().parents[1]/'port').glob('*.c')):
        # Existing delegate guards are replaced, never counted as providers.
        if source.name == 'player_movement.c' or (source.name == 'player_loop.c' and source.parent.name == 'port'): continue
        provided.update(re.findall(r'^(?:static\s+)?(?:inline\s+)?\w+\s*\*?\s+(\w+)\([^;{}]*\)[^{;]*\{',source.read_text(encoding='utf-8'),re.M))
    cutoff = original.index('void Game_SavegameResetPlayer')
    wanted = {name for name in function_names if original.index(function(original,name)) < cutoff and name not in provided}
    wanted.add('func_8007FC48')
    # Follow player-local helpers, but retain the already migrated world services.
    while True:
        expanded = wanted | {name for caller in wanted for name in re.findall(r'\b(\w+)\s*\(',function(original,caller)) if name in function_names and name not in provided}
        if expanded == wanted: break
        wanted = expanded
    codes = {name:function(original,name).replace('static inline ', '') for name in function_names if name in wanted}
    text = ''.join(codes.values())
    notice = '/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations. */\n'
    header = notice+'#ifndef SH_PLAYER_MOVEMENT_H\n#define SH_PLAYER_MOVEMENT_H\n#include "native_player_loop.h"\n#include "collision.h"\n'
    header += enumeration(player_header,'PlayerFlags').replace('1 << 31','(s32)0x80000000u')
    header += between(player_header,'// Used in player lower body state handling.', '// ========\n// GLOBALS')
    body = read('include/bodyprog/bodyprog.h')
    attacks = between(body,'/** Related to weapon attacks.','/** @brief Radio noise')
    header += attacks.replace('STATIC_ASSERT_SIZEOF(s_800AD4C8, 24);','// PORT: Native auxiliary pointer; disk attacks remain 24 bytes.\nSTATIC_ASSERT_SIZEOF(s_800AD4C8, 32);')
    header += between(read('include/bodyprog/collision/ray.h'),'/** @brief Ray trace line','bool Ray_TraceQuery')
    sfx_enum = enumeration(read('include/bodyprog/sound/sfx_id_enum.h'),'SfxId')
    for symbol in re.findall(r'\bSfx_Menu\w+\b',sfx_enum): sfx_enum=sfx_enum.replace(symbol,'PortUnused_'+symbol)
    header += sfx_enum+enumeration(body,'SfxFlags')
    type_start=body.rfind('typedef struct',0,body.index('} s_800C44F0;'))
    header += body[type_start:body.index('} s_800C44F0;')+len('} s_800C44F0;')]+'\n'
    header += '#define TIMESTEP_30_FPS Q12(1.0f/30.0f)\n#define BITMASK_RANGE(a,b) (((1u<<((b)-(a)+1))-1u)<<(a))\n'
    header += '#define HARRY_UPPER_BODY_BONE_MASK BITMASK_RANGE(HarryBone_Root,HarryBone_RightHand)\n#define HARRY_LOWER_BODY_BONE_MASK BITMASK_RANGE(HarryBone_Hips,HarryBone_RightFoot)\n'
    header += between(player_header,'#define Player_ExtraStateSet','/** @brief Sets the given animation flag')
    header += 'extern u8 g_Player_AnimResetRequest;\n'
    header += function(player_header,'Player_AnimFlagsSet')+function(player_header,'Player_AnimStateReset')
    header += between(read('include/bodyprog/chara/chara.h'),'#define Chara_DamageClear','/** @brief Sets a character')
    # All globals retain one native owner. Arrays with source initializers are GPL data.
    globals_code = ''
    existing = '\n'.join(source.read_text(encoding='utf-8') for source in list(out.glob('*.c'))+list((Path(__file__).resolve().parents[1]/'port').glob('*.c')) if source.name != 'player_movement.c')
    reset_globals = set(re.findall(r'\bg_Player_\w+',function(original,'func_8007E9C4')))
    for name in sorted(set(re.findall(r'\b(?:g_Player_\w+|D_800\w+)\b',text))):
        declaration = re.search(r'(?:extern\s+)?(\w+)\s+'+name+r'\s*(\[[^;=]*\])?\s*(?:=[^;]*|);',player_header+'\n'+original+'\n'+body)
        if not declaration: raise ValueError('missing movement global '+name)
        typ, array = declaration[1], declaration[2] or ''
        typ = {'g_Player_DisableControl':'bool','D_800C4588':'s16'}.get(name,typ)
        header += f'extern {typ} {name}{array};\n'
        if name in reset_globals or re.search(r'\b\w+\s+'+name+r'\s*(?:\[[^;]*?\])?\s*(?:=|;)',existing): continue
        if array:
            if re.search(r'^'+typ+r'\s+'+name+r'\s*\[[^;]*?=',original,re.M):
                globals_code += initializer(original,name)
            else:
                globals_code += f'{typ} {name}{array};\n'
        else:
            match = re.search(r'^'+typ+r'\s+'+name+r'\s*=[^;]*;',original,re.M)
            globals_code += (match[0] if match else f'{typ} {name};')+'\n'
    prototypes = ''
    for name,code in codes.items():
        prototypes += re.sub(r'//[^\n]*','',code.split('{',1)[0]).strip()+';\n'
    # Obtain exact declarations for services, including guarded leaves, from source.
    headers = '\n'.join(path.read_text(encoding='utf-8') for path in (decomp/'include').rglob('*.h'))
    for name in sorted(set(re.findall(r'\b(\w+)\s*\(',text))):
        if name in codes: continue
        prototype = re.search(r'^(?:extern\s+)?(?:void|bool|s32|s16|s8|u32|u16|u8|q19_12|q3_12|s_\w+\s*\*)\s*'+name+r'\([^;{}]*\);',headers,re.M)
        if prototype and name not in ['Rng_Rand16','SD_Call','Player_CombatAnimUpdate']: prototypes += prototype[0]+'\n'
    prototypes += 'bool Player_CombatAnimUpdate(s_SubCharacter*,s_PlayerExtra*);\n'
    header += prototypes+'#endif\n'
    header=header.replace('#endif','void port_move_game_timer_update(void);\nvoid port_move_attack_tables_ensure(void);\n#endif')
    (out/'native_player_movement.h').write_text(header,encoding='utf-8')
    records = (out/'native_gameplay_records.h').read_text()
    source = notice+'#include "native_player_movement.h"\n'+globals_code
    aliases = {'playerChara':'g_SysWork.playerWork.player','playerProps':'g_SysWork.playerWork.player.properties.player','playerExtra':'g_SysWork.playerWork.extra','playerCombat':'g_SysWork.playerCombat'}
    for name,code in codes.items():
        if name=='Player_UpperBodyMainUpdate':
            # PORT: Lift the original GCC nested function with explicit captured inputs.
            code=code.replace('    bool Player_CombatAnimUpdate(void)', 'bool Player_CombatAnimUpdate(void)')
            inner=function(code,'Player_CombatAnimUpdate')
            code=code.replace(inner.rstrip(),'').replace('    static s32 D_800C44D0;','').replace('    static s32 D_800C44D4;','')
            inner=inner.replace('Player_CombatAnimUpdate(void)','Player_CombatAnimUpdate(s_SubCharacter* player,s_PlayerExtra* extra)')
            inner=inner.replace('{','{\n    s32 enemyAttackedIdx;',1)
            code=inner+'\n'+code.replace('Player_CombatAnimUpdate()', 'Player_CombatAnimUpdate(player,extra)')
        code=code.replace('g_SysWork.field_2388.field_154','g_SysWork.gameplayEnvironment.field_154')
        code=code.replace('Game_TimerUpdate();','port_move_game_timer_update();')
        if name=='Player_ReceiveDamage':code=code.replace('{','{\n    port_move_attack_tables_ensure();',1)
        if name=='Player_FootstepSfxGet':
            # PORT: The reference inline assembly only reloads the live map index.
            code=re.sub(r'asm volatile\(.*?: "memory"\);','mapIdx = g_SavegamePtr->mapIdx;',code,flags=re.S)
        if name=='Player_LogicUpdate':
            code=code.replace('q3_12         headingAngle0;', 'q3_12         headingAngle0=0; // PORT: The undefined reference grab branch below remains guarded.')
            code=code.replace('q3_12         headingAngle1;', 'q3_12         headingAngle1=0; // PORT: The impossible pinned-grab state remains guarded below.')
            code=code.replace('Math_RotMatrixZxyNegGte(&player->rotation, &coords->coord);','// PORT: Actor rotation is six bytes; the SDK argument is padded.\n    SVECTOR nativeRotation={player->rotation.vx,player->rotation.vy,player->rotation.vz,0};\n    Math_RotMatrixZxyNegGte(&nativeRotation, &coords->coord);')
            code=code.replace('s_Model**     models;', '/* PORT: Remove a match-only uninitialized read whose body adds zero. */')
            code=re.sub(r'if \(\(\*models\) != NULL\).*?\{\s*g_Player_HeadingAngle \+= Q12_ANGLE\(0.0f\);\s*}', '',code,flags=re.S)
            code=code.replace('if (ABS(headingAngle0) <','// PORT: This reference grab branch reads an undefined angle before computing it.\n            port_unimplemented("Player_LogicUpdate/Romper grab undefined reference angle");\n            if (ABS(headingAngle0) <')
            # The switch handles exactly these original states; reject any other state.
            code=code.replace('                    break;\n            }\n\n            g_Player_HeadingAngle = headingAngle1;', '                    break;\n                default: port_unimplemented("Player_LogicUpdate/pinned grab state");\n            }\n\n            g_Player_HeadingAngle = headingAngle1;')
        if name=='Player_PositionUpdate':code=code.replace('VECTOR3            sp30;', 'VECTOR3            sp30={0}; // PORT: Only the school boss branch consumes this copied offset.')
        if name=='Player_CombatUpdate':
            code=re.sub(r'VECTOR\*\s+(attackPos[012])',r'VECTOR3* \1',code)
            code=re.sub(r'VECTOR\s+(blasterBeamFrom|partBeamFrom|partBeamTo);',r'VECTOR3 \1;',code)
            code=code.replace('g_MapOverlayHdr.particleHyperBlasterBeamDraw(&blasterBeamFrom, &rotToAttackPos.vx, &rotToAttackPos.vy);','// PORT: Original callback takes word angles; copy SDK halfwords explicitly.\n                q19_12 beamY=rotToAttackPos.vx,beamX=rotToAttackPos.vy;\n                g_MapOverlayHdr.particleHyperBlasterBeamDraw(&blasterBeamFrom,&beamY,&beamX);')
        if name=='func_8007F95C':code=code.replace('u16             sp30;','q3_12          sp30;')
        if name=='func_8007D090':code=code.replace('{','{\n    (void)extra;',1)
        code=code.replace('Math_ShortestAngleGet(player->rotation.vy, temp_s1_2,','Math_ShortestAngleGet(player->rotation.vy, (q3_12)temp_s1_2,').replace('Math_ShortestAngleGet(player->angleToTarget, temp_s1_2,','Math_ShortestAngleGet(player->angleToTarget, (q3_12)temp_s1_2,')
        for alias,target in aliases.items(): code = re.sub(r'(?<!\.)\b'+alias+r'\b',target,code)
        # Vector words and the character's word moveSpeed must not inherit the
        # same-named halfword fields from SVECTOR/PropsPlayer declarations.
        view=re.sub(r'\b(?:s16|s32|q3_12|q4_12)\s+(?:vx|vy|vz|moveSpeed)\s*;', '',records+player_header)
        # PORT: Generic field_N names occur in unrelated records of different
        # widths. Narrow their stores only through the actual player record.
        view=re.sub(r'\b(q3_12|q7_8|q11_4|u8|s8|s16|u16)\s+(field_\w+)\s*;',r'\1 port_hidden_\2;',view)
        code = narrow(code,view)
        for record,prefix in [('s_PropsPlayer',r'(?:player->properties.player|g_SysWork.playerWork.player.properties.player)\.'),('s_SubCharacter',r'(?:player|chara)->'),('s_PlayerExtra',r'(?:extra->|g_SysWork.playerWork.extra\.)')]:
            end=records.index('} '+record+';')
            start=records.rfind('typedef struct',0,end)
            for typ,field in re.findall(r'\b(q3_12|q7_8|q11_4|u8|s8|s16|u16)\s+(field_\w+)\s*;',records[start:end]):
                def record_store(m):
                    lhs,op,rhs=m[1],m[2],m[3]
                    expr=rhs if op=='=' else lhs+' '+op[0]+' ('+rhs+')'
                    return lhs+' = ('+typ+')('+expr+');'
                code=re.sub(r'\b('+prefix+field+r')\s*(\+=|-=|=(?!=))\s*([^;{}]+);',record_store,code)
        for target,typ in [(r'(?:player->properties.player|g_SysWork.playerWork.player.properties.player)\.moveSpeed','q3_12'),(r'player->collision.cylinder.field_2','q3_12'),(r'\w+(?:->|\.)(?:rotationSpeed|collision.shapeOffsets\.(?:box|cylinder))\.v[xyz]','s16'),(r'g_SysWork.playerWork.player.collision.shapeOffsets\.(?:box|cylinder)\.v[xyz]','s16'),(r'rotToAttackPos\.v[xy]','s16')]:
            def target_store(m):
                lhs,op,rhs=m[1],m[2],m[3]
                expr=rhs if op=='=' else lhs+' '+op[0]+' ('+rhs+')'
                return lhs+' = ('+typ+')('+expr+');'
            code=re.sub(r'\b('+target+r')\s*(\+=|-=|=(?!=))\s*([^;{}]+);',target_store,code)
        # PORT: Retain original global halfword/byte writes and signed angle stores.
        for symbol,typ in [('g_Player_FlexRotationX','q3_12'),('g_Player_FlexRotationY','q3_12'),('g_SysWork.targetNpcIdx','s8'),('D_800AF220','u8'),('enemyRotY','q4_12')]:
            def global_store(m):
                op,rhs=m[1],m[2]
                expr=rhs if op=='=' else symbol+' '+op[0]+' ('+rhs+')'
                return symbol+' = ('+typ+')('+expr+');'
            code=re.sub(r'\b'+re.escape(symbol)+r'\s*(\+=|-=|=(?!=))\s*([^;{}]+);',global_store,code)
        source += code
    (out/'player_movement.c').write_text(source,encoding='utf-8')
    loop = out/'player_loop.c'
    code = loop.read_text().replace('#include "native_player_loop.h"','#include "native_player_movement.h"')
    loop.write_text(code)
    prepare_player_services(decomp,out)
    prepare_npc_loop(decomp,out)
    prepare_player_rays(decomp,out)
    prepare_player_effects(decomp,out)
    prepare_player_dms(decomp,out)
    prepare_npc_models(decomp,out)
    prepare_player_events(decomp,out)
    loop=out/'player_loop.c'
    text=loop.read_text(encoding='utf-8')
    watched={'Player_Update','Player_ReceiveDamage','Player_LogicUpdate','Player_PositionUpdate','Player_AnimUpdate','func_8007D090','Gfx_EffectsUpdate','WorldGfx_CharaDraw','Player_CombatUpdate','func_8008A3AC','Game_NpcRoomInitSpawn','Game_NpcUpdate','func_8005E89C','WorldGfx_CloseRangeChunksInit','WorldGfx_Draw','vcMoveAndSetCamera','World_NearbyPlayerCollisionTriggersGet'}
    lines=[]
    for line in text.splitlines():
        match=re.match(r'^(\s*)(\w+)\(',line)
        if match and match[2] in watched:
            lines.append(match[1]+'port_move_stage("'+match[2]+'");')
        lines.append(line)
        if match and match[2] in watched:
            lines.append(match[1]+'port_move_stage("'+match[2]+'/done");')
    loop.write_text('#include "player_trace.h"\n'+'\n'.join(lines)+'\n',encoding='utf-8')


def prepare_player_services(decomp,out):
    """Original small providers and explicit guards at remaining production imports."""
    from prepare_maps import initializer
    from prepare_camera import narrow
    def read(path):return (decomp/path).read_text(encoding='utf-8')
    source='/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations. */\n#include "native_player_movement.h"\n#include <stdio.h>\n'
    source+='u8 g_Player_AnimResetRequest;\n'
    source+=function(read('src/bodyprog/player_control.c'),'Rng_RandQ12')
    source+=narrow(function(read('src/bodyprog/bodyprog_math_8005BF38.c'),'Math_AngleNormalizeSigned'),'')
    source+='void func_800892DC(s32 index,u8 strength) { // PORT: PS1 motor requests have no native motor backend.\n printf("HAPTIC index=%d strength=%u\\n",index,strength);}\n'
    original=read('src/bodyprog/bodyprog_80089090.c')
    for name in ['func_800893D0','func_8008944C','func_80089470','func_80089494']:
        code=function(original,name).replace('func_800892DC(10, var);','func_800892DC(10, (u8)var);')
        source+=code
    original=read('src/bodyprog/world/world_draw.c')
    source+=function(original,'Map_SpeedZoneTypeGet')
    for name in ['func_8003D01C','func_8003D03C']:
        source+=function(original,name).replace('1 << 31','1u << 31')
    mesh=function(original,'WorldGfx_CharaMeshSwap')
    for name in set(re.findall(r'\b(WorldGfx_\w+MeshSwap)\(',mesh))-{'WorldGfx_CharaMeshSwap','WorldGfx_HarryMeshSwap'}:
        if name=='WorldGfx_StalkerMeshSwap':
            source+='enum {StalkerVariantMesh_None=0,StalkerVariantMesh_1=1,StalkerVariantMesh_2=2};\n'
            source+=function(original,name)
        else:source+=f'static void {name}(s_Skeleton* skeleton,s32 status) {{(void)skeleton;(void)status;port_unimplemented("{name}/character mesh variant");}}\n'
    source+=mesh
    code=function(read('src/bodyprog/world/bodyprog_bone_80044F14.c'),'func_80044F14')
    code=code.replace('rot    = PSX_SCRATCH;', 'SVECTOR nativeRotation; MATRIX nativeMatrix;\n    rot=&nativeRotation; // PORT: Typed SDK scratch, independent of PS1 arena addresses.')
    code=code.replace('rotMat = PSX_SCRATCH_ADDR(sizeof(SVECTOR));','rotMat=&nativeMatrix;').replace('rot->vy = rotY;','rot->vy = (s16)rotY;')
    source+=code
    # PORT: Exact pinned libkmath.s RotMatrixZ_Apply: shift each signed product
    # independently before the wrapping add/subtract, preserving row 2 and t.
    source+='''MATRIX* Math_RotMatrixZ(s32 angle,MATRIX* matrix) {
    s32 sine=Math_Sin(angle),cosine=Math_Cos(angle);
    for(s32 i=0;i<3;i++) {
        s32 a=matrix->m[0][i],b=matrix->m[1][i];
        matrix->m[0][i]=(s16)((u32)(((s64)cosine*a)>>12)-(u32)(((s64)sine*b)>>12));
        matrix->m[1][i]=(s16)((u32)(((s64)sine*a)>>12)+(u32)(((s64)cosine*b)>>12));
    }
    return matrix;
}
'''
    items=read('src/bodyprog/items/item_screens_3.c')
    source+=initializer(items,'g_Items_GunsMaxLoadAmmo').replace('NO_VALUE','(u8)NO_VALUE')
    source+=function(items,'Items_AmmoReloadCompute')
    timer=function(read('src/bodyprog/items/item_utils.c'),'Game_TimerUpdate').replace('Game_TimerUpdate(','port_move_game_timer_update(')
    # PORT: The original 290-hour constant is an unsigned Q20.12 value, beyond
    # the signed float-to-word range of the bootstrap Q12 macro.
    timer=timer.replace('Q12((290.0f * 60.0f) * 60.0f)','4276224000u').replace('Q12((130.0f * 60.0f) * 60.0f)','1916928000u').replace('CLAMP(g_SavegamePtr->gameplayTimer, 1,','CLAMP(g_SavegamePtr->gameplayTimer, 1u,').replace('UINT_MAX','0xffffffffu')
    source+=timer
    # PORT: Reuse the combat lane's production decoder on decrypted owned data.
    decoder=(Path(__file__).resolve().parents[1]/'port/sys/combat/combat.c').read_text()
    source+=function(decoder,'half')+function(decoder,'word')+function(decoder,'sh_combat_attack_decode')
    source+=function(decoder,'sh_combat_sqrt_decode')
    source+='''extern int port_move_bodyprog_read(u32,u32,u8*);
static u32 nativeAttackAux;
extern s16 SQRT[192];
void port_move_attack_tables_ensure(void) {
    static bool loaded;
    if(loaded)return;
    u8 bytes[70*24];s_800AD4C8 decoded[70];
    if(port_move_bodyprog_read(0x800AD4C8u-0x80024B60u,sizeof(bytes),bytes))port_unimplemented("combat attack table read");
    for(size_t i=0;i<70;i++)if(!sh_combat_attack_decode(bytes+i*24,24,0x800AD4C4u,&decoded[i],&nativeAttackAux))port_unimplemented("combat attack auxiliary identity");
    u8 sqrtBytes[384];s16 sqrtDecoded[192];
    if(port_move_bodyprog_read(0x800AFFCCu-0x80024B60u,sizeof(sqrtBytes),sqrtBytes) || !sh_combat_sqrt_decode(sqrtBytes,sizeof(sqrtBytes),sqrtDecoded,192))port_unimplemented("combat square-root table read");
    memcpy(SQRT,sqrtDecoded,sizeof(sqrtDecoded));
    memcpy(D_800AD4C8,decoded,sizeof(decoded));loaded=true;
}
'''
    guards=['func_8005CD38','func_8005D50C','func_8006342C']
    header=(out/'native_player_movement.h').read_text(encoding='utf-8')
    for name in guards:
        signature=re.search(r'^((?:void|bool|s32)\s+'+name+r'\([^;{}]*\));',header,re.M)[1]
        params=signature.split('(',1)[1].rsplit(')',1)[0].split(',')
        unused=''.join('(void)'+re.findall(r'\b\w+',p)[-1]+';' for p in params if p.strip()!='void')
        source+=signature+' {'+unused+'port_unimplemented("'+name+'/native movement dependency");'+('return 0;' if not signature.startswith('void ') else '')+'}\n'
    # PORT: NPC rendering enters through move's public character bridge. Use
    # the original draw routine and the original field_6/field_8 metadata;
    # the renderer's earlier Harry-only entry remains independently owned.
    draw=function(read('src/bodyprog/world/world_draw.c'),'WorldGfx_CharaDraw')
    draw=draw.replace('WorldGfx_CharaDraw(', 'port_move_npc_draw(')
    draw=draw.replace('{','{\n    if(charaId<=Chara_None || charaId>=Chara_Count || !g_WorldGfxWork.registeredCharaModels[charaId])port_unimplemented("NPC rendering model publication");',1)
    draw=draw.replace('Q8_TO_Q12(CHARA_FILE_INFOS[charaId].field_6)', '(q3_12)Q8_TO_Q12(CHARA_FILE_INFOS[charaId].field_6)')
    draw=draw.replace('WorldGfx_HeldItemDraw();','port_render_held_item();')
    draw=draw.replace('clutY = WorldGfx_CharaClutYGet(charaId, paletteIdx);','clutY = (s16)WorldGfx_CharaClutYGet(charaId,paletteIdx);')
    source=source.replace('#include "native_player_movement.h"', '#include "native_player_movement.h"\n#include "render_generated.h"\n#include "render_services.h"')+draw
    (out/'player_services.c').write_text(source,encoding='utf-8')
    sfx=read('src/bodyprog/sound/sfx.c')
    sound_source='/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations. */\n#include "native_player_movement.h"\n#include <stdio.h>\n'
    sound_source+=initializer(sfx,'g_Pow2NegFracTable')
    sound_source+='static VECTOR3 g_Sfx_CameraPosition;static VECTOR3* g_Sfx_PlayerPosition;\n'
    sound_source+='void Sfx_WithFlagsAndPitchPlay(e_SfxId,const VECTOR3*,q23_8,s32,s32);\n'
    # PORT: Positional callers use audio's real driver.
    sound_source+='void Sd_SfxWithPitchPlay(u16,s8,u8,s8);\nvoid Sd_SfxAttributesUpdate(u16,s8,u8,s8);\n'
    for name in ['Math_Pow2Neg','Math_Pow2NegClamped','Sfx_DistanceAttenuatedVolumeGet','Sfx_WithFlagsPlay','Sfx_WithFlagsAndPitchPlay','Sfx_WithPitchPlay']:
        code=function(sfx,name)
        code=code.replace('Sd_SfxWithPitchPlay(sfxId, balance, ~adjVol, pitch);','Sd_SfxWithPitchPlay((u16)sfxId,(s8)balance,(u8)~adjVol,(s8)pitch);')
        code=code.replace('Sd_SfxWithPitchPlay(sfxId, balance, ~volCpy, pitch);','Sd_SfxWithPitchPlay((u16)sfxId,(s8)balance,(u8)~volCpy,pitch);')
        code=code.replace('Sd_SfxAttributesUpdate(sfxId, balance, ~adjVol, pitch);','Sd_SfxAttributesUpdate((u16)sfxId,(s8)balance,(u8)~adjVol,(s8)pitch);')
        sound_source+=code
    (out/'player_sfx.c').write_text(sound_source,encoding='utf-8')


def prepare_npc_loop(decomp,out):
    """Preserve original NPC update order, including empty-group gameplay."""
    from prepare_camera import narrow
    from prepare_maps import initializer
    def read(path):return (decomp/path).read_text(encoding='utf-8')
    original=read('src/bodyprog/events/npc_main.c')
    code=function(original,'Game_NpcUpdate')
    # PORT: Lift nested helpers with explicit access to their original frame-local work.
    code=code.replace('    s32 func_800382B0(', 's32 func_800382B0(').replace('    static s32 func_800382EC()', 'static s32 func_800382EC()')
    type_start=code.index('    typedef struct _CloseNpcInfo')
    type_end=code.index('} s_CloseNpcInfo;',type_start)+len('} s_CloseNpcInfo;')
    records=code[type_start:type_end]+'\n'
    code=code[:type_start]+code[type_end:]
    helpers=''
    for name in ['func_800382B0','func_800382EC']:
        inner=function(code,name)
        code=code.replace(inner.rstrip(),'')
        inner=inner.replace('ARRAY_SIZE(closestNpcInfos)','3')
        if name=='func_800382B0':
            inner=inner.replace('s32 bitIdx)', 's32 bitIdx,const s_CloseNpcInfo* closestNpcInfos)')
            code=code.replace('func_800382B0(closeNpcInfoIdx1)','func_800382B0(closeNpcInfoIdx1,closestNpcInfos)')
        else:
            inner=inner.replace('func_800382EC()', 'func_800382EC(const s_CloseNpcInfo* closestNpcInfos,u32* idxBits)')
            signature,body=inner.split('{',1)
            inner=signature+'{'+re.sub(r'\bidxBits\b','(*idxBits)',body)
            code=code.replace('func_800382EC()', 'func_800382EC(closestNpcInfos,&idxBits)')
        helpers+=inner
    code=code.replace('g_SysWork.field_2388.field_154','g_SysWork.gameplayEnvironment.field_154')
    code=code.replace('Chara_Flag8Clear(', 'sh_combat_Chara_Flag8Clear(').replace('Chara_DamagedFlagUpdate(', 'sh_combat_Chara_DamagedFlagUpdate(')
    code=code.replace('    s32             temp2;','')
    code=code.replace('animDataIdx = g_CharaAnimDataIdxs[curNpc->model.charaId];','// PORT: Reject missing graph publication before indexing native slots.\n        if(curNpc->model.charaId<=0 || curNpc->model.charaId>=Chara_Count)port_unimplemented("NPC character identity");\n        animDataIdx = g_CharaAnimDataIdxs[curNpc->model.charaId];\n        if(animDataIdx<0 || animDataIdx>=CHARA_GROUP_COUNT || !g_CharaModelAnimsData[animDataIdx].activeAnmHdr || !g_CharaModelAnimsData[animDataIdx].boneCoords)port_unimplemented("NPC animation graph publication");')
    view=re.sub(r'\b(?:s16|s32|q3_12)\s+v[xyz]\s*;', '',(out/'native_gameplay_records.h').read_text())
    code=narrow(code,view)
    for lhs,typ in [('closestNpcInfos[j].bitIdx','s8'),('g_RadioNoise[l].idx','s8'),('g_RadioNoise[l].closeNpcInfoIdx','s8'),('vol','u8')]:
        code=re.sub(re.escape(lhs)+r'\s*=(?!=)\s*([^;{}]+);',lambda m:lhs+' = ('+typ+')('+m[1]+');',code)
    code=code.replace('curDistToNpc >= closestNpcInfos[j].distanceToNpc','curDistToNpc >= (u32)closestNpcInfos[j].distanceToNpc')
    code=code.replace('curDistToNpcCpy > ((', 'curDistToNpcCpy > (u32)((')
    code=code.replace('k < ARRAY_SIZE(g_SysWork.npcs)', 'k < (s32)ARRAY_SIZE(g_SysWork.npcs)').replace('l < ARRAY_SIZE(g_RadioNoise)', 'l < (s32)ARRAY_SIZE(g_RadioNoise)')
    code=code.replace('Sd_SfxAttributesUpdate(Sfx_RadioInterferenceLoop + l, balance, vol, 0);','Sd_SfxAttributesUpdate((u16)(Sfx_RadioInterferenceLoop+l),(s8)balance,vol,0);')
    code=code.replace('Sd_SfxStop(Sfx_RadioInterferenceLoop + l);','Sd_SfxStop((u16)(Sfx_RadioInterferenceLoop+l));')
    body=read('include/bodyprog/bodyprog.h')
    radio=between(body,'typedef struct _RadioNoise','STATIC_ASSERT_SIZEOF(s_RadioNoise, 4);')+'STATIC_ASSERT_SIZEOF(s_RadioNoise,4);\n'
    source='/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations. */\n#include "native_player_movement.h"\n'+radio+'extern s_RadioNoise g_RadioNoise[2];\n#define ItemToggleFlag_RadioOn (1<<0)\ns32 g_RadioPitchState;\nu16 g_CollisionTriggerFlags;\n'
    source+=initializer(read('src/bodyprog/game_boot/fs_chara_anim.c'),'g_CharaAnimDataIdxs').replace('0xFF','-1')
    source+='void Collision_FlagsLocationUpdate(const s_SubCharacter*);void Collision_FlagsUpdate(void);\nvoid func_80037E78(s_SubCharacter*);\n'
    source+='void Sd_SfxAttributesUpdate(u16,s8,u8,s8);\n#define CLEAR_FLAG(p,n) (((u32*)(p))[(n)>>5]&=~(1u<<((n)&31)))\n'
    source+=function(read('include/game.h'),'SysWork_NpcFlagClear')
    source+=records+helpers+function(original,'Camera_Distance2dGet').replace('q25_6','s32')+code
    for name in ['Collision_FlagsLocationUpdate','Collision_FlagsUpdate']:
        source+=narrow(function(read('src/bodyprog/world/world_draw.c'),name),'')
    source+='void func_80037E78(s_SubCharacter* chara) {if(chara->health<=0 && (chara->flags&(CharaFlag_Damaged|CharaFlag_Dead))==CharaFlag_Damaged)port_unimplemented("NPC death statistics integration");}\n'
    (out/'npc_loop.c').write_text(source,encoding='utf-8')


def prepare_player_rays(decomp,out):
    """Original production ray queries over the same active collision graphs."""
    from prepare_camera import narrow
    from prepare_maps import enumeration
    def read(path):return (decomp/path).read_text(encoding='utf-8')
    original=read('src/bodyprog/collision/ray.c')
    ray_header=read('include/bodyprog/collision/ray.h')
    records=between(ray_header,'typedef struct\n','/** @brief Ray trace line')
    # PORT: PS1 used trailing scratch memory after this nominal one-entry array.
    # Reserve a bounded native range arena rather than writing past a C object.
    records=records.replace('field_8C[1]','field_8C[128]').replace('field_20[2]','field_20[128]')
    names=re.findall(r'^\w+\s+(\w+)\([^;{}]*\)[^{;]*\{',original,re.M)
    header='/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations. */\n#ifndef SH_PLAYER_RAYS_H\n#define SH_PLAYER_RAYS_H\n#include "native_player_movement.h"\n'+records
    header+=enumeration(read('include/bodyprog/bodyprog.h'),'OrientationFlags')
    header+='''// PORT: Numeric work aliases preserve the reference's contiguous ray bounds.
_Static_assert(offsetof(s_RayState,field_8C)-offsetof(s_RayState,field_6C)==32,"native ray numeric bounds");
_Static_assert(sizeof(s_func_8006E490)==544,"native ray bounds work");
_Static_assert(offsetof(s_RayState,field_8C)+sizeof(((s_RayState*)0)->field_8C)-offsetof(s_RayState,field_6C)==sizeof(s_func_8006E490),"native complete ray tail");
#define gte_stMAC0() ((s32)port_gte_read_data(24))
#define gte_ldsv3_(x,y,z) do {port_gte_write_data(9,(u32)(x));port_gte_write_data(10,(u32)(y));port_gte_write_data(11,(u32)(z));} while(0)
'''
    sources=[]
    for name in names:
        code=function(original,name)
        if name in ['Ray_TraceQuery','Ray_CharaTraceQuery','Ray_LosHitCheck','func_8006DC18']:
            code=re.sub(r'^\s*s32\s+prevScratch(?:Addr)?;\n','\n',code,flags=re.M)
            code=re.sub(r'^\s*s_RayState\*\s+state;','\n    s_RayState nativeState={0}; // PORT: Native scratch remains live for the entire query.\n    s_RayState* state=&nativeState;',code,flags=re.M)
            code=code.replace('(s_RayState*)PSX_SCRATCH','state')
            code=code.replace('(s32)PSX_SCRATCH','state')
            code=re.sub(r'^\s*prevScratch(?:Addr)?\s*= SetSp[^\n]*\n','\n',code,flags=re.M)
            code=re.sub(r'^\s*SetSp\(prevScratch(?:Addr)?\);\n','\n',code,flags=re.M)
        code=code.replace('state->field_8C[state->field_88]', 'state->field_8C[state->field_88]')
        # Range producers must fit the explicitly owned native trailing arena.
        code=code.replace('state->field_88++;','if(state->field_88>=128)port_unimplemented("ray range capacity");\n        state->field_88++;')
        view=re.sub(r'\b(?:s16|s32|q3_12|q7_8|s8|u8)\s+(?:v[xyz]|field_0|groundHeight|hitDistance)\s*;', '',records)
        code=narrow(code,view)
        code=code.replace('curUnk = &state->field_8C;', 'curUnk = state->field_8C;')
        code=code.replace('idx < collData->subcellCount','(u32)idx < collData->subcellCount')
        if name=='func_8006E490':
            code=code.replace('arg0->field_20[arg0->field_1C].vx =', 'if(arg0->field_1C<0 || arg0->field_1C>=128)port_unimplemented("ray bounds capacity");\n        arg0->field_20[arg0->field_1C].vx =')
        code=code.replace('func_8006E150(&state->field_6C, ((DVECTOR*)&state->offset)[0], ((DVECTOR*)&state->offset)[1]);','// PORT: Copy the asserted numeric alias into correctly typed local work.\n    s_func_8006E490 nativeBounds; DVECTOR packedOffsets[2];\n    memcpy(&nativeBounds,&state->field_6C,sizeof(nativeBounds));\n    memcpy(packedOffsets,&state->offset,sizeof(packedOffsets));\n    func_8006E150(&nativeBounds,packedOffsets[0],packedOffsets[1]);\n    memcpy(&state->field_6C,&nativeBounds,sizeof(nativeBounds));')
        for target,typ in [(r'state->offset.v[xyz]','s16'),(r'trace->headingAngle','q3_12'),(r'trace->groundType','u8'),(r'state->field_6C.groundHeight','q7_8'),(r'subroutine_arg4.vy','s16'),(r'arg2.vx','s16'),(r'state->hitDistance','q7_8'),(r'arg0->groundHeight','q7_8')]:
            code=re.sub(r'('+target+r')\s*=(?!=)\s*([^;{}]+);',lambda m:m[1]+' = ('+typ+')('+m[2]+');',code)
        code=code.replace('state->field_6C.positionX - state->from.vx,', '(q3_12)(state->field_6C.positionX - state->from.vx),')
        code=code.replace('state->field_6C.positionZ - state->from.vz,', '(q7_8)(state->field_6C.positionZ - state->from.vz),')
        code=code.replace('bound));','(q7_8)bound));')
        header+=re.sub(r'//[^\n]*','',code.split('{',1)[0]).strip()+';\n'
        sources.append(code)
    header+='#endif\n'
    (out/'native_player_rays.h').write_text(header)
    (out/'player_rays.c').write_text('/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations. */\n#include "native_player_rays.h"\n'+''.join(sources))


def prepare_player_effects(decomp,out):
    """Original shared effect scheduling; active missing renderers stay guarded."""
    from prepare_camera import narrow
    def read(path):return (decomp/path).read_text(encoding='utf-8')
    body=read('include/bodyprog/bodyprog.h')
    records=''
    for name in ['s_800C42E8','s_func_8005E89C']:
        end=body.index('} '+name+';')+len('} '+name+';')
        start=body.rfind('typedef struct',0,end)
        records+=body[start:end]+'\n'
    original=read('src/bodyprog/gfx/bodyprog_effects_8005E0DC.c')
    code=function(original,'func_8005E89C')
    code=code.replace('ptr = PSX_SCRATCH;', 's_func_8005E89C nativeWork;\n    ptr = &nativeWork; // PORT: Typed native scratch has the original numeric fields.')
    code=code.replace('g_SysWork.field_2388.isFlashlightUnavailable','g_SysWork.gameplayEnvironment.isFlashlightUnavailable')
    code=narrow(code,records)
    code=re.sub(r'(g_MapOverlayHdr.field_(?:5C|7C)->field_10)\s*=(?!=)\s*([^;{}]+);',lambda m:m[1]+' = (q3_12)('+m[2]+');',code)
    # Arrays of original halfwords have explicit PS1 narrowing at each write.
    code=re.sub(r'(\bptr->(?:field_34|field_64|field_94|field_DC|field_E4)\[[^]]+\])\s*=(?!=)\s*([^;{}]+);',lambda m:m[1]+' = (s16)('+m[2]+');',code)
    code=re.sub(r'(\bptr->u_field_(?:EC|FC).field_0\[[^]]+\].v[xy])\s*=(?!=)\s*([^;{}]+);',lambda m:m[1]+' = (s16)('+m[2]+');',code)
    code=re.sub(r'(D_800C42E8\[i\].field_2)\s*\+=\s*([^;{}]+);',lambda m:m[1]+' = (s16)('+m[1]+' + ('+m[2]+'));',code)
    source='/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations. */\n#include "native_player_movement.h"\n'+records+'extern s8 D_800C4414;\ns_800C42E8 D_800C42E8[24];\n'
    for name in ['func_80060044','func_800611C0','func_80062708','func_80063A50','func_80064334','func_80064FC0']:
        signature=re.sub(r'//[^\n]*','',function(original,name).split('{',1)[0]).strip()
        params=signature.split('(',1)[1].rsplit(')',1)[0].split(',')
        unused=''.join('(void)'+re.findall(r'\b\w+',p)[-1]+';' for p in params)
        source+=signature+' {'+unused+'port_unimplemented("'+name+'/active effect renderer");'+('return false;' if signature.startswith('bool ') else '')+'}\n'
    source+=code
    (out/'player_effects.c').write_text(source,encoding='utf-8')


def prepare_player_dms(decomp,out):
    """Original cutscene interpolation over explicitly decoded native DMS owners."""
    from prepare_camera import narrow
    original=(decomp/'src/bodyprog/dms.c').read_text(encoding='utf-8')
    header=(decomp/'include/bodyprog/dms.h').read_text(encoding='utf-8')
    header=header.replace('STATIC_ASSERT_SIZEOF(s_DmsEntry, 16);','// PORT: Two native pointers; the wire entry stays 16 bytes.\nSTATIC_ASSERT_SIZEOF(s_DmsEntry,24);')
    header=header.replace('STATIC_ASSERT_SIZEOF(s_DmsHeader, 44);','// PORT: Native segments/entries and embedded native camera entry.\nSTATIC_ASSERT_SIZEOF(s_DmsHeader,64);')
    header=header.replace('#define _BODYPROG_DMS_H','#define _BODYPROG_DMS_H\n#include "native_player_movement.h"')
    header+='_Static_assert(offsetof(s_DmsHeader,characterEntries)==32,"native DMS entries");\n'
    (out/'native_player_dms.h').write_text('/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations. */\n'+header)
    source='/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations. */\n#include "native_player_dms.h"\n'
    names=re.findall(r'^\w+\s*\*?\s+(\w+)\([^;{}]*\)[^{;]*\{',original,re.M)
    for name in names:
        if name in ['Dms_HeaderFixOffsets','Dms_EntryFixOffsets']:continue
        code=function(original,name)
        if name=='Dms_CharacterTransformGet':code=code.replace('Dms_CharacterGetIdxByName(charaName,','Dms_CharacterGetIdxByName((char*)charaName,')
        if name=='Dms_CharacterTransformGetByIdx':
            code=code.replace('charaEntry = &dmsHdr->characterEntries[charaIdx];','if(charaIdx<0 || charaIdx>=dmsHdr->characterEntryCount)port_unimplemented("DMS character index");\n    charaEntry = &dmsHdr->characterEntries[charaIdx];')
        # Only rotation/keyframe halfwords narrow; world outputs remain words.
        code=narrow(code,re.sub(r'\b(?:s16|q3_12)\s+v[xyz]\s*;', '',header))
        for target in [r'result->(?:position|rotation|positionTarget|lookAtTarget).v[xyz]',r'rot->v[xyz]']:
            code=re.sub(r'('+target+r')\s*=(?!=)\s*([^;{}]+);',lambda m:m[1]+' = (s16)('+m[2]+');',code)
        source+=code
    # PORT: Relocation is complete at publication; never add a host address to
    # a serialized offset or rebase the already owned pointers.
    source+='void Dms_HeaderFixOffsets(s_DmsHeader* header) {if(!header || !header->isLoaded)port_unimplemented("DMS native graph readiness");}\n'
    (out/'player_dms.c').write_text(source,encoding='utf-8')


def prepare_npc_models(decomp,out):
    """Original character/model services with stable native descriptor slots."""
    from prepare_maps import initializer
    from prepare_camera import narrow
    def read(path):return (decomp/path).read_text(encoding='utf-8')
    info=read('src/bodyprog/sys/chara_data_info.c')
    table=initializer(info,'CHARA_FILE_INFOS')
    table=re.sub(r'/\*.*?\*/|//[^\n]*','',table,flags=re.S)
    rows=re.findall(r'\{([^{}]+)\}',table)
    if len(rows)!=45:raise ValueError('character file table inventory drift')
    source='/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations. */\n#include "native_player_movement.h"\n'
    for name in ['D_800A90A4','D_800A90B4']:source+=initializer(read('src/bodyprog/screen/screen_data.c'),name)
    source+='const PortCharaFileInfo CHARA_FILE_INFOS[Chara_Count]={\n'
    for row in rows:
        fields=[part.strip() for part in row.split(',')]
        if len(fields)!=8:raise ValueError('character info initializer field drift')
        fields=[fields[i] for i in [0,1,2,4,6,7,3,5]]
        entry=','.join(fields)
        entry=entry.replace('BlendMode_Average','0').replace('BlendMode_Additive','1').replace('BlendMode_Subtractive','2')
        source+='{'+entry+'},\n'
    source+='};\n'
    world=read('src/bodyprog/world/world_draw.c')
    source+='void WorldGfx_CharaFree(s_CharaModel*);void WorldGfx_CharaLoad(e_CharaId,s32,s_LmHeader*,s_FsImageDesc*);void WorldGfx_CharaLmBufferAssign(s8);\n'
    for name in ['WorldGfx_CharaFree','WorldGfx_CharaLoad','WorldGfx_CharaModelLoad']:
        code=function(world,name)
        if name=='WorldGfx_CharaModelLoad':code=code.replace('e_CharaId charaId','s32 charaId')
        code=narrow(code,(out/'native_asset_records.h').read_text())
        code=code.replace('model->charaId  = charaId;', 'model->charaId  = (s8)charaId;')
        source+=code
    source+='''void WorldGfx_CharaLmBufferAssign(s8 forceFree) {
    // PORT: Original forced-free selection is retained. Native headers use
    // descriptor slots rather than overlapping packed PS1 LM byte arenas.
    s32 next=0;
    for(s32 i=0;i<CHARA_GROUP_COUNT;i++) {
        s_CharaModel* model=&g_WorldGfxWork.charaModels[i];
        if((forceFree>>i)&1)WorldGfx_CharaFree(model);
        if(model->charaId!=Chara_None) {
            for(s32 slot=0;slot<CHARA_GROUP_COUNT;slot++)if(model->lmHdr==&port_npc_models[slot] && next<slot+1)next=slot+1;
        }
    }
    g_WorldGfxWork.charaLmBuffer=(u8*)&port_npc_models[next];
}
'''
    spawn=read('src/bodyprog/events/chara_spawn.c')
    for name in ['Chara_Load','Chara_ProcessLoads','Chara_BonesInit']:
        source+=function(spawn,name)
    (out/'npc_models.c').write_text(source,encoding='utf-8')


def prepare_player_events(decomp,out):
    from prepare_maps import enumeration
    def read(path):return (decomp/path).read_text(encoding='utf-8')
    game=read('include/game.h')
    header='/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations. */\n#ifndef SH_PLAYER_EVENTS_H\n#define SH_PLAYER_EVENTS_H\n#include "native_player_dms.h"\n'
    for name in ['SysWork_StateSetNext','SysWork_StateStepIncrement','SysWork_StateStepSet','SysWork_StateStepReset']:
        code=function(game,name)
        if name=='SysWork_StateSetNext':code=code.replace(name,'port_move_state_next')
        header+=code
    events_header=read('include/bodyprog/events/events_util.h')
    for tag in ['CharaAnimCmd','ScreenFadeCmd','ScreenFadeType']:
        files=events_header+read('include/bodyprog/screen/screen_fade.h')+read('include/bodyprog/anim.h')+read('include/maps/shared.h')
        header+=enumeration(files,tag)
    original=read('src/bodyprog/events/events_util.c')
    source='/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations. */\n#include "native_player_events.h"\n'
    source+='static VECTOR3 g_Event_PathWaypoints[2][8];static q3_12 g_Event_PathWaypointHeadingAngles[8];static q19_12 g_Event_TweenTimers[6];\n'
    source+='void port_events_helpers_reset(void){memset(g_Event_PathWaypoints,0,sizeof(g_Event_PathWaypoints));memset(g_Event_PathWaypointHeadingAngles,0,sizeof(g_Event_PathWaypointHeadingAngles));memset(g_Event_TweenTimers,0,sizeof(g_Event_TweenTimers));}\n'
    for name in ['Event_SysStateStepIncrement','Event_SysStateStepSet','Event_WaitTimer','Event_CharaAnimCmdExecute','Event_ScreenFadeCmd','Event_DisplayMapMsg','Event_WaitPlayerStop','Event_PathWaypointSet','Event_PathWaypointExecutePlayer','Event_PathWaypointExecuteChara','Event_PathWaypointExecuteCharaNoWait','Event_TweenReset','Event_TweenLinear','Event_CameraPositionSet','Event_CameraLookAtSet','Event_DisplayMapMsgWithAudio']:
        code=function(original,name)
        from prepare_camera import narrow
        code=narrow(code,read('include/bodyprog/view/structs.h'))
        header+=re.sub(r'//[^\n]*','',code.split('{',1)[0]).strip()+';\n'
        source+=code
    source+='q19_12 g_Cutscene_Timer=NO_VALUE;\nVECTOR3 g_CameraPositionTarget,g_CameraLookAtTarget;\n'
    header+='extern q19_12 g_Cutscene_Timer;extern VECTOR3 g_CameraPositionTarget,g_CameraLookAtTarget;\n'
    header+='bool Chara_Load(s32,s8,GsCOORDINATE2*,s8,s_LmHeader*,s_FsImageDesc*);bool Chara_ProcessLoads(void);void Chara_BonesInit(s32);\n'
    border=read('include/bodyprog/screen/cutscene_border.h')
    header+=between(border,'#define CutsceneBorder_ForceShow()', 'void Screen_CutsceneCameraStateUpdate')
    msg_header=read('include/bodyprog/events/map_msg.h')
    for tag in ['MapMsgReturnCode','MapMsgState','MapMsgAudioType']:
        header+=enumeration(msg_header,tag)
    header+=between(msg_header,'typedef struct _MapMsgSelect','s32 Gfx_MapMsg_Draw')
    header+='#define MAP_MSG_UNSKIPPABLE_AUDIO_TYPE_FLAG (1<<0)\n#define MAP_MESSAGE_DISPLAY_ALL_LENGTH 400\n'
    header+='#define BgmStatusFlag_VoiceDialog (1<<5)\n#define AudioStreamingState_XaLoadPending 4\n'
    header+='s32 Gfx_MapMsg_Draw(s32);s32 Gfx_MapMsg_SelectionUpdate(u8,s32*);\ns32 Gfx_MapMsg_WidthsCompute(s32);s32 Gfx_MapMsg_StringDraw(char*,s32);void Gfx_MapMsg_Reset(void);\n#endif\n'
    header=header.replace('#endif','q3_12 Math_AngleNormalizeSigned(q3_12);\nstatic inline q3_12 port_events_angle(s32 angle){return Math_AngleNormalizeSigned((q3_12)angle);}\n#endif')
    source=source.replace('g_SysWork.bgmStatusFlags |= BgmStatusFlag_VoiceDialog;', 'g_SysWork.bgmStatusFlags |= (1<<5);')
    (out/'native_player_events.h').write_text(header,encoding='utf-8')
    (out/'player_events.c').write_text(source,encoding='utf-8')
    prepare_map_text(decomp,out)
    for file in ['player_loop.c','player_movement.c']:
        p=out/file;code=p.read_text(encoding='utf-8').replace('#include "native_player_movement.h"','#include "native_player_events.h"').replace('SysWork_StateSetNext(', 'port_move_state_next(')
        p.write_text(code,encoding='utf-8')


def prepare_map_text(decomp,out):
    """Extend the shared font unit with original USA map glyphs and rollout."""
    def read(path):return (decomp/path).read_text(encoding='utf-8')
    text=read('src/bodyprog/text/text_draw.c')
    header=read('include/bodyprog/text/text_draw.h')
    source='\n/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations. */\n#include "native_player_events.h"\n'
    source+='\n'.join(line for line in header.splitlines() if line.startswith('#define MAP_MSG_CODE_'))+'\n'
    source+='s_MapMsgLine g_MapMsg_ActiveLine; s32 g_MapMsg_WidthIdx,g_MapMsg_Widths[12];\n'
    for name in ['Gfx_MapMsg_WidthsCompute','Gfx_MapMsg_StringDraw','Gfx_MapMsg_Reset']:
        code=function(text,name)
        if name=='Gfx_MapMsg_WidthsCompute':
            # PORT: USA has no result; return a defined zero for its JP-only caller.
            code=code.replace('mapMsg = g_MapOverlayHdr.mapMessages[mapMsgIdx];','mapMsg = (u8*)g_MapOverlayHdr.mapMessages[mapMsgIdx];')
            code=code[:-2]+'    return 0;\n}\n'
        code=code.replace('((glyphPosY) << 16)','((u32)(u16)glyphPosY << 16)')
        code=code.replace('((glyphPosY) << 16)', '((u32)(u16)glyphPosY << 16)')
        for target in ['g_StringPosition.vx','g_StringPosition.vy','g_StringColorId']:
            code=re.sub(r'('+re.escape(target)+r'\s*=(?!=))\s*([^;{}]+);',r'\1 (s16)(\2);',code)
        code=code.replace('g_MapMsg_ActiveLine.positionIdx = posIdx;', 'g_MapMsg_ActiveLine.positionIdx = (u8)posIdx;')
        code=code.replace('g_MapMsg_AudioType = arg + 1;', 'g_MapMsg_AudioType = (u8)(arg + 1);')
        code=code.replace('    u32       temp_a0;', '    u32       temp_a0;')
        # PORT: GPU packet aliases remain fixed 32-bit; keep byte/halfword narrowing explicit.
        code=re.sub(r'(\*\(\(u16\*\)&glyphPoly->u[23]\) =) ([^;]+);',r'\1 (u16)(\2);',code)
        source+=code
    rollout=read('src/bodyprog/text/map_msg_display.c')
    source+='static s32 g_MapMsg_CurrentIdx;static q3_12 g_MapMsg_SelectFlashTimer;\nu8 g_MapMsg_AudioType;s8 g_MapMsg_SelectCancelIdx;\n'
    source+='static s32 rolloutState,menuSelection,port_msg_displayLength,activeMapMsgIdx,displayLengthInc;static bool loadAudio;\nextern void port_events_message_trace(s32,s32,s32,s32);extern void port_events_page_trace(s32);\n'
    for name in ['Gfx_MapMsg_Draw','Gfx_MapMsg_SelectionUpdate']:
        code=function(rollout,name)
        if name=='Gfx_MapMsg_Draw':
            code=re.sub(r'^    static (?:s32|bool)\s+\w+;\n','',code,flags=re.M)
            code=code.replace('    if (menuSelection != FINISH_MAP_MSG)', '    port_events_message_trace(g_MapMsg_CurrentIdx,displayLength,rolloutState,menuSelection);\n    if (menuSelection != FINISH_MAP_MSG)')
            code=re.sub(r'\bdisplayLength\b','port_msg_displayLength',code)
            code=code.replace('g_MapMsg_CurrentIdx++;','port_events_page_trace(g_MapMsg_CurrentIdx);\n                    g_MapMsg_CurrentIdx++;')
            code=code.replace('    g_SysWork.isMgsStringSet         = false;', '    port_events_page_trace(g_MapMsg_CurrentIdx);\n    g_SysWork.isMgsStringSet         = false;')
        code=code.replace('    s32         unkJapVal;', '')
        code=code.replace('unkJapVal = Gfx_MapMsg_WidthsCompute', '(void)Gfx_MapMsg_WidthsCompute')
        code=code.replace('Gfx_MapMsg_SelectionUpdate(g_MapMsg_CurrentIdx,','Gfx_MapMsg_SelectionUpdate((u8)g_MapMsg_CurrentIdx,')
        code=code.replace('Gfx_MapMsg_StringDraw(g_MapOverlayHdr.mapMessages[mapMsgIdx],', 'Gfx_MapMsg_StringDraw((char*)g_MapOverlayHdr.mapMessages[mapMsgIdx],')
        code=code.replace('g_MapMsg_SelectFlashTimer += g_DeltaTimeRaw;', 'g_MapMsg_SelectFlashTimer = (q3_12)(g_MapMsg_SelectFlashTimer + g_DeltaTimeRaw);')
        code=code.replace('g_MapMsg_Select.maxIdx           = curMenuSelection;', 'g_MapMsg_Select.maxIdx           = (s8)curMenuSelection;')
        code=code.replace('g_MapMsg_Select.maxIdx = curMenuSelection;', 'g_MapMsg_Select.maxIdx = (s8)curMenuSelection;')
        code=code.replace('g_MapMsg_Select.selectedEntryIdx = g_MapMsg_SelectCancelIdx;', 'g_MapMsg_Select.selectedEntryIdx = (u8)g_MapMsg_SelectCancelIdx;')
        code=code.replace('Gfx_StringColorSet(((g_MapMsg_SelectFlashTimer >> 10) * 3) + 4);','Gfx_StringColorSet((s16)(((g_MapMsg_SelectFlashTimer >> 10) * 3) + 4));')
        source+=code
    source+='void port_events_text_reset(void){g_MapMsg_CurrentIdx=0;g_MapMsg_SelectFlashTimer=0;rolloutState=0;menuSelection=0;port_msg_displayLength=0;activeMapMsgIdx=0;displayLengthInc=0;loadAudio=false;g_MapMsg_AudioType=0;g_MapMsg_SelectCancelIdx=0;memset(&g_MapMsg_Select,0,sizeof(g_MapMsg_Select));Gfx_MapMsg_Reset();}\n'
    (out/'player_map_text.inc').write_text(source,encoding='utf-8')


def extend_event_maps(decomp,out):
    """Events-owned additions to generated map code; maps' generator is unchanged."""
    from prepare_camera import narrow
    def read(path):return (decomp/path).read_text(encoding='utf-8')
    descriptor=out/'native_map_records.h'
    declarations=descriptor.read_text(encoding='utf-8')
    # PORT: The original unused descriptor field lacked a prototype. Its now
    # linked implementation takes an actor and reset mode; pointer size is unchanged.
    declarations=declarations.replace('(*charaAnimReset)()', '(*charaAnimReset)(s_SubCharacter*, bool)')
    descriptor.write_text(declarations,encoding='utf-8')
    consumers=out/'gameplay_consumers.c'
    startup=consumers.read_text(encoding='utf-8')
    startup='extern void port_events_text_reset(void),port_events_helpers_reset(void),sh_combat_reset_scratch(void);\n'+startup
    startup=startup.replace('void GameBoot_WorldInit(void)\n{','void GameBoot_WorldInit(void)\n{\n    port_events_text_reset();port_events_helpers_reset();sh_combat_reset_scratch();')
    consumers.write_text(startup,encoding='utf-8')
    spawn=out/'player_spawn.c'
    code=spawn.read_text(encoding='utf-8')
    original=function(read('src/bodyprog/items/item_utils.c'),'func_8004C564')
    original=narrow(original,'')
    original=original.replace('D_800C3960 = g_SavegamePtr->mapIdx','D_800C3960 = (s8)g_SavegamePtr->mapIdx')
    combat=read('src/bodyprog/bodyprog_combat_8008A058.c')
    providers='#include "native_player_events.h"\nvoid Sfx_WithFlagsAndPitchPlay(e_SfxId,const VECTOR3*,q23_8,s32,s32);void func_800892DC(s32,u8);q19_12 Rng_RandQ12(void);\n'
    provider_names=['func_8008B438','func_8008B3E4','func_8008B40C','func_8008B474']
    providers+=''.join(re.sub(r'//[^\n]*','',function(combat,name).split('{',1)[0]).strip()+';\n' for name in provider_names)
    motor=function(read('src/bodyprog/bodyprog_80089090.c'),'func_80089314')
    motor=motor.replace('func_800892DC(21, D_800AFD04 + 32)', 'func_800892DC(21, (u8)(D_800AFD04 + 32))')
    motor=motor.replace('D_800AFD05 += g_VBlanks;', 'D_800AFD05 = (u8)(D_800AFD05 + g_VBlanks);')
    providers+='static u8 D_800AFD04,D_800AFD05;\n'+narrow(motor,'')
    for name in provider_names:providers+=narrow(function(combat,name),'')
    code=code.replace(function(code,'func_8004C564'),providers+original)
    # The earlier generated no-weapon statics move inside their complete original owner.
    code=code.replace('static s8 D_800C3960,D_800C3961,D_800C3962;\nstatic u8 D_800C3963;\n','')
    spawn.write_text(code,encoding='utf-8')
    path=out/'map0_s00.c'
    code=path.read_text(encoding='utf-8')
    player=read('src/maps/characters/player.c')
    events=read('src/maps/map0_s00/map0_s00_2.c')
    names=['Player_MoveSpeedIsZero','Player_PathWaypointExecute','Chara_PathWaypointExecute','Chara_MovementReset','Chara_AnimReset','MapEvent_CutsceneCherylFootsteps0','MapEvent_CutsceneCherylFootsteps1','MapEvent_CutsceneCherylFootsteps2','MapEvent_CutsceneCherylSpotted','MapEvent_CutsceneCherylRedirect0','MapEvent_CutsceneCherylRedirect1','MapEvent_CutsceneCherylRedirect2','MapEvent_CutsceneCherylRedirect3','MapEvent_CutsceneCherylIntoTheAlley','func_800DB26C','func_800DB870','func_800DBE00','func_800DC33C','func_800DC694','func_800DC8D8','func_800DCA30']
    extra='\n/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations. */\n#undef playerChara\n#undef playerExtra\n#undef playerProps\n#undef cherylProps\n'
    objects='static q19_12 D_800DFAB8,sharedData_800DD5A0_0_s00;static s16 sharedData_800DD5A4_0_s00;static q3_12 sharedData_800E39E0_0_s00,sharedData_800E39E2_0_s00;\n'
    objects+='static s32 sharedData_800DF1F4_0_s00;static s16 sharedData_800DF1F8_0_s00,sharedData_800DF1FA_0_s00;static u16 g_Cutscene_MapMsgAudioCmds[3];static u8 D_800DFAC2,g_Cutscene_MapMsgAudioIdx;static bool g_WarpCamera0;\n'
    objects+='static q19_12 D_800DFAD0,D_800DFAD4;static bool g_WarpCamera;\n'
    helpers=['Cheryl_DistantFootstepSfxPlay']
    for name in helpers:
        extra+=f'#define {name} sh_map0_s00_{name}\n'
        extra+=re.sub(r'//[^\n]*','',function(events,name).split('{',1)[0]).strip()+';\n'
    extra+=objects
    record=(out/'native_gameplay_records.h').read_text(encoding='utf-8')
    record=re.sub(r'\b(?:s16|s32|q3_12|q4_12)\s+(?:vx|vy|vz)\s*;', '',record)
    for name in names:
        new=function(player if name.startswith('Player_') else read('src/maps/chara_util.c') if name.startswith('Chara_') else events,name)
        new=new.replace('SysWork_StateSetNext(', 'port_move_state_next(')
        new=new.replace('Math_AngleNormalizeSigned(', 'port_events_angle(')
        new=new.replace('block_7:\n','')
        if name=='Player_PathWaypointExecute':
            # PORT: The source reads one past its last waypoint on completion.
            # Retain its completion state without reading outside the native path.
            new=new.replace('playerVecDist = SquareRoot0(Q12_2D_DISTANCE_SQR(localVec[sharedData_800DD5A4_0_s00], playerChara->position));\n\n                if', 'playerVecDist = sharedData_800DD5A4_0_s00 < vecCount ? SquareRoot0(Q12_2D_DISTANCE_SQR(localVec[sharedData_800DD5A4_0_s00], playerChara->position)) : 0;\n\n                if')
        if name=='Chara_MovementReset':
            new=new.replace('*(s32*)&npc->properties.npc.field_EC = Q12(0.0f);','npc->properties.npc.field_EC=0;npc->properties.npc.field_EE=0;')
        if name=='Chara_PathWaypointExecute':
            new=new.replace('Math_ShortestAngleGet(chara->rotation.vy, angleIn,','Math_ShortestAngleGet(chara->rotation.vy, (q3_12)angleIn,')
            new=new.replace('dist = SquareRoot0(Q12_2D_DISTANCE_SQR(arg2[sharedData_800DF1F8_0_s00], chara->position));\n\n                if','dist = sharedData_800DF1F8_0_s00 < arg4 ? SquareRoot0(Q12_2D_DISTANCE_SQR(arg2[sharedData_800DF1F8_0_s00], chara->position)) : 0;\n\n                if')
        new=narrow(new,record)
        new=re.sub(r'(sharedData_800E39E[02]_0_s00\s*=(?!=))\s*([^;{}]+);',r'\1 (q3_12)(\2);',new)
        new=re.sub(r'(sharedData_800DF1FA_0_s00\s*=(?!=))\s*([^;{}]+);',r'\1 (s16)(\2);',new)
        if name=='func_800DC33C':new=new.replace('var_a0 = var_s1 << 16;', 'var_a0 = (s32)((u32)(u16)var_s1 << 16);')
        old=function(code,name)
        if 'port_unimplemented' not in old:raise ValueError('events/map callback already linked; reconcile '+name)
        if old.startswith('static '):new='static '+new
        code=code.replace(old,re.sub(r'//[^\n]*','',new.split('{',1)[0]).strip()+';\n')
        extra+=new
    foot=function(events,'Cheryl_DistantFootstepSfxPlay')
    for declaration in ['    s32     temp_s0;\n','    s32     temp_v0_5;\n','    s32     temp_v1;\n','    s32     var_a3;\n']:
        foot=foot.replace(declaration,'')
    foot=foot.replace('Rng_GenerateUInt(75, 106), Rng_GenerateInt(-16, 15)', '(u8)Rng_GenerateUInt(75, 106), (s8)Rng_GenerateInt(-16, 15)')
    extra+=narrow(foot,record)
    code=code.replace('void sh_map0_s00_reset(void)', 'static void port_events_base_reset(void)')
    extra+='void sh_map0_s00_reset(void){port_events_base_reset();D_800DFAB8=0;sharedData_800DD5A0_0_s00=0;sharedData_800DD5A4_0_s00=0;sharedData_800E39E0_0_s00=0;sharedData_800E39E2_0_s00=0;sharedData_800DF1F4_0_s00=0;sharedData_800DF1F8_0_s00=0;sharedData_800DF1FA_0_s00=0;memset(g_Cutscene_MapMsgAudioCmds,0,sizeof(g_Cutscene_MapMsgAudioCmds));D_800DFAC2=0;g_Cutscene_MapMsgAudioIdx=0;g_WarpCamera0=false;D_800DFAD0=0;D_800DFAD4=0;g_WarpCamera=false;}\n'
    code=code.replace('int sh_map0_s00_load_data(void)', 'static int port_events_base_load_data(void)')
    extra+='int sh_map0_s00_load_data(void){u8 bytes[12];if(port_events_base_load_data() || port_map_data_read(FILE_VIN_MAP0_S00_BIN,0x16544,sizeof(bytes),bytes))return 1;for(size_t i=0;i<3;i++)g_Cutscene_MapMsgAudioCmds[i]=move_half(bytes+i*2);D_800DFAC2=bytes[6];g_WarpCamera0=bytes[8]!=0;return 0;}\n'
    path.write_text(code+extra,encoding='utf-8')
    print('events: 17 additional original descriptor callbacks, 4 Cheryl companion helpers, 15 reset objects')
    prepare_events_combat(decomp,out)
    prepare_events_npcs(decomp,out)
    prepare_transit_opening(decomp,out)


def prepare_transit_opening(decomp,out):
    """Original post-gate environment ramp and its bounded numeric waypoints."""
    from prepare_camera import narrow
    def read(path):return (decomp/path).read_text(encoding='utf-8')
    path=out/'map0_s00.c'
    code=path.read_text(encoding='utf-8')
    names=['func_800DCC54','func_800DCDA8','func_800DCF38']
    source=read('src/maps/map0_s00/map0_s00_2.c')
    bodies={name:function(source,name) for name in names}
    # PORT: func_174 is unused and has different prototypes between overlays.
    # Keep its descriptor guard; direct calls use the original typed provider.
    bodies['port_transit_particle_select']=function(read('src/maps/particle.c'),'sharedFunc_800D0B18_0_s00').replace('sharedFunc_800D0B18_0_s00','port_transit_particle_select').replace('g_SysWork.field_2348','port_transit_particle_previous')
    bodies['func_800DCC54']=bodies['func_800DCC54'].replace('sharedFunc_800D0B18_0_s00','port_transit_particle_select')
    code=code.replace(function(code,'func_800DCC54'), 'static void func_800DCC54(void);\n')
    records={
        'D_800DFADC':'q19_12 D_800DFADC',
        'D_800DFAE0':'VECTOR3 D_800DFAE0[6]',
        'D_800DFB28':'VECTOR3 D_800DFB28[3]',
        'port_transit_particle_previous':'s8 port_transit_particle_previous',
        'g_Particle_PrevPosition':'VECTOR3 g_Particle_PrevPosition',
        'sharedData_800E0CA8_0_s00':'s32 sharedData_800E0CA8_0_s00',
        'sharedData_800E0CAC_0_s00':'s32 sharedData_800E0CAC_0_s00',
        'sharedData_800E0CB4_0_s00':'u16 sharedData_800E0CB4_0_s00',
        'sharedData_800E0CB6_0_s00':'u16 sharedData_800E0CB6_0_s00',
        'sharedData_800E0CB8_0_s00':'u16 sharedData_800E0CB8_0_s00',
        'sharedData_800E32D0_0_s00':'s32 sharedData_800E32D0_0_s00',
    }
    added={name:decl for name,decl in records.items() if '#define '+name+' ' not in code}
    prefix='sh_map0_s00_'
    extra='// PORT: Native numeric imports; no map code or pointer is relocated.\n'
    extra+=''.join(f'#define {name} {prefix}{name}\n' for name in added)
    extra+=''.join('static '+decl+';\n' for decl in added.values())
    extra+='u32 func_8005C478(s16*,q19_12,q19_12,q19_12,q19_12,q19_12,q19_12);\n'
    extra+='\n'.join('static '+re.sub(r'//[^\n]*','',body.split('{',1)[0]).strip()+';' for body in bodies.values())+'\n'
    extra+='#define MAP_PARTICLE_HAS_RAIN 1\n#define MAP_USE_PARTICLES 1\n'
    for name,body in bodies.items():
        body=narrow(body,read('include/maps/particle.h'))
        body=body.replace('g_SysWork.field_2349 = arg0;', 'g_SysWork.field_2349 = (s8)arg0;')
        body=body.replace('port_transit_particle_previous = arg0;', 'port_transit_particle_previous = (s8)arg0;')
        body=re.sub(r'(sharedData_800E0CB[48]_0_s00)\s*\+=\s*([^;]+);',r'\1 = (u16)(\1 + (\2));',body)
        body=re.sub(r'(temp\s*=(?!=))\s*([^;]+);',r'\1 (u16)(\2);',body) if name=='port_transit_particle_select' else body
        body=body.replace('var_s4 = temp_v1_3;', 'var_s4 = (s32)temp_v1_3;')
        body=body.replace('temp_v1_3 = func_8005C478(', 'temp_v1_3 = (s32)func_8005C478(')
        extra+='static '+body+'\n'
    reset=function(code,'sh_map0_s00_reset')
    code=code.replace(reset,reset[:-2]+''.join(f'memset(&{name},0,sizeof({name}));' for name in added)+'}\n')
    code=code.replace('int sh_map0_s00_load_data(void)', 'static int port_transit_prior_load(void)')
    probe=function(code,'sh_map0_s00_reset_probe')
    revised=probe.replace('{','{'+''.join(f'memset(&{name},0xa5,sizeof({name}));' for name in added),1)
    revised=revised.replace('return ', 'return '+''.join(f'port_map_zero(&{name},sizeof({name})) && ' for name in added),1)
    code=code.replace(probe,revised)
    # The complete original table spans six + three 12-byte VECTOR3 records.
    # Their named GPL declarations/address suffixes and YAML data range bound it.
    declarations=read('include/maps/map0/map0_s00.h')
    assert 'extern VECTOR3 D_800DFAE0[];' in declarations and 'extern VECTOR3 D_800DFB28[3];' in declarations
    assert '[0x1653C, data]' in read('configs/USA/maps/map0_s00.yaml')
    extra+='static int port_transit_alley_data(void){u8 wire[112];if(port_map_data_read(FILE_VIN_MAP0_S00_BIN,0x16564,sizeof(wire),wire))return 1;D_800DFADC=(s32)move_word(wire);for(size_t i=0;i<6;i++){D_800DFAE0[i].vx=(s32)move_word(wire+4+i*12);D_800DFAE0[i].vy=(s32)move_word(wire+8+i*12);D_800DFAE0[i].vz=(s32)move_word(wire+12+i*12);}for(size_t i=0;i<3;i++){D_800DFB28[i].vx=(s32)move_word(wire+76+i*12);D_800DFB28[i].vy=(s32)move_word(wire+80+i*12);D_800DFB28[i].vz=(s32)move_word(wire+84+i*12);}return 0;}\n'
    # Declarations precede reset/use; numeric decoder follows shared word helpers.
    head='#include "npc_startup.h"\n'+''.join(f'#define {name} {prefix}{name}\nstatic {decl};\n' for name,decl in added.items())+'static int port_transit_alley_data(void);\n'
    extra=extra[extra.index('u32 func_8005C478'):]
    extra+='int sh_map0_s00_load_data(void){if(port_transit_prior_load())return 1;return port_transit_alley_data();}\n'
    path.write_text(head+code+extra,encoding='utf-8')


def prepare_events_combat(decomp,out):
    """Reuse the combat lane's production migration; never compile its harness."""
    from prepare_combat import generate as prepare_combat, names as combat_names
    from prepare_maps import initializer
    from prepare_camera import narrow
    def read(path):return (decomp/path).read_text(encoding='utf-8')
    prepared=out/'combat-production'
    prepare_combat(decomp,prepared)
    code=(prepared/'combat_original.c').read_text(encoding='utf-8')
    code=code.replace('#include "combat.h"','#include "native_player_events.h"\n#include "native_player_rays.h"\n')
    code=code.replace(initializer(code,'D_800AD4C8'),'')
    # These sound handlers already have a shared native owner reached by text.
    for name in ['func_8008B398','func_8008B3E4','func_8008B40C','func_8008B438','func_8008B474']:
        code=code.replace(function(code,name),'')
    code=code.replace('D_800297B8','HARRY_BASE_ANIM_INFOS')
    code=code.replace('sh_combat_harry_map_anims','g_MapOverlayHdr.harryMapAnimInfos')
    code=code.replace('sh_combat_dark_environment','(g_SysWork.gameplayEnvironment.field_154.effectsInfo.flags.field_0 & SpecialEnvEventFlags_DarkEnvironment)')
    combat=read('src/bodyprog/bodyprog_combat_8008A058.c')
    prototypes=''.join(re.sub(r'//[^\n]*','',function(combat,name).split('{',1)[0]).strip()+';\n' for name in combat_names(combat))
    prototypes+='s32 sh_combat_ps1_div(s32,s32);u32 sh_combat_lzc(u32);s32 sh_combat_npc_index(const s_SubCharacter*);extern u32 sh_combat_lzc_input;extern s16 SQRT[192];\n'
    prototypes+='s_AnimInfo* func_80044918(s_ModelAnim*);q19_12 func_8007FD2C(void);s32 func_80080540(q19_12,q19_12,q19_12);q19_12 Math_Distance2dGet(const VECTOR3*,const VECTOR3*);\n'
    prototypes+='void func_8005F6B0(s_SubCharacter*,VECTOR3*,s32,s32);void func_80089314(s32);void func_8009151C(u32,s32,q19_12);s32 func_8009146C(s32);void func_800914C4(s32,s32);s32 Game_HyperBlasterBeamColorGet(void);\n'
    prototypes+='void Sfx_WithFlagsAndPitchPlay(e_SfxId,const VECTOR3*,q23_8,s32,s32);q19_12 Rng_RandQ12(void);\n'
    prototypes+='void port_events_hit_trace(const s_SubCharacter*,const s_SubCharacter*,s32);\n'
    # Match the engine's enum-width SFX ABI while preserving the migrated values.
    code=code.replace('Sfx_WithFlagsPlay((u16)sfxId,','Sfx_WithFlagsPlay((e_SfxId)sfxId,')
    code=code.replace('Sfx_WithFlagsPlay(sp1C->field_12','Sfx_WithFlagsPlay(sp1C->field_12')
    hit=function(code,'func_8008B714')
    instrumented=hit.replace('{','{\n    s32 previousDamage=target->damage.amount;',1)
    instrumented=re.sub(r'\breturn ([^;]+);',r'{if(target->damage.amount>previousDamage)port_events_hit_trace(attacker,target,target->damage.amount-previousDamage);return \1;}',instrumented)
    code=code.replace(hit,instrumented)
    # Original player/character flag helpers retain their already-public names.
    damaged=function(read('src/bodyprog/events/npc_main.c'),'Chara_DamagedFlagUpdate')
    code+=damaged
    for name in ['Chara_Flag8Clear','Chara_DamagedFlagUpdate']:
        prototypes=prototypes.replace(name+'(', 'sh_combat_'+name+'(')
        code=code.replace(name+'(', 'sh_combat_'+name+'(')
    helpers=(Path(__file__).resolve().parents[1]/'port/sys/combat/combat.c').read_text(encoding='utf-8')
    for name in ['sh_combat_ps1_div','sh_combat_lzc','sh_combat_npc_index']:
        helper=function(helpers,name).replace('abort();','port_unimplemented("combat NPC identity");return 0;')
        code+=helper
    code+='u32 sh_combat_lzc_input;s16 SQRT[192];\n'
    anim=function(read('src/bodyprog/world/bodyprog_anim_800445A4.c'),'func_80044918')
    # PORT: Select the map element without forming a pointer before its owned array.
    anim=anim.replace('animInfos  = mapAnimInfos;\n        animInfos -= mapAnimStatusStart;', 'return &mapAnimInfos[animStatus-mapAnimStatusStart];')
    code+=anim
    for name in ['func_8007FD2C','Math_Distance2dGet']:
        body=function(read('src/bodyprog/player_control.c'),name)
        body=body.replace('playerProps.', 'g_SysWork.playerWork.player.properties.player.')
        code+=body
    code+=function(read('src/bodyprog/items/item_utils.c'),'Game_HyperBlasterBeamColorGet')
    ranking=read('src/bodyprog/ranking.c')
    code+=narrow(function(ranking,'func_8009151C'),read('include/game.h'))
    # PORT: Translate the pinned MULT/MFHI/MFLO Q12 square operations to wide C
    # products, retaining the original wrapping 32-bit sum.
    code+='s32 func_80080540(q19_12 x,q19_12 y,q19_12 z){return (s32)((u32)(((s64)x*x)>>12)+(u32)(((s64)y*y)>>12)+(u32)(((s64)z*z)>>12));}\n'
    code+='void func_8005F6B0(s_SubCharacter* target,VECTOR3* pos,s32 kind,s32 group){(void)target;(void)pos;(void)kind;(void)group;port_unimplemented("combat blood/impact native effect publication");}\n'
    (out/'player_combat.c').write_text('#include "native_player_events.h"\n#include "native_player_rays.h"\n'+prototypes+code,encoding='utf-8')


def prepare_events_npcs(decomp,out):
    """Original first-map Stalker AI and real movement/LOS/shape providers."""
    from prepare_combat import names as names_in, mask
    from prepare_camera import narrow
    def read(path):return (decomp/path).read_text(encoding='utf-8')
    records=(out/'native_gameplay_records.h').read_text(encoding='utf-8')
    records=re.sub(r'\b(?:s16|s32|q3_12|q4_12)\s+(?:vx|vy|vz)\s*;', '',records)
    providers={}
    for path in ['src/bodyprog/bodyprog_npc_8005BF38.c','src/bodyprog/collision/los.c','src/bodyprog/collision/chara.c']:
        text=read(path)
        for name in names_in(text):providers[name]=function(text,name)
    existing='\n'.join(path.read_text(encoding='utf-8') for path in out.glob('*.c') if path.name!='npc_ai.c')
    owned={name:body for name,body in providers.items() if not re.search(r'^\w+\s*\**\s+'+name+r'\([^;{}]*\)[^{;]*\{',existing,re.M) or name in ['func_8005CD38','func_8005D50C']}
    header='/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations. */\n#include "native_player_events.h"\n#include "native_player_rays.h"\n'
    header+=''.join(re.sub(r'//[^\n]*','',body.split('{',1)[0]).strip()+';\n' for body in providers.values())
    header+='void Savegame_EnemyStateUpdate(s_SubCharacter*);q19_12 Rng_RandQ12(void);void func_8005F6B0(s_SubCharacter*,VECTOR3*,s32,s32);\n'
    header+='void Math_MatrixTransform(VECTOR3*,const SVECTOR3*,GsCOORDINATE2*);void func_800622B8(s32,s_SubCharacter*,s32,s32);\n'
    header+='s32 sh_combat_ps1_div(s32,s32);\n'
    header+='static inline q3_12 port_events_atan(s32 y,s32 x){return (q3_12)ratan2(y,x);}\n'
    header+='#ifndef USHRT_MAX\n#define USHRT_MAX 65535\n#endif\n'
    header+='#define Chara_HasFlag(chara,flag) ((chara)->flags & (flag))\n'
    (out/'native_npc_ai.h').write_text(header,encoding='utf-8')
    code=header
    for name,body in owned.items():
        body=body.replace('Math_AngleNormalizeSigned(', 'port_events_angle(')
        body=body.replace('g_SysWork.field_2388.field_154.', 'g_SysWork.gameplayEnvironment.field_154.')
        body=body.replace('ARRAY_SIZE(g_SysWork.npcs)', '(s32)ARRAY_SIZE(g_SysWork.npcs)')
        body=body.replace('q25_6','s32')
        body=narrow(body,records)
        if name=='func_8006FD90':
            # PORT: The source's two stack vectors alias a PS1 RayTrace; its
            # +16 word is the character identity. Own the complete native trace.
            body=body.replace('VECTOR3 sp10;','s_RayTrace trace={0};').replace('VECTOR3 sp20;','')
            body=body.replace('Ray_CharaTraceQuery(&sp10,','Ray_CharaTraceQuery(&trace,').replace('sp20.vx != Q12(0.0f)', 'trace.character != NULL')
        body=re.sub(r'(angles[02]\[[^]]+\]\s*=(?!=))\s*([^;{}]+);',r'\1 (q3_12)(\2);',body)
        body=re.sub(r'(chara->collision.shapeOffsets\.(?:box|cylinder)\.v[xz]\s*=(?!=))\s*([^;{}]+);',r'\1 (s16)(\2);',body)
        body=re.sub(r'(scale\[[012]\]\s*=(?!=))\s*([^;{}]+);',r'\1 (q3_12)(\2);',body)
        body=body.replace('i == npcIdx', '(u32)i == npcIdx')
        code+=body
    code+=function(read('src/bodyprog/events/npc_main.c'),'Savegame_EnemyStateUpdate')
    code+='void func_800622B8(s32 a,s_SubCharacter* chara,s32 status,s32 group){(void)a;(void)chara;(void)status;(void)group;port_unimplemented("Stalker death/blood native effect publication");}\n'
    # Replace the two aim-selection boundaries with these real NPC/LOS providers.
    services=out/'player_services.c'
    service_code=services.read_text(encoding='utf-8')
    for name in ['func_8005CD38','func_8005D50C']:
        service_code=service_code.replace(function(service_code,name),'')
    services.write_text(service_code,encoding='utf-8')
    (out/'npc_ai.c').write_text(code,encoding='utf-8')
    # Every Stalker function belongs to this overlay's namespace and reset image.
    stalker=read('src/maps/characters/stalker.c')
    ai_names=names_in(stalker)
    path=out/'map0_s00.c'
    code=path.read_text(encoding='utf-8')
    code=code.replace(function(code,'Stalker_Update'),'void Stalker_Update(s_SubCharacter*,s_AnmHeader*,GsCOORDINATE2*);\n')
    extra='\n#include "native_npc_ai.h"\n#undef cherylProps\n#undef playerChara\n#define stalkerProps stalker->properties.stalker\n'
    for name in ['STALKER_ANIM_INFOS','g_Stalker_TargetPositionX','g_Stalker_TargetPositionZ','sharedData_800E3A20_0_s00','sharedData_800E3A24_0_s00','sharedData_800E3A28_0_s00','sharedData_800E3A2C_0_s00']:
        extra+=f'#define {name} sh_map0_s00_{name}\n'
    extra+=re.sub(r'^#include[^\n]*','',read('include/maps/characters/stalker.h'),flags=re.M)+'\n'
    for name in ai_names:
        if name!='Stalker_Update':extra+=f'#define {name} sh_map0_s00_{name}\n'
    extra+=''.join(re.sub(r'//[^\n]*','',function(stalker,name).split('{',1)[0]).strip()+';\n' for name in ai_names)
    extra+='// PORT: Clear the whole native property union, including its host-pointer padding.\n#undef Chara_PropsClear\n#define Chara_PropsClear(chara) memset(&(chara)->properties,0,sizeof((chara)->properties))\n'
    extra+='#define Chara_CollisionSet(chara,keyframe) do {(chara)->collision.box=(keyframe).box;(chara)->collision.shapeOffsets=(keyframe).shapeOffsets;} while(0)\n'
    chara_header=read('include/bodyprog/chara/chara.h')
    for name in ['Chara_AnimSet']:
        extra+=narrow(function(chara_header,name),records)
    shared=read('include/maps/shared.h')
    extra+=function(shared,'ModelAnim_AnimInfoSet')
    extra+=between(shared,'#define APPROACH(', '#define APPROACH_ALT(')
    extra+=between(shared,'#define Chara_MoveSpeedUpdate(', '// TODO: Is it possible to merge these macros?')
    extra+=between(shared,'#define Chara_MoveSpeedUpdate3(', '#define Chara_MoveSpeedUpdate4(')
    table_names=re.findall(r'extern s_Keyframe (\w+)(\[\d+\])?;',stalker)
    counts={name:int(array[1:-1]) if array else 1 for name,array in table_names}
    extra+='s_AnimInfo STALKER_ANIM_INFOS[96];\n'
    extra+='static u8 sharedData_800DD5A6_0_s00;static s32 sharedData_800E39E4_0_s00,sharedData_800E39E8_0_s00,sharedData_800E39EC_0_s00[8];static u16 sharedData_800E3A0C_0_s00[6];\n'
    extra+='q19_12 sharedData_800E3A20_0_s00,sharedData_800E3A24_0_s00,sharedData_800E3A28_0_s00,sharedData_800E3A2C_0_s00;static q19_12 g_Stalker_TargetPositionX,g_Stalker_TargetPositionZ;\n'
    extra+=''.join(f'static s_Keyframe {name}'+(f'[{count}]' if count>1 else '')+';\n' for name,count in counts.items())
    extra+='static s_Keyframe* port_events_keyframe(s_Keyframe* table,s32 count,s32 index){if(index<0 || index>=count)port_unimplemented("Stalker keyframe table bounds");return &table[index];}\n'
    for name in ai_names:
        body=function(stalker,name)
        # PORT: Some original control states are empty for MAP0_S00.
        body=body.replace('{','{\n    (void)stalker;',1)
        body=body.replace('ratan2(', 'port_events_atan(')
        body=body.replace('Math_AngleNormalizeSigned(', 'port_events_angle(').replace('g_SysWork.field_2388.field_154.', 'g_SysWork.gameplayEnvironment.field_154.')
        body=body.replace('*(s32*)&stalkerProps.flags','stalkerProps.flags')
        # PORT: Keep the PS1 DIV zero/overflow results at both variable frame
        # divisors. Native division must not trap during these original states.
        body=body.replace('(Q12_MULT_PRECISE(duration, g_DeltaTime) * animMult) / animDiv', 'sh_combat_ps1_div((s32)((u32)Q12_MULT_PRECISE(duration,g_DeltaTime)*(u32)animMult),animDiv)')
        body=body.replace('(s32)(dist * (u32)Q12_MULT_PRECISE(STALKER_ANIM_INFOS[stalker->model.anim.status].duration.constant, g_DeltaTime)) /\n               FP_TO(distDiv, Q12_SHIFT)', 'sh_combat_ps1_div((s32)(dist * (u32)Q12_MULT_PRECISE(STALKER_ANIM_INFOS[stalker->model.anim.status].duration.constant,g_DeltaTime)),FP_TO(distDiv,Q12_SHIFT))')
        body=body.replace('    s32 i;\n','') if name=='Stalker_Init' else body
        body=body.replace('ptr = PSX_SCRATCH;', 's_sharedFunc_800D6970_0_s00 nativeScratch={0};ptr=&nativeScratch; // PORT: Typed native scratch retains the original numeric fields.')
        body=body.replace('Math_RotMatrixZxyNegGte(&stalker->rotation,', 'SVECTOR nativeRotation={stalker->rotation.vx,stalker->rotation.vy,stalker->rotation.vz,0};\n    Math_RotMatrixZxyNegGte(&nativeRotation,')
        body=body.replace('s_CollisionResult* sp10[7];','s_CollisionResult sp10={0};')
        body=body.replace('&g_SysWork.playerWork,', '&g_SysWork.playerWork.player,')
        body=body.replace('newHealth          = stalker->health','newHealth          = (u32)stalker->health').replace('MIN(newHealth, stalkerProps.health_110)', '(s32)MIN(newHealth,(u32)stalkerProps.health_110)')
        body=narrow(body,records.replace('q4_12','q3_12')+body)
        body=re.sub(r'(sharedData_800E3A0C_0_s00\[[^]]+\]\s*=(?!=))\s*([^;{}]+);',r'\1 (u16)(\2);',body)
        for table,count in counts.items():
            if count==1:continue
            body=re.sub(r'&'+table+r'\[([^]]+)\]',lambda m:f'port_events_keyframe({table},{count},{m[1]})',body)
            body=re.sub(table+r'\[([^]]+)\]',lambda m:f'(*port_events_keyframe({table},{count},{m[1]}))',body)
        extra+=body
    # Decode only numeric data leaves from the owned map; no code bytes are relocated.
    decoder=(Path(__file__).resolve().parents[1]/'port/sys/combat/combat.c').read_text(encoding='utf-8')
    extra+=function(decoder,'sh_combat_keyframe_decode').replace('half(bytes+i*2)','move_half(bytes+i*2)')
    reset_names=['STALKER_ANIM_INFOS','g_Stalker_TargetPositionX','g_Stalker_TargetPositionZ','sharedData_800DD5A6_0_s00','sharedData_800E39E4_0_s00','sharedData_800E39E8_0_s00','sharedData_800E39EC_0_s00','sharedData_800E3A0C_0_s00','sharedData_800E3A20_0_s00','sharedData_800E3A24_0_s00','sharedData_800E3A28_0_s00','sharedData_800E3A2C_0_s00',*counts]
    extra+='static void port_events_stalker_reset(void){'+''.join(f'memset(&{name},0,sizeof({name}));' for name in reset_names)+'}\n'
    # PORT: Stalker's original first blend has status 0xff, and its idle
    # playback has status 0. Status is a reset/base identity, not the row index.
    extra+='static int port_events_stalker_load(void){u8 rows[96*16];s_AnimInfo decoded[96]={0};if(port_map_data_read(FILE_VIN_MAP0_S00_BIN,0x14030,sizeof(rows),rows))return 1;for(size_t i=0;i<96;i++){const u8* row=rows+i*16;s_AnimInfo* info=&decoded[i];switch(move_word(row)){case 0:info->playbackFunc=NULL;break;case 0x80044CA4:info->playbackFunc=Anim_BlendLinear;break;case 0x80044B38:info->playbackFunc=Anim_PlaybackLoop;break;case 0x800449F0:info->playbackFunc=Anim_PlaybackOnce;break;default:return 1;}if((row[4]!=255 && row[4]>=96) || row[5] || (row[6]!=255 && row[6]>=96))return 1;info->status=row[4];info->linkStatus=row[6];info->duration.constant=(s32)move_word(row+8);info->startKeyframeIdx=(s16)move_half(row+12);info->endKeyframeIdx=(s16)move_half(row+14);}memcpy(STALKER_ANIM_INFOS,decoded,sizeof(decoded));\n'
    for table,count in counts.items():
        symbols=read('configs/USA/maps/sym.map0_s00.txt')
        pinned=int(re.search(r'^'+re.escape(table)+r'\s*=\s*(0x[0-9a-fA-F]+)',symbols,re.M)[1],16)
        if pinned!=int(re.search(r'800[0-9A-Fa-f]+',table)[0],16):raise ValueError('Stalker numeric table symbol drift '+table)
        address=pinned-0x800C9578
        extra+=f'{{u8 bytes[{count}*20];if(port_map_data_read(FILE_VIN_MAP0_S00_BIN,0x{address:x},sizeof(bytes),bytes))return 1;for(size_t i=0;i<{count};i++)if(!sh_combat_keyframe_decode(bytes+i*20,20,'+(f'&{table}[i]' if count>1 else f'&{table}')+'))return 1;}\n'
    extra+='return 0;}\n'
    event_objects=['D_800DFAB8','sharedData_800DD5A0_0_s00','sharedData_800DD5A4_0_s00','sharedData_800E39E0_0_s00','sharedData_800E39E2_0_s00','sharedData_800DF1F4_0_s00','sharedData_800DF1F8_0_s00','sharedData_800DF1FA_0_s00','g_Cutscene_MapMsgAudioCmds','D_800DFAC2','g_Cutscene_MapMsgAudioIdx','g_WarpCamera0','D_800DFAD0','D_800DFAD4','g_WarpCamera']
    all_objects=event_objects+reset_names
    extra+='static bool port_events_is_zero(const void* data,size_t size){const u8* bytes=data;for(size_t i=0;i<size;i++)if(bytes[i])return false;return true;}\n'
    extra+='u32 port_events_reset_probe(void){if(sh_combat_ps1_div(7,0)!=-1 || sh_combat_ps1_div(-7,0)!=1 || sh_combat_ps1_div((-2147483647-1),-1)!=(-2147483647-1))return 0;'+''.join(f'memset(&{name},0xa5,sizeof({name}));' for name in all_objects)+'sh_map0_s00_reset();'+''.join(f'if(!port_events_is_zero(&{name},sizeof({name})))return 0;' for name in all_objects)+f'return {len(all_objects)};'+'}\n'
    reset=function(code,'sh_map0_s00_reset')
    code=code.replace(reset,reset[:-2]+'port_events_stalker_reset();}\n')
    load=function(code,'sh_map0_s00_load_data')
    code=code.replace(load,load.replace('return 0;', 'return port_events_stalker_load();'))
    code='static void port_events_stalker_reset(void);static int port_events_stalker_load(void);\n'+code
    path.write_text(code+extra,encoding='utf-8')
    print(f'events: {len(ai_names)} Stalker functions, {sum(counts.values())} collision keyframes; {len(all_objects)} extra reset objects')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--decomp', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--extend-maps',action='store_true')
    args = parser.parse_args()
    if args.extend_maps:extend_event_maps(args.decomp,args.out)
    else:generate(args.decomp, args.out)
