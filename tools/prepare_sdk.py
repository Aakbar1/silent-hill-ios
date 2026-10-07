"""Generate fixed-width PsyQ headers from pinned GPL source; retain its notices."""
from pathlib import Path
import argparse
import re
from prepare_option import prepare as prepare_option
from prepare_gte import generate as prepare_gte
from prepare_gameplay import generate as prepare_gameplay
from prepare_maps import prepare as prepare_maps


def prepare(decomp: Path, output: Path):
    (output / "psyq").mkdir(parents=True, exist_ok=True)
    for name in ("libgte.h", "libgpu.h"):
        text = (decomp / "include" / "psyq" / name).read_text()
        # PORT: PsyQ long is a 32-bit word, including on LP64 iOS.
        (output / "psyq" / name).write_text(re.sub(r"\blong\b", "int32_t", text))
    def section(path, start, end):
        text = (decomp / path).read_text()
        first = text.index(start)
        return text[first:text.index(end, first)]
    joy = section("include/bodyprog/sys/joy.h", "/** @brief PSX controller input flags.", "extern s_ControllerData* const g_Controller0;")
    save = section("include/bodyprog/savegame.h", "/** @brief Savegame data.", "/** @brief User options configuration.")
    font = section("include/bodyprog/text/text_draw.h", "#define MAP_MSG_CODE_MARKER", "// ====================")
    (output / "game_records.h").write_text("/* SPDX-License-Identifier: GPL-3.0-only; derived from silent-hill-decomp. */\n#define INV_ITEM_COUNT_MAX 40\n#define Chara_Count 45\ntypedef u32 q20_12;\ntypedef struct {u8 id,count,command,field_3;} s_InventoryItem;\n" + joy + "\n" + save + "\n" + font + "\n")
    gpu = section("include/gpu.h", "/** @brief 2D screen-space line.", "/** @brief Primitive color.")
    (output / "gpu_records.h").write_text("/* SPDX-License-Identifier: GPL-3.0-only; derived from silent-hill-decomp. */\n#define RECT_VERT_COUNT 4\n" + gpu)
    prepare_option(decomp, output)
    prepare_gte(decomp, output)
    prepare_gameplay(decomp, output)
    prepare_maps(decomp, output)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--decomp", type=Path, default=Path("game/decomp"))
    parser.add_argument("--out", type=Path, default=Path("target/core-layouts/sdk"))
    args = parser.parse_args()
    prepare(args.decomp, args.out)
