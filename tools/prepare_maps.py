"""Native map descriptor/data slices from pinned GPL C, without game bytes.

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
        scalar = re.search(r'^([^\n;{}]+\b' + re.escape(name) + r'\s*=\s*[^\n;{}]+;)',text,re.M)
        if scalar:
            return scalar[1] + '\n'
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
    # PORT: Match the shared implementations; pointer-sized descriptor ABI stays fixed.
    header = re.sub(r'void(\s+\(\*playerAnimIsLocked\))\(void\)',r's32\1(void)',header)
    header = header.replace('(*charaAnimReset)()', '(*charaAnimReset)(s_SubCharacter*, bool)')
    header = header.replace('s32*                   func_88;', 'void*                  func_88;')
    # PORT: Replace the opaque numeric 5C view with its original named view.
    # Size and offsets are unchanged; init bodies now retain all original stores.
    field_end = header.index('} s_MapOverlayHdr_5C;') + len('} s_MapOverlayHdr_5C;')
    field_start = header.rfind('typedef struct', 0, field_end)
    old_field_view = header[field_start:field_end]
    field_view = old_field_view
    for old,new in [('u8    unk_0;', 'u8    flags;'), ('u8    unk_3;', 'u8    field_3;'), ('u8    unk_8[2];', 'u16   field_8;'), ('u8    unk_C[2];', 'u16   field_C;'), ('u8    unk_12[4];', 's16   field_12; s16 field_14;'), ('u8    unk_18[8];', 's16   field_18; s16 field_1A; u16 field_1C; s16 field_1E;')]:
        assert field_view.count(old) == 1
        field_view = field_view.replace(old,new)
    field_view += '\nSTATIC_ASSERT_SIZEOF(s_MapOverlayHdr_5C,40);\n_Static_assert(offsetof(s_MapOverlayHdr_5C,field_12)==18 && offsetof(s_MapOverlayHdr_5C,field_1C)==28,"named 5C numeric offsets");\n'
    header = header.replace(old_field_view, field_view)
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
void Sfx_WithFalloffAndPitchPlay(e_SfxId,VECTOR3*,s32,q19_12,s8);
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
    source += f'u32 {prefix}point_count(void) {{return (u32)ARRAY_SIZE(MAP_POINTS);}}\n'
    source += f'u32 {prefix}event_count(void) {{return (u32)ARRAY_SIZE(MAP_EVENTS);}}\n'
    source += f'u32 {prefix}callback_count(void) {{return (u32)ARRAY_SIZE(g_MapEventFuncs);}}\n'
    source += f'int {prefix}load_data(void) {{ return move_cheryl_table_load() || port_map_data_read(FILE_VIN_MAP0_S00_BIN,0x15c84,224,MAP_ROOM_IDXS) || port_map_data_read(FILE_VIN_MAP0_S00_BIN,0x15d64,25,sharedData_800DF2DC_0_s00); }}\n'
    (output / (lane + '.c')).write_text(source, encoding='utf-8')
    print(f'{lane}: {len(writable)+len(zeros)} writable data objects; {len(linked & callbacks.keys())} original callbacks; {len(no_draw)} no-draw; {len(callbacks)-len(linked & callbacks.keys())-len(no_draw)} guarded callbacks')
    prepare_all_maps(decomp, output, header)


def prepare_all_maps(decomp, output, records):
    """Compile the descriptor/data closure, with typed guards at unmigrated code.

    MAP0_S00 remains the events lane's provider. Every other descriptor uses
    the same inventory/reset algorithm, derived exclusively from GPL source.
    """
    import json
    paths = sorted((decomp / 'src/maps').glob('map*/*_header.c'))
    assert len(paths) == 43, 'review pinned map inventory'
    declarations = '\n'.join(p.read_text(encoding='utf-8') for p in (decomp / 'include/maps').rglob('*.h'))
    signatures = {m[2]: (m[1], m[3]) for m in re.finditer(r'(\w+)\s+\(\*(\w+)\)\(([^;]*)\);', records)}
    inventory = []
    registry = NOTICE + '#include "maps_registry.h"\n'
    shared = (decomp / 'include/maps/shared.h').read_text(encoding='utf-8')
    object_macros = between(shared, '#define WorldObject_PoseInit', '#define APPROACH(')
    common_names = initializer((decomp / 'src/bodyprog/events/bodyprog_data_800A99B4.c').read_text(encoding='utf-8'), 'g_CommonWorldObjectNames')
    poses = between(shared, 'typedef struct _WorldObjectPose', '#define WorldObject_PoseInit')
    poses = re.sub(r'STATIC_ASSERT_SIZEOF\([^;]+;', '', poses)
    poses += 'STATIC_ASSERT_SIZEOF(s_WorldObjectPose,72);\nSTATIC_ASSERT_SIZEOF(s_WorldObjectPlacement,64);\n_Static_assert(offsetof(s_WorldObjectPose,rotation)==60,"native object pose rotation");\n'
    gpu_types = (decomp/'include/gpu.h').read_text(encoding='utf-8')
    color_start = gpu_types.index('typedef struct _PrimColor')
    color_end = gpu_types.index('} s_PrimColor;',color_start) + len('} s_PrimColor;')
    poses += gpu_types[color_start:color_end] + '\n'
    item_enum = enumeration((decomp / 'include/bodyprog/items.h').read_text(encoding='utf-8'), 'CommonPickupItemId')
    ending_enum = enumeration((decomp / 'include/game.h').read_text(encoding='utf-8'), 'GameEnding')
    services = '''
bool Player_ItemRemove(u8 id,u8 count);
void func_8008D438(void);void func_8008D448(void);bool Chara_ProcessLoads(void);
s32 port_maps_effect_alloc(s32);
void port_maps_init_note(const char*);
void port_maps_object_sfx(s32);
int port_maps_spawn_read(u32,u32,u32,s_SpawnInfo*);
bool Chara_Load(s32,s8,GsCOORDINATE2*,s8,s_LmHeader*,s_FsImageDesc*);
void Dms_HeaderFixOffsets(s_DmsHeader*);
// PORT: Cafe DMS publication is still unavailable; guard before exposing any buffer.
static inline void* port_maps_cafe_dms(void) {port_unimplemented("map0_s01/cafe DMS native publication");return NULL;}
#define FS_BUFFER_11 port_maps_cafe_dms()
// PORT: Guard missing shared records at their first actual use, retaining init logic.
extern u8 g_EndingIdx;
extern s_FsImageDesc g_LoadingScreenImg;
// PORT: The existing bounded native TIM scratch replaces the PS1 image address.
#define IMAGE_BUFFER_4 port_fs_buffers[2]
static inline void SysWork_NpcFlagSet(s32 i) {g_SysWork.npcFlags|=(s32)(1u<<i);}
'''
    (output / 'maps_objects.h').write_text(NOTICE + '#include "npc_startup.h"\n#include "native_player_controls.h"\n#include "native_player_dms.h"\n#define WorldObject_ModelNameSet port_world_object_name_set\n#undef Math_SetSVectorFast\n#define Math_SetSVectorFast(p,x,y,z) Math_SVectorSet(p,x,y,z)\n' + poses + item_enum + ending_enum + '\n' + object_macros + services + 'static ' + common_names + '\n', encoding='utf-8')
    for index, path in enumerate(paths):
        lane = path.parent.name
        prefix = 'sh_' + lane + '_'
        if index == 0:
            inventory.append(dict(name=lane.upper(), index=index, provider='events', objects=None))
        else:
            original = path.read_text(encoding='utf-8')
            local = '\n'.join(p.read_text(encoding='utf-8') for p in sorted(path.parent.glob('*.c')))
            local = re.sub(r'/\*.*?\*/', '', local, flags=re.S)
            try:
                init_code = function(local, 'Map_WorldObjectsInit')
            except ValueError:
                init_code = 'void Map_WorldObjectsInit(void) {port_unimplemented("unresolved map init provider");}'
            init_body = re.sub(r'//[^\n]*', '', init_code.split('{',1)[1].rsplit('}',1)[0]).strip()
            empty_init = not init_body
            # These bounded init closures contain original state setup only.
            init_extra = {}
            if lane == 'map1_s05':
                draw = (decomp/'src/maps/unk_draw.c').read_text(encoding='utf-8')
                init_extra = {name:function(draw,name) for name in ['sharedFunc_800CAAD0_1_s05','sharedFunc_800CABF8_1_s05']}
            elif lane == 'map6_s04':
                init_extra = {name:function(local,name) for name in ['func_800DE26C','func_800DF64C','func_800E02E0','func_800E10F8']}
            elif lane == 'map2_s00':
                init_extra = {name:function(local,name) for name in ['func_800EE518','func_800EE5D0']}
            elif lane == 'map4_s03':
                init_extra = {name:function(local,name) for name in ['func_800D7408','func_800D7450','func_800D7548','func_800D761C']}
            elif lane == 'map7_s03':
                init_extra = {name:function(local,name) for name in ['func_800D5D24','func_800E16FC','func_800E1788']}
            macros = '\n'.join(line for line in (decomp / f'include/maps/map{lane[3]}/{lane}.h').read_text(encoding='utf-8').splitlines() if line.startswith('#define MAP_'))
            callbacks = {}
            for field, name in re.findall(r'\.(\w+)\s*=\s*(\w+)\s*,', original):
                if field in signatures and name != 'NULL':
                    callbacks[name] = signatures[field]
            for array in ['g_LoadScreenFuncs', 'g_MapEventFuncs']:
                obj = initializer(original, array)
                for name in re.findall(r'\b(?:GameBoot_\w+|MapEvent_\w+|MapEven_\w+|func_\w+|sharedFunc_\w+)\b', obj):
                    callbacks[name] = ('void', 'void')
            charas = re.search(r'\.charaUpdateFuncs\s*=\s*\{(.*?)\}', original, re.S)[1]
            for name in re.findall(r'\b\w+_Update\b', charas):
                callbacks[name] = ('void', 's_SubCharacter*, s_AnmHeader*, GsCOORDINATE2*')
            callbacks.pop('GameBoot_LoadScreen_PlayerRun', None)
            callbacks.pop('Map_RoomIdxGet', None)
            callbacks.pop('Map_WorldObjectsInit', None)
            for name in init_extra:
                callbacks.pop(name, None)
            for name in re.findall(r'charaUpdateFuncs\[[^]]+\]\s*=\s*(func_\w+)', init_code):
                callbacks[name] = ('void', 's_SubCharacter*, s_AnmHeader*, GsCOORDINATE2*')
            animation_source=(path.parent / f'{lane}_anim_info.c').read_text(encoding='utf-8')
            duration_names=set(re.findall(r'\{\s*((?:sharedFunc|Player_)\w+)\s*}',animation_source))
            for name in duration_names:
                if name != 'Player_VariableAnimDurationGet':
                    callbacks[name]=('q19_12','s_Model*')
            code = ''
            for name, (ret, params) in sorted(callbacks.items()):
                args = []
                for i, param in enumerate(params.split(',')):
                    param = param.strip()
                    if not param or param == 'void':
                        continue
                    # PORT: Preserve the descriptor's exact function type.
                    param = re.sub(r'\b([A-Za-z_]\w*)\s*$', lambda m: '' if m[1] not in ['bool','s8','u8','s16','u16','s32','u32','q3_12','q19_12'] and not m[1].startswith('s_') else m[1], param).strip()
                    args.append(f'{param} arg{i}')
                code += '// PORT: Unmigrated callback fails explicitly; no substitute game logic.\n'
                code += f'static {ret} {name}({", ".join(args) or "void"}) {{' + ''.join(f'(void)arg{i};' for i in range(len(args))) + f'port_unimplemented("{lane}/{name}");' + ('return 0;' if ret != 'void' else '') + '}\n'
            header = original
            for included in ['map_points.h', 'chara_spawns.h', 'vc_road_data.h', 'header_field_D2C.h']:
                header = header.replace(f'#include "{included}"', (path.parent / included).read_text(encoding='utf-8'))
            header = no_includes(header)
            header = re.sub(r'^extern[^;]*;\s*', '', header, flags=re.M)
            header = header.replace('const s_MapOverlayHdr g_MapOverlayHdr', 's_MapOverlayHdr g_MapOverlayHdr')
            data = no_includes((path.parent / f'{lane}_anim_info.c').read_text(encoding='utf-8'))
            data += no_includes((path.parent / f'{lane}_events_data.c').read_text(encoding='utf-8'))
            data = re.sub(r'/\*.*?\*/', '', data, flags=re.S)
            messages_missing = False
            try:
                messages = initializer(local, 'MAP_MESSAGES')
            except ValueError:
                messages_missing = True
                messages = 'const char* MAP_MESSAGES[1]={NULL};\n'
            messages = messages.replace('#include "maps/shared/map_msg_common.h"', (decomp / 'include/maps/shared/map_msg_common.h').read_text(encoding='utf-8'))
            data += messages
            defined = re.findall(r'^\s*(?:const\s+)?[\w]+(?:\s*\*)*\s+(\w+)\s*(?:\[[^\n]*?\])?\s*=\s*\{', data, re.M)
            defined += re.findall(r'^\s*(?:const\s+)?[\w]+(?:\s*\*)*\s+(\w+)\s*=\s*[^;{}\n]+;', data, re.M)
            # PORT: Only objects reachable from this descriptor enter this slice.
            # Callback-local state enters the inventory when its code is migrated.
            fields = ['loadableItems','field_38','unkTable1_4C','bloodSplats','field_5C','field_7C','func_88','field_94','ptr_A0','particleWindSpeedX','particleWindSpeedZ','data_18C','data_190']
            zeros = []
            raw_loads = []
            symbols = (decomp / f'configs/USA/maps/sym.{lane}.txt').read_text(encoding='utf-8')
            addresses = {m[1]: int(m[2],16) for m in re.finditer(r'^(\w+)\s*=\s*0x([0-9A-Fa-f]+);',symbols,re.M)}
            yaml = (decomp / f'configs/USA/maps/{lane}.yaml').read_text(encoding='utf-8')
            base_address = int(re.search(r'\bvram:\s*0x([0-9A-Fa-f]+)',yaml)[1],16)
            if messages_missing:
                # PORT: The next pinned symbol bounds the PS1 pointer table.
                # Rust reads/decodes owned bytes at runtime into these reset arenas.
                begin = addresses['MAP_MESSAGES']
                end = min(address for address in addresses.values() if address > begin)
                count, remainder = divmod(end-begin, 4)
                assert not remainder and 0 < count <= 64, (lane, count)
                data = data.replace('const char* MAP_MESSAGES[1]={NULL};', f'const char* MAP_MESSAGES[{count}]={{NULL}};\nu8 port_maps_message_arena[{count * 4096}];')
                zeros.append('port_maps_message_arena')
                raw_loads.append(f'port_maps_messages_read(FILE_VIN_{lane.upper()}_BIN,{base_address}u,{begin-base_address},{count},MAP_MESSAGES,port_maps_message_arena,sizeof(port_maps_message_arena))')
                messages_missing = False
            # PORT: Link the original init body and inventory its complete writable
            # closure. Declarations come from this map before shared headers.
            map_decls = (decomp / f'include/maps/map{lane[3]}/{lane}.h').read_text(encoding='utf-8')
            init_decls = re.sub(r'/\*.*?\*/|//[^\n]*', '', map_decls + '\n' + local + '\n' + declarations, flags=re.S)
            init_globals = sorted(set(re.findall(r'\b(?:D_\w+|g_WorldObject\w*|WorldObject_D_\w+|g_GeneratorMakeNoise|g_Obj\w+|g_CommonWorldObjects|g_Cutscene\w*|sharedData_\w+)\b', re.sub(r'"[^"\n]*"', '', init_code + ''.join(init_extra.values())))))
            init_types = ''
            emitted_types = set()
            def add_init_type(typ):
                nonlocal init_types
                if typ in emitted_types:
                    return
                end = init_decls.find('} ' + typ + ';')
                if end < 0:
                    return
                emitted_types.add(typ)
                start = init_decls.rfind('typedef struct', 0, end)
                definition = init_decls[start:end + len('} ' + typ + ';')]
                for dependency in sorted(set(re.findall(r'\bs_\w+\b',definition)) - {typ}):
                    if '} ' + dependency + ';' in map_decls:
                        add_init_type(dependency)
                if typ == 's_800ED848':
                    definition = definition.replace('typedef struct', 'typedef struct s_800ED848',1)
                init_types += definition + '\n'
            for name in init_globals:
                if name in ['D_800C4418','D_800C4414']:
                    continue  # Guarded shared boss workspace, not map-owned BSS.
                if name in defined or re.search(r'\.' + r'(?:' + '|'.join(fields) + r')\s*=\s*&?' + re.escape(name) + r'\b', original):
                    continue
                decl = re.search(r'(?:extern\s+)?(\w+\s+' + re.escape(name) + r'\s*(?:\[[^;\n]*\])?)\s*;', init_decls)
                if not decl:
                    raise ValueError(f'{lane}: init global declaration absent: {name}')
                declaration = decl[1]
                typ = declaration.split()[0]
                if name == 'D_800F1CAC':
                    # PORT: Upstream memcpy uses a misnamed point table as 32
                    # serialized spawn records. Native strides differ: decode
                    # each 12-byte spawn before the original table selection.
                    assert declaration.split() == ['s_MapPoint2d', 'D_800F1CAC[3][32]']
                    declaration = declaration.replace('s_MapPoint2d', 's_SpawnInfo')
                    raw_loads.append(f'port_maps_spawn_read(FILE_VIN_MAP2_S00_BIN,{0x800F1CAC-base_address},96,&D_800F1CAC[0][0])')
                if typ.startswith('s_') and typ not in ['s_WorldObjectModel','s_WorldObjectPose','s_WorldObjectPlacement','s_Pose']:
                    add_init_type(typ)
                if '[]' in declaration:
                    # PORT: These original model-only BSS arrays have six common
                    # pickup slots; other unknown arrays stay explicitly bounded.
                    if name == 'g_CommonWorldObjects':
                        declaration = declaration.replace('[]', '[6]')
                    elif name == 'D_800F2418':
                        # PORT: Only the original six-element init footprint is
                        # linked; the broader finale effect consumers stay guarded.
                        declaration = declaration.replace('[]', '[6]')
                    else:
                        raise ValueError(f'{lane}: unbounded init array {name}')
                data += declaration + ';\n'
                zeros.append(name)
                if name in ['D_800DB7D4','D_800DB7E4']:
                    # Original source declares exactly ten pointer-free SVECTOR
                    # pairs at these named PS1 addresses. Read only that footprint.
                    raw_loads.append(f'port_map_data_read(FILE_VIN_MAP4_S03_BIN,{int(name[2:],16)-base_address},sizeof({name}),(u8*)&{name})')
            data = init_types + data
            init_helpers = ''
            init_prototypes = ''
            for name,provider in sorted(init_extra.items()):
                init_prototypes += 'static ' + re.sub(r'//[^\n]*','',provider.split('{',1)[0]).strip()+';\n'
                if lane == 'map1_s05':
                    effect_fields = dict((field,typ) for typ,field in re.findall(r'\b(u8|u16|s16|q3_12)\s+(\w+)\s*;', shared[shared.index('typedef struct _MapHeader_field_5C'):shared.index('} s_MapHeader_field_5C;')]))
                    for field,typ in effect_fields.items():
                        provider = re.sub(r'(sharedData_800D8568_1_s05\.'+field+r'\s*=) ([^;]+);',r'\1 ('+typ+r')(\2);',provider)
                    provider = re.sub(r'(randAngle\s*=) ([^;]+);',r'\1 (q3_12)(\2);',provider)
                    provider = re.sub(r'(sharedData_800DFB7C_0_s00\[idx\]\.(?:field_C\.s_0\.field_[02]|field_10\.s_0\.field_2|vy_8)\s*=) ([^;]+);',r'\1 (s16)(\2);',provider)
                    provider = re.sub(r'(sharedData_800DFB7C_0_s00\[idx\]\.field_B\s*=) ([^;]+);',r'\1 (u8)(\2);',provider)
                if name == 'func_800D761C':
                    provider=provider.replace('{','{\n    (void)arg3;',1).replace('field_20 = arg2;', 'field_20 = (u8)arg2;')
                init_helpers += 'static ' + provider + '\n'
            for name in sorted(set(re.findall(r'\b(?:func_\w+|sharedFunc_\w+)(?=\()', init_code))):
                if name in ['func_8008D438','func_8008D448'] or name in init_extra:
                    continue
                try:
                    provider = function(local, name)
                except ValueError:
                    signature_match = re.search(r'\b(void|s32|q19_12)\s+' + re.escape(name) + r'\([^;]*\);', declarations)
                    if not signature_match:
                        raise
                    provider = signature_match[0].rstrip(';') + '{}'
                signature = re.sub(r'//[^\n]*', '', provider.split('{',1)[0]).strip()
                if name == 'func_800CB0D8':
                    provider = provider.replace('func_8005E7E0(', 'port_maps_effect_alloc(')
                    provider = provider.replace('field_C.s_1.field_2 = Rng_GenerateInt(0, 2)', 'field_C.s_1.field_2 = (s16)Rng_GenerateInt(0, 2)')
                    provider = provider.replace('field_C.s_2.field_0 = Rng_GenerateUInt(0, Q12_CLAMPED(1.0f))', 'field_C.s_2.field_0 = (s16)Rng_GenerateUInt(0, Q12_CLAMPED(1.0f))')
                elif name == 'func_800DD348':
                    provider = provider.replace('{', '{\n    (void)unused;',1)
                    provider = provider.replace('u8* curSpawnFlags;', 's8* curSpawnFlags;').replace('*curSpawnFlags = spawnFlags;', '*curSpawnFlags = (s8)spawnFlags;')
                    init_code = init_code.replace('func_800DD348(16,', 'func_800DD348(NULL,')
                else:
                    params = signature.split('(',1)[1].rsplit(')',1)[0].split(',')
                    unused = ''.join('(void)'+re.findall(r'\b\w+', p)[-1]+';' for p in params if p.strip()!='void')
                    provider = signature + '{' + unused + f'port_unimplemented("{lane}/{name}");' + ('return 0;' if not signature.startswith('void') else '') + '}\n'
                init_helpers += 'static ' + provider + '\n'
            if 'Chara_SpawnFlagsSet' in init_code:
                spawn_helper = function((decomp / 'src/bodyprog/events/chara_spawn.c').read_text(encoding='utf-8'), 'Chara_SpawnFlagsSet')
                spawn_helper = spawn_helper.replace('spawnInfo->flags = spawnFlags;', 'spawnInfo->flags = (u8)spawnFlags;')
                init_helpers += 'static ' + spawn_helper
            room_sizes = {}
            for name in ['MAP_ROOM_IDXS','sharedData_800DF2DC_0_s00','sharedData_800ED430_2_s02']:
                if name not in addresses:
                    continue
                begin=addresses[name]
                end=min(addr for addr in addresses.values() if addr>begin)
                size=end-begin
                assert 0<size<=4096,(lane,name,size)
                room_sizes[name]=size
                data+=f'u8 {name}[{size}];\n'
                zeros.append(name)
                raw_loads.append(f'port_map_data_read(FILE_VIN_{lane.upper()}_BIN,{begin-base_address},sizeof({name}),{name})')
            for field in fields:
                expression = re.search(r'\.' + field + r'\s*=\s*([^,\n]+)', original)[1]
                name = re.search(r'\b\w+\b', expression)[0]
                if name == 'NULL' or name in defined:
                    continue
                try:
                    obj = initializer(local, name)
                    data += obj
                    defined.append(name)
                except ValueError:
                    decl = re.search(r'extern\s+([^;\n]*\b' + name + r'\s*(?:\[[^;\n]*\])?)\s*;', declarations)
                    if not decl:
                        raise ValueError(f'{lane}: no declaration for {name}')
                    declaration = decl[1]
                    typ = declaration.split()[0]
                    if typ in ['s_sharedData_800DFB10_0_s01','s_800E3A40','s_MapHeader_field_5C']:
                        end = declarations.index('} ' + typ + ';') + len('} ' + typ + ';')
                        start = declarations.rfind('typedef struct', 0, end)
                        data = declarations[start:end] + '\n' + data
                        # PORT: Both upstream 5C names describe the same record.
                        if typ == 's_MapHeader_field_5C':
                            declaration = declaration.replace(typ, 's_MapOverlayHdr_5C')
                    if '[]' in declaration:
                        # PORT: The pinned symbol interval bounds this guarded
                        # data slice; no disc/disassembly is consulted.
                        if name not in addresses:
                            indices = [int(m) for m in re.findall(re.escape(name) + r'\[(\d+)\]',local)]
                            assert indices, (lane,name)
                            # PORT: Guarded provider exposes only the source's
                            # constant-index footprint until its code migrates.
                            declaration = declaration.replace('[]',f'[{max(indices)+1}]')
                            data += declaration + ';\n'
                            zeros.append(name)
                            continue
                        begin = addresses[name]
                        end = min(addr for addr in addresses.values() if addr > begin)
                        size = {'u8':1,'s_UnkStruct3_Mo':8,'s_sharedData_800DFB10_0_s01':12,'s_800E3A40':24}[typ]
                        assert (end-begin) % size == 0, (lane,name,end-begin,size)
                        declaration = declaration.replace('[]', f'[{(end-begin)//size}]')
                    data += declaration + ';\n'
                    zeros.append(name)
                    if typ in ['u8', 's_UnkStruct3_Mo']:
                        raw_loads.append(f'port_map_data_read(FILE_VIN_{lane.upper()}_BIN,{addresses[name]-base_address},sizeof({name}),(u8*)&{name})')
            # Some pinned animation units are INCLUDE_RODATA-only. Keep a
            # named blocker until their pointer-bearing decoder is available.
            anim_name = re.search(r'\.harryMapAnimInfos\s*=\s*(\w+)', original)[1]
            anim_missing = anim_name not in defined
            if anim_missing:
                data += f's_AnimInfo {anim_name}[128];\n'
                zeros.append(anim_name)
            data += header
            writable = defined + ['g_LoadScreenFuncs','MAP_POINTS','g_MapEventFuncs','g_MapOverlayHdr']
            initials = []
            for name in writable:
                obj = initializer(data, name).replace(name, name + '_initial', 1)
                if obj.startswith('void (*'):
                    obj = obj.replace('void (*', 'void (* const ', 1)
                elif obj.startswith('const char*'):
                    obj = obj.replace('const char*', 'const char* const ', 1)
                else:
                    obj = 'const ' + obj
                initials.append('static ' + obj)
            owned = sorted(set(writable + zeros + list(callbacks)+['Map_RoomIdxGet','GetXIdx','GetYIdx','Map_WorldObjectsInit']))
            namespace = '\n'.join(f'#define {name} {prefix}{name}' for name in owned) + '\n'
            reset = f'void {prefix}reset(void) {{' + ''.join(f'memcpy(&{name},&{name}_initial,sizeof({name}));' for name in writable) + ''.join(f'memset(&{name},0,sizeof({name}));' for name in zeros) + '}\n'
            reset += f'int {prefix}reset_probe(void) {{' + ''.join(f'memset(&{name},0xa5,sizeof({name}));' for name in writable + zeros) + f'{prefix}reset();return ' + ' && '.join([f'memcmp(&{name},&{name}_initial,sizeof({name}))==0' for name in writable] + [f'port_map_zero(&{name},sizeof({name}))' for name in zeros]) + ';}\n'
            utility=(decomp/'src/maps/map_util.c').read_text(encoding='utf-8')
            room=between(utility,'#ifdef MAP5_S01','u8 Map_RoomIdxGet')+function(utility,'Map_RoomIdxGet')
            room=re.sub(r'return (res|ret|result);',r'return (u8)\1;',room)
            room=room.replace('u8 Map_RoomIdxGet(q19_12 posX, q19_12 posZ)\n{', 'u8 Map_RoomIdxGet(q19_12 posX, q19_12 posZ)\n{\n    (void)posX; (void)posZ;')
            # PORT: Some compile-time map branches do not use both grid axes.
            room=room.replace('s32 yIdx;', 's32 yIdx=0; (void)yIdx;')
            room=room.replace('s32 xIdx;', 's32 xIdx=0; (void)xIdx;')
            room=room.replace('(u32)yIdx > xIdx', '(u32)yIdx > (u32)xIdx')
            room=room.replace('Collision_GroundHeightGet(', 'port_maps_ground_height(')
            for name,size in room_sizes.items():
                room=re.sub(re.escape(name)+r'\[([^\]\n]+)\]',lambda m:f'port_maps_room_byte({name},{size},(s32)({m[1]}))',room)
            helpers='static inline bool CheckRange(s32 v,s32 low,s32 high) {return low<=v && v<=high;}\n#define CheckNotInRange(v,l,h) (!CheckRange(v,l,h))\n'
            helpers+='static inline u8 port_maps_room_byte(const u8* bytes,size_t count,s32 index) {if(index<0 || (size_t)index>=count)port_unimplemented("map room grid bounds");return bytes[index];}\n'
            prototypes='static u8 Map_RoomIdxGet(q19_12,q19_12);\nq19_12 port_maps_ground_height(q19_12,q19_12);\nstatic void Map_WorldObjectsInit(void);\nint port_maps_messages_read(u32,u32,u32,u32,const char**,u8*,u32);\n'
            if init_types:
                data = data.replace(init_types, '', 1)
            source = NOTICE + '#include "maps_objects.h"\nextern s8 D_800C4414;\n#undef g_MapOverlayHdr\n#undef CHUNK_SIZE\n#define CHUNK_SIZE 40\n#define '+lane.upper()+'\n' + macros + '\n' + namespace + init_types+prototypes+init_prototypes+code+data+''.join(initials)+helpers+room+reset
            source += f'const s_MapOverlayHdr* {prefix}descriptor(void) {{return &g_MapOverlayHdr;}}\n'
            source += f'u32 {prefix}point_count(void) {{return (u32)ARRAY_SIZE(MAP_POINTS);}}\n'
            source += f'u32 {prefix}event_count(void) {{return (u32)ARRAY_SIZE(MAP_EVENTS);}}\n'
            source += f'u32 {prefix}callback_count(void) {{return (u32)ARRAY_SIZE(g_MapEventFuncs);}}\n'
            blocker = 'animation info decoder' if anim_missing else ('message pointer decoder' if messages_missing else None)
            source += '// PORT: Never publish a placeholder for an undecoded pointer-bearing table.\n' if blocker else ''
            source += f'int {prefix}load_data(void) {{' + (f'port_unimplemented("{lane}/{blocker}");' if blocker else '') + 'return ' + (' || '.join(raw_loads) or '0') + ';}\n'
            source = re.sub(r'\{\s*}', '{0}', source)
            source+=init_helpers+init_code
            source=source.replace('SD_Call(', 'port_maps_object_sfx(')
            # PORT: Keep the original signed 16-bit constant without unsigned negation.
            source = source.replace('-0x2CCCu', '-0x2CCC')
            if lane == 'map7_s02':
                # PORT: The original pose starts with a model; name that member
                # explicitly instead of passing an incompatible native pointer.
                source = source.replace('WorldObject_ModelNameSet(&g_WorldObject_Beans[i],', 'WorldObject_ModelNameSet(&g_WorldObject_Beans[i].object,')
                source = source.replace("beanObjName[4] = (i >> 1) + '1';", "beanObjName[4] = (char)((i >> 1) + '1');")
                source = re.sub(r'(g_WorldObject_Beans\[i\]\.rotation\.v[xyz]\s*=) ([^;]+);', r'\1 (s16)(\2);', source)
                for name in ['D_800EB9F4', 'D_800EBA14']:
                    source = re.sub(r'(' + name + r'\[i\]\s*=) ([^;]+);', r'\1 (s16)(\2);', source)
            source = source.replace('port_maps_object_sfx(Sfx_Unk1640)', 'port_maps_object_sfx(1640)')
            source = source.replace('g_EndingIdx += 2;', 'g_EndingIdx = (u8)(g_EndingIdx + 2);')
            if lane == 'map1_s05':
                source=source.replace('    sharedFunc_800CAAD0_1_s05();', '    port_maps_init_note("MAP1_S05/effect-init begin");\n    sharedFunc_800CAAD0_1_s05();\n    port_maps_init_note("MAP1_S05/effect-init complete");')
                source=source.replace('    port_maps_object_sfx(Sfx_Unk1478);', '    port_maps_init_note("MAP1_S05/Sfx_Unk1478 begin");\n    port_maps_object_sfx(Sfx_Unk1478);\n    port_maps_init_note("MAP1_S05/Sfx_Unk1478 complete");')
            source = source.replace('false, NO_VALUE,', 'false, (u8)NO_VALUE,')
            source = source.replace('true, NO_VALUE,', 'true, (u8)NO_VALUE,')
            source = source.replace('{ Player_VariableAnimDurationGet }','{ .variableFunc = Player_VariableAnimDurationGet }')
            for name in duration_names:
                source=re.sub(r'\{\s*'+re.escape(name)+r'\s*}', '{ .variableFunc = '+name+' }',source)
            # PORT: PS1 unsigned 32-bit animation time sentinels retain their
            # bit patterns without overflowing a signed constant expression.
            source = re.sub(r'Q12\((0x[89A-Fa-f][0-9A-Fa-f]{7})\)',r'(s32)((u32)\1 * 4096u)',source)
            source = re.sub(r'Q12\((\d+)\)', lambda m: f'(s32)({int(m[1])*4096 & 0xffffffff}u)' if int(m[1])>524287 else m[0],source)
            # PORT: events links Chara_AnimReset(actor, mode); keep generated stubs on the same prototype.
            source = re.sub(r'static void Chara_AnimReset\(void\) \{port_unimplemented\("([^"]+)"\);\}',
                            r'static void Chara_AnimReset(s_SubCharacter* npc, bool cond) {(void)npc;(void)cond;port_unimplemented("\1");}', source)
            (output / f'{lane}.c').write_text(source, encoding='utf-8')
            inventory.append(dict(name=lane.upper(), index=index, provider='maps', objects=writable + zeros, guards=sorted(callbacks), blocker=blocker,original_empty_init=empty_init,
                init_helpers=sorted(init_extra),init_leaf_guards=re.findall(r'port_unimplemented\("([^"\n]+)"\)',init_helpers),
                init_services=sorted(set(re.findall(r'\b((?:WorldObject_|Math_|Chara_|Savegame_|Gfx_|Rng_)\w+)\(',init_code)))))
        registry += f'extern const s_MapOverlayHdr* {prefix}descriptor(void);\nextern void {prefix}reset(void);\nextern int {prefix}reset_probe(void);\nextern int {prefix}load_data(void);\n'
        registry += f'extern u32 {prefix}point_count(void);\n'
        registry += f'extern u32 {prefix}event_count(void);\n'
        registry += f'extern u32 {prefix}callback_count(void);\n'
    registry += 'const PortMapEntry port_maps[43]={\n'
    for item in inventory:
        prefix = 'sh_' + item['name'].lower() + '_'
        registry += '{"' + item['name'] + '",' + ','.join(prefix + name for name in ['descriptor','reset','reset_probe','load_data','point_count','event_count','callback_count']) + '},\n'
    registry += '};\n'
    (output / 'maps_registry.c').write_text(registry, encoding='utf-8')
    player=(decomp/'src/bodyprog/player_control.c').read_text(encoding='utf-8')
    ground=NOTICE+'#include "collision.h"\n#define g_CollisionPointCache port_maps_ground_cache\n#define Collision_Fill port_maps_collision_fill\n#define Collision_GroundHeightGet port_maps_ground_height\n'
    ground+='static s_CollisionPoint port_maps_ground_cache;\nstatic const s_CollisionPoint cache_initial={.groundType=NO_VALUE};\n'
    ground+=function(player,'Collision_Fill')+function(player,'Collision_GroundHeightGet')
    allocator = function((decomp/'src/bodyprog/gfx/bodyprog_effects_8005E0DC.c').read_text(encoding='utf-8'),'func_8005E7E0')
    allocator = allocator.replace('func_8005E7E0(', 'port_maps_effect_alloc(').replace('field_A = arg0;', 'field_A = (u8)arg0;').replace('D_800C4408 = idx + 1;', 'D_800C4408 = (s16)(idx + 1);')
    ground += 'extern s16 D_800C4408;\n' + allocator
    ground += '''
// PORT: Numeric PS1 spawn records expand the signed difficulty nibble and
// preserve all original scalar bits without relying on native struct stride.
static u32 spawn_word(const u8* p) {return (u32)p[0]|((u32)p[1]<<8)|((u32)p[2]<<16)|((u32)p[3]<<24);}
int port_maps_spawn_decode(const u8* wire,u32 count,s_SpawnInfo* records) {
    if(!wire || !records || !count || count>96)return 1;
    for(u32 i=0;i<count;i++) {
        const u8* p=wire+i*12;s_SpawnInfo* r=&records[i];
        memset(r,0,sizeof(*r));
        r->positionX=(s32)spawn_word(p);r->charaId=(s8)p[4];r->rotationY=p[5];r->flags=(s8)p[6];
        s32 difficulty=p[7]&15;r->gameDifficultyMin=difficulty>=8?difficulty-16:difficulty;
        r->positionZ=(s32)spawn_word(p+8);
    }
    return 0;
}
int port_maps_spawn_read(u32 file,u32 offset,u32 count,s_SpawnInfo* records) {
    u8 wire[96*12];
    if(!count || count>96 || port_map_data_read(file,offset,count*12,wire))return 1;
    return port_maps_spawn_decode(wire,count,records);
}
int port_maps_spawn_probe(void) {
    const u8 wire[12]={0x2e,0xfb,0xff,0xff,7,128,14,0xaf,0x2e,0x16,0,0};
    struct {u32 before;s_SpawnInfo record;u32 after;} result={0};
    result.before=0x12345678;result.after=0x76543210;
    if(port_maps_spawn_decode(wire,1,&result.record))return 0;
    return result.before==0x12345678 && result.after==0x76543210 &&
        result.record.positionX==-1234 && result.record.positionZ==5678 &&
        result.record.charaId==7 && result.record.rotationY==128 && result.record.flags==14 &&
        result.record.gameDifficultyMin==-1 && port_maps_spawn_decode(wire,97,&result.record)==1;
}
'''
    ground+='// PORT: Native overlay activation invalidates the separately owned room-query cache.\nvoid port_maps_ground_reset(void) {memcpy(&port_maps_ground_cache,&cache_initial,sizeof(cache_initial));}\n'
    ground+='int port_maps_ground_reset_probe(void) {memset(&port_maps_ground_cache,0xa5,sizeof(port_maps_ground_cache));port_maps_ground_reset();return memcmp(&port_maps_ground_cache,&cache_initial,sizeof(cache_initial))==0;}\n'
    (output/'maps_ground.c').write_text(ground,encoding='utf-8')
    (output / 'maps_inventory.json').write_text(json.dumps(inventory, indent=2), encoding='utf-8')
    # PORT: Keep the unowned base service file intact; replace only its old
    # single-map lifecycle exports at compilation, through the shared build seam.
    base = Path(__file__).resolve().parents[1] / 'port/map.c'
    (output / 'maps_base.c').write_text(NOTICE + '#define port_map_activate port_maps_legacy_activate\n#define port_map_active port_maps_legacy_active\n#define Map_EffectTexturesLoad port_maps_legacy_effect_load\n' + base.read_text(encoding='utf-8'), encoding='utf-8')
    extend_transit_callbacks(decomp, output)


def extend_transit_callbacks(decomp, output):
    """Bind shared loading/control/BGM callbacks using each map's original source."""
    import json
    from prepare_camera import narrow
    def read(path):return (decomp/path).read_text(encoding='utf-8')
    player=read('src/maps/characters/player.c')
    declarations='\n'.join(p.read_text(encoding='utf-8') for p in (decomp/'include').rglob('*.h'))
    player_names=['Player_ControlFreeze','Player_ControlUnfreeze','Player_AnimStateSet','Player_AnimReset',
        'Player_AnimLock','Player_AnimIsLocked','Player_AnimUnlock','Player_AnimPlaybackStateGet',
        'Player_MoveSpeedIsZero','Player_MoveSpeedClear','Player_CutsceneWeaponUnequip',
        'Player_MatchArmAnimDisable','Player_EmptyWeaponHandSet','Player_WeaponAttackRestore',
        'Player_PropertyField10DGet','Player_PathWaypointExecute']
    inventory=json.loads((output/'maps_inventory.json').read_text(encoding='utf-8'))
    for item in inventory[1:]:
        lane=item['name'].lower();prefix='sh_'+lane+'_'
        path=output/(lane+'.c');code=path.read_text(encoding='utf-8')
        local='\n'.join(p.read_text(encoding='utf-8') for p in (decomp/'src/maps'/lane).glob('*.c'))
        # Includes are GPL source providers, never original overlay code addresses.
        for _ in range(4):
            revised=re.sub(r'^#include "(maps/shared/[^"\n]+)"[^\n]*',lambda m:read('include/'+m[1]),local,flags=re.M)
            if revised==local:break
            local=revised
        bodies={name:function(player,name) for name in player_names if name in item['guards']}
        for name in ['Map_RoomBgmInit','Map_RoomBgmInit_CondTrue','Map_RoomBgmInit_CondFalse','GameBoot_LoadScreen_StageString']:
            try:bodies[name]=function(local,name)
            except ValueError:pass
        text=''.join(bodies.values())
        used=set(re.findall(r'\b(?:sharedData_\w+|D_800[CDEF]\w+)\b',re.sub(r'"[^"\n]*"','',text)))
        used={name for name in used if not name.startswith('D_') or int(name[2:],16)>=0x800C9578}
        used-={name for name in used if re.search(r'\b(?:const\s+static|static\s+const)\s+\w+\s+'+re.escape(name)+r'\s*=',text)}
        used|=set(re.findall(r'\b(?:g_Player_PrevWeaponAttack|g_Player_MoveSpeed)\b',text))
        limits=set(re.findall(r'Bgm_LayersUpdate\([^;{}]*?&(\w+)',text))
        for pointer in re.findall(r'\bu8\s*\*\s*(\w+)\s*;',text):
            limits.update(re.findall(r'\b'+pointer+r'\s*=\s*&(\w+)',text))
        objects={};loads=[]
        symbols=read(f'configs/USA/maps/sym.{lane}.txt')
        addresses={m[1]:int(m[2],16) for m in re.finditer(r'^(\w+)\s*=\s*0x([0-9A-Fa-f]+);',symbols,re.M)}
        base=int(re.search(r'\bvram:\s*0x([0-9A-Fa-f]+)',read(f'configs/USA/maps/{lane}.yaml'))[1],16)
        for name in sorted(used):
            if '#define '+name+' ' in code:continue
            decl=re.search(r'\b(?:extern\s+)?(u8|s8|u16|s16|u32|s32|q3_12|q19_12|VECTOR3|s_BgmLayerLimits)\s+'+re.escape(name)+r'\s*(\[[^;\n]*\])?\s*;',declarations)
            if not decl:raise ValueError(f'{lane}: transit numeric declaration missing: {name}')
            typ,dims=decl[1],decl[2] or ''
            if name in limits:
                # PORT: The original scalar label begins eight byte channel limits.
                typ,dims='u8','[8]'
                bodies={key:body.replace('&'+name,name) for key,body in bodies.items()}
            if name not in addresses and re.fullmatch(r'D_800[CDEF][0-9A-Fa-f]{4}',name):
                # Named address-labelled data has an exact GPL header footprint.
                addresses[name]=int(name[2:],16)
            if '[]' in dims:
                begin=addresses[name];end=min(value for value in addresses.values() if value>begin)
                size={'u8':1,'s8':1,'u16':2,'s16':2,'q3_12':2,'u32':4,'s32':4,'q19_12':4,'VECTOR3':12,'s_BgmLayerLimits':8}[typ]
                count,remainder=divmod(end-begin,size)
                if remainder or not 0<count<=256:raise ValueError(f'{lane}: unbounded transit numeric array {name}')
                dims=dims.replace('[]',f'[{count}]')
            objects[name]=typ+' '+name+dims
            # Player overlay work is BSS; BGM numeric tables retain pinned data.
            if dims or typ=='VECTOR3':
                if name not in addresses:raise ValueError(f'{lane}: transit table address missing: {name}')
                loads.append(f'port_map_data_read(FILE_VIN_{lane.upper()}_BIN,{addresses[name]-base},sizeof({name}),(u8*)&{name})')
        head='#include "audio_records.h"\n#include "native_player_events.h"\n'
        head+=''.join(f'#define {name} {prefix}{name}\nstatic {decl};\n' for name,decl in objects.items())
        # Native globals are shared with the bodyprog player, retaining their ABI.
        for name in sorted(set(re.findall(r'\bg_Player_\w+',text))-objects.keys()):
            if '#define '+name+' ' in code:continue
            decl=re.search(r'\b(?:extern\s+)?(\w+)\s+'+re.escape(name)+r'\s*;',declarations)
            if not decl:raise ValueError(f'{lane}: shared player declaration missing: {name}')
            typ={'g_Player_CutsceneState':'s32','g_Player_DisableControl':'bool'}.get(name,decl[1])
            head+=f'extern {typ} {name};\n'
        head+=enumeration(read('include/bodyprog/player.h'),'PlayerCutsceneState').replace('PlayerCutsceneState_RunForward','PortTransit_RunForward')
        head+='void func_8003D01C(void);\n#define DEFAULT_PLAYER_CYLINDER_FIELD_2 Q12(0.23f)\n'
        head+=between(read('include/bodyprog/player.h'),'#define Player_AnimFlagsClear','/** @brief Resets the player character')
        head+='q19_12 Math_Distance2dGet(const VECTOR3*,const VECTOR3*);\nu32 func_800364BC(void);\n'
        for name in ['GameState_LoadStatusScreen','GameState_SaveScreen']:
            value=re.search(r'\b'+name+r'\s*=\s*(\d+)',read('include/game.h'))[1]
            head+=f'#define {name} {value}\n'
        head+=enumeration(read('include/maps/characters/split_head.h'),'SplitHeadFlags')
        # Remove the exact fatal wrappers, retaining descriptor namespace macros.
        for name in bodies:
            if name in item['guards']:
                code=code.replace(function(code,name),'')
        for name in ['GameBoot_LoadScreen_PlayerRun','GameBoot_LoadScreen_BackgroundImg']:
            if name in item['guards']:
                code=code.replace(function(code,name),'')
                code=code.replace(f'#define {name} {prefix}{name}',f'// PORT: {name} uses the shared original loading service.')
        prototypes=''.join('static '+re.sub(r'//[^\n]*','',body.split('{',1)[0]).strip()+';\n' for body in bodies.values())
        # Prototypes must follow the existing macros, before descriptor initializers.
        code=code.replace('static void Map_WorldObjectsInit(void);','static void Map_WorldObjectsInit(void);\n'+prototypes)
        extra=''
        for name,body in bodies.items():
            body=narrow(body,(output/'native_gameplay_records.h').read_text(encoding='utf-8'))
            # PORT: Global overlay/player halfwords retain their PS1 truncation.
            scalar_types={key:decl.split()[0] for key,decl in objects.items()}
            scalar_types.update({var:typ for typ,var in re.findall(r'extern\s+(u8|s8|u16|s16|q3_12)\s+(\w+)\s*;',head)})
            for var in set(re.findall(r'\b(?:g_Player_\w+|sharedData_\w+)\b',body)):
                actual=re.search(r'^(?:static\s+)?(u8|s8|u16|s16|q3_12)\s+'+re.escape(var)+r'\b',code,re.M)
                if actual:scalar_types[var]=actual[1]
            for scalar,typ in scalar_types.items():
                if typ not in ('u8','s8','u16','s16','q3_12'):continue
                body=re.sub(r'\b('+re.escape(scalar)+r'(?:\[[^]\n]+\])*)\s*=(?!=)\s*([^;{}]+);',lambda m:m[1]+' = ('+typ+')('+m[2]+');',body)
            for pointer in re.findall(r's_BgmLayerLimits\s*\*\s*(\w+)\s*;',body):
                body=re.sub(r'\b'+pointer+r'\s*=(?!=)\s*([^;{}]+);',lambda m:pointer+' = (s_BgmLayerLimits*)('+m[1]+');',body)
                # PORT: Serialized byte limits copy into their actual native
                # value instead of aliasing a byte array as a struct lvalue.
                body=re.sub(r'\b(\w+)\s*=\s*\*'+pointer+r'\s*;',lambda m:'memcpy(&'+m[1]+','+pointer+',sizeof('+m[1]+'));',body)
            if len(re.findall(r'\bdist0\b',body))==1:body=re.sub(r'\s*q19_12\s+dist0;','',body)
            body=body.replace('g_SysWork.field_2388.', 'g_SysWork.gameplayEnvironment.').replace('g_SysWork.gameplayEnvironment.isFlashlightOn','g_SysWork.field_2388.isFlashlightOn')
            body=re.sub(r'(Bgm_LayersUpdate\([^;{}]*,\s*)([^,()]+)(\);)',r'\1(s_BgmLayerLimits*)(\2)\3',body)
            body=body.replace('g_Player_AnimResetRequest++;','g_Player_AnimResetRequest=(u8)(g_Player_AnimResetRequest+1);')
            signature=body.split('{',1)[0]
            params=signature.split('(',1)[1].rsplit(')',1)[0].split(',')
            unused=''.join('(void)'+re.findall(r'\b\w+',param)[-1]+';' for param in params if param.strip()!='void')
            body=body.replace('{','{\n    '+unused,1)
            extra+='static '+body+'\n'
        reset=function(code,prefix+'reset')
        code=code.replace(reset,reset[:-2]+''.join(f'memset(&{name},0,sizeof({name}));' for name in objects)+'}\n')
        probe=function(code,prefix+'reset_probe')
        revised=probe.replace('{','{'+''.join(f'memset(&{name},0xa5,sizeof({name}));' for name in objects),1)
        revised=revised.replace('return ', 'return '+''.join(f'port_map_zero(&{name},sizeof({name})) && ' for name in objects),1)
        code=code.replace(probe,revised)
        load=function(code,prefix+'load_data')
        if loads:code=code.replace(load,load.replace('return ', 'return '+' || '.join(loads)+' || ',1))
        path.write_text(head+code+extra,encoding='utf-8')
        linked=set(bodies)&set(item['guards'])|{'GameBoot_LoadScreen_PlayerRun','GameBoot_LoadScreen_BackgroundImg'}
        item['transit_callbacks']=sorted(linked)
        item['guards']=sorted(set(item['guards'])-linked)
        item['objects']+=list(objects)
    (output/'maps_inventory.json').write_text(json.dumps(inventory,indent=2),encoding='utf-8')


def smoke(args):
    """Run each warp in an isolated process; retain logs, never game captures."""
    import json
    import os
    import subprocess
    root=Path(__file__).resolve().parents[1]
    catalog=[p.parent.name.upper() for p in sorted((root/'game/decomp/src/maps').glob('map*/*_header.c'))]
    if args.warp:
        match=re.fullmatch(r'(MAP[0-7]_S\d{2})(?::(\d{1,3}))?',args.warp)
        if not match or match[1] not in catalog or int(match[2] or '0')>255:
            raise ValueError('warp needs an existing MAPx_Syy[:spawn], spawn 0..255')
    if args.capture and (not args.warp or args.smoke_all):
        raise ValueError('private capture requires one explicit --warp')
    private=Path(__file__).resolve().parents[3]/'private'
    lane=os.environ.get('SH_MILESTONE_LANE','objects')
    if lane not in ('objects','transit'):raise ValueError('unsupported private map smoke lane')
    evidence=private/'work'/lane
    evidence.mkdir(parents=True,exist_ok=True)
    executable=args.test_executable
    if executable is None:
        build=subprocess.run([str(root/'tools/dev-cargo.cmd'),'test','--release','-p','silent-hill-boot','--lib','--no-run','--message-format=json'],cwd=root,capture_output=True,text=True,encoding='utf-8',errors='replace')
        (evidence/'smoke-build.log').write_text(build.stdout+build.stderr,encoding='utf-8')
        if build.returncode:
            print(build.stderr[-4000:])
            return build.returncode
        artifacts=[json.loads(line) for line in build.stdout.splitlines() if line.startswith('{')]
        executable=Path(next(item['executable'] for item in artifacts if item.get('executable') and item.get('profile',{}).get('test') and 'lib' in item.get('target',{}).get('kind',[])))
    names=catalog if args.smoke_all else [args.warp]
    results=[]
    for name in names:
        env=dict(os.environ,SH_MAP_WARP=name,SH_MAP_LIGHT=str(int(args.flashlight)),SH_MAP_CAPTURE=str(int(args.capture)),SH_MAP_DISC=str(args.disc or private/'disc/Silent Hill (USA).bin'))
        try:
            result=subprocess.run([str(executable),'maps::warp_tests::debug_map_warp','--exact','--ignored','--nocapture','--test-threads=1'],cwd=root,env=env,capture_output=True,text=True,encoding='utf-8',errors='replace',timeout=60)
            log=result.stdout+result.stderr
            code=result.returncode
        except subprocess.TimeoutExpired as error:
            def timeout_text(value):
                return value.decode('utf-8',errors='replace') if isinstance(value,bytes) else (value or '')
            log=timeout_text(error.stdout)+timeout_text(error.stderr)+'\nBLOCKED warp timeout after 60 seconds\n'
            code=124
        (evidence/(name.replace(':','-')+('-lit' if args.flashlight else '')+'.log')).write_text(log,encoding='utf-8')
        guard=re.search(r'BLOCKED native service:[^\n]*',log)
        frame=re.search(r'MAP_WARP_FRAME colors=(\d+) primitives=(\d+)',log)
        blank='VISIBLE lit_pixels=0\n' in log and 'MAP_WARP rendered=' in log
        geometry=re.search(r'MAP_WARP geometry=(\d+)',log)
        detail=re.search(r'MAP_WARP geometry=\d+ detail=(\d+)',log)
        phase=[line for line in log.splitlines() if line.startswith(('MAP_INIT ','MAP_WARP ','READ file='))]
        abnormal=f'native exit {code} after '+(phase[-1] if phase else 'startup') if code not in [0,101] else None
        item=dict(map=name,exit=code,flashlight_fixture=args.flashlight,world_meshes=int(geometry[1]) if geometry else None,rendered='MAP_WARP rendered=' in log,selected='MAP_WARP selected=' in log,
                  varied_pixels=int(detail[1]) if detail else None,
                  colors=int(frame[1]) if frame else None,primitives=int(frame[2]) if frame else None,
                  blocker=guard[0] if guard else (abnormal or ('black frame: lit_pixels=0, colors='+frame[1]+', primitives='+frame[2] if blank and frame else (None if code==0 else log[-700:]))))
        results.append(item)
        print(f'{name}: '+('PASS' if code==0 and item['rendered'] else 'BLOCKED')+' '+str(item['blocker'] or ''),flush=True)
    (evidence/(('smoke' if args.smoke_all else 'warp')+('-lit' if args.flashlight else '')+'-results.json')).write_text(json.dumps(results,indent=2),encoding='utf-8')
    return int(any(item['exit'] or not item['rendered'] for item in results))


def check_clang(compiler):
    import subprocess
    from prepare_sdk import prepare as sdk
    from prepare_audio import prepare as audio
    root=Path(__file__).resolve().parents[1]
    output=root/'target/maps-clang'
    sdk(root/'game/decomp',output)
    audio(root/'game/decomp',output)
    sources=sorted(output.glob('map*_s*.c'))+[output/'maps_registry.c',output/'maps_ground.c',output/'maps_base.c',root/'port/maps_runtime.c']
    flags=['-target','aarch64-apple-ios15.0','-ffreestanding','-std=c11','-Wall','-Wextra','-Werror','-nostdinc','-DSH_NATIVE_AUDIO']
    for include in [root/'tools/layout-include',root/'port',root/'port/include',output,root/'game/decomp/include',root/'game/decomp/src/main']:
        flags+=['-I',str(include)]
    for name in ['ccos','csin','csqrt','catan']:
        flags.append('-fno-builtin-'+name)
    failed=0
    for source in sources:
        result=subprocess.run([str(compiler),str(source),'--checks=-*,clang-analyzer-core.DivideZero','--warnings-as-errors=*','--',*flags],capture_output=True,text=True)
        print(('PASS' if result.returncode==0 else 'FAIL')+': '+source.name,flush=True)
        if result.returncode:
            print(result.stdout+result.stderr,flush=True)
            failed+=1
    print(f'Maps arm64 frontend: {len(sources)-failed} passed, {failed} failed; Apple SDK/runtime not exercised.')
    return int(failed!=0)


def transition_candidate():
    """Keep a source-checked, explicitly unverified replay for the events handoff."""
    import json
    root=Path(__file__).resolve().parents[1]
    import os
    lane=os.environ.get('SH_MILESTONE_LANE','objects')
    if lane not in ('objects','transit'):raise ValueError('unsupported private transition lane')
    out=root.parents[1]/('private/work/'+lane)
    out.mkdir(parents=True,exist_ok=True)
    route=[]
    for lane,index,trigger,target,destination in [('map2_s04',15,4,'MAP2_S02',9),('map2_s02',16,21,'MAP2_S04',23)]:
        source=root/f'game/decomp/src/maps/{lane}/{lane}_events_data.c'
        text=source.read_text(encoding='utf-8')
        block=re.search(r'// \['+str(index)+r'\]\s*\{(.*?)\}',text,re.S)[1]
        fields=dict(re.findall(r'\.(\w+)\s*=\s*(\w+)',block))
        assert fields['sysState']=='SysState_LoadOverlay' and fields['mapIdx']=='MapIdx_'+target
        assert fields['mapPointIdx']==str(trigger) and fields['eventParam']==str(destination)
        assert not {'requiredEventFlag','requiredItemId','completeEventFlag'} & fields.keys()
        points=(source.parent/'map_points.h').read_text(encoding='utf-8')
        point=re.search(r'// \['+str(destination)+r'\].*?(\{.*?\})',points,re.S)[1]
        route.append(dict(map=lane.upper(),event=index,trigger_point=trigger,destination_map=target,
            destination_point_in_source_map=destination,point_source=point))
    replay='''# BLOCKED candidate; pad timings and movement have NOT been calibrated or run.
# Start MAP2_S04:4 through original loading, then require MAP2_S02 -> MAP2_S04.
# First walk away/back to the exit; then turn and approach the street-side door.
0 0000
15 0040
60 0000
70 0010
115 0000
120 4000
121 0000
600 0020
673 0000
690 0010
705 0000
710 4000
711 0000
'''
    (out/'police-return.candidate.txt').write_text(replay,encoding='utf-8')
    (out/'police-return.candidate.json').write_text(json.dumps(dict(status='BLOCKED: not a passing milestone',
        start='MAP2_S04:4',required_maps=['MAP2_S04','MAP2_S02','MAP2_S04'],route=route,
        blocker='Host native.rs currently accepts only HB_M0S00.ANM; police_return.txt fails before gameplay at the map animation identity gate. Later map event/enemy services remain guarded.',
        timing='unverified; validate pad-driven position, original trigger selection, both queued activations and return before marking pass'),indent=2),encoding='utf-8')
    print('Source-checked reversible route; candidate replay BLOCKED/unrun: '+str(out/'police-return.candidate.json'))
    return 0


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--decomp', type=Path)
    parser.add_argument('--out', type=Path)
    parser.add_argument('--warp', help='test-only MAPx_Syy[:spawn] headless warp')
    parser.add_argument('--smoke-all', action='store_true')
    parser.add_argument('--flashlight', action='store_true', help='explicit diagnostic spotlight/loading-pose fixture; no gameplay claim')
    parser.add_argument('--capture', action='store_true', help='capture one warp to private/work/objects only')
    parser.add_argument('--transition-candidate', action='store_true', help='write the source-checked MAP2_S04 -> MAP2_S02 -> MAP2_S04 candidate; not a replay pass')
    parser.add_argument('--test-executable', type=Path)
    parser.add_argument('--disc', type=Path)
    parser.add_argument('--check-clang', type=Path)
    args = parser.parse_args()
    if args.transition_candidate:
        raise SystemExit(transition_candidate())
    if args.check_clang:
        raise SystemExit(check_clang(args.check_clang))
    if args.warp or args.smoke_all:
        raise SystemExit(smoke(args))
    if not args.decomp or not args.out:
        parser.error('generation requires --decomp and --out')
    prepare(args.decomp, args.out)
