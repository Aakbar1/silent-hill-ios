# Media lane handoff — 2026-10-07

Goal: native STR/MDEC v2/v3, XA and SPU/VAG decoding in the owned standalone crate.
Constraints: zero native/emulator/runtime dependencies, GPL-3.0, no game bytes or
extracted assets in Git, private output only, no root workspace edits.

Completed: sector parsing and bounded STR assembly; v2/v3 VLC, quantization,
separable integer IDCT, 4:2:0 colour conversion and RGBA; XA 4/8-bit mono/stereo
37.8/18.9 kHz with independent file/channel history; SPU blocks and VAG loop ranges;
rational video/sample timestamps, filter selection and seek reset. README.md
records the decomp evidence and all XA/FMVs' start LBAs. No gameplay changes.

Checks: `cargo fmt --check` pass; `cargo clippy --all-targets -- -D warnings` pass,
zero warnings; `cargo test` 19 passed, 0 failed (2 unit + 17 integration).
Synthetic coverage includes signed/nibble/unit ordering, predictor persistence,
clipping, flags, VAG headers/loops, both XA bit depths/rates/channel modes, v3
DC differentials, AC escapes, qscale-zero ordering, chunk loss/conflicts, rational
timestamps and malformed/truncated inputs.

Real-disc example: 300 intro frames + 375 XA packets (20 seconds), 320×208 at
15 fps, 37.8 kHz stereo. Exact 10-second and 20-second WAVs and 300 PNGs are in
`C:/Claude Projects/Silent Hill iOS/private/work/media/`. Both tracks cover exactly
20 seconds; timestamp residual against source-sector cadence is below 0.4 ns
(CSV decimal rounding). No missing/failed frames.

Release performance: Intel Core i7-14700HX, rustc 1.99.0, MSVC Windows target;
process pinned to one logical CPU (affinity mask 2 verified). Three in-memory
single-thread passes including XA: 975.58 / 1014.65 / 1036.97 frames/s.
Median **67.64× realtime**; threshold is 4×. File I/O/PNG encoding excluded.

Independent oracle: local private-venv FFmpeg 7.1, used only for verification.
All 1,512,000 interleaved PCM values match exactly. Across all 300 frames, mean
RGB error 0.484532/255, max 5/255, 99.3863% within 2/255. `tools/verify_dump.py`
reproduces the comparison and checks the timeline. Private evidence files:
`comparison-results.json`, `timeline.csv`, `decode-results.txt`, `contact-sheet.png`.

Visual review: inspected contact-sheet frames 1,30,60,90,120,150,180,210,240,260,
280,300. Opening text is legible, portrait/character proportions and tree/sky
colours are coherent, fog and fades look consistent; no scrambled blocks or
chroma planes. Did not individually inspect every frame or listen to playback.

Limitations: MDEC silicon rounding is not bit-exact (documented `// PORT:`);
XA emphasis is explicitly rejected; iPhone/arm64 speed/build untested. Real-disc
v3/SPU samples untested; those formats have synthetic coverage. Audio resampling,
SPU interpolation/pitch/envelopes and playback scheduling are host/audio duties.

Next: director adds workspace membership/removes crate `[workspace]`, disc lane
supplies raw 2352/2336 sectors with XA metadata, host consumes timestamped events.
REPORT.md is the uncommitted root handoff. No merge or push by this lane.
