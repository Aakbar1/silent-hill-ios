# Move replay evidence, 8 October 2026

Run `python tools/milestones.py --first-map` after the release build. The original
logic uses real BGM updates; `--audio off` only selects the output backend.
The numeric gate renders at 1x by default (`--scale 4` is available); game default
and the separate `first_map_view` test remain 4x wgpu. Output stays in
`private/work/move/`. Movement samples span 3299..3720; the original input read
occurs between presentations, so held-input assertions start at 3302/3452/3542,
one sample after the former unverified candidate assumed. Any further delay
still fails. Position, heading, run-speed and camera checks retain their limits.

Verified result: state 11/step 2 at VBlank 3720, 422 samples, walk 9173 Q12,
right turn 1711 angle units, walk 93.39/run 266.77 Q12 units per VBlank.
The VBlank 3380 screenshot shows Harry on the first road during the measured
forward-walk phase; both its log and PNG accompany the full numeric replay.
Evidence: `milestones/20261008T191556888153Z/{first-map.log,mid-walk.log,
mid-walk.png,results.json}`. Evaluator additionally rejects seven negative
controls based on this real log; `trace-controls.log` records those results.

Audio's base-VAB reads move the intro movie begin to tick 1338 (state 6 begins
at 1336). STR video runs at 15 fps against 60 Hz virtual VBlank: at tick 1900,
`floor((1900-1338)/4)+1 = 141` decoded frames. The tick-1400 skip delivers 16
intro frames; New Game's opening begins at 1676 and supplies 131 more by 2200,
for total 147. The full intro still delivers all 2060 frames at the title gate.
The three changed manifest thresholds have exact movie begin/count assertions
in `tools/milestones.py`; a later start or a lower decode count is rejected.
Evidence: `milestones/20261008T190237635239Z/`, ten available cases pass.
No assertion about deterministic image hashes is made without repeat runs.

The opening movie and opening event are skipped by the movement replay. A
separate run retaining the opening event reaches map-message text rollout at
VBlank 2443. Full opening dialogue, later Cheryl events, combat, MAP0_S01 and
transitions are not passed by this replay.
