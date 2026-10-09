"""Inspect real alley SFX writes and retain only their private PCM window.

SPDX-License-Identifier: GPL-3.0-only. No fixture can establish this replay pass.
"""
import argparse
from array import array
import json
from pathlib import Path
import re
import sys
import wave

from milestones import project_root


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--log', type=Path, required=True)
    parser.add_argument('--wav', type=Path, required=True)
    args = parser.parse_args()
    private = (project_root(Path(__file__).resolve().parents[1]) / 'private/work').resolve()
    if not all(p.resolve().is_relative_to(private) for p in (args.log, args.wav)):
        parser.error('evidence must stay under private/work')
    log = args.log.read_text(encoding='utf-8')
    rows = [{k: int(v) for k, v in re.findall(r'(\w+)=(-?\d+)', line)}
            for line in log.splitlines() if line.startswith('POSITIONAL_SFX ')]
    rows = [r for r in rows if r.get('id') == 1358 and r.get('tick', 0) >= 12961]
    if not rows:
        parser.error('no real wheel voice-register samples')
    first, last = min(r['tick'] for r in rows), max(r['tick'] for r in rows)
    with wave.open(str(args.wav), 'rb') as source:
        if (source.getnchannels(), source.getsampwidth(), source.getframerate()) != (2, 2, 44100):
            parser.error('expected original stereo 44.1 kHz PCM')
        start = (first - 1) * 735
        count = min(source.getnframes() - start, (last - first + 61) * 735)
        if count <= 0:
            parser.error('PCM does not cover the observed voice writes')
        source.setpos(start)
        pcm = source.readframes(count)
    samples = array('h', pcm)
    if sys.byteorder != 'little':
        samples.byteswap()
    capture = args.log.with_name(args.log.stem + '-wheel.wav')
    with wave.open(str(capture), 'wb') as output:
        output.setparams((2, 2, 44100, 0, 'NONE', 'not compressed'))
        output.writeframes(pcm)
    attenuations = sorted({r['attenuation'] for r in rows})
    active = [r for r in rows if r['left'] or r['right']]
    result = dict(voice_samples=len(rows), first_tick=first, last_tick=last,
                  attenuation_range=[min(attenuations), max(attenuations)],
                  distinct_attenuations=len(attenuations), active_voice_samples=len(active),
                  pcm_frames=len(samples)//2, peak=max(map(abs, samples), default=0),
                  nonzero_samples=sum(v != 0 for v in samples),
                  output_rails=sum(v in (-32768, 32767) for v in samples),
                  mixed_pcm_capture=str(capture))
    result['pass'] = (len(rows) > 1 and len(attenuations) > 1 and bool(active)
                      and all(0 <= r['voice'] < 24 and r['spu_pitch'] > 0 for r in rows)
                      and all(not (r['left'] or r['right']) for r in rows if r['attenuation'] == 255)
                      and result['nonzero_samples'] > 0 and result['output_rails'] == 0
                      and 'FIGHT_FIXTURE ' not in log
                      and 'MOVIE end id=2055 decoded=338' in log)
    args.log.with_name(args.log.stem + '-audio.json').write_text(json.dumps(result, indent=2))
    print(json.dumps(result, indent=2))
    return int(not result['pass'])


if __name__ == '__main__':
    raise SystemExit(main())
