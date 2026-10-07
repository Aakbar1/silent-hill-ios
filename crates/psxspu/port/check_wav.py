#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Check a private 44.1 kHz stereo PCM16 recording (never infers A/V sync)."""
import argparse
import array
import json
import math
import sys
import wave
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("wav", type=Path)
    parser.add_argument("--json", type=Path)
    args = parser.parse_args()
    count = nonzero = rails = squares = peak = 0
    with wave.open(str(args.wav), "rb") as audio:
        assert (audio.getframerate(), audio.getnchannels(), audio.getsampwidth()) == (44100, 2, 2)
        frames = audio.getnframes()
        while raw := audio.readframes(16384):
            samples = array.array("h", raw)
            if sys.byteorder != "little":
                samples.byteswap()
            count += len(samples)
            nonzero += sum(value != 0 for value in samples)
            rails += sum(value in (-32768, 32767) for value in samples)
            squares += sum(value * value for value in samples)
            peak = max(peak, max(map(abs, samples), default=0))
    result = dict(path=str(args.wav), frames=frames, seconds=frames / 44100,
                  sample_count=count, nonzero_samples=nonzero, full_scale_samples=rails,
                  peak=peak, peak_dbfs=20 * math.log10(peak / 32768) if peak else None,
                  rms=math.sqrt(squares / count) if count else 0,
                  non_silent=bool(nonzero), no_full_scale_clipping=rails == 0)
    text = json.dumps(result, indent=2)
    print(text)
    if args.json:
        args.json.write_text(text + "\n", encoding="utf-8")
    if not nonzero or rails:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
