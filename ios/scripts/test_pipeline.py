"""Checks for platform selection and packaging safety that can run on Windows."""
import tempfile
from pathlib import Path
import unittest
import zipfile

from package import EXE, NAME, archive, metadata, validate_ipa
from simulator import choose_device, verify_smoke_log


class PipelineTests(unittest.TestCase):
    def test_synthetic_requires_accelerated_audio_callbacks_and_all_checks(self):
        good = "\n".join([
            "IOS_GPU probe scale=3 samples=4 mean_ms=4.000 max_ms=5.000",
            "IOS_GPU selected=wgpu backend=Metal scale=3 stats=false wide=false",
            "IOS_PRESENT UIImage installed 960x672",
            "IOS_AUDIO SpuCpal initialized with real device output",
            "IOS_SMOKE PASS rendered=90 audio_frames=66150 audio_callbacks=100 touch=PASS saves=PASS",
        ])
        verify_smoke_log(good)
        for bad in (good + "\nfallback=Raster", good.replace("audio_callbacks=100", "audio_callbacks=0"),
                    good.replace("touch=PASS", "touch=FAIL"), good.replace("scale=3", "scale=4"),
                    good.replace("max_ms=5.000", "max_ms=13.000"), good.replace("IOS_GPU probe", "missing measurement"),
                    good.replace("IOS_PRESENT UIImage installed", "never presented")):
            with self.subTest(log=bad), self.assertRaises(AssertionError):
                verify_smoke_log(bad)
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
