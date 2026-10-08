"""Launch the real host without a disc in a NEW simulator; verify importer pixels."""
import json
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import time

from package import EXE, IOS, NAME, run

BUNDLE = "com.bramley.silenthillport"


def wait_log(log, required, timeout=60, start=0, min_recoveries=0):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        content = log.read_text(errors="replace") if log.exists() else ""
        if any(marker in content for marker in ("panic:", "IOS_SMOKE FAIL", "recovery failed", "pause deadline expired")):
            raise RuntimeError(content)
        generations = re.findall(r"audio recovery progress generation=(\d+)", content[start:])
        if (all(marker in content[start:] for marker in required)
                and max(map(int, generations), default=0) >= min_recoveries):
            return content
        time.sleep(0.5)
    raise RuntimeError(f"Missing log markers after {timeout}s: {required}\n{content}")


def verify_smoke_log(content):
    required = ("IOS_GPU selected=wgpu backend=Metal", "stats=false wide=false",
                "IOS_PRESENT UIImage installed", "IOS_AUDIO SpuCpal initialized with real device output",
                "IOS_SMOKE PASS rendered=90 audio_frames=66150", "touch=PASS saves=PASS")
    if not all(marker in content for marker in required):
        raise AssertionError("Incomplete real GPU/audio/touch/save evidence")
    if any(marker in content for marker in ("fallback=Raster", "IOS_SMOKE FAIL", "panic:", "GPU ERROR", "recovery failed")):
        raise AssertionError("Synthetic smoke must use Metal successfully; fallback is not a pass")
    callbacks = re.findall(r"audio_callbacks=(\d+)", content)
    if not callbacks or int(callbacks[-1]) == 0:
        raise AssertionError("The real audio device did not run callbacks")
    scales = re.findall(r"selected=wgpu backend=Metal scale=(\d+)", content)
    if not scales or not 1 <= int(scales[-1]) <= 3:
        raise AssertionError("Invalid phone scale")
    selected = int(scales[-1])
    probes = re.findall(r"IOS_GPU probe scale=(\d+) samples=4 mean_ms=([0-9.]+) max_ms=([0-9.]+)", content)
    measured = [(float(mean), float(worst)) for scale, mean, worst in probes if int(scale) == selected]
    if not measured or not 0 <= measured[-1][0] <= measured[-1][1]:
        raise AssertionError("Selected scale has no valid measured render sample")
    if selected > 1 and measured[-1][1] > 12:
        raise AssertionError("Selected scale exceeds the measured phone render budget")


def choose_device(data):
    candidates = []
    for runtime, devices in data["devices"].items():
        if ".iOS-" not in runtime:
            continue
        version = tuple(map(int, re.findall(r"\d+", runtime.rsplit("iOS-", 1)[1])))
        for device in devices:
            if device.get("isAvailable") and device["name"].startswith("iPhone"):
                candidates.append((version, "Pro Max" in device["name"], device["name"], device["udid"]))
    if not candidates:
        raise RuntimeError("No available iPhone runtime. Inspect xcrun simctl list; do not silently skip the smoke test.")
    return max(candidates)[-1]


def main():
    out = IOS / "out/simulator"
    app = out / "Payload" / f"{NAME}.app"
    devices = json.loads(run(["xcrun", "simctl", "list", "devices", "available", "--json"], capture_output=True, text=True).stdout)
    template = choose_device(devices)
    runtime, device = next((runtime, d) for runtime, ds in devices["devices"].items() for d in ds if d["udid"] == template)
    types = json.loads(run(["xcrun", "simctl", "list", "devicetypes", "--json"], capture_output=True, text=True).stdout)
    device_type = next(t["identifier"] for t in types["devicetypes"] if t["name"] == device["name"])
    # Never reuse/clone an app data container: even a manual run cannot capture
    # a previously imported disc's game content. No game data is used in CI.
    udid = run(["xcrun", "simctl", "create", "SilentHillPort importer smoke", device_type, runtime], capture_output=True, text=True).stdout.strip()
    print(f"Fresh simulator: {udid}")
    try:
        run(["xcrun", "simctl", "boot", udid])
        run(["xcrun", "simctl", "bootstatus", udid, "-b"])
        run(["xcrun", "simctl", "install", udid, app])
        container = Path(run(["xcrun", "simctl", "get_app_container", udid, BUNDLE, "data"], capture_output=True, text=True).stdout.strip())
        log = container / "Documents/game.log"
        assert not (container / "Library/Application Support/SilentHillPort/disc.bin").exists()
        assert not list((container / "Documents").glob("*.bin"))
        run(["xcrun", "simctl", "launch", "--terminate-running-process", udid, BUNDLE])
        deadline = time.monotonic() + 60
        while time.monotonic() < deadline:
            content = log.read_text(errors="replace") if log.exists() else ""
            if "panic:" in content:
                raise RuntimeError(content)
            if "importer screen appeared" in content:
                break
            time.sleep(1)
        else:
            raise RuntimeError("App did not show the importer within 60 seconds.")
        time.sleep(2)
        screenshot = out / "simulator.png"
        run(["xcrun", "simctl", "io", udid, "screenshot", screenshot])
        data = screenshot.read_bytes()
        assert data[:8] == b"\x89PNG\r\n\x1a\n"
        width, height = struct.unpack(">II", data[16:24])
        # simctl always captures in the device's native portrait orientation, so judge
        # orientation from the app's own surface size instead of the screenshot.
        content = log.read_text(errors="replace")
        assert "starting shared native C host" not in content, "No-disc test must not start game content."
        sizes = re.findall(r"resize (\d+)x(\d+)", content)
        assert sizes, "App never logged a surface size."
        surface_w, surface_h = map(int, sizes[-1])
        assert surface_w > surface_h, f"Expected landscape surface, got {surface_w}x{surface_h}"
        # Preserve recognition diagnostics on failure as well as the screenshot.
        ocr = subprocess.run(["xcrun", "swift", str(IOS / "scripts/verify-importer.swift"), str(screenshot)], capture_output=True, text=True)
        (out / "importer-ocr.txt").write_text(ocr.stdout + ocr.stderr)
        ocr.check_returncode()
        print(ocr.stdout)
        print(f"PASS: real host installed, no disc, importer text visible in captured pixels; landscape view {surface_w}x{surface_h}; screenshot {width}x{height}")
        shutil.copy2(log, out / "importer-game.log")
        # Same freshly created data-free simulator/app; no extra build or boot.
        env = os.environ.copy()
        env["SIMCTL_CHILD_SH_IOS_SMOKE"] = "1"
        run(["xcrun", "simctl", "launch", "--terminate-running-process", udid, BUNDLE], env=env)
        content = wait_log(log, ["IOS_SMOKE PASS", "IOS_PRESENT UIImage installed"])
        verify_smoke_log(content)
        content = wait_log(log, ["IOS_SMOKE synthetic interruption notification began",
                                "IOS_SMOKE synthetic route notification",
                                "IOS_SMOKE audio recovery progress", "spu_state=PASS"], timeout=30, min_recoveries=2)
        screenshot = out / "synthetic-metal.png"
        run(["xcrun", "simctl", "io", udid, "screenshot", screenshot])
        result = subprocess.run(["xcrun", "swift", str(IOS / "scripts/verify-synthetic.swift"), str(screenshot)], capture_output=True, text=True)
        (out / "synthetic-pixels.txt").write_text(result.stdout + result.stderr)
        result.check_returncode()
        # Settings backgrounds the running process; launch without terminate
        # resumes that SAME worker. Tick/audio/save state must survive.
        offset = len(content)
        run(["xcrun", "simctl", "launch", udid, "com.apple.Preferences"])
        content = wait_log(log, ["worker paused; synchronous save callbacks completed", "pause acknowledged"], timeout=15, start=offset)
        offset = len(content)
        run(["xcrun", "simctl", "launch", udid, BUNDLE])
        content = wait_log(log, ["worker resumed", "IOS_AUDIO recovered; SPU state and sample clock preserved",
                                "IOS_SMOKE audio recovery progress", "spu_state=PASS"], timeout=30, start=offset)
        verify_smoke_log(content)
        run(["xcrun", "simctl", "io", udid, "screenshot", out / "synthetic-resumed.png"])
        result = subprocess.run(["xcrun", "swift", str(IOS / "scripts/verify-synthetic.swift"), str(out / "synthetic-resumed.png")], capture_output=True, text=True)
        (out / "resumed-pixels.txt").write_text(result.stdout + result.stderr)
        result.check_returncode()
        assert not (container / "Library/Application Support/SilentHillPort/disc.bin").exists()
        print("PASS: synthetic Metal screenshot, real audio callbacks, menu/walking/release pad adapter, atomic synthetic saves, same-worker background/resume.")
    finally:
        if "log" in locals() and log.exists():
            shutil.copy2(log, out / "game.log")
        result = subprocess.run(["xcrun", "simctl", "spawn", udid, "log", "show", "--last", "2m", "--style", "compact", "--predicate", f'process == "{EXE}"'], capture_output=True, text=True)
        (out / "simulator-system.log").write_text(result.stdout + result.stderr)
        subprocess.run(["xcrun", "simctl", "shutdown", udid], check=False)
        subprocess.run(["xcrun", "simctl", "delete", udid], check=False)


if __name__ == "__main__":
    main()
