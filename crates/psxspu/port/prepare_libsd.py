#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Normalize three polling calls and two casts in GENERATED libsd C only.

Run before core's native-header/pointer transforms. The pinned decomp stays
read-only. This is deliberately strict: upstream drift must fail the build.
"""
import argparse
from pathlib import Path


EDITS = {
    "smf_io.c": [
        (
            "addr_p = (u8*)vab_h[vabId].vh_addr_4 + (vab0->vab_h.ps * 512) + 2080",
            "addr_p = (u16*)((u8*)vab_h[vabId].vh_addr_4 + (vab0->vab_h.ps * 512) + 2080)",
        ),
        (
            "while (SpuGetKeyStatus(spu_ch_tbl[vo] == 1) == 0);",
            "// PORT: Poll the actual voice mask; retry key-on while OFF.\n"
            "                while (SpuGetKeyStatus(spu_ch_tbl[vo]) == 0);",
        ),
    ],
    "smf_snd.c": [
        (
            "addr_p = (u8*)vab_h[vabid].vh_addr_4 + (sd_vh->vab_h.ps << 9) + 0x820;",
            "// PORT: Preserve the byte offset and the declared u16 sample-size stride.\n"
            "            addr_p = (u16*)((u8*)vab_h[vabid].vh_addr_4 + (sd_vh->vab_h.ps << 9) + 0x820);",
        ),
        (
            "while (SpuGetKeyStatus(spu_ch_tbl[vo] == 1) == 0);",
            "// PORT: Poll the actual voice mask; retry key-on while OFF.\n"
            "            while (SpuGetKeyStatus(spu_ch_tbl[vo]) == 0);",
        ),
        (
            "while (SpuGetKeyStatus(spu_ch_tbl[voice] == 1) == SPU_OFF);",
            "// PORT: Poll the actual voice mask; retry key-on while OFF.\n"
            "        while (SpuGetKeyStatus(spu_ch_tbl[voice]) == SPU_OFF);",
        ),
    ],
}


def transform(name: str, text: str) -> str:
    for before, after in EDITS[name]:
        count = text.count(before)
        if count != 1:
            raise ValueError(f"{name}: expected one pinned occurrence, found {count}: {before}")
        text = text.replace(before, after)
    if name == "smf_io.c":
        # The assignment is in a for initializer; put the notice above it.
        text = text.replace(
            "for (i = 0, vag_addr = 0, addr_p = (u16*)",
            "// PORT: Preserve the byte offset and the declared u16 sample-size stride.\n"
            "            for (i = 0, vag_addr = 0, addr_p = (u16*)",
        )
    return text


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--decomp", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    source = (args.decomp / "src/bodyprog/libsd").resolve()
    output = args.out.resolve()
    if output == source or source in output.parents or args.decomp.resolve() in output.parents:
        parser.error("--out must be outside the read-only decomp")
    # Validate both sources before publishing either generated copy.
    files = {name: transform(name, (source / name).read_text(encoding="utf-8")) for name in EDITS}
    output.mkdir(parents=True, exist_ok=True)
    for name, text in files.items():
        (output / name).write_text(text, encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()
