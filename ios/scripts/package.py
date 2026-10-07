"""Build/assemble an unsigned iPhone IPA or ad-hoc signed simulator app.
No provisioning profile, Apple account, third-party bundler, or game data.
"""
import argparse
import json
import os
from pathlib import Path
import plistlib
import re
import shlex
import shutil
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
import zipfile

from assets import generate

IOS = Path(__file__).resolve().parents[1]
NAME = "SilentHillPort"
EXE = "silent-hill-ios"
TARGETS = {"device": ("aarch64-apple-ios", "iphoneos", "iPhoneOS"), "simulator": ("aarch64-apple-ios-sim", "iphonesimulator", "iPhoneSimulator")}


def run(args, **kwargs):
    print(shlex.join(map(str, args)), flush=True)
    return subprocess.run(list(map(str, args)), check=True, **kwargs)


def metadata(mode):
    with (IOS / "bundle/Info.plist").open("rb") as f:
        info = plistlib.load(f)
    info["CFBundleSupportedPlatforms"] = [TARGETS[mode][2]]
    return info


def archive(app, output):
    with zipfile.ZipFile(output, "w", zipfile.ZIP_DEFLATED) as ipa:
        for path in sorted(app.rglob("*")):
            if path.is_file():
                entry = zipfile.ZipInfo("Payload/" + app.name + "/" + path.relative_to(app).as_posix())
                entry.create_system = 3
                entry.external_attr = (0o100755 if path.name == EXE else 0o100644) << 16
                entry.compress_type = zipfile.ZIP_DEFLATED
                ipa.writestr(entry, path.read_bytes())


def validate_ipa(path):
    with zipfile.ZipFile(path) as ipa:
        prefix = f"Payload/{NAME}.app/"
        assert all(n.startswith(prefix) and ".." not in n.split("/") for n in ipa.namelist())
        info = plistlib.loads(ipa.read(prefix + "Info.plist"))
        assert info["CFBundleIdentifier"] == "com.bramley.silenthillport"
        assert info["UIFileSharingEnabled"] and info["LSSupportsOpeningDocumentsInPlace"]
        assert info["UISupportedInterfaceOrientations"] == ["UIInterfaceOrientationLandscapeLeft", "UIInterfaceOrientationLandscapeRight"]
        assert {ext for t in info["UTExportedTypeDeclarations"] for ext in t["UTTypeTagSpecification"]["public.filename-extension"]} == {"bin"}
        assert info["CFBundleExecutable"] == EXE
        assert (ipa.getinfo(prefix + EXE).external_attr >> 16) & 0o111
        assert not any("_CodeSignature" in n or n.endswith("embedded.mobileprovision") for n in ipa.namelist())
        assert any(n.startswith(prefix + "LaunchScreen.storyboardc/") for n in ipa.namelist())
        assert prefix + "Assets.car" in ipa.namelist()


def dry_run():
    ET.parse(IOS / "bundle/LaunchScreen.storyboard")
    with tempfile.TemporaryDirectory(prefix="ios-dry-") as tmp:
        root = Path(tmp)
        generate(root / "Assets.xcassets")
        images = json.loads((root / "Assets.xcassets/AppIcon.appiconset/Contents.json").read_text())["images"]
        assert len(images) == 9
        for image in images:
            assert (root / "Assets.xcassets/AppIcon.appiconset" / image["filename"]).read_bytes().startswith(b"\x89PNG")
        app = root / f"{NAME}.app"
        app.mkdir()
        (app / "Info.plist").write_bytes(plistlib.dumps(metadata("device")))
        # Explicit fixtures: this archive is NOT installable and is discarded.
        (app / EXE).write_bytes(b"dry-run fixture, not a Mach-O executable")
        (app / "Assets.car").write_bytes(b"dry-run fixture")
        (app / "LaunchScreen.storyboardc").mkdir()
        (app / "LaunchScreen.storyboardc/fixture").write_text("dry-run")
        archive(app, root / "fixture.ipa")
        validate_ipa(root / "fixture.ipa")
    for mode, (target, sdk, _) in TARGETS.items():
        print(f"{mode}: SDKROOT=$(xcrun --sdk {sdk} --show-sdk-path) IPHONEOS_DEPLOYMENT_TARGET=15.0 cargo build --manifest-path ios/app/Cargo.toml --locked --release --target {target}")
    print("PASS: plist, storyboard XML, 9 generated icons, IPA layout/permissions/signature-file exclusions (fixtures only).")


def build(mode):
    if sys.platform != "darwin":
        raise SystemExit("Real packaging requires macOS + Xcode. Use --dry-run on Windows.")
    target, sdk, platform = TARGETS[mode]
    out = IOS / "out" / mode
    if out.exists():
        # Only delete this fixed generated directory under ios/out.
        assert out.resolve().is_relative_to((IOS / "out").resolve())
        shutil.rmtree(out)
    out.mkdir(parents=True)
    app = out / "Payload" / f"{NAME}.app"
    app.mkdir(parents=True)
    env = os.environ.copy()
    env["SDKROOT"] = run(["xcrun", "--sdk", sdk, "--show-sdk-path"], capture_output=True, text=True).stdout.strip()
    env["IPHONEOS_DEPLOYMENT_TARGET"] = "15.0"
    if env.get("SH_IOS_RUST_CHECK_ONLY"):
        raise SystemExit("Packaging must compile C and UIKit; SH_IOS_RUST_CHECK_ONLY is forbidden.")
    env["CARGO_TARGET_DIR"] = str(IOS.parent / "target")
    # cc gets Apple's clang and SDK explicitly. No system install.
    env["CC"] = run(["xcrun", "--sdk", sdk, "--find", "clang"], capture_output=True, text=True).stdout.strip()
    env["AR"] = run(["xcrun", "--sdk", sdk, "--find", "ar"], capture_output=True, text=True).stdout.strip()
    run(["cargo", "build", "--manifest-path", IOS / "app/Cargo.toml", "--locked", "--release", "--target", target], env=env)
    shutil.copy2(IOS.parent / "target" / target / "release" / EXE, app / EXE)
    (app / EXE).chmod(0o755)
    generate(out / "Assets.xcassets")
    partial = out / "asset-info.plist"
    run(["xcrun", "--sdk", sdk, "actool", out / "Assets.xcassets", "--compile", app,
         "--platform", sdk, "--minimum-deployment-target", "15.0", "--target-device", "iphone",
         "--app-icon", "AppIcon", "--output-partial-info-plist", partial, "--output-format", "human-readable-text"])
    run(["xcrun", "--sdk", sdk, "ibtool", "--compile", app / "LaunchScreen.storyboardc",
         IOS / "bundle/LaunchScreen.storyboard", "--minimum-deployment-target", "15.0", "--target-device", "iphone"])
    info = metadata(mode)
    with partial.open("rb") as f:
        info.update(plistlib.load(f))
    info["DTPlatformName"] = sdk
    info["DTSDKName"] = sdk + run(["xcrun", "--sdk", sdk, "--show-sdk-version"], capture_output=True, text=True).stdout.strip()
    (app / "Info.plist").write_bytes(plistlib.dumps(info))
    (app / "PkgInfo").write_bytes(b"APPL????")
    run(["plutil", "-lint", app / "Info.plist"])
    build_info = run(["xcrun", "vtool", "-show-build", app / EXE], capture_output=True, text=True).stdout
    print(build_info)
    expected = "IOSSIMULATOR" if mode == "simulator" else "IOS"
    assert re.search(rf"\bplatform\s+{expected}\s*$", build_info, re.MULTILINE), build_info
    if mode == "device":
        # Apple's arm64 linker may insert an ad-hoc Mach-O signature. Remove it too.
        if subprocess.run(["codesign", "-d", str(app / EXE)], capture_output=True).returncode == 0:
            run(["codesign", "--remove-signature", app / EXE])
        assert subprocess.run(["codesign", "-d", str(app / EXE)], capture_output=True).returncode != 0
        ipa = out / "SilentHillPort-unsigned.ipa"
        archive(app, ipa)
        validate_ipa(ipa)
        print(f"Unsigned device IPA: {ipa}")
    else:
        # Simulator arm64 needs local ad-hoc signing; no identity or paid account.
        run(["codesign", "--force", "--sign", "-", "--timestamp=none", app])
        run(["codesign", "--verify", "--strict", app])
        print(f"Simulator app ({platform}): {app}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--mode", choices=TARGETS, default="device")
    args = parser.parse_args()
    dry_run() if args.dry_run else build(args.mode)
