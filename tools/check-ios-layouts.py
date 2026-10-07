"""Compile-only arm64/LP64 gate. clang-tidy uses its embedded compiler frontend.
No tools are downloaded or installed; no game data is read. Negative control
proves the assertions execute rather than silently skipping the translation unit.
"""
from pathlib import Path
import argparse
import subprocess
from prepare_sdk import prepare


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--compiler", default="clang")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    out = root / "target" / "core-layouts"
    prepare(root / "game/decomp", out / "sdk")
    positive = root / "port/layout_check.c"
    negative = out / "negative-ios.c"
    negative.write_text('#include "layout_check.c"\n_Static_assert(sizeof(void*) == 4, "negative control must fail on arm64");\n')
    flags = ["-target", "aarch64-apple-ios15.0", "-ffreestanding", "-std=c11", "-Wall", "-Wextra", "-Werror",
             "-DSH_CHECK_BOOT_LAYOUT", "-DSH_CHECK_IOS_LAYOUT"]
    for include in (root / "port", root / "port/include", out / "sdk", root / "game/decomp/include"):
        flags.extend(["-I", str(include)])
    if Path(args.compiler).name.startswith("clang-tidy"):
        # VS bundles a frontend but no clang resource headers. Use target builtin
        # type declarations for this compile-only check, not host SDK types.
        flags.extend(["-nostdinc", "-I", str(root / "tools/layout-include")])

    def check(source):
        if Path(args.compiler).name.startswith("clang-tidy"):
            command = [args.compiler, str(source), "--checks=-*,clang-analyzer-core.DivideZero", "--warnings-as-errors=*", "--", *flags]
        else:
            command = [args.compiler, *flags, "-fsyntax-only", str(source)]
        return subprocess.run(command, capture_output=True, text=True)

    passed = check(positive)
    if passed.returncode:
        raise SystemExit(passed.stdout + passed.stderr)
    rejected = check(negative)
    if rejected.returncode == 0 or "negative control must fail" not in rejected.stdout + rejected.stderr:
        raise SystemExit("FAIL: compiler did not reject the negative control\n" + rejected.stdout + rejected.stderr)
    print("PASS: aarch64-apple-ios15.0 wire/native/SDK layouts; LP64 and 64-bit pointers asserted; negative control rejected.")


if __name__ == "__main__":
    main()
