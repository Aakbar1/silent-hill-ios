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
    # PORT: Duration callbacks actually consume the model; retain its native type.
    data.append(no_includes(read('include/bodyprog/anim.h')).replace('(*variableFunc)(void)', '(*variableFunc)(struct _Model*)'))
    data.append(no_includes(read('include/bodyprog/model.h')).replace('model->anim.status = ANIM_STATUS', 'model->anim.status = (u8)ANIM_STATUS'))
    chara = between(read('include/bodyprog/chara/chara.h'), '#define NPC_COUNT_MAX', 'typedef struct _CharaFileInfo')
    chara = chara[:chara.rindex('/**')]
    data.append(chara.replace('Chara_Count,', 'PortChara_Count,'))
    game = read('include/game.h')
    player = between(game, 'typedef struct _PlayerExtra', '/** @brief Map effects info.')
    data.append(player.replace('e_InvItemId', 's32'))
    # PORT: Pointer-free environment presets retain their original assertions.
    data.append(between(game, '/** @brief Map effects info.', '// Current enviroment effects information.'))
    env_work=between(game,'// Current enviroment effects information.', '/** @brief Main system workspace.')
    env_work=env_work.replace('STATIC_ASSERT_SIZEOF(s_SysWork_2388, 392);','// PORT: Native primitive-data pointer and alignment.\nSTATIC_ASSERT_SIZEOF(s_SysWork_2388, 400);')
    data.append(env_work)
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
    source.append('static void GameBoot_LoadingScreen(void);\nstatic s32 g_MapAreaLoadCounter;\n')
    from prepare_maps import initializer
    source.append(initializer(read('src/bodyprog/game_boot/fs_chara_anim.c'),'D_800A998C'))
    # PORT: Retain the entire original startup dispatcher. Unavailable leaves
    # fail with their own names rather than replacing or skipping its states.
    guards = ['Demo_DemoFileSavegameUpdate','Demo_PlayFileBufferSetup','Demo_PlayDataRead','Demo_Start',
              'AreaLoad_TransitionSound']
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
        'src/bodyprog/world/world_effects.c': ['WorldEnv_MapPresetSet', 'Gfx_MapEnvSet', 'Gfx_MapEnvUpdate', 'Gfx_MapEnvStepUpdate', 'Gfx_FogParametersSet'],
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
    effects = read('src/bodyprog/world/world_effects.c')
    for name in ['D_800A9F80','D_800A9F84','D_800A9F88','D_800A9F8C','D_800A9F98']:
        source.append(initializer(effects, name))
    source.append(initializer(read('src/bodyprog/sys/map_info.c'), 'MAP_EFFECTS_INFOS'))
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
        source.append(f'void {name}(void) {{port_unimplemented("{name}/native system state");}}\n')
    names=['GameState_InGame_Update','SysState_Gameplay_Update','SysState_EventCallback_Update']
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
    for name in names:
        code=function(states,name)
        code=code.replace('g_SysWork.field_2388.', 'g_SysWork.gameplayEnvironment.')
        code=code.replace('g_SysWork.gameplayEnvironment.isFlashlightOn','g_SysWork.field_2388.isFlashlightOn')
        code=code.replace('Player_Update(player, FS_BUFFER_0,','Player_Update(player, (s_AnmHeader*)FS_BUFFER_0,')
        # PORT: First member is the player, but native C calls it by its actual type.
        code=code.replace('Player_CombatUpdate(&g_SysWork.playerWork,','Player_CombatUpdate(&g_SysWork.playerWork.player,')
        code=code.replace('Chara_Flag8Clear(', 'sh_combat_Chara_Flag8Clear(')
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
            code=code.replace('while (true)','for (;; mapEvent++)').replace('        mapEvent++;','        if(mapEvent >= g_MapOverlayHdr.mapEvents+72)port_unimplemented("MAP0_S00 event terminator");')
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


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--decomp', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    generate(args.decomp, args.out)
