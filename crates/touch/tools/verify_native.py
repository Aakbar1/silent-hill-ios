"""Actual C game touch milestones; no synthesized pad/context replay allowed."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--exe", type=Path, required=True)
    parser.add_argument("--disc", type=Path, required=True)
    parser.add_argument("--repeat", type=int, default=2, choices=range(1, 4))
    args = parser.parse_args()
    crate = Path(__file__).resolve().parents[1]
    root = crate.parents[1]
    manifest = json.loads((crate / "milestones.json").read_text())
    if manifest["version"] != 1:
        raise SystemExit("unsupported touch milestone manifest")
    private = root.parents[1] / "private/work/touchwire"
    output = private / "milestones" / datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    output.mkdir(parents=True)
    results = []
    for case in manifest["cases"]:
        first_hash = None
        for repeat in range(args.repeat):
            stem = case["name"] + f"-{repeat + 1}"
            capture = output / (stem + ".png")
            command = [str(args.exe.resolve()), "--disc", str(args.disc.resolve()),
                       "--input", str(crate / "tests/replays" / case["input"]),
                       "--frames", str(case["frames"]), "--screenshot", str(capture),
                       "--min-lit-pixels", "1000", "--expect-state", str(case["state"]),
                       "--expect-step", str(case["step"]), "--expect-option-entry", str(case["option_entry"])]
            # Windowed until core3 supplies backend injection for headless runs.
            try:
                run = subprocess.run(command, cwd=root, capture_output=True, text=True, timeout=120)
            except subprocess.TimeoutExpired:
                raise SystemExit(f"FAIL {stem}: native replay exceeded 120 seconds")
            log = run.stdout + run.stderr
            (output / (stem + ".log")).write_text(log)
            digest = hashlib.sha256(capture.read_bytes()).hexdigest() if capture.exists() else None
            records = [line for line in log.splitlines() if line.startswith("TOUCH_CHECK ")]
            expected = sum(1 for line in (crate / "tests/replays" / case["input"]).read_text().splitlines()
                           if json.loads(line)["type"] == "check")
            ok = run.returncode == 0 and len(records) == expected and all(" PASS " in r for r in records)
            ok = ok and "TOUCH_RESULT PASS" in log and digest is not None and (repeat == 0 or digest == first_hash)
            first_hash = first_hash or digest
            results.append({"case": case["name"], "run": repeat + 1, "exit": run.returncode,
                            "checks": records, "png_sha256": digest, "pass": ok})
            (output / "results.json").write_text(json.dumps(results, indent=2))
            print(f"{'PASS' if ok else 'FAIL'} {stem}: {len(records)}/{expected} checks", flush=True)
            if not ok:
                print(log[-4000:], flush=True)
                print("Evidence: " + str(output), flush=True)
                return 1
    print(f"PASS {len(results)} native touch runs, {len(manifest['cases']) if args.repeat > 1 else 0} PNG comparisons", flush=True)
    print("Evidence: " + str(output), flush=True)
    print("Pending integration: " + "; ".join(manifest["pending"]), flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
