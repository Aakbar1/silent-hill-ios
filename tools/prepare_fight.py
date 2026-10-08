"""Pinned native combat integration, generated callers only; GPL-3.0-only.

No owned game data is read during generation. Shared source owners stay intact.
"""
import argparse
from pathlib import Path
import re
import subprocess

from prepare_combat import function, names, PIN, NOTICE
from prepare_camera import narrow
from prepare_maps import initializer, enumeration


def record(body, name):
    end = body.index('} ' + name + ';') + len('} ' + name + ';')
    start = body.rfind('typedef struct', 0, end)
    return body[start:end] + '\n'


def remove_definition(out, filename, name):
    path = out / filename
    text = path.read_text(encoding='utf-8')
    if name in names(text):
        text = text.replace(function(text, name), '')
    path.write_text(text, encoding='utf-8')


def arguments(text):
    result = []
    depth = 0
    start = 0
    for i, char in enumerate(text):
        if char in '([':
            depth += 1
        elif char in ')]':
            depth -= 1
        elif char == ',' and depth == 0:
            result.append(text[start:i].strip())
            start = i + 1
    result.append(text[start:].strip())
    return result


def prepare(decomp, out):
    revision = subprocess.check_output(['git', '-C', str(decomp), 'rev-parse', 'HEAD'], text=True).strip()
    if revision != PIN:
        raise ValueError('fight reference revision differs from reviewed pin')
    read = lambda p: (decomp / p).read_text(encoding='utf-8')
    body = read('include/bodyprog/bodyprog.h')
    original = read('src/bodyprog/gfx/bodyprog_effects_8005E0DC.c')
    selected = [n for n in names(original) if n not in ['Map_EffectTexturesLoad', 'func_8005E414', 'func_8005E650', 'func_8005E70C']]
    types = ['s_800C42E8', 's_800C4418', 's_func_8005E89C', 's_func_80060044', 's_func_800611C0',
             's_func_80062708', 's_func_8006342C', 's_func_80063A50', 's_func_80064334', 's_func_80064FC0']
    records = ''.join(record(body, name) for name in types if name != 's_800C4418')
    header = NOTICE + '#ifndef SH_FIGHT_FX_H\n#define SH_FIGHT_FX_H\n#include "render_services.h"\n#include "native_player_events.h"\n#include "render_gte.h"\n' + records
    header += ''.join(re.sub(r'//[^\n]*', '', function(original, n).split('{', 1)[0]).strip()+';\n' for n in selected)
    header += 'extern s_800C42E8 D_800C42E8[24];\nextern s8 D_800C4414;\nextern s16 D_800C4408;\n'
    header += 's32 Chara_NpcIdxGet(s_SubCharacter*);\n#endif\n'
    header = header.replace('#endif\n', 'extern u32 port_fight_effect_bytes;\n#endif\n')
    header = header.replace('#endif\n', '''// PORT: Defined halfword stores replace signed-shift packing macros.
static inline void port_fight_svector(SVECTOR* p,s32 x,s32 y,s32 z) {p->vx=(s16)x;p->vy=(s16)y;p->vz=(s16)z;}
static inline MATRIX* port_fight_trans_matrix(MATRIX* p,const VECTOR* v) {p->t[0]=v->vx;p->t[1]=v->vy;p->t[2]=v->vz;return p;}
#undef Math_SetSVectorFast
#undef Math_SetSVectorFastSum
#define Math_SetSVectorFast(p,x,y,z) port_fight_svector((SVECTOR*)(p),x,y,z)
#define Math_SetSVectorFastSum(p,x,y,z) port_fight_svector((SVECTOR*)(p),x,y,z)
#endif
''')
    # PORT: Impact position has exactly three meaningful words; callers supply
    # VECTOR3. The original effect never accesses a fourth padding word.
    header = header.replace('VECTOR* pos,', 'VECTOR3* pos,')
    (out / 'fight_fx.h').write_text(header, encoding='utf-8')
    arena = 'typedef union {\n'+''.join(f'    {t} {t};\n' for t in types[2:])+'} FightFxWork;\nstatic FightFxWork fight_fx_work;\n'
    globals_code = ''.join(initializer(original, n) for n in ['D_800AE5CC', 'D_800AE5F0', 'D_800AE700'])
    globals_code = re.sub(r'0x[Ff]([0-9A-Fa-f]{3})\b', lambda m: str(int(m[0], 16)-65536), globals_code)
    globals_code += 's_800C42E8 D_800C42E8[24];\ns16 D_800C4408;\nGsCOORDINATE2* D_800C440C;\nGsCOORDINATE2* D_800C4410;\nextern s_800C4418 D_800C4418;\n'
    globals_code += enumeration(body, 'EffectTextureFlags')
    for name in ['D_800A9084', 'D_800A908C', 'D_800A9094']:
        globals_code += initializer(read('src/bodyprog/screen/screen_data.c'), name)
    textures = function(original, 'Map_EffectTexturesLoad')
    textures = re.sub(r'\(s32\)FONT24_BUFFER - ALIGN\(Fs_GetFileSize\((\w+)\), 0x800\)', r'port_fight_texture_buffer(\1)', textures)
    textures = textures.replace('loadedEffectTextureFlags = effectTexFlags;', 'loadedEffectTextureFlags = (u16)effectTexFlags;')
    textures = textures.replace('loadedEffectTextureFlags |= 1 << i;', 'loadedEffectTextureFlags = (u16)(loadedEffectTextureFlags | (1u << i));')
    # PORT: The pinned GPU macro returns the MIPS zero register unconditionally.
    textures = textures.replace('gte_IsDisabled()', 'false')
    textures = re.sub(r'^\s*static s16 __pad_bss_800C42DA\[7\];\n', '\n', textures, flags=re.M)
    globals_code += '''// PORT: Synchronous queue reads use native scratch, not a PS1 font-arena address.
static _Alignas(8) u8 fight_texture_buffer[0x10000];
static void* port_fight_texture_buffer(s32 file) {if((u32)Fs_GetFileSize(file)>sizeof(fight_texture_buffer))port_unimplemented("effect texture scratch bounds");return fight_texture_buffer;}
'''+textures
    codes = []
    for name in selected:
        code = function(original, name).replace('VECTOR* pos,', 'VECTOR3* pos,')
        code = re.sub(r'\bTransMatrix\(', 'port_fight_trans_matrix(', code)
        typ = re.search(r'(s_func_\w+)\*\s+ptr;', code)
        if typ:
            code = code.replace('ptr = PSX_SCRATCH;', f'ptr = &fight_fx_work.{typ[1]}; // PORT: Shared scheduler prefix, typed native scratch.')
        code = code.replace('g_SysWork.field_2388.isFlashlightUnavailable', 'g_SysWork.gameplayEnvironment.isFlashlightUnavailable')
        code = code.replace('g_SysWork.field_2388.field_154.', 'g_SysWork.gameplayEnvironment.field_154.')
        # Numeric unions reuse short field names at different widths. Only
        # unambiguous work fields may use the generic store-width adapter.
        view = re.sub(r'\b(?:s16|u16|s8|u8|q3_12)\s+field_[0-9A-C]\s*;', '', records)
        code = narrow(code, view)
        if name == 'func_80060044':
            code = re.sub(r'\s*POLY_FT4\*\s+next;\n', '\n', code)
        if name == 'func_8005E89C':
            code = code.replace('GsOUT_PACKET_P = (PACKET*)poly;', 'port_fight_effect_bytes += (u32)((PACKET*)poly-GsOUT_PACKET_P);\n    GsOUT_PACKET_P = (PACKET*)poly;')
        code = code.replace('D_800C4408 = idx + 1;', 'D_800C4408 = (s16)(idx + 1);')
        code = code.replace('g_MapOverlayHdr.func_80(i);', '((void (*)(s32))g_MapOverlayHdr.func_80)(i);')
        if name == 'func_8005F6B0':
            code = code.replace('{', '{\n    // PORT: The original distance scratch has 150 entries. Reject malformed map counts before filling it.\n    if(g_MapOverlayHdr.bloodSplatCount<0 || g_MapOverlayHdr.bloodSplatCount>150)port_unimplemented("blood splat count");', 1)
            code = code.replace('q20_12             dists[150];', 'q20_12             dists[150]={0};')
        code = code.replace('dists[k] <= curDist', 'dists[k] <= (u32)curDist')
        code = code.replace('*g_MapOverlayHdr.data_190 != NULL', '*g_MapOverlayHdr.data_190 != 0')
        code = code.replace('Math_RotMatrixZxyNegGte(&ptr->field_14C,', 'Math_RotMatrixZxyNegGte(ptr->field_14C,')
        code = re.sub(r'\*poly = ptr->field_12C \+ (0x18|24);', r'*poly = (POLY_FT4*)(ptr->field_12C + \1);', code)
        # PORT: Keep packed byte/halfword fields at their original widths.
        for target, typ2 in [
            (r'g_MapOverlayHdr\.unkTable1_4C\[[^;\n]+?\]\.vy_8', 's16'),
            (r'g_MapOverlayHdr\.unkTable1_4C\[[^;\n]+?\]\.field_[AB]', 'u8'),
            (r'g_MapOverlayHdr\.unkTable1_4C\[[^;\n]+?\]\.field_C\.s_1\.field_[0-3]', 'u8'),
            (r'g_MapOverlayHdr\.unkTable1_4C\[[^;\n]+?\]\.field_C\.s_0\.field_[02]', 's16'),
            (r'g_MapOverlayHdr\.unkTable1_4C\[[^;\n]+?\]\.field_10\.s_0\.field_[02]', 's16'),
            (r'D_800C42E8\[[^]]+\]\.field_2', 's16'),
            (r'D_800C42E8\[[^]]+\]\.field_1', 'u8'),
            (r'g_MapOverlayHdr\.bloodSplats\[[^]]+\]\.field_0', 's16'),
            (r'g_MapOverlayHdr\.unkTable1_4C\[[^;\n]+?\]\.field_10\.field_0', 'u32'),
            (r'g_MapOverlayHdr\.field_(?:5C|7C)->field_10', 's16'),
            (r'ptr->(?:field_34|field_64|field_94|field_DC|field_E4)\[[^]]+\]', 's16'),
            (r'ptr->u_field_(?:EC|FC)\.field_0\[[^]]+\]\.v[xy]', 's16'),
            (r'ptr->field_14C\[[^]]+\]\.v[xyz]', 's16'),
            (r'\*\(u16\*\)&\(\*poly\)->r0', 'u16'),
            (r'\(\*poly\)->b0', 'u8')]:
            code = re.sub(r'('+target+r')\s*(\+=|-=|=(?!=))\s*([^;{}]+);',
                          lambda m:m[1]+' = ('+typ2+')('+(m[3] if m[2]=='=' else m[1]+' '+m[2][0]+' ('+m[3]+')')+');', code)
        # PORT: The fast SDK setters take halfwords; explicit narrowing matches
        # their MIPS argument stores without narrowing world-space outputs.
        code = re.sub(r'Math_SetSVectorFastSum\(([^;]+)\);',
                      lambda m: 'Math_SetSVectorFastSum('+', '.join([arguments(m[1])[0]]+['(s16)('+v+')' for v in arguments(m[1])[1:]])+');', code)
        code = re.sub(r'(func_80055A90\([^,]+,[^,]+,)\s*([^,]+),', r'\1 (u8)(\2),', code)
        code = code.replace('(s16)(Q12(50.0f))', '(u32)(Q12(50.0f))').replace('(s16)(Q12(45.0f))', '(u32)(Q12(45.0f))')
        code = code.replace('i < ARRAY_SIZE(D_800C42E8)', 'i < (s32)ARRAY_SIZE(D_800C42E8)')
        code = code.replace('i < ARRAY_SIZE(dists)', 'i < (s32)ARRAY_SIZE(dists)')
        code = code.replace('chara->position,', 'chara->position,')
        codes.append(code)
    (out / 'player_effects.c').write_text(NOTICE + '#include "fight_fx.h"\n'+arena+globals_code+''.join(codes)+'\n#include "fx_anm.c"\n#include "fx_debug.c"\n', encoding='utf-8')
    remove_definition(out, 'player_combat.c', 'func_8005F6B0')
    remove_definition(out, 'npc_ai.c', 'func_800622B8')
    remove_definition(out, 'player_services.c', 'func_8006342C')
    remove_definition(out, 'maps_base.c', 'Map_EffectTexturesLoad')
    remove_definition(out, 'npc_loop.c', 'func_80037E78')
    path = out/'npc_ai.c'
    text = path.read_text(encoding='utf-8')
    if 'func_80037E78' not in names(text):
        stats = function(read('src/bodyprog/events/npc_main.c'), 'func_80037E78')
        stats = stats.replace('(*(s32*)&chara->headingAngle & ((CharaFlag_Damaged | CharaFlag_Dead) << 16)) == (CharaFlag_Damaged << 16)',
                              '(chara->flags & (CharaFlag_Damaged | CharaFlag_Dead)) == CharaFlag_Damaged')
        stats = stats.replace('if (idx < 39)', 'if (idx >= 0 && idx < 39)')
        text += '\n// PORT: Read the native flags field directly; reject the no-attack sentinel before indexing.\n'
        text += 'void func_800914C4(s32,u32);u32 func_8009146C(s32);\n'+stats
        ranking = read('src/bodyprog/ranking.c')
        for name in ['func_8009146C', 'func_800914C4']:
            code = narrow(function(ranking, name), read('include/game.h'))
            code = code.replace('res = var_v1_2 + var_v0;', 'res = var_v1_2 + (u32)var_v0;')
            code = code.replace('meleeKillCount  = val;', 'meleeKillCount  = (u8)val;').replace('rangedKillCount = val;', 'rangedKillCount = (u8)val;')
            text += code
    path.write_text(text, encoding='utf-8')
    path = out/'player_combat.c'
    text = path.read_text(encoding='utf-8').replace('s32 func_8009146C(s32);void func_800914C4(s32,s32);', 'u32 func_8009146C(s32);void func_800914C4(s32,u32);')
    path.write_text(text, encoding='utf-8')
    prepare_weapons(decomp, out)
    caller = out / 'audio_caller_runtime.c'
    text = caller.read_text(encoding='utf-8')
    # PORT: Keep the original native graph bridge for every other format/file.
    text = text.replace('port_asset_load_native((u32)job.file,', 'port_fight_asset_load((u32)job.file,')
    declaration = 'int port_fight_asset_load(u32,u8*,u32);\n'
    if declaration not in text:
        text = '#include "boot.h"\n'+declaration+text
    declaration = 'void port_fight_frame(u32);\n'
    if declaration not in text:
        text = '#include "boot.h"\n'+declaration+text
        text = text.replace('    vblanks++;', '    vblanks++;\n    port_fight_frame((u32)vblanks);')
    caller.write_text(text, encoding='utf-8')
    path = out/'render_consumers.c'
    text = path.read_text(encoding='utf-8')
    held = function(text, 'port_render_held_item')
    if 'port_fight_held_draws++' not in held:
        changed = held.replace('func_80057090(&heldItem->bone.modelInfo,', 'port_fight_held_draws++;\n        func_80057090(&heldItem->bone.modelInfo,')
        text = 'extern unsigned int port_fight_held_draws;\n'+text.replace(held, changed)
    path.write_text(text, encoding='utf-8')
    from prepare_audio import prepare_fight_audio
    prepare_fight_audio(decomp, out)
    path = out/'render_milestone.rs'
    text = path.read_text(encoding='utf-8')
    marker = '\n// Fight replay driver: numeric evaluator owns the pass criteria.\n'
    text = text.split(marker)[0] + marker + r'''
#[cfg(test)]
mod fight_replay {
    use super::*;
    #[test]
    #[ignore = "owned disc; tools/check_fight.py --run"]
    fn fight_warp() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let private = root.join("../../private");
        let evidence = private.join("work/fight");
        std::fs::create_dir_all(&evidence).unwrap();
        let disc = GameDisc::open(private.join("disc/Silent Hill (USA).bin")).unwrap();
        let warp = crate::maps::DebugWarp::parse(&std::env::var("SH_FIGHT_WARP").unwrap()).unwrap();
        let input = std::env::var("SH_FIGHT_INPUT").unwrap();
        let replay = ReplayPad::parse(&std::fs::read_to_string(input).unwrap()).unwrap();
        let frames = std::env::var("SH_FIGHT_FRAMES").unwrap().parse().unwrap();
        crate::spu_cpal::configure(crate::spu_cpal::AudioMode::Off).unwrap();
        let options = crate::gpu_wgpu::Options {scale: 1, ..Default::default()};
        crate::gpu_wgpu::configure(options).unwrap();
        let result = run_headless(disc,frames,Some(evidence.join("combat.png")),replay,
            ReplayCheck {warp: Some(warp),state: Some(11),min_lit_pixels: 1,..Default::default()});
        assert!(result.is_ok(), "fight warp: {result:?}");
    }
}
'''
    path.write_text(text, encoding='utf-8')


def prepare_weapons(decomp, out):
    read = lambda p: (decomp / p).read_text(encoding='utf-8')
    original = read('src/bodyprog/player_control.c')
    data = read('src/bodyprog/bodyprog_data_80028B94.c')
    table = initializer(data, 'HARRY_WEAPON_ANIM_INFOS')
    # PORT: Original copies twenty entries, including unused tail entries. Own
    # that tail explicitly instead of reading beyond the HyperBlaster table.
    table = table.replace('HARRY_WEAPON_ANIM_INFOS[]', 'HARRY_WEAPON_ANIM_INFOS[152]')
    table = table.replace('NO_VALUE,', '(u8)NO_VALUE,')
    table += initializer(data, 'D_800294F4')
    code = function(original, 'GameFs_WeaponInfoUpdate')
    code = code.replace('playerCombat.', 'g_SysWork.playerCombat.')
    code = code.replace('func_8007F14C(playerCombat.weaponAttack);', 'func_8007F14C((u8)playerCombat.weaponAttack);')
    code = code.replace('func_8007F14C(g_SysWork.playerCombat.weaponAttack);', 'func_8007F14C((u8)g_SysWork.playerCombat.weaponAttack);')
    code = re.sub(r'Fs_QueueStartRead\((FILE_ANIM_HB_WEP\w+), FS_BUFFER_12\)', r'port_fight_weapon_frames(\1)', code)
    path = out / 'player_controls.c'
    text = path.read_text(encoding='utf-8')
    text = text.split('\n#include "native_player_movement.h"\n#include "fx_weapons.h"\n')[0]
    if 'GameFs_WeaponInfoUpdate' in names(text):
        text = text.replace(function(text, 'GameFs_WeaponInfoUpdate'), '')
    text += '\n#include "native_player_movement.h"\n#include "fx_weapons.h"\n' + table
    text += function(original, 'func_8007F14C') + code
    text += '\n#include "fx_weapons.c"\n'
    text = text.replace('extra.lowerBodyState != lowerBodyState', 'extra.lowerBodyState != (s32)lowerBodyState')
    path.write_text(text, encoding='utf-8')


def check_clang(decomp, out, compiler):
    """Check the actual Cargo-generated playable closure, including new units."""
    root = Path(__file__).resolve().parents[1]
    if not (out/'fight_fx.h').exists():
        raise ValueError('build the host first, then pass its native-source directory with --out')
    native = ['camera_services', 'world_services', 'player_loop', 'player_controls', 'player_trace',
              'player_dms', 'render_services', 'maps_runtime', 'audio_services', 'audio_session', 'gte_services', 'layout_check']
    generated = ['audio_caller_runtime', 'audio_caller_gameplay', 'audio_caller_title_services', 'audio_caller_npc_startup',
                 'gameplay_consumers', 'player_startup', 'player_controls', 'player_loop', 'player_combat', 'player_movement',
                 'player_collision', 'player_services', 'player_sfx', 'player_effects', 'player_dms', 'player_events',
                 'npc_models', 'npc_loop', 'npc_ai', 'player_rays', 'render_consumers', 'world_consumers', 'collision_consumers',
                 'maps_base', 'maps_registry', 'maps_ground', 'map_info', 'player_spawn', 'native_math', 'camera_globals',
                 'vc_main', 'vc_util', 'vw_main', 'vw_calc', 'audio_driver']
    sources = [root/'port'/(n+'.c') for n in native]+[out/(n+'.c') for n in generated]
    sources += sorted(out.glob('map?_s??.c'))
    flags = ['-target', 'aarch64-apple-ios15.0', '-ffreestanding', '-std=c11', '-Wall', '-Wextra', '-Werror',
             '-nostdinc', '-DSH_NATIVE_AUDIO', '-DSH_CHECK_BOOT_LAYOUT']
    for include in [root/'tools/layout-include', root/'port', root/'port/include', out, decomp/'include', decomp/'src/main']:
        flags += ['-I', str(include.resolve())]
    flags += ['-fno-builtin-'+n for n in ['ccos', 'csin', 'csqrt', 'catan']]
    failed = 0
    for source in sources:
        run = subprocess.run([str(compiler), str(source.resolve()), '--checks=-*,clang-analyzer-core.DivideZero',
                              '--warnings-as-errors=*', '--', *flags], capture_output=True, text=True)
        print(('FAIL' if run.returncode else 'PASS')+': '+source.name, flush=True)
        if run.returncode:
            failed += 1
            print(run.stdout+run.stderr, flush=True)
    print(f'Fight arm64 frontend: {len(sources)-failed} passed, {failed} failed; Apple SDK/device untested.')
    return int(failed != 0)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--decomp', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--check-clang', type=Path)
    args = parser.parse_args()
    if args.check_clang:
        raise SystemExit(check_clang(args.decomp, args.out, args.check_clang))
    prepare(args.decomp, args.out)
