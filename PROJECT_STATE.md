# core2 checkpoint

## Active core3 work
Branch lane/core3. Goal: exact GTE, native world bootstrap, opening map walking,
43 map overlays and area transitions. Own host (except gpu_wgpu.rs, spu_cpal.rs,
pad_touch.rs, platform_ios.rs), port, non-submodule game, tools, docs/core and
root Cargo files. Submodule initialised at d9e28f8315c7938117224f21516786d9d149a145.
Game bytes and captures must remain under private/work/core3.

GTE step: host delegates PsyQ/COP2 to psxgpu's native C engine, with exact
integer results by default. Build-time SDK macro lowering covers transfer,
command and composite helpers; 65 command variants cover all 22 commands and
match direct engine register results. `--features precise-vertices` adds optional
polygon metadata without changing logic or integer flags. Backends can opt into
the backward-compatible packet_precise hook; the menu raster ignores metadata.
Standalone psxgpu/psxspu workspaces are excluded by the root manifest rather than
edited. fmt, workspace/all-targets clippy -D warnings and workspace tests passed;
the three GTE tests also pass with precise-vertices. First map remains unproven.

## Inherited core2 goal and constraints
Reach title -> New Game -> first controllable opening map, then two area transitions/back and all 43 overlays. Own only the core2 brief paths. Branch lane/core2; no merge/push/main edits or system installs. Decomp d9e28f8315c7938117224f21516786d9d149a145 is read-only. Game bytes/captures remain outside Git under private/work/core2.

## Current result
Requested pass is NOT achieved. 3/5 screens linked (B_KONAMI, STREAM, OPTION); 0/43 maps. Original title and difficulty menus work. NORMAL confirmation at tick 1650 runs original save initialization, then stops at GameBoot_WorldInit: state 7, step 1, menu 3, native code 3/process exit 1. Opening movie and controllable gameplay are not reached. Title idle-demo loading is also guarded.

## Completed decisions and work
- STREAM uses psxmedia raw STR/XA decoding, bounded queues, 15 fps on virtual 60 Hz ticks, RGB24 VRAM/display through GpuBackend. Start skip and natural end return via original state handlers. CD PCM/stop use SpuBackend hooks; default sink logs absence. Audible sync/warm reset/alternate-intro semantics remain unverified.
- Original title/text/Joy/RNG and GPL sine table linked. Pointer-free save/input sizes and save offsets retain assertions. No raw PLM/IPD/ILM/DMS relocation or full gameplay assertions were weakened.
- OPTION namespaces exports and restores 10 globals + 14 local statics from compiled initial images. Dirty/reset probe covers all 24 objects. Main screen, brightness, controller bindings and return/re-entry have visually reviewed private captures. Other option subpages remain untested.
- Native save files preserve exact 636-byte payloads at LOCALAPPDATA/SilentHillIOS/saves/slot-NNN.shs, 330 stable identities. Real Windows tests cover replacement and malformed sizes. C hooks are ready; SAVELOAD/UI/settings persistence still unconnected. Cards still report absent.
- Menu raster draws untextured polygons/lines/fixed-size sprites; full PS1 raster calibration belongs to the GPU integration work. No psxgpu/psxspu edits. Frame output is checked for nonblack pixels; fresh filenames avoid stale image previews.

## Verification and evidence
fmt check, workspace/all-targets clippy -D warnings, workspace tests: 240 unit/integration + 3 doctests pass; linked C /W4 /WX passes. Disk32 MSVC x86/x64 and arm64 LP64 native/SDK/record assertions pass, including rejected negative control. Broad original-header probe still fails with 43 C2118 assertions (private full-abi-probe.log).
Nine replay cases each passed twice: 18 runs, 9 matching PNG SHA-256 comparisons. Seven-case results: private/work/core2/milestones/20261007T120623066930Z/results.json; two submenu results: 20261007T121326770302Z/results.json. Strict tools/milestones.cmd exits 2 for pending full-brief work; mismatched-state negative control exits 1. See docs/core/MILESTONES.md and OVERLAYS.md. Remote CI and full iOS runtime were not run.

## Next work
1. Port real gameplay work records and WorldGfx/model/animation/collision consumers onto checked native asset graphs; implement GTE/scratch/packet contracts. Do not turn declaration-only PortMapHeader or guards into successful map loads.
2. Wire SAVELOAD/native memory-card UI and settings storage; namespace/reset STF_ROLL with migrated native records.
3. Link MAP0_S00/MAP0_S01, prove opening/walking; then generic namespacing/reset across maps and two actual door/area transitions/back. Add passing replays only when proven.

Commits: 54a5fdb movie/headless service; 1e9641b title/Joy/save-init; fce402f OPTION/native file saves; replay/docs checkpoint is recorded in the branch log. REPORT.md is the uncommitted, gitignored handoff.
