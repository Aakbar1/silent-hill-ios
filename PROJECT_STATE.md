# Core lane checkpoint

## Goal and constraints
Progress from merged Konami boot toward the first controllable map, in brief order. Branch lane/core. Game bytes/captures remain under ../../private/work/core/. No system installations, pushes, merges or GPU/SPU lane edits.

## Completed checkpoints
1. Pinned game/decomp submodule d9e28f8315c7938117224f21516786d9d149a145, SH_DECOMP_DIR override, root workspace and data-free Windows CI. Minimal standalone manifest/profile removals were explicitly required by goal 1. See docs/core/BUILD.md.
2. psxdisc archive reads by original file ID; GPU/SPU/pad traits, software raster default, VRAM transfers, keyboard/XInput and deterministic replay. Full game-owned joy/libkpad and audible audio are unlinked. See docs/core/SEAMS.md.
3. Explicit model/map/collision/DMS disk32 decode and save roundtrip; native asset store/checked C leaf views; fixed-width SDK declarations and C layout gates. Full gameplay consumer adapters remain necessary when linking those units. See docs/core/ABI.md.

## Verified state
- fmt, workspace/all-targets clippy with warnings denied, workspace build, 233 unit/integration + 3 doctests pass. Native C builds /W4 /WX.
- MSVC x86/x64 wire assertions pass. Installed LLVM frontend checks actual iOS arm64/LP64 SDK/wire/native layouts and rejects a negative control. Remote CI is configured, not run.
- Disc audit: 622 assets decode, one TEST2.DMS rejects unsupported runtime layout; complete audit exits nonzero. No bounds gate weakened.
- Neutral replay: 600 ticks succeeds with Konami; 800 requested stops at KCET guard after 731, code 3. Actual frame saved/presented. Private step2/step3 logs and PNGs. Screenshot reviewed; no screenshot/data in Git.

## Next and open limits
Integrate native overlays with namespaces and initial-data reset. All 43 maps/five screens are not yet linked. Rendering/GTE, real joy/libkpad, audio sequencer and memory cards remain beyond the logo subset. Title/New Game/controllable map is not proven. Keep original ABI assertions active while porting the full C consumers; do not recover truncated pointers or cast disk headers to native records.
