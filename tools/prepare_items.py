"""Pinned GPL items source preparation. Never reads or emits game image code.

SPDX-License-Identifier: GPL-3.0-only
Generated native headers are work records, not PS1 file views. Wire decoders are
in port/sys/items/abi.c. Core's broad original-header gate is left untouched.
"""
from pathlib import Path
import argparse
import hashlib
import json
import re
import sys
import subprocess

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from prepare_gameplay import function
from prepare_gte import generate as prepare_gte

PIN = "d9e28f8315c7938117224f21516786d9d149a145"
ROOT = Path(__file__).resolve().parents[1]
NOTICE = "/* SPDX-License-Identifier: GPL-3.0-only; derived from silent-hill-decomp.\n * Copyright (C) 2026 shdecompilations. See LICENSE.decomp. */\n"
SOURCES = {
    "item_screens_2": "src/bodyprog/items/item_screens_2.c",
    "item_screens_3": "src/bodyprog/items/item_screens_3.c",
    "item_screens_cam": "src/bodyprog/items/item_screens_cam.c",
    "item_unk_data": "src/bodyprog/items/item_unk_data.c",
    "item_utils": "src/bodyprog/items/item_utils.c",
    "mapscreen": "src/bodyprog/bodyprog_mapscreen_80066D90.c",
    "radio": "src/bodyprog/events/radio.c",
    "saveload": "src/screens/saveload/saveload.c",
    "stf_roll": "src/screens/credits/credits.c",
    "ranking": "src/bodyprog/ranking.c",
    "save_menu": "src/bodyprog/sys/memcard_2.c",
    "text_messages": "src/bodyprog/text/map_msg_display.c",
}


def patched(text, old, new, path):
    if '\n' not in old and old[:1].isspace():
        text, count = re.subn('^' + re.escape(old) + '$', lambda m: new, text, flags=re.M)
        if not count:
            raise ValueError(f'patch drift: {path}: {old}')
        return text
    if old not in text:
        raise ValueError(f'patch drift: {path}: {old}')
    return text.replace(old, new)


def fixed_words(text):
    # PORT: SDK long is a 32-bit word on PS1, never a host pointer or size_t.
    text = re.sub(r"\bunsigned long\b(?! long)", "uint32_t", text)
    return re.sub(r"(?<!long )\blong\b(?! long)", "int32_t", text)


def usa_only(text):
    """Resolve version-only branches, including directives inside SDK arguments."""
    def predicate(expr):
        dates = dict(PROTO_981216=981216, JAP0=990126, USA=990210, JAP1=990602, EUR=990607, JAP2=990616)
        expr = re.sub(r'//.*|/\*.*?\*/', '', expr).strip()
        def value(m):
            op, ver = m.groups()
            return str(int({'IS': ver == 'USA', 'REGION_IS': ver == 'NTSC',
                            'EQUAL_OR_NEWER': 990210 >= dates.get(ver, 999999),
                            'EQUAL_OR_OLDER': 990210 <= dates.get(ver, 0)}[op]))
        expr = re.sub(r'VERSION_(IS|REGION_IS|EQUAL_OR_NEWER|EQUAL_OR_OLDER)\((\w+)\)', value, expr)
        if not re.fullmatch(r'[01\s!&|()]+', expr):
            raise ValueError(f'unsupported version branch: {expr}')
        return bool(eval(expr.replace('&&', ' and ').replace('||', ' or ').replace('!', ' not ')))
    stack, result = [], []
    for line in text.splitlines(True):
        m = re.match(r'\s*#(if|ifdef|ifndef|elif|else|endif)\b(.*)', line)
        if m:
            op, expr = m.groups()
            if op in ('if', 'ifdef', 'ifndef'):
                known = op == 'if' and 'VERSION_' in expr
                chosen = predicate(expr) if known else True
                stack.append([known, chosen, chosen])
                if known:
                    continue
            elif op in ('elif', 'else') and stack[-1][0]:
                frame = stack[-1]
                frame[1] = not frame[2] and (op == 'else' or predicate(expr))
                frame[2] |= frame[1]
                continue
            elif op == 'endif':
                known = stack.pop()[0]
                if known:
                    continue
        if all(s[1] for s in stack):
            result.append(line)
    return ''.join(result)


def mask(text):
    return re.sub(r'//[^\n]*|/\*.*?\*/|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|^\s*#[^\n]*(?:\\\n[^\n]*)*',
                  lambda m: ''.join('\n' if c == '\n' else ' ' for c in m[0]), text, flags=re.S | re.M)


def cast_calls(text, name, types):
    """Make PS1 argument narrowing explicit without changing evaluation order."""
    clean = mask(text)
    for match in reversed(list(re.finditer(r'\b' + name + r'\(', clean))):
        start = match.end()
        depth, args, begin = 1, [], start
        for i in range(start, len(text)):
            c = clean[i]
            depth += (c == '(') - (c == ')')
            if (c == ',' and depth == 1) or depth == 0:
                args.append(text[begin:i].strip())
                begin = i + 1
            if depth == 0:
                args = [f'({types[j]})({a})' if j in types else a for j, a in enumerate(args)]
                text = text[:start] + ', '.join(args) + text[i:]
                break
    return text


def event_native(text):
    # PORT: Original SDK/event parameters narrow to PS1 words at the call boundary.
    text = cast_calls(text, 'Gfx_CursorDraw', {1:'s16'})
    text = cast_calls(text, 'Sd_SfxAttributesUpdate', {2:'q0_8', 3:'s8'})
    text = cast_calls(text, 'vcChangeProjectionValue', {0:'u16'})
    text = cast_calls(text, 'PaperMap_ExpandingBoxesDraw', {i:'q3_12' for i in range(9)})
    text = cast_calls(text, 'Screen_BackgroundImgTransition', {2:'q3_12'})
    text = cast_calls(text, 'PaperMap_DrawScaled', {0:'u16', 1:'u16', 2:'q4_12'})
    text = cast_calls(text, 'func_80068E0C', {4:'u16', 5:'u16', 6:'q4_12'})
    text = cast_calls(text, 'Gfx_RectangleDraw', {i:'s16' for i in range(4)})
    text = cast_calls(text, 'Event_ScreenFadeCmd', {2:'e_ScreenFadeType'})
    text = cast_calls(text, 'Event_BgTextureCmd', {0:'e_BgTextureCmd'})
    # PORT: Keep local enum constants while acknowledging an intentionally
    # unused typedef created solely for upstream matching.
    text = text.replace('} e_EventState;', '} e_EventState; (void)sizeof(e_EventState);')
    text = re.sub(r'(Vc_SetLookAtMatFromBoneCoord\([^;]*?),\s*&SVECTOR3_Zero\s*,', r'\1, &(const SVECTOR){0},', text)
    text = text.replace('while (true)', 'for (;;)')
    return text


def map_symbols(decomp, header, code):
    symbols = set(re.findall(r'\bshared(?:Data|Func)_\w+', code))
    header_text = (decomp / 'include' / header).read_text(encoding='utf-8')
    for declaration in re.findall(r'^extern[^;]+;', header_text, re.M):
        match = re.search(r'(\w+)\s*(?:\[[^]]*\]\s*)*;', declaration)
        if match:
            symbols.add(match[1])
    return symbols


def declarations(text):
    """Top-level object declarations (never function bodies or extern imports)."""
    clean = mask(text)
    start, depth, in_function = 0, 0, False
    for i, c in enumerate(clean):
        if c == '{':
            if depth == 0:
                prefix = clean[start:i]
                in_function = '(' in prefix and '=' not in prefix
            depth += 1
        elif c == '}':
            depth -= 1
            if depth == 0 and in_function:
                start = i + 1  # function body, not an initializer
        elif c == ';' and depth == 0:
            segment = clean[start:i].strip()
            original = text[start:i+1].strip()
            start = i + 1
            if not segment or segment.startswith(('extern ', 'typedef ')):
                continue
            # Drop comments/directives before the real declaration.
            match = re.search(r'(?:static\s+)?(?:const\s+)?[A-Za-z_]\w*(?:\s*\*|\s+)[^;]+', original)
            if not match:
                continue
            # Original can begin with a comment; use the masked first-token offset.
            # Find actual declaration with the first token of the clean segment.
            token = re.search(r'[A-Za-z_]\w*', segment)[0]
            offset = re.search(r'\b' + re.escape(token) + r'\b', mask(original)).start()
            original = original[offset:]
            before = segment.split('=', 1)[0].strip()
            if '(' in before and '(*' not in before:
                continue  # prototype
            pointer_fn = re.search(r'\(\*\s*(?:const\s+)?(\w+)', before)
            name = pointer_fn[1] if pointer_fn else re.search(r'(\w+)\s*(?:\[[^]]*\]\s*)*$', before)[1]
            yield name, original


def screen(text, name, out):
    """Namespace and reset all compiled writable objects, including local statics."""
    # PORT: The dispatch pointer arrays are read-only by use. Enforce that
    # instead of treating code identities as mutable overlay state.
    text = re.sub(r'\(\*(g_GameState_(?:SaveScreen|AutoLoadSavegame)_Funcs)\[', r'(*const \1[', text)
    funcs = list(re.finditer(r'^(?:static\s+)?(?:void|s32|u32|s16|u16|bool)\s+(\w+)\([^;{}]*\)[^{;]*\{', text, re.M))
    locals_, comparisons, resets, probes = [], [], [], []
    # Move function statics to file scope so reset can restore compiled images.
    for match in reversed(funcs):
        body = function(text, match[1])
        for local in list(re.finditer(r'^    static\s+[^;]+;', mask(body), re.M)):
            declaration = body[local.start():local.end()].strip()
            clean = mask(declaration)
            if clean.startswith('static const '):
                continue
            before = clean.split('=', 1)[0].rstrip(';').strip()
            variable = re.search(r'(\w+)\s*(?:\[[^]]*\]\s*)*$', before)[1]
            renamed = f'{match[1]}_{variable}'
            body = body.replace(body[local.start():local.end()], '')
            body = re.sub(r'\b' + variable + r'\b', renamed, body)
            locals_.append(re.sub(r'\b' + variable + r'\b', renamed, declaration))
        original = function(text, match[1])
        text = text.replace(original, body)
    text = '\n'.join(locals_) + '\n' + text
    objects = list(declarations(text))
    owned = [m[1] for m in funcs] + [n for n, _ in objects]
    namespace = NOTICE + '\n'.join(f'#define {n} sh_{name}_{n}' for n in owned) + '\n'
    (out / f'{name}_namespace.h').write_text(namespace)
    images = []
    writable = []
    for variable, declaration in objects:
        if re.match(r'(?:static )?const ', declaration) or '(*const ' in declaration:
            continue
        writable.append(variable)
        before, sep, init = declaration.rstrip(';').partition('=')
        initial = re.sub(r'\b' + variable + r'\b', variable + '_initial', before)
        initial = re.sub(r'^static\s+', '', initial.strip())
        # Use const object images; pointer targets remain mutable native records.
        initial = 'static ' + initial + (' = ' + init if sep else ' = {0}') + ';'
        images.append(initial)
        resets.append(f'memcpy(&{variable}, &{variable}_initial, sizeof({variable}));')
        comparisons.append(f'memcmp(&{variable}, &{variable}_initial, sizeof({variable})) == 0')
        probes.append(f'memset(&{variable}, 0xa5, sizeof({variable}));')
    text += '\n' + '\n'.join(images)
    text += f'\nvoid sh_{name}_reset(void) {{\n' + '\n'.join(resets) + '\n}\n'
    text += f's32 sh_{name}_reset_probe(void) {{\n' + '\n'.join(probes) + f'\nsh_{name}_reset();\nreturn ' + (' && '.join(comparisons) or '1') + ';\n}\n'
    return '#include "' + name + '_namespace.h"\n#include "game.h"\n' + text, {'symbols': owned, 'writable': writable}


def prepare(decomp, out):
    actual = subprocess.check_output(['git', '-C', str(decomp), 'rev-parse', 'HEAD'], text=True).strip()
    if actual != PIN:
        raise ValueError(f'unsupported decomp revision: {actual}')
    out.mkdir(parents=True, exist_ok=True)
    inc = out / "include"
    sizes = json.loads((ROOT / "port/sys/items/native_sizes.json").read_text(encoding="utf-8"))
    patches = json.loads((ROOT / "port/sys/items/source_patches.json").read_text(encoding="utf-8"))
    for original in (decomp / "include").rglob("*.h"):
        rel = original.relative_to(decomp / "include")
        dest = inc / rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        text = original.read_text(encoding="utf-8")
        if rel.parts[0] == "psyq" or str(rel).replace("\\", "/") == "decomp/types.h":
            text = fixed_words(text)
        for typ, size in sizes.items():
            text = re.sub(r"STATIC_ASSERT_SIZEOF\(" + re.escape(typ) + r",\s*(?:\d+|0x[0-9A-Fa-f]+)\);",
                          f"// PORT: {typ} owns native pointers; separate from disk32.\nSTATIC_ASSERT_SIZEOF({typ}, {size});", text)
        # PORT: MSVC cannot coalesce different declared bitfield types.
        if rel.as_posix() == "psyq/libgs.h":
            text = text.replace("unsigned char num:8;", "unsigned num:8;")
        if rel.as_posix() == 'libkpad.h':
            # PORT: These eight flags are one PS1 byte, not separate MSVC
            # allocation units for mixed u8/u32 bitfields.
            text = re.sub(r'u32(\s+field_5_[567]\s*:\s*1;)', r'u8\1', text)
        if rel.as_posix() == "psyq/libgpu.h":
            text += '\n#include "items_gpu.h"\n'
        if rel.as_posix() == "main/fsqueue.h":
            text = re.sub(r'(#define\s+\w+\s+)\(([^)]+)\)0x([0-9A-Fa-f]+)', r'\1(\2)sh_items_address(0x\3u)', text)
            text = 'void* sh_items_address(uint32_t address); // PORT: bounded native arenas.\n' + text
        if rel.as_posix() == "bodyprog/memcard.h":
            text = re.sub(r'\(\(u8\*\)0x801E09E0\)', '((u8*)sh_items_address(0x801E09E0u))', text)
            text = re.sub(r'\(\(u8\*\)0x801E1430\)', '((u8*)sh_items_address(0x801E1430u))', text)
            text = text.replace('((s_SaveScreenElement*)&SAVEGAME_ENTRY_BUFFER_0[2640 * (slotIdx)])', 'sh_items_save_elements((u32)(slotIdx))')
            text = 'struct _SaveScreenElement* sh_items_save_elements(uint32_t slot);\n' + text
        if rel.as_posix() == 'screens/saveload.h':
            text = re.sub(r'\(\*(g_GameState_(?:SaveScreen|AutoLoadSavegame)_Funcs)\[', r'(*const \1[', text)
        if rel.as_posix() == 'bodyprog/item_screens.h':
            aliases = ['g_Items_Lights', 'D_800C3A88', 'D_800C3AC8', 'D_800C3E18', 'g_Inventory_EquippedItemIdx', '__pad_bss_800C3E38', 'g_Items_ItemsModelData', 'D_800C3E08']
            for variable in aliases:
                text = re.sub(r'^extern[^;\n]*\b' + variable + r'\b[^;\n]*;[^\n]*', '', text, flags=re.M)
            text += '\n#include "items_aliases.h"\n'
        if rel.as_posix() == 'bodyprog/formats/tmd.h':
            text = text.replace('struct TMD_STRUCT models[1];', 'struct TMD_STRUCT* models; // PORT: owned decoded models, never a wire tail.')
            text += '\n_Static_assert(sizeof(s_TmdFile)==24,"native TMD header");\n_Static_assert(sizeof(struct TMD_STRUCT)==48,"native TMD object");\nvoid sh_items_tmd_prepared(s_TmdFile* header);\n'
        if rel.as_posix() == "psyq/strings.h":
            text = NOTICE + '// PORT: use native CRT sizes and declarations.\n#include <string.h>\n#define bzero(p,n) memset((p),0,(n))\n#define bcopy(s,d,n) memmove((d),(s),(n))\n'
        if rel.as_posix() == "decomp/common.h":
            text = text.replace('#include "include_asm.h"', '#include <stdint.h>\n#include <stddef.h>\n#include <string.h>\n#include "include_asm.h"')
            text = text.replace('#define SECTION(x) \\\n    __attribute__((section(x)))', '#define SECTION(x) /* PORT: ordinary native storage. */')
            text = text.replace('#define PSX_SCRATCH ((void*)0x1F800000)', 'extern _Alignas(8) u8 port_scratch[1024];\n#define PSX_SCRATCH ((void*)port_scratch) // PORT: shared native scratch arena.')
        if rel.as_posix() == "psyq/sys/types.h":
            text = "#include <stdint.h>\n#include <stddef.h>\n#define _SIZE_T\n" + text
        for old, new in patches.get("include/" + rel.as_posix(), []):
            text = patched(text, old, new, rel)
        dest.write_text(text)
    for original in (decomp / "include").rglob("*.inc"):
        dest = inc / original.relative_to(decomp / "include")
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(original.read_bytes())
    for name in ['r3000.h', 'asm.h']:
        (inc / name).write_text((inc / 'psyq' / name).read_text(encoding='utf-8'))
    version = (inc / "decomp/version.h").read_text(encoding="utf-8")
    # PORT: Expand the pinned USA predicates before macro use (defined in a
    # macro expansion is undefined by the preprocessor standard).
    version = re.sub(r"defined\((VER_\w+)\)", lambda m: "1" if m[1] == "VER_USA" else "0", version)
    version = version.replace('defined(VER_##release)', 'SH_ITEMS_VERSION_##release').replace('defined(VERSION_##region)', 'SH_ITEMS_REGION_##region')
    version += '\n#define SH_ITEMS_VERSION_USA 1\n#define SH_ITEMS_VERSION_JAP0 0\n#define SH_ITEMS_VERSION_JAP1 0\n#define SH_ITEMS_VERSION_JAP2 0\n#define SH_ITEMS_VERSION_EUR 0\n#define SH_ITEMS_REGION_NTSC 1\n#define SH_ITEMS_REGION_NTSCJ 0\n#define SH_ITEMS_REGION_PAL 0\n'
    (inc / "decomp/version.h").write_text(version)
    # Upstream quoted includes use decomp's flat search directory.
    for file in (inc / "decomp").glob("*.h"):
        (inc / file.name).write_text(file.read_text(encoding="utf-8"))
    (out / "psyq").mkdir(exist_ok=True)
    prepare_gte(decomp, out)
    (inc / "psyq/inline_c.h").write_text((out / "psyq/inline_c.h").read_text(encoding="utf-8"))
    (inc / "inline_no_dmpsx.h").write_text((out / "inline_no_dmpsx.h").read_text(encoding="utf-8"))
    (inc / "gte_native.h").write_text((ROOT / "port/gte_native.h").read_text(encoding="utf-8"))
    (inc / "items_gpu.h").write_text((ROOT / "port/sys/items/items_gpu.h").read_text(encoding="utf-8"))
    (inc / 'items_imports.h').write_text((ROOT / 'port/sys/items/imports.h').read_text(encoding='utf-8'))
    (inc / 'items_aliases.h').write_text((ROOT / 'port/sys/items/items_aliases.h').read_text(encoding='utf-8'))
    manifest = []
    for name, path in SOURCES.items():
        text = (decomp / path).read_text(encoding="utf-8")
        manifest.append({"unit": name, "source": path, "sha256": hashlib.sha256(text.encode()).hexdigest(), "mode": "complete"})
        text = usa_only(fixed_words(text))
        # PORT: Static padding symbols only matched PS1 linker placement; the
        # native BSS aliases below are explicitly owned allocations.
        text = re.sub(r'^static const s32 __pad_rodata_\w+\s*=\s*0;[^\n]*', '// PORT: PS1 rodata padding has no native placement role.', text, flags=re.M)
        if name == 'mapscreen':
            text = re.sub(r'^\s*static [^;\n]*__pad_bss_800C444B[^;\n]*;[^\n]*', '// PORT: PS1 matching padding is unnecessary in native storage.', text, flags=re.M)
        if name == 'stf_roll':
            text = re.sub(r'static const (s16|s32) (D_801E2E(?:1E|20|24))', r'const \1 \2', text)
        if name == 'item_screens_3':
            # PORT: The PS1 source indexes across adjacent BSS symbols. Native
            # storage must make these aliases explicit, not just enlarge arrays.
            replacements = {
                'GsF_LIGHT g_Items_Lights[7][2];': 'GsF_LIGHT sh_items_lights[20];',
                'GsF_LIGHT D_800C3A88[4];': '', 'GsF_LIGHT D_800C3AC8[2];': '',
                'GsDOBJ2 g_Items_ItemsModelData[9];': 'GsDOBJ2 sh_items_models[10];',
                'GsDOBJ2 D_800C3E08;': '',
                's32 D_800C3E18[7];': 's32 sh_items_slot_indices[10];',
                's32 g_Inventory_EquippedItemIdx;': '', 's32 __pad_bss_800C3E38[2];': '',
            }
            for old, new in replacements.items():
                if old not in text:
                    raise ValueError('BSS alias source drift: ' + old)
                text = text.replace(old, new)
        if name in ('item_screens_3', 'item_unk_data'):
            # PORT: unsigned-byte sentinel initializers retain the PS1 0xff bits.
            text = re.sub(r'(^[^\n]*&D_800AD4C4[^\n]*$)', lambda m: m[0].replace(', -1,', ', (u8)-1,'), text, flags=re.M)
        for old, new in patches.get(path, []):
            text = patched(text, old, new, path)
        if name in ('saveload', 'stf_roll'):
            if name == 'stf_roll':
                for header in ('stringtable.h', 'widthtable.h'):
                    text = text.replace(f'#include "{header}"', (decomp / 'src/screens/credits' / header).read_text(encoding='utf-8'))
                # PORT: upstream BSS declarations have no C definition. Use
                # checked native storage, restored together with this overlay.
                text = text.replace('extern s_CreditTextState   g_CreditTextState;', 's_CreditTextState g_CreditTextState;')
                text = text.replace('extern s_CreditText3dState g_CreditText3dState;', 's_CreditText3dState g_CreditText3dState;')
            text, inventory = screen(text, name, out)
            expected = json.loads((ROOT / 'port/sys/items/overlay_inventory.json').read_text(encoding='utf-8'))[name]
            if inventory != expected:
                raise ValueError(f'{name} namespace/reset inventory drift; review every new object')
            manifest[-1]['overlay'] = inventory
        (out / (name + ".c")).write_text(NOTICE + '// PORT: explicit PS1 scalar narrowings; native addresses and SDK packet tokens.\n' + text)
    for path in ["src/bodyprog/items/item_rotations.h", "src/screens/credits/stringtable.h", "src/screens/credits/widthtable.h"]:
        (out / Path(path).name).write_text((decomp / path).read_text(encoding="utf-8"))
    puzzle_sources = json.loads((ROOT / 'port/sys/items/puzzle_sources.json').read_text(encoding='utf-8'))
    for path, names in puzzle_sources.items():
        original = (decomp / path).read_text(encoding='utf-8')
        unit = Path(path).stem + '_items'
        map_name = Path(path).parent.name
        header = f'maps/map{map_name[3]}/{map_name}.h'
        includes = ['game.h', 'bodyprog/bodyprog.h', 'bodyprog/dms.h', 'bodyprog/item_screens.h', 'bodyprog/player.h', 'bodyprog/math/math.h',
                    'bodyprog/gfx/map_effects.h', 'bodyprog/events/bodyprog_data_800A99B4.h', 'bodyprog/sound/sound_system.h', 'main/rng.h', header]
        code = usa_only(fixed_words(''.join(function(original, name) for name in names)))
        if map_name == 'map7_s02':
            helper = original[original.index('static inline D_800E9ED8_Set'):original.index('void func_800D97FC')]
            code = helper.replace('static inline D_800E9ED8_Set', 'static inline void D_800E9ED8_Set') + code
        # Preserve source-scope extern declarations used by selected event bodies.
        externs = '\n'.join(re.findall(r'^extern[^;]+;', original, re.M))
        constants = '\n'.join(decl for variable, decl in declarations(usa_only(original)) if re.match(r'(?:static )?const ', decl) and re.search(r'\b' + variable + r'\b', code))
        definitions = '\n'.join(re.findall(r'^#define\s+(?!MAP_MESSAGES)[^\n]+', original, re.M))
        symbols = set(names) | map_symbols(decomp, header, code)
        symbols.update(variable for variable, _ in declarations(constants))
        # Map-owned extern objects have separate namespaces; core globals in
        # shared.h are imports. Do not prefix those shared engine identities.
        namespace = NOTICE + '\n'.join(f'#define {name} sh_items_{map_name}_{name}' for name in sorted(symbols)) + '\n'
        (out / (unit + '_namespace.h')).write_text(namespace)
        code = NOTICE + f'#define {map_name.upper()}\n#include "{unit}_namespace.h"\n' + ''.join(f'#include "{h}"\n' for h in includes) + '\n#include "items_imports.h"\n' + externs + '\n' + constants + '\n' + definitions + '\n' + code
        for old, new in patches.get('slice/' + path, []):
            code = patched(code, old, new, path)
        code = event_native(code)
        (out / (unit + '.c')).write_text(code)
        manifest.append({'unit': unit, 'source': path, 'mode': 'selected event functions', 'functions': names, 'sha256': hashlib.sha256(original.encode()).hexdigest()})
    # Compile both original variants of the 27-button light panel and elevator.
    for map_name in ['map3_s01', 'map3_s03', 'map3_s04', 'map3_s05', 'map7_s01', 'map7_s02']:
        headers = ['sharedFunc_800D15F0_3_s01.h']
        if map_name in ('map7_s01', 'map7_s02'):
            headers.append('sharedFunc_800DB60C_7_s01.h')
        for shared in headers:
            unit = map_name + '_' + Path(shared).stem + '_items'
            code = (decomp / 'include/maps/shared' / shared).read_text(encoding='utf-8')
            header = f'maps/map{map_name[3]}/{map_name}.h'
            symbols = map_symbols(decomp, header, code)
            namespace = NOTICE + '\n'.join(f'#define {name} sh_items_{map_name}_{name}' for name in sorted(symbols)) + '\n'
            (out / (unit + '_namespace.h')).write_text(namespace)
            code = NOTICE + f'#include "{unit}_namespace.h"\n' + '#include "game.h"\n#include "bodyprog/bodyprog.h"\n#include "bodyprog/item_screens.h"\n#include "bodyprog/sound/sound_system.h"\n#include "bodyprog/math/math.h"\n#include "items_imports.h"\n' + f'#define {map_name.upper()}\n#include "{header}"\n' + code
            for old, new in patches.get('shared/' + unit, []):
                code = patched(code, old, new, unit)
            code = event_native(code)
            (out / (unit + '.c')).write_text(code)
            manifest.append({'unit': unit, 'source': 'include/maps/shared/' + shared, 'mode': map_name + ' instantiation'})
    (out / "LICENSE.decomp").write_text((decomp / "LICENSE").read_text(encoding="utf-8"))
    (out / "manifest.json").write_text(json.dumps({"pin": PIN, "sources": manifest}, indent=2))
    (out / 'units.txt').write_text('\n'.join(s['unit'] for s in manifest))
    tables = json.loads((ROOT / 'port/sys/items/data_tables.json').read_text(encoding='utf-8'))
    table_source = ['pub const DISC_TABLES: &[(&str,u32,u32,usize,&str)] = &[']
    for table in tables:
        table_source.append(f'("{table["file"]}",{table["base"]}u32,{table["address"]}u32,{table["count"]},"{table["kind"]}"),')
    (out / 'disc_tables.rs').write_text('\n'.join(table_source) + '\n];\n')
    # The headless link retains exact prepared function bodies and compiled
    # reset images. Unrelated renderer/world dependencies belong to core and
    # are compile-gated above; no success-returning substitutes are generated.
    headless = [NOTICE, '#include "game.h"\n#include "bodyprog/bodyprog.h"\n#include "bodyprog/item_screens.h"\n#include "bodyprog/player.h"\n#include "bodyprog/math/math.h"\n#include "bodyprog/memcard.h"\n#include "bodyprog/ranking.h"\n#include "bodyprog/text/text_draw.h"\n#include "screens/credits/credits.h"\n#include "saveload_namespace.h"\n#include "screens/saveload.h"\n#include "stf_roll_namespace.h"\n#include "map1_s01_items_namespace.h"\n#include "maps/map1/map1_s01.h"\n#include "abi.h"\n#include "bodyprog/events/npc_main.h"\n#include "bodyprog/gfx/map_effects.h"\n#include "bodyprog/sound/sound_system.h"\nvoid Game_TimerUpdate(void);\n']
    for unit, names in {
        'item_screens_2': ['Inventory_ItemUse', 'Player_ItemRemove', 'func_8004F190', 'func_8004EF48'],
        'item_screens_3': ['Inventory_AddSpecialItem'],
        'item_utils': ['Game_TimerUpdate', 'Inventory_HyperBlasterUnlockTest', 'Game_HyperBlasterBeamColorGet'],
        'map1_s01_items': ['PianoPuzzle_Control'],
        'saveload': ['SaveScreen_SaveGame', 'SaveScreen_LoadSave', 'sh_saveload_reset', 'sh_saveload_reset_probe'],
        'stf_roll': ['sh_stf_roll_reset', 'sh_stf_roll_reset_probe'],
        'ranking': ['Ranking_EvaluateScore', 'Ranking_PrepareSavegame', 'func_8009146C'],
        'save_menu': ['WrapIdx', 'MemCard_FilesDamagedCheck', 'MemCard_ElementsUpdate'],
    }.items():
        source = (out / (unit + '.c')).read_text(encoding='utf-8')
        for variable, declaration in declarations(source):
            if (unit in ('saveload', 'stf_roll', 'ranking', 'save_menu') and '(*const ' not in declaration) or variable in ['D_80025EB0', 'sh_items_lights', 'sh_items_models', 'sh_items_slot_indices']:
                headless.append(declaration + '\n')
        for name in names:
            headless.append(function(source, name))
    (out / 'headless_original.c').write_text(''.join(headless))


def check_clang(out, compiler):
    if not compiler.is_file():
        print('SKIP: clang frontend is unavailable; no tools installed.')
        return 0
    # Match core's freestanding arm64 frontend gate. These CRT declarations
    # are compile-only; no generated shim is used by runtime/Apple SDK builds.
    layout = out / 'clang-layout'
    layout.mkdir(exist_ok=True)
    for file in (ROOT / 'tools/layout-include').glob('*.h'):
        (layout / file.name).write_bytes(file.read_bytes())
    with (layout / 'string.h').open('a') as f:
        f.write('\nvoid* memmove(void*,const void*,size_t);\n')
    (layout / 'memory.h').write_text('#include <string.h>\n')
    with (layout / 'stdlib.h').open('a') as f:
        f.write('\n_Noreturn void abort(void); // PORT: compile-only native CRT declaration.\n')
    (layout / 'limits.h').write_text('// PORT: compile-only fixed scalar limits.\n#define UINT_MAX 0xffffffffu\n#define INT_MAX 2147483647\n#define INT_MIN (-2147483647-1)\n#define SHRT_MAX 32767\n#define SHRT_MIN (-32767-1)\n#define UCHAR_MAX 255\n')
    flags = ['-target', 'aarch64-apple-ios15.0', '-ffreestanding', '-std=c11', '-Wall', '-Wextra', '-Werror', '-nostdinc', '-DVER_USA', '-DSKIP_ASM']
    for path in [layout, out / 'include', out, ROOT / 'port/sys/items']:
        flags += ['-I', str(path)]
    for name in ['ccos', 'csin', 'csqrt', 'catan']:
        flags.append('-fno-builtin-' + name)
    sources = [out / (name + '.c') for name in (out / 'units.txt').read_text().splitlines()]
    sources += [ROOT / 'port/sys/items/abi.c', ROOT / 'port/sys/items/save_backend.c']
    results = {}
    for source in sources:
        result = subprocess.run([str(compiler), str(source), '--checks=-*,clang-diagnostic-*,clang-analyzer-core.DivideZero', '--warnings-as-errors=*', '--', *flags], capture_output=True, text=True)
        (out / (source.stem + '.clang.log')).write_text(result.stdout + result.stderr)
        results[source.name] = result.returncode
        print(('PASS ' if result.returncode == 0 else 'FAIL ') + source.name, flush=True)
    (out / 'clang-results.json').write_text(json.dumps(results, indent=2))
    return int(any(results.values()))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--decomp", type=Path, default=ROOT / "game/decomp")
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument('--clang-compiler', type=Path)
    args = parser.parse_args()
    prepare(args.decomp, args.out)
    if args.clang_compiler:
        raise SystemExit(check_clang(args.out, args.clang_compiler))
