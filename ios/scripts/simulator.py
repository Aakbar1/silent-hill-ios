"""Install, launch, verify a rendered landscape frame, and collect diagnostics."""
import json
from pathlib import Path
import re
import shutil
import struct
import subprocess
import time

from package import IOS, NAME, run

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
    udid = choose_device(devices)
    print(f"Simulator: {udid}")
    booted = any(d["udid"] == udid and d["state"] == "Booted" for ds in devices["devices"].values() for d in ds)
    try:
        if not booted:
            run(["xcrun", "simctl", "boot", udid])
        run(["xcrun", "simctl", "bootstatus", udid, "-b"])
        run(["xcrun", "simctl", "install", udid, app])
        container = Path(run(["xcrun", "simctl", "get_app_container", udid, BUNDLE, "data"], capture_output=True, text=True).stdout.strip())
        log = container / "Documents/shell.log"
        # Reject stale success from an earlier launch when running manually.
        offset = log.stat().st_size if log.exists() else 0
        run(["xcrun", "simctl", "launch", "--terminate-running-process", udid, BUNDLE])
        deadline = time.monotonic() + 60
        while time.monotonic() < deadline:
            content = log.read_text(errors="replace")[offset:] if log.exists() else ""
            if "panic:" in content:
                raise RuntimeError(content)
            if "first frame presented" in content:
                break
            time.sleep(1)
        else:
            raise RuntimeError("App did not log a presented frame within 60 seconds.")
        time.sleep(2)
        screenshot = out / "simulator.png"
        run(["xcrun", "simctl", "io", udid, "screenshot", screenshot])
        data = screenshot.read_bytes()
        assert data[:8] == b"\x89PNG\r\n\x1a\n"
        width, height = struct.unpack(">II", data[16:24])
        assert width > height, f"Expected landscape screenshot, got {width}x{height}"
        print(f"PASS: installed, launched, rendered, landscape screenshot {width}x{height}")
    finally:
        if "log" in locals() and log.exists():
            shutil.copy2(log, out / "shell.log")
        result = subprocess.run(["xcrun", "simctl", "spawn", udid, "log", "show", "--last", "2m", "--style", "compact", "--predicate", 'process == "silenthill-shell"'], capture_output=True, text=True)
        (out / "simulator-system.log").write_text(result.stdout + result.stderr)
        subprocess.run(["xcrun", "simctl", "shutdown", udid], check=False)


if __name__ == "__main__":
    main()
