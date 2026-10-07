"""Compile a private three-way audit without editing host/ or copying game data into Git.

All output (including the compiled source snapshot) is confined to --private-root.
Use --reference to test the proposed software shim and write host-raster.patch.
Optional --trace reads gpuwire's private kind/count/words trace, not PSXCAP1.
"""
import argparse
import difflib
import json
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--private-root", type=Path, required=True)
    parser.add_argument("--reference", action="store_true")
    parser.add_argument("--trace", type=Path)
    args = parser.parse_args()
    crate = Path(__file__).resolve().parents[1]
    root = crate.parents[1]
    private = args.private_root.resolve()
    if private.name != "parity" or private.parent.name != "work" or private.parent.parent.name != "private":
        raise ValueError("expected private/work/parity evidence root")
    if args.trace and not args.trace.resolve().is_relative_to(private.parent.parent):
        raise ValueError("game traces must stay in private/")
    name = "reference" if args.reference else "legacy"
    output = private / ("host-audit-" + name)
    output.mkdir(parents=True, exist_ok=True)
    original = (root / "host/src/raster.rs").read_text()
    proposed = (crate / "tools/host_reference.rs").read_text()
    # Keep the existing host regression tests in the reviewable patch.
    tests = original[original.index("#[cfg(test)]"):]
    proposed += "\n" + tests
    raster = output / "raster.rs"
    raster.write_text(proposed if args.reference else original)
    if args.reference:
        patch = "".join(difflib.unified_diff(
            original.splitlines(True), proposed.splitlines(True), fromfile="a/host/src/raster.rs", tofile="b/host/src/raster.rs"))
        adapter = (root / "host/src/gpu_wgpu.rs").read_text()
        old = '''        // The boot fallback samples polygons at half-pixel centers, omits dither,
        // and uses different line rounding/endpoints. It is not a PS1 golden.
        assert_ne!(actual, legacy.read([0, 0, 64, 64]));'''
        new = '''        // Both host backends now use the documented PS1 rules at native resolution.
        assert_eq!(actual, legacy.read([0, 0, 64, 64]));'''
        if adapter.count(old) != 1:
            raise ValueError("host parity assertion changed; review core's current test before writing patch")
        revised = adapter.replace(old, new)
        patch += "".join(difflib.unified_diff(adapter.splitlines(True), revised.splitlines(True),
                                            fromfile="a/host/src/gpu_wgpu.rs", tofile="b/host/src/gpu_wgpu.rs"))
        (private / "host-raster.patch").write_text(patch)
    header = "\n".join(f'#[path = {json.dumps(str(p))}] mod {module};' for p, module in [
        (raster, "host_raster"), (crate / "tests/support/parity_cases.rs", "fixtures")])
    # Audit harness also exposes methods used only by the unchanged host tests.
    (output / "main.rs").write_text("#![allow(dead_code)]\n" + header + "\n" + (crate / "tools/host_audit.rs").read_text())
    (output / "Cargo.toml").write_text(f'''[package]
name = "parity-host-audit-{name}"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
psxgpu = {{ path = {json.dumps(str(crate))} }}
[[bin]]
name = "host-audit"
path = "main.rs"
''')
    import os
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(crate / "target")
    if args.reference:
        env["AUDIT_REQUIRE_HOST_EXACT"] = "1"
    command = ["cargo", "run", "--release", "--manifest-path", str(output / "Cargo.toml"), "--"]
    if args.trace:
        command += [str(args.trace.resolve()), str(private / (args.trace.stem + "-" + name))]
    run = subprocess.run(command, env=env, capture_output=True, text=True)
    (private / ("host-audit-" + name + ("-" + args.trace.stem if args.trace else "") + ".log")).write_text(run.stdout + run.stderr)
    print(run.stdout + run.stderr)
    return run.returncode


if __name__ == "__main__":
    raise SystemExit(main())
