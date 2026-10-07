# SPDX-License-Identifier: GPL-3.0-only
import argparse
import unittest
from pathlib import Path
from prepare_libsd import EDITS, transform


class PortFixes(unittest.TestCase):
    def test_pinned_sources_and_exact_edit_counts(self):
        for name in EDITS:
            path = DECOMP / "src/bodyprog/libsd" / name
            before = path.read_bytes()
            text = before.decode("utf-8")
            fixed = transform(name, text)
            self.assertEqual(before, path.read_bytes())
            self.assertNotIn("SpuGetKeyStatus(spu_ch_tbl[vo] == 1)", fixed)
            self.assertNotIn("SpuGetKeyStatus(spu_ch_tbl[voice] == 1)", fixed)
            self.assertEqual(fixed.count("// PORT:") - text.count("// PORT:"), len(EDITS[name]))
            for old, new in EDITS[name]:
                self.assertNotIn(old, fixed)
                self.assertIn(new, fixed)

    def test_source_drift_is_rejected(self):
        for name in EDITS:
            text = (DECOMP / "src/bodyprog/libsd" / name).read_text(encoding="utf-8")
            with self.assertRaises(ValueError):
                transform(name, text.replace(EDITS[name][0][0], "changed upstream expression"))
            with self.assertRaises(ValueError):
                transform(name, text + EDITS[name][0][0])


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--decomp", type=Path, required=True)
    args, remaining = parser.parse_known_args()
    DECOMP = args.decomp.resolve()
    unittest.main(argv=[__file__, *remaining])
