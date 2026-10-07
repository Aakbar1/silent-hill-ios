#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Emit a synthetic-data C probe of the actual generated casts/poll conditions.

Compile the result with MSVC /std:c11 /W4 /WX and run it. This does not compile
all of libsd or prove its native ABI; that remains core's integration check.
"""
import argparse
import re
from pathlib import Path
from prepare_libsd import transform


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--decomp", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    expressions, conditions = [], []
    for name in ("smf_io.c", "smf_snd.c"):
        text = transform(name, (args.decomp / "src/bodyprog/libsd" / name).read_text(encoding="utf-8"))
        expressions.extend(re.findall(r"addr_p = (\(u16\*\)\(\(u8\*\)[^;\n]+)", text))
        conditions.extend(re.findall(r"while \((SpuGetKeyStatus\(spu_ch_tbl\[(?:vo|voice)\]\) == (?:0|SPU_OFF))\);", text))
    assert len(expressions) == 2 and len(conditions) == 3
    probe = r'''/* SPDX-License-Identifier: GPL-3.0-only; source expressions derived from silent-hill-decomp. */
#include <stdint.h>
#include <stdio.h>
typedef uint8_t u8;
typedef uint16_t u16;
typedef uint32_t u32;
typedef struct { struct { int ps; } vab_h; } Header;
static u32 spu_ch_tbl[24];
static u32 queried;
enum { SPU_OFF = 0 };
static int SpuGetKeyStatus(u32 mask) { queried=mask; return mask == (1u<<23) ? 1 : -1; }
int main(void) {
    u16 storage[2048] = {0};
    struct { void* vh_addr_4; } vab_h[1] = {{storage}};
    Header header = {{0}};
    Header* vab0 = &header;
    Header* sd_vh = &header;
    int vabId = 0, vabid = 0, vo = 23, voice = 23;
    u16* addr_p;
    storage[1040]=513; storage[1041]=1027;
    spu_ch_tbl[23]=1u<<23;
'''
    for expression in expressions:
        probe += f"    addr_p = {expression};\n"
        probe += "    if (*addr_p != 513 || *(addr_p+1) != 1027) return 1;\n"
    for condition in conditions:
        probe += f"    if ({condition}) return 2;\n"
        probe += "    if (queried != (1u<<23)) return 3;\n"
    probe += '    puts("PASS: two u16 size-table casts; three voice-mask polls");\n    return 0;\n}\n'
    args.out.write_text(probe, encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()
