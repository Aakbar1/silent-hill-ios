# Core lane checkpoint

## Goal and constraints
Progress from merged Konami boot toward the first controllable map, in brief order. Branch lane/core. Game bytes/captures remain under ../../private/work/core/. No system installations, pushes, merges or GPU/SPU lane edits.

## Committed checkpoints
1. game/decomp submodule pinned to d9e28f8315c7938117224f21516786d9d149a145, SH_DECOMP_DIR override, root workspace and data-free Windows CI. Only explicitly required standalone manifest/profile removals outside core directories. See docs/core/BUILD.md.
2. psxdisc archive reads by file ID; GPU/SPU/pad traits, software raster default, VRAM transfers, keyboard/XInput and tick replay. See docs/core/SEAMS.md.
3. Explicit model/map/collision/DMS disk32 decode and save roundtrip; native asset store/checked C leaf views; fixed-width SDK declarations and C layout gates. Full C gameplay consumers still need adapters when linked. See docs/core/ABI.md.
4. Partial overlay milestone: B_KONAMI exports namespaced, writable initial image restored on load; full native KCET and original USA settings/FS helpers linked. Remaining four screens and 43 maps are not linked. See docs/core/OVERLAYS.md.

## Verified current state
- fmt, workspace/all-targets clippy with warnings denied, workspace build, 234 unit/integration + 3 doctests pass. Selected native C builds /W4 /WX.
- MSVC x86/x64 wire assertions pass. Installed LLVM frontend checks actual iOS arm64/LP64 SDK/wire/native layouts and rejects a negative control. Remote CI configured, not run.
- Disc audit: 622 assets decode, TEST2.DMS rejects unsupported runtime layout; complete audit exits nonzero. No bounds gate weakened.
- 850-tick neutral replay succeeds and visibly renders KCET (state 2/step 6). Neutral 1600 requested stops after 989 at MovieIntroFadeIn (state 3, code 3). Actual final frame saved/presented. Input Start at 850/release 851 reaches the same guard at 929, proving the game reacts to the pad packet.
- Private captures/logs: kcet-850.png/log, final-stop.png/final-neutral.log, kcet-skip-stop.png/log; original step2/step3 evidence retained. Captures visually reviewed. No game data/screenshots in Git.

## Open work and decisions
Step 4 remains partial; step 5 title/New Game/walking is not achieved. Broad original-header probe still fails 43 size assertion diagnostics (expected opt-in gate, not the native passing configuration). Do not disable those assertions: port resident/map clients to native records and decoder adapters, plus GTE/scratch/packet contracts, then add all overlay namespaces/data-reset descriptors. STREAM/memory-card/native gameplay services and physical controller verification remain pending. Coordinate GPU/SPU lane adapters through the host traits.
