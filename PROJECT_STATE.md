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

