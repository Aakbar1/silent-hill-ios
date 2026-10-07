"""Pinned GPL combat source preparation; no game bytes or disassembly.

SPDX-License-Identifier: GPL-3.0-only. Outputs are generated build inputs.
The source census reports partial migrations explicitly, never as full units.
"""
import argparse
import ast
import difflib
import json
from pathlib import Path
import re
import subprocess
from prepare_sdk import prepare as prepare_sdk
from prepare_gameplay import between

PIN = "d9e28f8315c7938117224f21516786d9d149a145"
NOTICE = "/* Copyright (C) 2026 shdecompilations; SPDX-License-Identifier: GPL-3.0-only. */\n"
COMBAT = "src/bodyprog/bodyprog_combat_8008A058.c"
SELECTED = {
    "src/bodyprog/events/npc_main.c": ["Savegame_EnemyStateUpdate", "Chara_DamagedFlagUpdate", "func_80037E78"],
    "src/bodyprog/collision/chara.c": ["Collision_CharaCollisionSet", "Chara_ModelBoneScaleSet"],
    "src/bodyprog/items/item_screens_3.c": ["Items_AmmoReloadCompute"],
    "src/maps/chara_util.c": ["Chara_MovementReset", "Chara_AnimReset", "Chara_AnimStateSet", "Chara_ControlStateReset", "Chara_AnimLock", "Chara_AnimIsLocked", "Chara_AnimUnlock"],
}
STALKER = ['Stalker_Init', 'sharedFunc_800D3308_0_s00', 'Stalker_Control_13', 'sharedFunc_800D7E04_0_s00']
SMALL_AI = ['cat', 'parasite', 'flauros']


def mask(text):
    return re.sub(r'//[^\n]*|/\*.*?\*/|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'', lambda m: " " * len(m[0]), text, flags=re.S)


def names(text):
    return re.findall(r"^(?:static[ \t]+)?(?:inline[ \t]+)?(?:void|u32|s32|s64|u8|s8|u16|s16|bool|q19_12|q3_12)[ \t]+(\w+)\([^;{}]*\)[^{;]*\{", mask(text), re.M)


def function(text, name):
    masked = mask(text)
    match = re.search(r'^(?:static[ \t]+)?(?:inline[ \t]+)?[\w* ]+[ \t]+' + re.escape(name) + r'\([^;{}]*\)[^{;]*\{', masked, re.M)
    if not match:
        raise ValueError(f'missing pinned function {name}')
    start = masked.index('{', match.start())
    depth = 0
    for i in range(start, len(masked)):
        depth += (masked[i] == '{') - (masked[i] == '}')
        if not depth:
            return text[match.start():i+1] + '\n'
    raise ValueError(f'unterminated {name}')


def generate(decomp, out):
    head = subprocess.check_output(["git", "-C", str(decomp), "rev-parse", "HEAD"], text=True).strip()
    if head != PIN:
        raise ValueError(f"reference revision {head} != {PIN}")
    out.mkdir(parents=True, exist_ok=True)
    prepare_sdk(decomp, out)
    read = lambda p: (decomp / p).read_text(encoding="utf-8")
    combat = read(COMBAT)
    own = names(combat) + [n for ns in SELECTED.values() for n in ns]
    # PORT: BODYPROG helpers use a shared namespace; overlay symbols must instead
    # use sh_<map-id>_ prefixes when the remaining AI modules are migrated.
    namespace = "".join(f"#define {n} sh_combat_{n}\n" for n in own)
    namespace += "#define D_800AD4C8 sh_combat_attacks\n#define D_800AD4C4 sh_combat_attack_aux\n"
    namespace += "#define D_800AFD1C sh_combat_angles\n#define g_rodataPad_8002AF9C sh_combat_rodata_pad\n"
    (out / "combat_namespace.h").write_text(NOTICE + namespace)
    attack_type = between(read("include/bodyprog/bodyprog.h"), "/** Related to weapon attacks.", "/** @brief Radio noise")
    attack_type = attack_type.replace("STATIC_ASSERT_SIZEOF(s_800AD4C8, 24);", "// PORT: Native pointer, wire record remains 24 bytes.\nSTATIC_ASSERT_SIZEOF(s_800AD4C8, 32);\n_Static_assert(offsetof(s_800AD4C8,unk_14)==24,\"native attack pointer\");")
    ray_type = between(read("include/bodyprog/collision/ray.h"), "/** @brief Ray trace line", "bool Ray_TraceQuery")
    prototypes = "\n".join(function(combat, n).split("{")[0].split("//")[0].strip() + ";" for n in names(combat))
    for p, ns in SELECTED.items():
        prototypes += "\n" + "\n".join(function(read(p), n).split("{")[0].split("//")[0].strip() + ";" for n in ns)
    weapon_enums = ''
    game = read('include/game.h') + read('include/bodyprog/items.h')
    for tag in ['_AttackInputType', '_EquippedWeaponId']:
        m = re.search(r'typedef enum '+tag+r'\s*\{.*?\}\s*\w+;', game, re.S)
        if not m:
            raise ValueError('missing enum '+tag)
        weapon_enums += m[0]+'\n'
    sfx = read('include/bodyprog/sound/sfx_id_enum.h')
    sfx_values = {'NO_VALUE': -1}
    next_sfx = 0
    for entry in re.finditer(r'^\s*(Sfx_\w+)\s*(?:=\s*([^,}\n]+))?\s*[,}]', mask(sfx), re.M):
        if entry[2]:
            next_sfx = sum(sfx_values[x.strip()] if x.strip() in sfx_values else int(x.strip(), 0) for x in entry[2].split('+'))
        sfx_values[entry[1]] = next_sfx
        next_sfx += 1
    sfx_consumer_text = combat + ''.join(read(f'src/maps/characters/{kind}.c') for kind in SMALL_AI+['stalker'])
    for symbol in sorted(set(re.findall(r'\bSfx_\w+\b', sfx_consumer_text)) & sfx_values.keys()):
        if symbol not in sfx_values:
            raise ValueError('missing SFX '+symbol)
        weapon_enums += f'#define {symbol} {sfx_values[symbol]}\n'
    (out / "combat_records.h").write_text(NOTICE + attack_type + ray_type + weapon_enums + "\nSTATIC_ASSERT_SIZEOF(s_RayTrace,40);\n" + prototypes)
    items = read("src/bodyprog/items/item_screens_3.c")
    tables = between(items, "static u8 g_Items_GunsMaxLoadAmmo", "s32 g_Inventory_CmdSelectedIdx")
    tables = tables.replace("NO_VALUE\n};", "(u8)NO_VALUE\n};")
    tables = tables.replace(', -1,', ', (u8)-1,')
    code = re.sub(r'^#include[^\n]*\n', '', combat, flags=re.M)
    code = code.replace('    s32         var_v0;\n', '')
    code = code.replace('    var_t1    = chara->field_44.field_0;', '    (void)unused; // PORT: Preserve unused upstream API argument.\n    var_t1    = chara->field_44.field_0;')
    # PORT: Remove MIPS register-allocation annotations, retain scalar algorithm.
    code = re.sub(r'\s+asm\("\w+"\)', '', code)
    code = code.replace("gte_ldlzc(arg0);", "sh_combat_lzc_input = (u32)arg0;")
    code = code.replace("gte_stlzcr(var_t2);", "var_t2 = sh_combat_lzc(sh_combat_lzc_input);")
    # PORT: Do not derive a 64-bit NPC pointer/index from PS1 struct sizes.
    code = code.replace("target->field_40 = (((s32)((u32)((u8*)attacker - sizeof(s_PlayerWork)) - (u32)target) * -0x6EB3E453) >> 3);", "target->field_40 = (s8)sh_combat_npc_index(attacker);")
    code = code.replace("sp10 = 1 << (((s32)((u32)((u8*)target - sizeof(s_PlayerWork)) - (u32)&g_SysWork.playerWork) * -0x6EB3E453) >> 3);", "sp10 = 1 << sh_combat_npc_index(target);")
    code = code.replace("chara0 = (u8*)chara + sizeof(s_PlayerWork);", "chara0 = g_SysWork.npcs;")
    code = code.replace("g_SysWork.field_2388.field_154.effectsInfo.flags.field_00[0] & SpecialEnvEventFlags_DarkEnvironment", "sh_combat_dark_environment")
    code = code.replace("g_MapOverlayHdr.harryMapAnimInfos", "sh_combat_harry_map_anims")
    # PORT: Explicit PS1 narrowing preserves the original low field bits.
    for before, after in {
        'field_44.field_2 = weaponAttack':'field_44.field_2 = (u8)weaponAttack',
        'field_44.field_3 = sp3C':'field_44.field_3 = (u8)sp3C',
        'field_44.field_4 = sp40':'field_44.field_4 = (u8)sp40',
        'headingAngle =':'headingAngle = (s16)',
        'field_44.field_10 = temp_s3':'field_44.field_10 = (s16)temp_s3',
        'field_44.field_12 = temp_s2':'field_44.field_12 = (s16)temp_s2',
        'Sfx_WithFlagsPlay(sfxId,':'Sfx_WithFlagsPlay((u16)sfxId,',
        'attackReceived     = weaponAttack':'attackReceived     = (s8)weaponAttack',
        'case 35:\n    }':'case 35:;\n    }',
    }.items():
        code = code.replace(before, after)
    code = re.sub(r'(D_800C4748\[[^\]]+\]\.v[xy]\s*=) (.*?);', r'\1 (s16)(\2);', code)
    # PORT: Preserve the original MIPS DIV zero/overflow behavior on native C.
    for numerator, denominator in [('Q12(temp_s3)','var_t1'),('Q12(var_t2)','temp_s0'),('Q12(-var_t2)','temp_s0')]:
        code=code.replace(numerator+' / '+denominator,f'sh_combat_ps1_div({numerator}, {denominator})')
    for field, typename in [('field_0','s16'),('field_3','u8'),('field_4','u8')]:
        code = re.sub(r'(chara->field_44\.'+field+r'\s*=) (sp\w+);', rf'\1 ({typename})\2;', code)
    code = code.replace("register s32  temp_t0", "s32 temp_t0").replace("register u32  var_t1", "u32 var_t1").replace("register u32  var_t2", "u32 var_t2").replace("register s16* ptr", "s16* ptr")
    code = code.replace("static s32        __pad_bss_800C47C4;", "").replace("static s32        __pad_bss_800C4E0[2];", "").replace("static s32        __pad_bss_800C4F4;", "")
    # PORT: Hoist original static scratch to preserve its BODYPROG lifetime,
    # exposing an explicit warm-boot reset without making it frame-local.
    scratch=[]
    def hoist(m):
        scratch.append((m[1],m[2]))
        return ''
    code=re.sub(r'static (s_RayTrace|DVECTOR|VECTOR3)\s+(D_\w+(?:\[\d+\])?);',hoist,code)
    storage=''.join(f'static {kind} {name};\n' for kind,name in scratch)
    reset='void sh_combat_reset_scratch(void) {\n'+''.join(f'    memset(&{name.split("[")[0]},0,sizeof({name.split("[")[0]}));\n' for _,name in scratch)+'}\n'
    (out / "combat_original.c").write_text(NOTICE + '#include "combat.h"\n' + tables + storage + reset + '\nu8 sh_combat_gun_capacity(u8 attack) {return g_Items_GunsMaxLoadAmmo[attack];}\n' + code)
    selected = [NOTICE, '#include "combat.h"\n']
    for p, ns in SELECTED.items():
        for n in ns:
            body = function(read(p), n)
            if n == "func_80037E78":
                # PORT: Named flags replace PS1 aliasing across heading/flags.
                body = body.replace("(*(s32*)&chara->headingAngle & ((CharaFlag_Damaged | CharaFlag_Dead) << 16)) == (CharaFlag_Damaged << 16)", "(chara->flags & (CharaFlag_Damaged | CharaFlag_Dead)) == CharaFlag_Damaged")
            if n == "Chara_MovementReset":
                body = body.replace("*(s32*)&npc->properties.npc.field_EC = Q12(0.0f);", "npc->properties.npc.field_EC = 0; npc->properties.npc.field_EE = 0;")
            if n == "Items_AmmoReloadCompute":
                # Same pinned GPL capacity data in this independent unit.
                body = body.replace("g_Items_GunsMaxLoadAmmo[gunIdx]", "sh_combat_gun_capacity(gunIdx)")
            if n == "Collision_CharaCollisionSet":
                body = re.sub(r'(chara->collision[^=\n]*=) (.*?);', r'\1 (s16)(\2);', body)
            if n == "Chara_ModelBoneScaleSet":
                body = re.sub(r'(scale\[\d\] =) (scale[XYZ]);', r'\1 (s16)\2;', body)
                body = re.sub(r'(boneCoord.coord.m\[j\]\[i\] =) (.*?);', r'\1 (s16)(\2);', body)
            selected.append(body)
    (out / "combat_helpers.c").write_text("".join(selected))
    ai_namespace = ''
    for name in STALKER + ['STALKER_ANIM_INFOS', 'sharedData_800E3A20_0_s00', 'sharedData_800E3A24_0_s00', 'sharedData_800E3A28_0_s00', 'sharedData_800E3A2C_0_s00']:
        ai_namespace += f'#define {name} sh_map0_s00_{name}\n'
    for kind in SMALL_AI:
        text = read(f'src/maps/characters/{kind}.c')
        symbols = names(text) + re.findall(r'extern s_AnimInfo (\w+)\[', read(f'include/maps/characters/{kind}.h'))
        for name in symbols:
            ai_namespace += f'#define {name} sh_combat_ai_{kind}_{name}\n'
    (out/'combat_ai_namespace.h').write_text(NOTICE+ai_namespace)
    for kind in SMALL_AI + ['stalker']:
        text = read(f'src/maps/characters/{kind}.c')
        if kind == 'stalker':
            text = '#define stalkerProps stalker->properties.stalker\n' + ''.join(function(text,n) for n in STALKER)
            text = text.replace('    s32 i;\n', '')
            text = text.replace('g_SysWork.charaGroupFlags[3]', 'sh_combat_chara_group_flags[3]')
            text = text.replace('newHealth          = stalker->health', 'newHealth          = (u32)stalker->health').replace('MIN(newHealth, stalkerProps.health_110)', '(s32)MIN(newHealth, (u32)stalkerProps.health_110)')
            for field in ['keyframeIdx_FC','relKeyframeIdx_FE','sfxId_102']:
                text = re.sub(r'(stalkerProps\.'+field+r'\s*=) ([^;]+);', r'\1 (s16)(\2);', text)
            for field in ['headingAngle']:
                text = re.sub(r'(stalker->'+field+r'\s*=) ([^;]+);', r'\1 (s16)(\2);', text)
            text = re.sub(r'(keyframeIdx\s*=) (FP_FROM[^;]+);',r'\1 (u16)(\2);',text)
            text = re.sub(r'(angle\s*=) (ABS[^;]+);',r'\1 (s16)(\2);',text)
        else:
            text = re.sub(r'^#include[^\n]*\n', '', text, flags=re.M)
        (out/f'combat_ai_{kind}.c').write_text(NOTICE + '#include "combat_ai.h"\n' + text)
    # Test services use the same GPL sine table as the host, not libm trig.
    sine = re.findall(r'\.short (0x[0-9A-Fa-f]+)', read('src/bodyprog/libkmath/libkmath.s').split('dlabel g_SineTable')[1])
    if len(sine) != 5120:
        raise ValueError('pinned sine table drift')
    (out/'combat_test_math.c').write_text(NOTICE+'#include "combat.h"\nstatic const u16 sine[5120]={'+','.join(sine)+'};\ns32 Math_Sin(s32 angle) {return (s16)sine[(u32)angle&4095];}\ns32 Math_Cos(s32 angle) {return (s16)sine[((u32)angle&4095)+1024];}\n')
    ids = dict((name,int(value)) for name,value in re.findall(r'(FILE_\w+)\s*=\s*(\d+)', read('include/main/fileenum.h.USA.inc')))
    pairs = re.findall(r'\{\s*(FILE_ANIM_\w+)\s*,\s*(FILE_CHARA_\w+_ILM)',read('src/bodyprog/sys/chara_data_info.c'))
    pairs = sorted(set((ids[model],ids[anim]) for anim,model in pairs if 'DUMMY' not in anim and 'HB_BASE' not in anim))
    (out/'enemy_asset_pairs.rs').write_text('&['+','.join(f'({model},{anim})' for model,anim in pairs)+']')
    bones = function(read('src/bodyprog/world/bodyprog_anim_800445A4.c'), 'Anim_BoneInit')
    bones = bones.replace('Anim_BoneInit(', 'sh_combat_Anim_BoneInit(')
    bones = bones.replace('anmHdr->bindPoses[boneIdx].translationInitial[i] << anmHdr->scaleLog2', '(s32)((u32)(s32)anmHdr->bindPoses[boneIdx].translationInitial[i] << anmHdr->scaleLog2)')
    assign = function(read('src/bodyprog/gfx/materials.c'),'Bone_ModelAssign').replace('Bone_ModelAssign(', 'sh_combat_Bone_ModelAssign(')
    (out/'combat_assets.c').write_text(NOTICE+'#include "combat.h"\n// PORT: Native pointers and defined signed translation scaling.\n'+bones+assign)
    census(decomp, out)


def census(decomp, out):
    root=Path(__file__).resolve().parents[1]
    baseline={}
    for node in ast.walk(ast.parse((root/'tools/prepare_gameplay.py').read_text())):
        if isinstance(node,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='selections' for t in node.targets):
            baseline=ast.literal_eval(node.value)
    baseline.update({
        'src/bodyprog/player_control.c':baseline.get('src/bodyprog/player_control.c',[])+['Game_SavegameResetPlayer','func_8007E9C4'],
        'src/bodyprog/game_boot/game_boot.c':baseline.get('src/bodyprog/game_boot/game_boot.c',[])+['GameBoot_SavegameInitialize'],
        'src/bodyprog/events/player_pos_update.c':['Chara_PositionSet'],
    })
    roots = [COMBAT, *SELECTED, "src/bodyprog/player_control.c", "src/bodyprog/bodyprog_npc_8005BF38.c",
             "src/bodyprog/collision/collision.c", "src/bodyprog/collision/ray.c", "src/bodyprog/collision/los.c",
             "src/bodyprog/events/chara_spawn.c", "src/bodyprog/events/radio.c", "src/bodyprog/game_boot/fs_chara_anim.c",
             "src/bodyprog/gfx/bodyprog_effects_8005E0DC.c", "src/bodyprog/world/world_effects.c",
             "src/bodyprog/world/world_draw.c", "src/bodyprog/world/bodyprog_anim_800445A4.c",
             "src/bodyprog/world/bodyprog_bone_80044F14.c", "src/bodyprog/gfx/materials.c", "src/bodyprog/sys/chara_data_info.c",
             "src/bodyprog/items/item_utils.c", "src/bodyprog/items/item_screens_2.c", "src/bodyprog/items/item_unk_data.c"]
    roots += ['src/bodyprog/bodyprog_math_8005BF38.c','src/bodyprog/bodyprog_80089090.c',
              'src/bodyprog/sys/npc_anims_clear.c','src/bodyprog/events/bodyprog_data_800A99B4.c',
              'src/bodyprog/bodyprog_data_80028B94.c','src/bodyprog/sound/sfx.c','src/bodyprog/world/collision_trigger.c']
    roots += [str(p.relative_to(decomp)).replace('\\','/') for p in (decomp/'src/maps/characters').glob('*.c')]
    roots += [str(p.relative_to(decomp)).replace('\\','/') for p in (decomp/'src/maps').glob('particle*.c')]
    # Conservatively include every map-owned registration, spawn/animation/
    # collision data table, AI/particle wrapper and included shared gameplay
    # function header. This is a source inventory, not reachability evidence.
    for pattern in ['map*/Chara_*.c','map*/player.c','map*/particle*.c','map*/*_anim_info.c',
                    'map*/*_header.c','map*/chara_spawns.h','map*/chara_util.c']:
        roots += [str(p.relative_to(decomp)).replace('\\','/') for p in (decomp/'src/maps').glob(pattern)]
    roots += ['src/maps/map_util.c','src/bodyprog/game_boot/game_boot.c','src/bodyprog/events/player_pos_update.c']
    roots += [str(p.relative_to(decomp)).replace('\\','/') for p in (decomp/'include/maps/shared').glob('*.h')]
    rows = []
    wrapper_texts = {str(p.relative_to(decomp)).replace('\\','/'):p.read_text(encoding='utf-8') for p in (decomp/'src/maps').glob('map*/*.c')}
    for p in sorted(set(roots)):
        text = (decomp/p).read_text(encoding='utf-8')
        wrappers = []
        for wrapper,text_wrapper in wrapper_texts.items():
            if Path(p).name in text_wrapper and 'characters/' in p:
                wrappers.append(wrapper)
        migrated = names(text) if p==COMBAT or p in [f'src/maps/characters/{kind}.c' for kind in SMALL_AI] else STALKER if p=='src/maps/characters/stalker.c' else SELECTED.get(p, [])
        if p=='src/bodyprog/world/bodyprog_anim_800445A4.c':migrated=['Anim_BoneInit']
        if p=='src/bodyprog/gfx/materials.c':migrated=['Bone_ModelAssign']
        rows.append(dict(source=p, functions=names(text), core_linked_functions=baseline.get(p,[]), migrated=migrated, wrappers=wrappers))
    (out/'combat_inventory.json').write_text(json.dumps(dict(revision=PIN, sources=rows),indent=2))


def export_inventory(out, destination):
    root=Path(__file__).resolve().parents[1]
    owned=(root/'port/sys/combat').resolve()
    destination=destination.resolve()
    if not destination.is_relative_to(owned):
        raise ValueError('committed inventory must stay inside owned port/sys/combat')
    destination.write_text((out/'combat_inventory.json').read_text())
    doc=root/'docs/sys/combat.md'
    if doc.exists():
        text=doc.read_text()
        marker='<!-- COMBAT SOURCE INVENTORY -->'
        if marker in text:
            rows=json.loads(destination.read_text())['sources']
            table=['| Source | Core baseline | Combat harness |\n|---|---|---|\n']
            for row in rows:
                core=', '.join(row['core_linked_functions']) or 'unlinked'
                migrated=row['migrated'];fs=row['functions']
                own='full unit' if fs and len(migrated)==len(fs) else ', '.join(migrated) or 'uncompiled'
                table.append(f'| `{row["source"]}` | {core} | {own} |\n')
            doc.write_text(text.split(marker)[0]+marker+'\n\n'+''.join(table))


def handoff_private():
    root=Path(__file__).resolve().parents[1]
    private=(root.parent.parent/'private/work/combat').resolve()
    if private.parts[-3:] != ('private','work','combat'):
        raise ValueError('unexpected private output path')
    private.mkdir(parents=True,exist_ok=True)
    def patch(changes,name):
        result=''
        for relative, before, after in changes:
            result+=''.join(difflib.unified_diff(before.splitlines(True),after.splitlines(True),fromfile='a/'+relative,tofile='b/'+relative))
        (private/name).write_text(result,encoding='utf-8',newline='\n')
    cargo=(root/'Cargo.toml').read_text()
    line=next(line for line in cargo.splitlines(True) if line.startswith('members ='))
    if 'crates/sys-combat' in line:
        raise ValueError('workspace already registers sys-combat; review handoff')
    updated=cargo.replace(line,line.rstrip().removesuffix(']')+', "crates/sys-combat"]\n')
    manifest=(root/'crates/sys-combat/Cargo.toml').read_text()
    marker='# PORT: This checkout lacks the brief\'s root crates/* glob.'
    start=manifest.index(marker);end=manifest.index('[dependencies]',start)
    patch([('Cargo.toml',cargo,updated),('crates/sys-combat/Cargo.toml',manifest,manifest[:start]+manifest[end:])],'register-harness.patch')
    animation=(root/'host/src/gameplay.rs').read_text()
    changed=animation.replace('        || bone_count > 32\n','')
    before='''            if (i > 0 && (pose.parent < 0 || pose.parent as usize >= i))
                || pose.rotation >= header[2] as i8
                || pose.translation >= header[3] as i8
'''
    after='''            // PORT: BIRD references later bones; MTH has 44 bones. Validate
            // bounds here and acyclicity below, not serialized parent ordering.
            if i > 0 && ((pose.parent < 0 || pose.parent as usize >= usize::from(bone_count))
                || (pose.rotation >= 0 && pose.rotation as usize >= usize::from(header[2]))
                || (pose.translation >= 0 && pose.translation as usize >= usize::from(header[3])))
'''
    if before not in changed:
        raise ValueError('core ANM decoder changed; review correction before regenerating')
    changed=changed.replace(before,after)
    marker='    let end = usize::from(data_offset) + usize::from(frame_size) * usize::from(keyframe_count);'
    cycle='''    for start in 1..poses.len() {
        let mut seen = vec![false; poses.len()];
        let mut current = start;
        while current != 0 {
            if seen[current] { return Err(bad("ANM parent cycle")); }
            seen[current] = true;
            current = poses[current].parent as usize;
        }
    }
'''
    if marker not in changed:
        raise ValueError('core ANM frame boundary changed')
    changed=changed.replace(marker,cycle+marker)
    patch([('host/src/gameplay.rs',animation,changed)],'enemy-animation-decoder.patch')
    (private/'combat-build-fragment.rs').write_text('''// SPDX-License-Identifier: GPL-3.0-only
// Apply inside host/build.rs after prepare_combat.py has generated into `generated`.
// This is a production compile fragment, NOT a complete runtime/link patch.
// Resolve every provider in docs/sys/combat.md before enabling it.
build.include(repo.join("port/sys/combat"))
    .file(repo.join("port/sys/combat/combat.c"))
    .file(generated.join("combat_original.c"))
    .file(generated.join("combat_helpers.c"))
    .file(generated.join("combat_assets.c"));
// Add chosen combat_ai_<kind>.c only after binding that overlay's real tables.
// NEVER compile harness.c or combat_test_math.c into the game executable.
''',encoding='utf-8')
    print(f'Private proposals: {private}; core files unchanged')


def check_clang(decomp, out, compiler):
    root=Path(__file__).resolve().parents[1]
    sources=[root/'port/sys/combat/combat.c',out/'combat_original.c',out/'combat_helpers.c',out/'combat_assets.c']
    sources += [out/f'combat_ai_{kind}.c' for kind in SMALL_AI+['stalker']]
    flags=['-target','aarch64-apple-ios15.0','-ffreestanding','-std=c11','-Wall','-Wextra','-Werror','-nostdinc','-DSH_COMBAT_FREESTANDING_CRT']
    for include in [root/'tools/layout-include',root/'port/sys/combat',root/'port',root/'port/include',out,decomp/'include']:
        flags+=['-I',str(include.resolve())]
    for name in ['ccos','csin','csqrt','catan']:
        flags.append('-fno-builtin-'+name)
    failed=0
    for source in sources:
        result=subprocess.run([str(compiler),str(source.resolve()),'--checks=-*,clang-analyzer-core.DivideZero','--warnings-as-errors=*','--',*flags],capture_output=True,text=True)
        print(('PASS' if result.returncode==0 else 'FAIL')+': '+source.name,flush=True)
        if result.returncode:
            print(result.stdout+result.stderr,flush=True)
            failed+=1
    print(f'Combat arm64 frontend: {len(sources)-failed} passed, {failed} failed. Apple SDK/runtime not exercised.')
    return int(failed!=0)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--decomp', type=Path, default=Path('game/decomp'))
    parser.add_argument('--out', type=Path, default=Path('target/combat-generated'))
    parser.add_argument('--check-clang',type=Path)
    parser.add_argument('--export-inventory',type=Path)
    parser.add_argument('--handoff-private',action='store_true')
    args = parser.parse_args()
    generate(args.decomp, args.out)
    if args.export_inventory:
        export_inventory(args.out,args.export_inventory)
    if args.handoff_private:
        handoff_private()
    if args.check_clang:
        raise SystemExit(check_clang(args.decomp,args.out,args.check_clang))
