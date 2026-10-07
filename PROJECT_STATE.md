# Core lane checkpoint

## Goal and constraints
Progress from the merged Konami boot toward the first controllable map, in the brief's order. Own branch: `lane/core`. Reference GPL-3.0 C only; game data and capture evidence stay under `../../private/work/core/`. GPU/SPU crates belong to parallel lanes.

## Completed: build hygiene
- `game/decomp` is a Git submodule pinned to `d9e28f8315c7938117224f21516786d9d149a145`; `SH_DECOMP_DIR` overrides it, with revision validation in the host build.
- Root workspace includes host, compiler probe, shared crates and iOS shell. Only required workspace/profile manifest removals were made outside core ownership, as explicitly requested by goal 1.
- Windows workflow checks/builds/tests the root workspace without requiring game data; generated reference C uses Cargo OUT_DIR. Baseline full-header probe remains opt-in and intentionally fails on unmigrated ABI assertions.
- Local checks: fmt check, clippy workspace/all-targets with warnings denied, workspace build, 225 unit/integration tests plus 3 doctests passed. Three owned-disc checks also ran against local private data.

## Current runtime boundary
Unchanged from Phase 1: Konami succeeds; KCET is an explicit guard at 731 ticks. This checkpoint does not claim title/gameplay or ABI migration.

## Next
Implement shared-disc/pad/backend seams, then explicit disk32 decoders and C layout checks; verify/commit each step before overlay work. References: `docs/survey/SURVEY.md`, `docs/boot/READINESS.md`, `docs/boot/PORT_FIXES.md`.


## Completed: shared services (step 2)
- Removed host disc implementation; psxdisc GameDisc verifies US 1.1 and serves bounded archive reads by original file ID.
- Swappable GPU/SPU/pad traits, software raster default, VRAM transfer shims, keyboard/XInput and tick-based replay. Full gameplay joy/libkpad and audible audio remain pending.
- Workspace fmt/clippy/tests passed; affected host retest passes 6 tests after final transfer/input edits. MSVC /W4 /WX is active.
- Replay at 600 succeeds: state 1/step 3, 3,413 primitives, Konami capture. At 800 requested stops at KCET after 731, C code 3; actual last frame saved/presented. Evidence: ../../private/work/core/step2-*.png/log. See docs/core/SEAMS.md.

Next: disk32/native views and 32/64-bit C layout checks before overlay integration.
