# core3 checkpoint

## Goal and constraints
Reach the first controllable opening map, with native GTE, world consumers, 43 map overlays, keyboard/controller walking and a position replay; then area transitions. Branch lane/core3. Own host except gpu_wgpu.rs/spu_cpal.rs/pad_touch.rs/platform_ios.rs, port, non-submodule game, tools, docs/core, PROJECT_STATE and root Cargo files. No main/merge/push/system installs. Submodule initialised at d9e28f8315c7938117224f21516786d9d149a145. Reference stays read-only; game bytes/captures stay under private/work/core3.

## Current result
The requested pass is NOT achieved. Screens 3/5; maps 0/43. NORMAL confirmation at VBlank 1650 now completes the original GameBoot_WorldInit, including 23 HERO skeletal model nodes, 18 HB_BASE bones, health 409600, collision flags 1 and fog 8192/8704. Execution then stops at GameBoot_MapLoad/native map descriptor: state 7, step 1, menu 3, native code 3, process exit 1. Opening movie and controllable map are not reached; no first_map.txt claim was made.

## Completed work and decisions
- Exact COP2/PsyQ bridge delegates to psxgpu's existing native C GTE, with worker-local registers. 241 straight-line SDK helpers generated from pinned headers; 65 command variants cover all 22 commands. Native projection, matrix and lighting entry points use the same engine. No second GTE/CPU runner.
- --features precise-vertices emits optional float polygon metadata without changing integer results/flags/logic. Metadata is consumed once and discarded after an OT. GpuBackend::packet_precise is additive/defaulted; current menu Raster ignores it. GPU renderer implementation is still needed for visual precision mode.
- Root manifest excludes the standalone psxgpu/psxspu workspaces, resolving their conflict with crates/* without editing lane-owned crate manifests.
- Native LM/material/model/mesh graphs own separately decoded leaves. Checked ANM decoding owns poses/frames. Immutable serialized data is never relocated. Original skeleton, bone, material, player and WorldInit bodies compile against explicitly sized native records. GameWork/SysWork cover bootstrap, not full gameplay.
- Native bootstrap resets empty map/collision caches and applies original fog/light/player initialization. Actual IPD streaming, collision queries, camera and world draw consumers remain unlinked.
- Fixed queued TIM descriptor lifetime: original Fs_QueueStartReadTim copies the local image descriptor; native queue now does too. This removed reproduced nondeterministic KCET atlas corruption. Regression probe included.
- Existing movie/title/difficulty/OPTION/native-save work preserved. No parallel lane files, crate sources or submodule contents edited. Backends/pad remain source compatible.

## Verification and evidence
Rust fmt check, workspace/all-targets clippy -D warnings (precise feature enabled), and workspace tests pass: 247 unit/integration plus 3 doctests. Standalone psxgpu CPU tests pass: 36. Linked C uses /W4 /WX. Four GTE tests pass with precise-vertices; synthetic and real HERO/HB_BASE graphs pass original C consumers. MSVC x86/x64 wire layouts and arm64 LP64 native/SDK sizes/offsets pass, with the negative pointer assertion rejected. Broad original-header probe remains failing: exit 101, 37 C2118 diagnostics before truncation; no original broad checks disabled.
Nine implemented replays passed twice: 18 runs, 9 matching PNG SHA-256 comparisons. Evidence: ../../private/work/core3/milestones/20261007T130453535579Z/results.json. A wrong-state negative control exits 1. Strict full-brief gate exits 2 for pending gameplay/screens/maps. Remote CI, actual iOS runtime, manual controller walking and PS1 golden-image comparison were not run.
Deepest-point evidence: ../../private/work/core3/new-game-map-load-guard.log and .png. Earlier corrupted-hash evidence: milestones/20261007T130209388924Z (retained privately). See docs/core/ABI.md, GTE.md, OVERLAYS.md and MILESTONES.md.

## Next work and open questions
1. Migrate s_MapOverlayHdr and referenced native data/callback types. Namespace/reset MAP0_S00/MAP0_S01 including shared included C and mutable statics, then generalise to 43 maps. Do not treat declaration-only PortMapHeader as a loaded map.
2. Add owned native IPD model-instance/collision graphs, chunk loading, camera, world draw and original gameplay/player update consumers. Current empty bootstrap caches cannot stand in for actual collision or a walkable world.
3. Link real map loading/player map animations, let original title/opening/startup execute, and add first_map.txt only after actual input-driven movement and player-position assertions pass with private screenshots.
4. Exercise two area transitions/back; SAVELOAD and STF_ROLL remain unlinked. Native save storage exists but memory-card UI/settings persistence are still open.
No user decision or tool installation is required for the current next step. The remaining work is implementation inside the lane's ownership.

## Commits
f16acd2: native PsyQ/COP2 GTE integration. The subsequent native-world/bootstrap commit is in git log. REPORT.md is the uncommitted gitignored handoff.
