"""Compare the existing host milestones at 1x; keep all game evidence private.

Only the Python standard library is needed. The PNG decoder accepts the RGB8,
noninterlaced screenshots emitted by the host, including all five PNG filters.
"""
import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
import struct
import subprocess
import zlib


def rgb_png(path):
    data = path.read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("invalid PNG signature")
    compressed = bytearray()
    position = 8
    width = height = None
    while position < len(data):
        length = struct.unpack_from(">I", data, position)[0]
        tag = data[position + 4:position + 8]
        chunk = data[position + 8:position + 8 + length]
        if tag == b"IHDR":
            width, height, depth, color, compression, filtering, interlace = struct.unpack(">IIBBBBB", chunk)
            if (depth, color, compression, filtering, interlace) != (8, 2, 0, 0, 0):
                raise ValueError("expected RGB8 noninterlaced host screenshot")
        elif tag == b"IDAT":
            compressed.extend(chunk)
        elif tag == b"IEND":
            break
        position += length + 12
    if not width or not height:
        raise ValueError("missing PNG dimensions")
    raw = zlib.decompress(compressed)
    stride = width * 3
    if len(raw) != (stride + 1) * height:
        raise ValueError("unexpected PNG decompressed length")
    pixels = bytearray()
    previous = bytearray(stride)
    for y in range(height):
        offset = y * (stride + 1)
        filter_type = raw[offset]
        row = bytearray(raw[offset + 1:offset + 1 + stride])
        for i in range(stride):
            left = row[i - 3] if i >= 3 else 0
            above = previous[i]
            upper_left = previous[i - 3] if i >= 3 else 0
            if filter_type == 0:
                predictor = 0
            elif filter_type == 1:
                predictor = left
            elif filter_type == 2:
                predictor = above
            elif filter_type == 3:
                predictor = (left + above) // 2
            elif filter_type == 4:
                p = left + above - upper_left
                distances = [abs(p - value) for value in [left, above, upper_left]]
                predictor = [left, above, upper_left][distances.index(min(distances))]
            else:
                raise ValueError("invalid PNG filter")
            row[i] = (row[i] + predictor) & 255
        pixels.extend(row)
        previous = row
    return width, height, pixels


def compare(soft, accelerated, tolerance):
    sw, sh, source = rgb_png(soft)
    gw, gh, target = rgb_png(accelerated)
    if (sw, sh) != (gw, gh):
        return {"pass": False, "soft_size": [sw, sh], "wgpu_size": [gw, gh]}
    deltas = [abs(a - b) for a, b in zip(source, target)]
    return {
        "size": [sw, sh],
        "differing_pixels": sum(any(deltas[i:i + 3]) for i in range(0, len(deltas), 3)),
        "max_channel_delta": max(deltas, default=0),
        "mean_channel_delta": sum(deltas) / max(len(deltas), 1),
        "channels_over_tolerance": sum(value > tolerance for value in deltas),
        "tolerance": tolerance,
        "pass": all(value <= tolerance for value in deltas),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[3])
    parser.add_argument("--executable", type=Path)
    parser.add_argument("--disc", type=Path)
    parser.add_argument("--audio-off", action="store_true", help="pass --audio off to hosts with the audio selector; isolates renderer verification")
    parser.add_argument("--only", nargs="+")
    # One native 5-bit channel quantization step expands to at most 9 in RGB8.
    # Coverage differences are not exempted or hidden by a spatial comparison.
    parser.add_argument("--tolerance", type=int, default=9, choices=range(10))
    args = parser.parse_args()
    root = args.root.resolve()
    private = root.parent.parent / "private/work/gpuwire"
    output = private / "parity" / datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    output.mkdir(parents=True)
    if not output.resolve().is_relative_to(private.resolve()):
        raise ValueError("evidence must stay under private/work/gpuwire")
    executable = (args.executable or root / "target/release/silent-hill-boot.exe").resolve()
    replays = root / "docs/core/replays"
    manifest = json.loads((replays / "milestones.json").read_text())
    if manifest["version"] != 1:
        raise ValueError("unsupported milestone version")
    cases = manifest["verified"]
    if args.only:
        unknown = set(args.only) - {case["name"] for case in cases}
        if unknown:
            raise ValueError("unknown milestones: " + ", ".join(sorted(unknown)))
        cases = [case for case in cases if case["name"] in args.only]
    flags = {"state": "--expect-state", "step": "--expect-step", "menu": "--expect-menu", "option_entry": "--expect-option-entry", "movie_skips": "--expect-movie-skips", "min_movie_frames": "--min-movie-frames", "min_lit_pixels": "--min-lit-pixels"}
    results = []
    for case in cases:
        runs = []
        screenshots = []
        for renderer in ["soft", "wgpu"]:
            stem = case["name"] + "-" + renderer
            screenshot = output / (stem + ".png")
            command = [str(executable), "--headless", "--renderer", renderer, "--scale", "1", "--stats", "--frames", str(case["frames"]), "--input", str(replays / case["input"]), "--screenshot", str(screenshot)]
            if args.disc:
                command += ["--disc", str(args.disc.resolve())]
            if args.audio_off:
                command += ["--audio", "off"]
            for key, flag in flags.items():
                if key in case:
                    command += [flag, str(case[key])]
            try:
                run = subprocess.run(command, cwd=root, capture_output=True, text=True, timeout=180)
                log = run.stdout + run.stderr
                exit_code = run.returncode
            except subprocess.TimeoutExpired:
                log = "FAIL: native replay timed out after 180 seconds\n"
                exit_code = -1
            (output / (stem + ".log")).write_text(log)
            checkpoint = [line for line in log.splitlines() if line.startswith("CHECK ")]
            hardware_selected = renderer == "soft" or "GPU wgpu adapter=" in log
            runs.append({"renderer": renderer, "command": command, "exit": exit_code, "checkpoint": checkpoint, "pass": exit_code == 0 and len(checkpoint) == 1 and hardware_selected and screenshot.exists()})
            screenshots.append(screenshot)
            print(f"{'PASS' if runs[-1]['pass'] else 'FAIL'} {stem}", flush=True)
        parity = compare(*screenshots, args.tolerance) if all(run["pass"] for run in runs) else {"pass": False}
        result = {"case": case["name"], "runs": runs, "parity": parity, "pass": parity["pass"] and runs[0]["checkpoint"] == runs[1]["checkpoint"]}
        results.append(result)
        (output / "results.json").write_text(json.dumps(results, indent=2))
        print(f"{'PASS' if result['pass'] else 'FAIL'} parity {case['name']}: {parity}", flush=True)
    print("Evidence: " + str(output), flush=True)
    return 0 if all(result["pass"] for result in results) else 1


if __name__ == "__main__":
    raise SystemExit(main())
