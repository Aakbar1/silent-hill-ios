"""Generate pinned GPL OPTION source with an explicit writable-state inventory.

No game image is read. Local statics become separate native objects so activation
can restore their compiled initial values, including initialized GPU packets.
"""
from pathlib import Path
import argparse
import re


def prepare(decomp: Path, output: Path):
    source = (decomp / "src/screens/options/options.c").read_text()
    functions = list(re.finditer(r"^(?:void|s32|bool)\s+(\w+)\([^\n]*\)\s*(?://[^\n]*)?\n\{", source, re.M))
    owned = [m[1] for m in functions]
    globals_ = re.findall(r"^(?:static )?(?:s32|bool)\s+(\w+)\s*(?:= [^;]+)?;", source, re.M)
    owned += globals_ + ["D_801E2D42"]
    declarations, resets, probes = [], [], []
    for index in range(len(functions) - 1, -1, -1):
        match = functions[index]
        end = functions[index + 1].start() if index + 1 < len(functions) else len(source)
        body = source[match.start():end]
        pattern = r"    static (DVECTOR|s32|s16|s_ControllerMenu_SelectedEntries|DR_MODE|POLY_G4) (\w+)(\[[^\]]+\])?(\s*=\s*\{.*?\})?;"
        for local in list(re.finditer(pattern, body, re.S)):
            typ, name, array, init = local.groups()
            native = f"sh_option_{match[1]}_{name}"
            array = array or ""
            value = (init or "= {0}").split("=", 1)[1].strip()
            if not array and typ in ("s32", "s16"):
                value = "0"
            declarations += [f"static {typ} {native}{array} = {value};", f"static const {typ} {native}_initial{array} = {value};"]
            resets += [f"memcpy(&{native}, &{native}_initial, sizeof({native}));"]
            probes += [f"memset(&{native}, 0xa5, sizeof({native}));"]
            body = body.replace(local[0], "")
            body = re.sub(rf"\b{re.escape(name)}\b", native, body)
        source = source[:match.start()] + body + source[end:]
    for name in globals_:
        resets += [f"{name} = 0;"]
        probes += [f"{name} = 1;"]
    assert len(globals_) == 10 and len(resets) == 24, "pinned OPTION writable inventory changed"
    # Both local string tables are read-only by use; make pointer-array constness explicit.
    source = source.replace("static const char* ", "static const char* const ")
    namespace = "/* SPDX-License-Identifier: GPL-3.0-only; derived from silent-hill-decomp. */\n" + "\n".join(f"#define {name} sh_option_{name}" for name in owned) + "\nvoid sh_option_reset(void);\nint sh_option_reset_probe(void);\nint sh_option_selected_entry(void);\n"
    (output / "option_namespace.h").write_text(namespace)
    source = re.sub(r"^#include[^\n]*", "", source, flags=re.M)
    # PORT: Fixed destinations, pointer arithmetic and explicit PS1 narrowings.
    source = source.replace("ot     = &g_OtTags", "ot     = (GsOT*)&g_OtTags")
    source = source.replace("s_Line2d* localLine;", "const s_Line2d* localLine;")
    # PORT: Preserve direction 1/2 borders; no direction must not index arrow -1.
    source = source.replace("for (i = dir - 1; i < dir; i++)",
        "// PORT: No held direction has no border arrow; avoid BORDER_ARROWS[-1].\n    if (dir != 0) for (i = dir - 1; i < dir; i++)")
    source = source.replace("interpAlpha = Math_Sin(", "interpAlpha = (s16)Math_Sin(")
    source = source.replace("false, g_GameWork.config.volumeBgm", "false, (u8)g_GameWork.config.volumeBgm")
    source = source.replace("true, g_GameWork.config.volumeSe", "true, (u8)g_GameWork.config.volumeSe")
    source = source.replace("Text_Debug_PositionSet(96, strYPos)", "Text_Debug_PositionSet(96, (s16)strYPos)")
    # Explicit native narrowing retains the original PS1 coordinate/color bits.
    narrow16 = r"(\b(?:quadVerts\[[^;]+?\]\.vy|sh_option_Options_ScreenPosMenu_Control_screenPosMenu_Position[XY]|highlightY[01])\s*=\s*)([^;]+);"
    source = re.sub(narrow16, lambda m: m[1] + "(s16)(" + m[2] + ");", source)
    source = re.sub(r"(\bposX\s*=\s*)(g_GameWorkConst->config.screenPositionX);", r"\1(s8)\2;", source)
    source = re.sub(r"(g_GameWork.background2dColor\.[rgb]\s*=\s*)([^;]+);", lambda m: m[1] + "(u8)(" + m[2] + ");", source)
    generated = "/* SPDX-License-Identifier: GPL-3.0-only; derived from silent-hill-decomp. */\n#include \"boot.h\"\n" + "\n".join(declarations) + "\n" + source
    generated += "\nvoid sh_option_reset(void) {\n" + "\n".join(resets) + "\n}\n"
    comparisons = [f"memcmp(&{line.split()[2].split('[')[0]}, &{line.split()[2].split('[')[0]}_initial, sizeof({line.split()[2].split('[')[0]})) == 0" for line in declarations if not line.startswith("static const")]
    generated += "int sh_option_reset_probe(void) {\n" + "\n".join(probes) + "\nsh_option_reset();\nreturn " + " && ".join(comparisons + [f"{name}==0" for name in globals_]) + ";\n}\n"
    generated += "int sh_option_selected_entry(void) { return g_MainOptionsMenu_SelectedEntry; }\n"
    (output / "option.c").write_text(generated)
    return len(globals_), len(resets) - len(globals_)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--decomp", type=Path, default=Path("game/decomp"))
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    print("OPTION writable inventory: globals={}, locals={}".format(*prepare(args.decomp, args.out)))
