"""Launch the real host without a disc in a NEW simulator; verify importer pixels."""
import json
from pathlib import Path
import re
import shutil
import struct
import subprocess
import time

from package import EXE, IOS, NAME, run

BUNDLE = "com.bramley.silenthillport"


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
    finally:
        if "log" in locals() and log.exists():
            shutil.copy2(log, out / "game.log")
        result = subprocess.run(["xcrun", "simctl", "spawn", udid, "log", "show", "--last", "2m", "--style", "compact", "--predicate", f'process == "{EXE}"'], capture_output=True, text=True)
        (out / "simulator-system.log").write_text(result.stdout + result.stderr)
        subprocess.run(["xcrun", "simctl", "shutdown", udid], check=False)
        subprocess.run(["xcrun", "simctl", "delete", udid], check=False)


if __name__ == "__main__":
    main()
