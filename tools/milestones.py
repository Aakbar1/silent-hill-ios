"""Headless milestone gate. Logs and screenshots always stay outside the repo."""
from pathlib import Path
from datetime import datetime, timezone
import argparse
import hashlib
import json
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--available", action="store_true", help="run implemented milestones; does not certify the brief's pass")
    parser.add_argument("--repeat", type=int, default=1, choices=range(1, 4), help="compare PNG hashes between isolated runs")
    parser.add_argument("--disc", type=Path)
    parser.add_argument("--only", nargs="+", help="debug a named subset of available milestones")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    replays = root / "docs/core/replays"
    manifest = json.loads((replays / "milestones.json").read_text())
    if manifest["version"] != 1:
        raise SystemExit("unsupported milestone manifest version")
    if manifest["pending"] and not args.available:
        print("FAIL: full brief milestone gate is incomplete:", flush=True)
        for pending in manifest["pending"]:
            print(" - " + pending, flush=True)
        print("Run --available to verify completed work; it cannot certify first-map walking.", flush=True)
        return 2
    cases = manifest["verified"]
    if args.only:
        unknown = set(args.only) - {case["name"] for case in cases}
        if unknown:
            raise SystemExit("unknown milestones: " + ", ".join(sorted(unknown)))
        cases = [case for case in cases if case["name"] in args.only]
    private = root.parent.parent / "private/work/core4"
    output = private / "milestones" / datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    output.mkdir(parents=True)
    executable = root / "target/release/silent-hill-boot.exe"
    flags = {"state":"--expect-state", "step":"--expect-step", "menu":"--expect-menu", "option_entry":"--expect-option-entry", "movie_skips":"--expect-movie-skips", "min_movie_frames":"--min-movie-frames", "min_lit_pixels":"--min-lit-pixels"}
    results = []
    passed = 0
    for case in cases:
        first_hash = None
        for repeat in range(args.repeat):
            stem = case["name"] + f"-{repeat+1}"
            screenshot = output / (stem + ".png")
            command = [str(executable), "--headless", "--audio", "off", "--frames", str(case["frames"]), "--input", str(replays / case["input"]), "--screenshot", str(screenshot)]
            if args.disc:
                command += ["--disc", str(args.disc.resolve())]
            for key, flag in flags.items():
                if key in case:
                    command += [flag, str(case[key])]
            try:
                run = subprocess.run(command, cwd=root, capture_output=True, text=True, timeout=180)
            except subprocess.TimeoutExpired as error:
                def decoded(value):
                    return value.decode(errors="replace") if isinstance(value, bytes) else value or ""
                run = subprocess.CompletedProcess(command, -1, decoded(error.stdout), decoded(error.stderr) + "\nFAIL: native replay timed out after 180 seconds\n")
            (output / (stem + ".log")).write_text(run.stdout + run.stderr)
            checkpoint = [line for line in run.stdout.splitlines() if line.startswith("CHECK ")]
            digest = hashlib.sha256(screenshot.read_bytes()).hexdigest() if screenshot.exists() else None
            ok = run.returncode == 0 and len(checkpoint) == 1 and digest is not None
            if repeat and digest != first_hash:
                ok = False
            first_hash = first_hash or digest
            results.append({"case":case["name"], "run":repeat+1, "exit":run.returncode, "checkpoint":checkpoint, "png_sha256":digest, "pass":ok})
            print(f"{'PASS' if ok else 'FAIL'} {stem}: " + (checkpoint[0] if checkpoint else "no checkpoint"), flush=True)
            if not ok:
                (output / "results.json").write_text(json.dumps(results, indent=2))
                print("Evidence: " + str(output), flush=True)
                return 1
            passed += 1
    (output / "results.json").write_text(json.dumps(results, indent=2))
    print(f"PASS available suite: {passed} runs; {len(cases) if args.repeat > 1 else 0} deterministic PNG comparisons. Full brief remains incomplete.", flush=True)
    print("Evidence: " + str(output), flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
