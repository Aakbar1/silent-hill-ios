"""Generate fixed-width PsyQ headers from pinned GPL source; retain its notices."""
from pathlib import Path
import argparse
import re


def prepare(decomp: Path, output: Path):
    (output / "psyq").mkdir(parents=True, exist_ok=True)
    for name in ("libgte.h", "libgpu.h"):
        text = (decomp / "include" / "psyq" / name).read_text()
        # PORT: PsyQ long is a 32-bit word, including on LP64 iOS.
        (output / "psyq" / name).write_text(re.sub(r"\blong\b", "int32_t", text))


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--decomp", type=Path, default=Path("game/decomp"))
    parser.add_argument("--out", type=Path, default=Path("target/core-layouts/sdk"))
    args = parser.parse_args()
    prepare(args.decomp, args.out)
