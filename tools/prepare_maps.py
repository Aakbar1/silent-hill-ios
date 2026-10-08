"""Native MAP0_S00 descriptor from pinned GPL C, without reading disc bytes.

SPDX-License-Identifier: GPL-3.0-only. Only migrated records receive native
assertions. Unlinked callbacks terminate explicitly, never masquerade as logic.
"""
from pathlib import Path
import argparse
import re
from prepare_gameplay import between, function

NOTICE = '/* SPDX-License-Identifier: GPL-3.0-only; Copyright (C) 2026 shdecompilations; derived from silent-hill-decomp. */\n'


def no_includes(text):
    return re.sub(r'^#include[^\n]*', '', text, flags=re.M)


def enumeration(text, name):
    return re.search(r'typedef enum _' + name + r'\b.*?}\s*[es]_' + name + r';', text, re.S)[0] + '\n'


def initializer(text, name):
    match = re.search(r'^([^\n;]+\b' + re.escape(name) + r'\s*(?:\[[^\n]*?\])?(?:\)\([^)]*\))?\s*=\s*)\{', text, re.M)
    if not match:
        raise ValueError(f'missing initializer {name}')
    masked = re.sub(r'//[^\n]*|/\*.*?\*/|"(?:\\.|[^"\\])*"', lambda m: ' ' * len(m[0]), text, flags=re.S)
    depth = 0
    start = text.index('{', match.start())
    for i in range(start, len(text)):
        depth += (masked[i] == '{') - (masked[i] == '}')
        if depth == 0:
            return text[match.start():text.index(';', i) + 1] + '\n'
    raise ValueError(f'unterminated initializer {name}')


def prepare(decomp, output):
    def read(path):
        return (decomp / path).read_text(encoding='utf-8')
    game = read('include/game.h')
    constants = '#include "shared_types.h"\n'
    constants += ''.join(enumeration(game, name) for name in ['PaperMapIdx', 'SysState', 'ProcessFlags', 'SysFlags'])
    constants += enumeration(read('include/bodyprog/items.h'), 'InvItemLoadFlags')
    constants += enumeration(read('include/bodyprog/bodyprog.h'), 'LoadingScreenId')
    constants += enumeration(read('include/bodyprog/items.h'), 'InvItemId')
    constants += enumeration(read('include/bodyprog/events/bgm_update.h'), 'BgmCmd')
    constants += enumeration(read('include/bodyprog/sound/sfx.h'), 'SfxPairIdx')
    constants += enumeration(read('include/maps/characters/harry.h'), 'HarrySwappableMesh')
    constants += enumeration(read('include/maps/characters/harry.h'), 'HarryVariantMesh')
    constants += enumeration(read('include/maps/characters/harry.h'), 'HarryAnim')
    constants += enumeration(read('include/maps/characters/harry.h'), 'HarryBone')
    player_header = read('include/bodyprog/player.h')
    for name in ['PlayerState', 'PlayerUpperBodyState', 'PlayerLowerBodyState', 'PlayerStopFlags']:
        constants += enumeration(player_header, name)
    constants += no_includes(read('include/event_flags.h'))
    (output / 'map_constants.h').write_text(NOTICE + constants, encoding='utf-8')
    view = no_includes(read('include/bodyprog/view/enums.h'))
    view += between(read('include/bodyprog/view/structs.h'), '#define CAMERA_PATH_COLL_COUNT_MAX', '#endif')
    # PORT: Camera fields are native values, not an implementation-dependent
    # mix of enum/int/short bitfield units. The original source initializers and
    # algorithms use named values; serialized PS1 camera bytes are never read.
    view = re.sub(r'(\b\w+)\s*:\s*\d+;', r'\1;', view)
    view = view.replace('STATIC_ASSERT_SIZEOF(VC_ROAD_DATA, 24);', 'STATIC_ASSERT_SIZEOF(VC_ROAD_DATA, 52);')
    for name, old, new in [('VC_NEAR_ROAD_DATA',36,40),('VC_WORK',744,808),('VbRVIEW',32,40),('VW_VIEW_WORK',132,160)]:
        view = view.replace(f'STATIC_ASSERT_SIZEOF({name}, {old});', f'// PORT: {name} contains native pointers.\nSTATIC_ASSERT_SIZEOF({name}, {new});')
    trigger = between(read('include/bodyprog/collision/trigger.h'), '/** @brief World-space collision trigger', '/** @brief Collection of nearby')
    trigger = re.sub(r'(\b\w+)\s*:\s*\d+;', r'\1;', trigger)
    trigger = trigger.replace('STATIC_ASSERT_SIZEOF(s_CollisionTrigger, 4);', 'STATIC_ASSERT_SIZEOF(s_CollisionTrigger, 24);')
    header = no_includes(read('include/bodyprog/map/map.h'))
    # PORT: Native point/event/spawn records also use explicit named scalar
    # fields. Original PS1 declarations remain unmodified in the submodule;
    # these named records consume C initializers, never serialized map bytes.
    header = re.sub(r'(\b\w+)\s*:\s*\d+;', r'\1;', header)
    header = header.replace('s_MapInfo*             mapInfo;', 'const s_MapInfo*       mapInfo;')
    header = re.sub(r's32(\s+\(\*playerAnimStateSet\))',r'void\1',header)
    # PORT: The pinned Player_AnimStateSet provider returns void in every map.
    header = header.replace('s32 (*playerAnimStateSet)(s32)', 'void (*playerAnimStateSet)(s32)')
    header = header.replace('s32*                   func_88;', 'void*                  func_88;')
    for name, size in [('s_MapPoint2d',32), ('s_EventData',40), ('s_SpawnInfo',16), ('s_MapOverlayHdr', 0)]:
        original = re.search(r'STATIC_ASSERT_SIZEOF\(' + name + r', \d+\);', header)[0]
        header = header.replace(original, f'STATIC_ASSERT_SIZEOF({name}, {size});' if size else '// PORT: Native descriptor size asserted in port/map.h.')
    (output / 'native_map_records.h').write_text(NOTICE + '#ifndef SH_NATIVE_MAP_RECORDS_H\n#define SH_NATIVE_MAP_RECORDS_H\n' + view + trigger + header + '\n#endif\n', encoding='utf-8')
    info = read('src/bodyprog/sys/map_info.c')
    info = info[info.index('const static s_WaterZone'):]
    (output / 'map_info.c').write_text(NOTICE + '#include "map.h"\n' + info, encoding='utf-8')

    reset_player = function(read('src/bodyprog/player_control.c'), 'func_8007E9C4')
    globals_ = sorted(set(re.findall(r'\bg_Player_\w+', reset_player)))
    declarations = []
    for name in globals_:
        decl = re.search(r'(?:extern\s+)?([\w]+)\s+' + name + r'\s*;', player_header + read('include/bodyprog/bodyprog.h') + read('src/bodyprog/player_control.c'))
        if not decl:
            raise ValueError(f'missing reset global declaration {name}')
        declarations.append(f'{decl[1]} {name};\n')
    damage = between(read('include/bodyprog/chara/chara.h'), '#define Chara_DamageClear', '/** @brief Sets a character')
    weapon = function(read('src/bodyprog/items/item_utils.c'), 'func_8004C564')
    weapon_none = between(weapon, '            D_800C3960 =', '        case 0:').replace('            break;', '')
    # PORT: Preserve the exact unequipped startup branch. Other weapon/audio
    # consumers remain explicit guards pending their native records.
    weapon_none = 'static s8 D_800C3960,D_800C3961,D_800C3962;\nstatic u8 D_800C3963;\nvoid func_8004C564(u8 arg0,s8 attack) {(void)arg0;if(attack!=NO_VALUE)port_unimplemented("weapon gas-state update");\n' + weapon_none + '}\n'
    weapon_none = weapon_none.replace('D_800C3960 = g_SavegamePtr->mapIdx', 'D_800C3960 = (s8)g_SavegamePtr->mapIdx')
    spawn = function(read('src/bodyprog/events/player_pos_update.c'), 'Chara_PositionSet')
    spawn = spawn.replace('g_SysWork.cameraAngleY = headingAngle;', 'g_SysWork.cameraAngleY = (s16)headingAngle;\n    port_player_spawn_note();')
    spawn = spawn.replace('g_SavegamePtr->paperMapIdx = mapPoint->paperMapIdx', 'g_SavegamePtr->paperMapIdx = (u8)mapPoint->paperMapIdx')
    room_update = function(read('src/bodyprog/events/bgm_update.c'), 'Game_MapRoomIdxUpdate')
    room_update = room_update.replace('newMapRoomIdx = g_MapOverlayHdr.mapRoomIdxGet(posX, posZ)', 'newMapRoomIdx = (s8)g_MapOverlayHdr.mapRoomIdxGet(posX, posZ)')
    reset_player = '#define playerExtra g_SysWork.playerWork.extra\n#define playerCombat g_SysWork.playerCombat\n' + reset_player + '\n#undef playerExtra\n#undef playerCombat\n'
    (output / 'player_spawn.c').write_text(NOTICE + '#include "gameplay.h"\n' + ''.join(declarations) + damage + weapon_none + reset_player + spawn + room_update, encoding='utf-8')

    lane = 'map0_s00'
    prefix = 'sh_' + lane + '_'
    path = 'src/maps/' + lane + '/'
    upstream_header = read(path + lane + '_header.c')
    callbacks = {}
    # Generate guards with the exact native field type, rather than calling an
    # incompatible void() stub through an unrelated function-pointer type.
    signatures = {m[2]: (m[1], m[3]) for m in re.finditer(r'(\w+)\s+\(\*(\w+)\)\(([^;]*)\);', header)}
    for field, name in re.findall(r'\.(\w+)\s*=\s*(\w+)\s*,', upstream_header):
        if field in signatures and name != 'NULL':
            callbacks[name] = signatures[field]
    extra = ['GameBoot_LoadScreen_PlayerRun', 'GameBoot_LoadScreen_BackgroundImg', 'GameBoot_LoadScreen_StageString']
    events = re.search(r'void \(\*g_MapEventFuncs\[\]\)\(\)\s*=\s*\{(.*?)\};', upstream_header, re.S)[1]
    extra += re.findall(r'\b(?:MapEvent_\w+|MapEven_\w+|func_\w+)\b', events)
    for name in extra:
        callbacks[name] = ('void', 'void')
    callbacks['Stalker_Update'] = callbacks['Cheryl_Update'] = ('void', 's_SubCharacter*, s_AnmHeader*, GsCOORDINATE2*')
    for name in ['Anim_BlendLinear', 'Anim_PlaybackOnce', 'Anim_PlaybackLoop']:
        callbacks[name] = ('void', 's_Model*, s_AnmHeader*, GsCOORDINATE2*, s_AnimInfo*')
    for name in ['Anim_BlendLinear','Anim_PlaybackOnce','Anim_PlaybackLoop','GameBoot_LoadScreen_PlayerRun']:
        del callbacks[name]
    # The original room callback is linked. All remaining unavailable callbacks
    # are guarded in this descriptor slice, and are listed in its inventory.
    callback_code = []
    opening_names = {'MapEvent_CutsceneOpening','Player_ControlFreeze','Player_ControlUnfreeze','Player_AnimStateSet','Player_AnimReset','sharedFunc_800CDAA8_0_s02','sharedFunc_800D1C38_0_s00'}
    linked = {'Map_RoomIdxGet','Particle_EnvironmentSet','sharedFunc_800D0A60_0_s00','Map_WorldObjectsInit','Map_WorldObjectsUpdate','MapEvent_GreyChildrenSpawn'}
    cheryl_names={'Cheryl_Update','Cheryl_AnimUpdate','Cheryl_MovementUpdate','Cheryl_ControlUpdate','Cheryl_FootstepSfxPlay','Cheryl_Init','Chara_CollisionReset','Npc_FootstepSoundPlay'}
    linked |= opening_names | cheryl_names
    # PORT: The particle renderer is deliberately absent in the player lane.
    # Environment selection below is original logic, not this no-draw bridge.
    no_draw = {'Particle_SystemUpdate'}
    for name, (ret, params) in callbacks.items():
        if name in linked:
            continue
        args, unused = [], []
        for i, typ in enumerate(params.split(',')):
            typ = typ.strip()
            if not typ or typ == 'void':
                continue
            # The header's known parameters sometimes already have names.
            typ = re.sub(r'\b(?:idx|arg1|poly|chara|extra|coords|setIdle|playerExtraState|vec|angle|vecCount|npc|player|afkTime|arg2In|angleIn|arg4|arg1|arg3|unused|mapId|vec0|rotX|rotY|from|to|animStatus|keyframeIdx0|keyframeIdx1|keyframeIdx|sfxId|pitch|x|z)\b', '', typ).strip()
            args.append(f'{typ} arg{i}')
            unused.append(f'(void)arg{i};')
        body = '// PORT: No-draw particle bridge; world lane supplies rendering.\n' if name in no_draw else f'port_unimplemented("{lane}/{name}");'
        callback_code.append(f'static {ret} {name}({", ".join(args) or "void"}) {{ {"".join(unused)} '+body + (' return 0;' if ret != 'void' else '') + ' }\n')
    data = no_includes(read(path + lane + '_anim_info.c'))
    data += no_includes(read(path + lane + '_events_data.c'))
    messages = initializer(read(path + lane + '_2.c'), 'MAP_MESSAGES')
    messages = messages.replace('#include "maps/shared/map_msg_common.h"', read('include/maps/shared/map_msg_common.h'))
    data += messages
    # Uninitialized shared buffers are defined by the map's pinned declarations.
    # Their compiled zero image replaces PS1 overlay BSS clearing.
    map_defines = read('include/maps/map0/map0_s00.h')
    for macro in ['MAP_FIELD_4C_COUNT', 'MAP_BLOOD_SPLAT_COUNT_MAX', 'MAP_ROOM_MIN_X', 'MAP_ROOM_MAX_X', 'MAP_ROOM_MIN_Z', 'MAP_ROOM_MAX_Z', 'MAP_HAS_SECONDARY_GRID']:
        m = re.search(r'^#define ' + macro + r'.*$', map_defines, re.M)
        if m:
            data = m[0] + '\n' + data
    data += 's_MapHdr_field_4C sharedData_800DFB7C_0_s00[MAP_FIELD_4C_COUNT];\ns_BloodSplat g_Effect_BloodSplats[MAP_BLOOD_SPLAT_COUNT_MAX];\ns32 g_Particle_SpeedX,g_Particle_SpeedZ,sharedData_800DFB6C_0_s00,sharedData_800DFB70_0_s00;\n'
    data += 'u8 MAP_ROOM_IDXS[224],sharedData_800DF2DC_0_s00[25];\n'
    data += 's_AnimInfo CHERYL_ANIM_INFOS[8];\n'
    data += 's32 D_800DF1CC; q19_12 D_800E3A30;\n'
    data += 's8 g_Player_PrevWeaponAttack;\n'
    data += 's32 sharedData_800E39D8_0_s00; q19_12 g_Player_MoveSpeed; s_CollisionResult sharedData_800E39BC_0_s00;\n'
    data += 's_WorldObjectModel D_800E3A5C[2];\nVECTOR3 D_800E3A9C;\nSVECTOR3 D_800E3AAC;\n'
    # PORT: Namespaced original particle environment scalars also reset on load.
    particle_names = ['D_800C39A0','sharedData_800E0CA8_0_s00','sharedData_800E0CAC_0_s00','sharedData_800E0CB0_0_s00','sharedData_800E0CB8_0_s00','sharedData_800E0CBA_0_s00']
    decls = read('include/maps/particle.h') + read('include/bodyprog/bodyprog.h')
    for name in particle_names:
        typ = re.search(r'extern\s+(\w+)\s+'+name+r';',decls)[1]
        data += f'{typ} {name};\n'
    # no_includes must run AFTER expanding the initializer includes.
    native_header = upstream_header
    for include in ['map_points.h', 'chara_spawns.h', 'vc_road_data.h', 'header_field_D2C.h']:
        native_header = native_header.replace('#include "' + include + '"', read(path + include))
    native_header = no_includes(native_header)
    # PORT: Ambient/event code mutates descriptor data; its native object must
    # be writable even though the upstream initializer is declared const.
    native_header = native_header.replace('const s_MapOverlayHdr g_MapOverlayHdr', 's_MapOverlayHdr g_MapOverlayHdr')
    data += native_header
    # An exhaustive inventory of this linked data slice; future source units
    # must add their writable objects before being linked.
    owned = list(callbacks) + ['g_LoadScreenFuncs', 'g_MapEventFuncs', 'MAP_POINTS', 'MAP_EVENTS', 'MAP_MESSAGES', 'HARRY_M0S00_ANIM_INFOS', 'g_MapHeaderTable_38', 'LOADABLE_INVENTORY_ITEMS', 'sharedData_800DFB7C_0_s00', 'g_Effect_BloodSplats', 'g_Particle_SpeedX', 'g_Particle_SpeedZ', 'sharedData_800DFB6C_0_s00', 'sharedData_800DFB70_0_s00', 'g_MapOverlayHdr', 'GetXIdx', 'GetYIdx', 'MAP_ROOM_IDXS', 'sharedData_800DF2DC_0_s00']
    object_names = ['D_800E3A5C','D_800E3A9C','D_800E3AAC']
    opening_data=['CHERYL_ANIM_INFOS','D_800DF1CC','D_800E3A30','g_Player_PrevWeaponAttack','sharedData_800E39D8_0_s00','g_Player_MoveSpeed','sharedData_800E39BC_0_s00']
    owned += particle_names + object_names + opening_data + sorted(cheryl_names-callbacks.keys())
    namespace = '\n'.join(f'#define {name} {prefix}{name}' for name in owned) + '\n'
    utility = read('src/maps/map_util.c')
    room = between(utility, '#ifdef MAP5_S01', 'u8 Map_RoomIdxGet') + function(utility, 'Map_RoomIdxGet')
    room = 'static bool CheckRange(s32 value,s32 low,s32 high) {return low<=value && value<=high;}\n' + room
    room = room.replace('return res;', 'return (u8)res;')
    writable = ['g_LoadScreenFuncs', 'g_MapEventFuncs', 'MAP_POINTS', 'MAP_EVENTS', 'MAP_MESSAGES', 'HARRY_M0S00_ANIM_INFOS', 'g_MapHeaderTable_38', 'LOADABLE_INVENTORY_ITEMS','g_MapOverlayHdr']
    # Capture each initializer in a const compiled initial image. Pointer-valued
    # fields point to this map's namespaced mutable objects after every reset.
    initials = []
    for name in writable:
        obj = initializer(data, name)
        obj = obj.replace(name, name + '_initial', 1)
        if obj.startswith('void (*'):
            obj = obj.replace('void (*', 'void (* const ', 1)
        elif obj.startswith('const char*'):
            obj = obj.replace('const char*', 'const char* const ', 1)
        else:
            obj = 'const ' + obj
        initials.append('static ' + obj)
    zeros = ['sharedData_800DFB7C_0_s00', 'g_Effect_BloodSplats', 'g_Particle_SpeedX', 'g_Particle_SpeedZ', 'sharedData_800DFB6C_0_s00', 'sharedData_800DFB70_0_s00', 'MAP_ROOM_IDXS', 'sharedData_800DF2DC_0_s00']
    zeros += particle_names + object_names + opening_data
    resets = [f'memcpy(&{name},&{name}_initial,sizeof({name}));' for name in writable]
    resets += [f'memset(&{name},0,sizeof({name}));' for name in zeros]
    probes = [f'memset(&{name},0xa5,sizeof({name}));' for name in writable + zeros]
    comparisons = [f'memcmp(&{name},&{name}_initial,sizeof({name}))==0' for name in writable]
    # The zero-buffer test uses a byte scan, avoiding stack copies of large BSS.
    comparisons += [f'port_map_zero(&{name},sizeof({name}))' for name in zeros]
    reset = f'void {prefix}reset(void) {{' + ''.join(resets) + '}\n'
    reset += f'int {prefix}reset_probe(void) {{' + ''.join(probes) + f'{prefix}reset();return ' + ' && '.join(comparisons) + ';}\n'
    particle = read('src/maps/particle.c')
    environment = function(particle,'sharedFunc_800D0A60_0_s00') + function(particle,'Particle_EnvironmentSet')
    environment = environment.replace('D_800C39A0 = caseArg','D_800C39A0 = (s8)caseArg').replace('D_800C39A0 = arg1','D_800C39A0 = (s8)arg1')
    environment = re.sub(r'(sharedData_800E0CB8_0_s00\s*=) ([^;]+);',r'\1 (u16)(\2);',environment)
    objects = function(read(path+'map0_s00_2.c'),'Map_WorldObjectsInit') + function(read(path+'map0_s00_2.c'),'Map_WorldObjectsUpdate') + function(read(path+'map0_s00_2.c'),'MapEvent_GreyChildrenSpawn')
    objects=objects.replace('rotXy = 0xFAE4FE17','rotXy = (s32)0xFAE4FE17u')
    objects=objects.replace('*(s32*)&D_800E3AAC.vx = rotXy;', '// PORT: Unpack the two rotation values without a misaligned word alias.\n    D_800E3AAC.vx=(s16)(u16)rotXy;D_800E3AAC.vy=(s16)(u16)((u32)rotXy>>16);')
    objects=objects.replace('D_800E3AAC.vz += Q12_MULT_PRECISE(g_DeltaTime, (Q12_ANGLE(-90.0f) - (Rng_Rand16() & 0x1FF)));', 'D_800E3AAC.vz = (s16)(D_800E3AAC.vz + Q12_MULT_PRECISE(g_DeltaTime, (Q12_ANGLE(-90.0f) - (Rng_Rand16() & 0x1FF))));')
    helpers = '''
void Map_WorldObjectsInit(void);void Map_WorldObjectsUpdate(void);void MapEvent_GreyChildrenSpawn(void);
static void WorldObject_ModelNameSet(s_WorldObjectModel* model,char* name) {
    // PORT: Native fixed-length metadata; only the eight-byte opening names are migrated.
    if(strlen(name)!=8)port_unimplemented("map0_s00/world object name length");
    model->metadata.modelLocation=0;model->modelInfo.field_0=0;
    memcpy(model->metadata.name.str,name,8);model->metadata.field_8=0;
}
static void WorldObjects_Add(s_WorldObjectModel* model,VECTOR3* pos,SVECTOR3* rot) {
    // PORT: No-draw object bridge for the world lane. Original wheel state still advances.
    (void)model;(void)pos;(void)rot;
}
static void Sfx_WithFalloffAndPitchPlay(s32 id,VECTOR3* pos,s32 volume,s32 distance,s32 pitch) {
    (void)id;(void)pos;(void)volume;(void)distance;(void)pitch;
    port_unimplemented("map0_s00/wheel positional SFX");
}
'''
    for name in ['func_800DC33C','func_800DC694','func_800DC8D8','func_800DCA30','func_800DCC54','func_800DD0CC']:
        helpers += f'static void {name}(void) {{port_unimplemented("map0_s00/{name}");}}\n'
    chunk_macros=between(game,'#define MAP_CHUNK_CHECK_VARIABLE_DECL()', '#define PLAYER_NOT_IN_MAP_CHUNK')
    helpers += chunk_macros + '\n#define BgmStatusFlag_6 (1<<6)\n#define Sfx_Unk1358 1358\n#define Sfx_Unk1361 1361\n'
    # PORT: This port targets pinned USA/NTSC, selecting its first region branch.
    helpers += re.search(r'^\s*#define ENEMY_CHARA_ID.*$',read(path+'map0_s00_2.c'),re.M)[0]+'\n'
    opening = read('src/maps/characters/player.c')
    opening_code = ''.join(function(opening,name) for name in sorted(opening_names) if name not in {'MapEvent_CutsceneOpening','sharedFunc_800CDAA8_0_s02'})
    opening_code=opening_code.replace('void sharedFunc_800D1C38_0_s00(s_SubCharacter* player, s_PlayerExtra* extra, GsCOORDINATE2* boneCoords)\n{','void sharedFunc_800D1C38_0_s00(s_SubCharacter* player, s_PlayerExtra* extra, GsCOORDINATE2* boneCoords)\n{\n    (void)extra;')
    opening_code += no_includes(read('include/maps/shared/sharedFunc_800CDAA8_0_s02.h'))
    opening_code += function(read(path+lane+'_2.c'),'MapEvent_CutsceneOpening')
    cheryl=read('src/maps/characters/cheryl.c')
    chara=read('src/maps/chara_util.c')
    cheryl_code=no_includes(cheryl)
    cheryl_code+=function(chara,'Chara_CollisionReset')+function(chara,'Npc_FootstepSoundPlay')
    cheryl_code=cheryl_code.replace('    pos          = cheryl->position;', '    pos          = cheryl->position; (void)pos;')
    cheryl_code=cheryl_code.replace('Math_RotMatrixZxyNegGte(&cheryl->rotation, &boneCoords->coord);', '// PORT: Copy actor rotation to padded SDK input.\n    SVECTOR nativeRotation={cheryl->rotation.vx,cheryl->rotation.vy,cheryl->rotation.vz,0};\n    Math_RotMatrixZxyNegGte(&nativeRotation,&boneCoords->coord);')
    cheryl_code=cheryl_code.replace('cheryl->rotation.vy  = Q12_ANGLE_ABS', 'cheryl->rotation.vy  = (s16)Q12_ANGLE_ABS')
    cheryl_code=re.sub(r'(cherylProps.moveSpeed)\s*([+-])=\s*([^;{}]+);',r'\1 = (q3_12)(\1 \2 (\3));',cheryl_code)
    cheryl_code=re.sub(r'(Sfx_WithPitchPlay\([^;]+,) pitch\);',r'\1 (s8)pitch);',cheryl_code)
    opening_code += cheryl_code

    opening_code = opening_code.replace('SysWork_StateSetNext(', 'port_move_state_next(')
    from prepare_camera import narrow
    records=(output/'native_gameplay_records.h').read_text()
    records=re.sub(r'\b(?:s16|s32|q3_12|q4_12)\s+(?:vx|vy|vz|moveSpeed)\s*;', '',records)
    records=re.sub(r'\b(q3_12|q7_8|q11_4|u8|s8|s16|u16)\s+(field_\w+)\s*;',r'\1 port_hidden_\2;',records)
    opening_code=narrow(opening_code,records)
    opening_code=re.sub(r'((?:playerProps|playerChara->properties.player)\.(?:moveSpeed|headingAngle))\s*=(?!=)\s*([^;{}]+);',r'\1 = (s16)(\2);',opening_code)
    opening_code=opening_code.replace('headingAngle = playerProps.headingAngle =', 'headingAngle = playerProps.headingAngle = (s16)')
    # PORT: Compound stores retain the original halfword wrap, explicitly.
    opening_code=re.sub(r'(playerProps.moveSpeed)\s*([+-])=\s*([^;{}]+);',r'\1 = (q3_12)(\1 \2 (\3));',opening_code)
    opening_code=opening_code.replace('player->rotationSpeed.vy = FP_TO(sharedData_800E39D8_0_s00, Q8_SHIFT) / g_DeltaTime;', 'player->rotationSpeed.vy = (s16)(FP_TO(sharedData_800E39D8_0_s00, Q8_SHIFT) / g_DeltaTime);')
    opening_code=opening_code.replace('Math_RotMatrixZxyNegGte(&player->rotation, &boneCoords[HarryBone_Root].coord);', '// PORT: Copy six actor rotation bytes into the padded SDK argument.\n    SVECTOR nativeRotation={player->rotation.vx,player->rotation.vy,player->rotation.vz,0};\n    Math_RotMatrixZxyNegGte(&nativeRotation,&boneCoords[HarryBone_Root].coord);')
    opening_code=opening_code.replace('vcChangeProjectionValue(Dms_CameraTargetsGet(', 'vcChangeProjectionValue((q3_12)Dms_CameraTargetsGet(')
    opening_code=opening_code.replace('adjMoveOffsetX = Q12_MULT(', 'adjMoveOffsetX = (q3_12)Q12_MULT(').replace('adjMoveOffsetZ = Q12_MULT(', 'adjMoveOffsetZ = (q3_12)Q12_MULT(')
    opening_prototypes=''.join(re.sub(r'//[^\n]*','',function(opening,name).split('{',1)[0]).strip()+';\n' for name in sorted(opening_names) if name not in {'MapEvent_CutsceneOpening','sharedFunc_800CDAA8_0_s02'})
    opening_prototypes+=no_includes(read('include/maps/characters/cheryl.h'))
    for name in ['Chara_CollisionReset','Npc_FootstepSoundPlay']:
        opening_prototypes+=function(chara,name).split('{',1)[0].strip()+';\n'
    chara_header=read('include/bodyprog/chara/chara.h')
    opening_prototypes+=function(chara_header,'Chara_AnimStateReset')
    opening_prototypes+=between(chara_header,'#define Chara_AnimUpdate(', '/** @brief Sets the animation of a character.')
    opening_prototypes+='void sharedFunc_800CDAA8_0_s02(s_SubCharacter*,s_PlayerExtra*,GsCOORDINATE2*);void MapEvent_CutsceneOpening(void);\n'
    player_defs=read('include/bodyprog/player.h')
    opening_prototypes+=enumeration(player_defs,'PlayerCutsceneState').replace('PlayerCutsceneState_RunForward','PortUnused_PlayerCutsceneState_RunForward')
    opening_prototypes+='extern s8 g_Player_PrevWeaponAttack;void func_8003D01C(void);void func_8003D03C(void);extern s_DmsHeader port_move_dms_header;\n#define FS_BUFFER_16 (&port_move_dms_header)\n#ifndef USHRT_MAX\n#define USHRT_MAX 65535\n#endif\n'
    opening_prototypes+='\n#define DEFAULT_PLAYER_CYLINDER_FIELD_2 Q12(0.23f)\n'
    map_defs=no_includes(read('include/maps/map0/map0_s00.h'))
    opening_prototypes+='\n'.join(line for line in map_defs.splitlines() if line.startswith('#define HAS_PlayerState_'))+'\n'
    source = NOTICE + '#include "npc_startup.h"\n#include "native_player_events.h"\n#include "camera.h"\n#define MAP0_S00\n#undef CHUNK_SIZE\n#define CHUNK_SIZE 40\n#undef g_MapOverlayHdr\n' + namespace + opening_prototypes + ''.join(callback_code) + 'static u8 Map_RoomIdxGet(q19_12,q19_12);\nvoid Particle_EnvironmentSet(s8,u32);\nvoid sharedFunc_800D0A60_0_s00(s32);\n' + helpers + data + ''.join(initials) + room + environment + objects + opening_code + reset
    # PORT: ISO C empty initializers retain zero values explicitly.
    source = re.sub(r'\{\s*}', '{0}', source)
    # PORT: Animation linkStatus is an unsigned PS1 byte; retain 0xff sentinel.
    source = source.replace('false, NO_VALUE,', 'false, (u8)NO_VALUE,')
    # PORT: Missing map animation rodata is decoded on the owned disc at runtime.
    # Function addresses are identities from pinned sym.bodyprog.txt, never casts.
    source += r"""
static u16 move_half(const u8* p) {return (u16)(p[0]|((u16)p[1]<<8));}
static u32 move_word(const u8* p) {return (u32)move_half(p)|((u32)move_half(p+2)<<16);}
static int move_cheryl_table_load(void) {
    u8 bytes[8*16]; s_AnimInfo decoded[8]={0};
    if(port_map_data_read(FILE_VIN_MAP0_S00_BIN,0x15bfc,sizeof(bytes),bytes))return 1;
    for(size_t i=0;i<8;i++) {
        const u8* row=bytes+i*16; s_AnimInfo* info=&decoded[i];
        switch(move_word(row)) {
            case 0x80044CA4: info->playbackFunc=Anim_BlendLinear;break;
            case 0x80044B38: info->playbackFunc=Anim_PlaybackLoop;break;
            case 0x800449F0: info->playbackFunc=Anim_PlaybackOnce;break;
            default:return 1;
        }
        if(row[4]!=i || row[5] || (row[6]!=255 && row[6]>=8))return 1;
        info->status=row[4];info->hasVariableDuration=0;info->linkStatus=row[6];
        info->duration.constant=(s32)move_word(row+8);
        info->startKeyframeIdx=(s16)move_half(row+12);info->endKeyframeIdx=(s16)move_half(row+14);
        if(info->startKeyframeIdx<-1 || info->endKeyframeIdx<0 || info->startKeyframeIdx>info->endKeyframeIdx)return 1;
    }
    memcpy(CHERYL_ANIM_INFOS,decoded,sizeof(decoded));return 0;
}
"""
    source += f'const s_MapOverlayHdr* {prefix}descriptor(void) {{ return &g_MapOverlayHdr; }}\n'
    source += f'int {prefix}load_data(void) {{ return move_cheryl_table_load() || port_map_data_read(FILE_VIN_MAP0_S00_BIN,0x15c84,224,MAP_ROOM_IDXS) || port_map_data_read(FILE_VIN_MAP0_S00_BIN,0x15d64,25,sharedData_800DF2DC_0_s00); }}\n'
    (output / (lane + '.c')).write_text(source, encoding='utf-8')
    print(f'{lane}: {len(writable)+len(zeros)} writable data objects; {len(linked & callbacks.keys())} original callbacks; {len(no_draw)} no-draw; {len(callbacks)-len(linked & callbacks.keys())-len(no_draw)} guarded callbacks')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--decomp', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    prepare(args.decomp, args.out)
