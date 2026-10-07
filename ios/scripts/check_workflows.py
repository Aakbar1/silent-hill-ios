"""Download a pinned, checksum-verified portable actionlint into ios/.tools.
No PATH changes or system install. actionlint also parses/validates the YAML.
"""
import hashlib
import io
from pathlib import Path
import platform
import subprocess
import sys
import tarfile
import urllib.request
import zipfile

IOS = Path(__file__).resolve().parents[1]
VERSION = "1.7.12"
ARCHIVES = {
    ("Windows", "AMD64"): ("windows_amd64.zip", "6e7241b51e6817ea6a047693d8e6fed13b31819c9a0dd6c5a726e1592d22f6e9"),
    ("Darwin", "arm64"): ("darwin_arm64.tar.gz", "aba9ced2dee8d27fecca3dc7feb1a7f9a52caefa1eb46f3271ea66b6e0e6953f"),
    ("Linux", "x86_64"): ("linux_amd64.tar.gz", "8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8"),
}


def main():
    key = (platform.system(), platform.machine())
    if key not in ARCHIVES:
        raise SystemExit(f"Use a matching portable actionlint from https://github.com/rhysd/actionlint/releases for {key}")
    suffix, digest = ARCHIVES[key]
    tools = IOS / ".tools" / f"actionlint-{VERSION}"
    tools.mkdir(parents=True, exist_ok=True)
    archive = tools / suffix
    if not archive.exists():
        url = f"https://github.com/rhysd/actionlint/releases/download/v{VERSION}/actionlint_{VERSION}_{suffix}"
        print(f"Downloading {url}", flush=True)
        with urllib.request.urlopen(url, timeout=60) as response:
            archive.write_bytes(response.read())
    data = archive.read_bytes()
    if hashlib.sha256(data).hexdigest() != digest:
        raise SystemExit(f"Checksum mismatch: remove {archive} and retry.")
    name = "actionlint.exe" if sys.platform == "win32" else "actionlint"
    if suffix.endswith(".zip"):
        with zipfile.ZipFile(io.BytesIO(data)) as z:
            binary = z.read(name)
    else:
        with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as t:
            binary = t.extractfile(name).read()
    executable = tools / name
    executable.write_bytes(binary)
    executable.chmod(0o755)
    # Embedded shell checks need shellcheck installed separately. All commands are
    # short; platform-dependent work lives in checked scripts instead.
    subprocess.run([str(executable), "-shellcheck=", "-pyflakes=", str(IOS.parent / ".github/workflows/ios.yml"), str(IOS.parent / ".github/workflows/windows.yml")], check=True)
    print(f"PASS: actionlint {VERSION}, 2 workflows; YAML/actions/expressions/runner checks. Optional shellcheck/pyflakes disabled.")


if __name__ == "__main__":
    main()
