"""Checks for platform selection and packaging safety that can run on Windows."""
import tempfile
from pathlib import Path
import unittest
import zipfile

from package import EXE, NAME, archive, metadata, validate_ipa
from simulator import choose_device


class PipelineTests(unittest.TestCase):
    def test_simulator_chooses_latest_available_iphone(self):
        def phone(name, udid, available=True):
            return {"name": name, "udid": udid, "isAvailable": available}
        data = {"devices": {
            "com.apple.CoreSimulator.SimRuntime.iOS-18-5": [phone("iPhone 16", "old")],
            "com.apple.CoreSimulator.SimRuntime.iOS-26-0": [phone("iPad Pro", "ipad"), phone("iPhone 17 Pro Max", "new"), phone("iPhone 18", "missing", False)],
        }}
        self.assertEqual(choose_device(data), "new")

    def test_missing_runtime_is_a_failure(self):
        with self.assertRaises(RuntimeError):
            choose_device({"devices": {}})

    def test_simulator_metadata_is_separate(self):
        self.assertEqual(metadata("simulator")["CFBundleSupportedPlatforms"], ["iPhoneSimulator"])
        self.assertEqual(metadata("device")["CFBundleSupportedPlatforms"], ["iPhoneOS"])

    def test_signed_archive_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            app = Path(tmp) / f"{NAME}.app"
            app.mkdir()
            import plistlib
            (app / "Info.plist").write_bytes(plistlib.dumps(metadata("device")))
            (app / EXE).write_bytes(b"fixture")
            (app / "embedded.mobileprovision").write_bytes(b"fixture")
            ipa = Path(tmp) / "fixture.ipa"
            archive(app, ipa)
            with self.assertRaises(AssertionError):
                validate_ipa(ipa)
            with zipfile.ZipFile(ipa) as z:
                self.assertTrue((z.getinfo(f"Payload/{NAME}.app/{EXE}").external_attr >> 16) & 0o111)


if __name__ == "__main__":
    unittest.main()
