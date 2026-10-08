# iosrun checkpoint - 8 October 2026

Goal: real iPhone host with sharp Metal graphics, menu/walking touch, lifecycle
and data-free simulator evidence. Branch: `lane/iosrun`. Apple CI remains the
source of truth; no Apple tools ran on this Windows PC and nothing was pushed.

## Implemented

- Live safe game-view bounds/density request native scale capped at 3x. A five
  frame shared wgpu probe warms once and measures four samples including both
  readbacks; choose the largest scale <=12 ms worst sample, with 1x last resort.
  Scaled pixels reach UIKit. Raster mirrors packets/VRAM for logged init/runtime
  fallback. Original aspect/logic/camera and debug-overlay defaults are retained.
- UIKit stable multi-touch IDs/local points feed the existing engine and live C
  provider. Menus keep PREV/NEXT/OK/BACK; exploring permits walking while the
  original player-control gate permits it. Display-copy overlay uses the same
  geometry. Cancellation overrides cached held input; queues are bounded.
- Worker pauses before audio/GPU/tick work; background-task acknowledgement allows
  current synchronous saves to finish. Resume resets pacing without catch-up.
  Interruption/route/media reset rebuild only the physical cpal sink, retaining
  SPU/RAM/CD/sample clock. Original sync+atomic save-point writes are unchanged.
- Existing fresh-simulator no-disc importer/OCR remains. Same app/simulator then
  runs SH_IOS_SMOKE: real Metal packets/readback/presentation, real cpal callbacks,
  Objective-C/Rust touch bridge -> engine -> pad checks, synthetic save replacement,
  synthetic interruption/route observers, Settings background/foreground, and
  SPU RAM/register preservation. Pixel checks reject blank/importer-only images.
- Small additive shared hooks: host lib/build/GPU/SPU, plus GPU build's explicit
  Windows Rust-only diagnostic guard. Real Apple CI cannot use that guard.
  Friend instructions updated; no game bytes/captures or private scratch created.

## Local evidence

- Submodule init: pinned d9e28f8315c7938117224f21516786d9d149a145.
- Workspace fmt/Clippy/build and x86/x64 C layouts passed; tests 304 passed,
  0 failed, 4 ignored. Pipeline tests 5/5; actionlint 2 workflows/0 diagnostics.
- Both iOS targets: host + app Rust-only Clippy passed (4 checks). Apple C, UIKit,
  linking, Swift pixel checking and simulator execution are excluded.
- Windows release GPU fixture: RTX 5070 Laptop/D3D12 selected 3x, mean 3.935 ms,
  max 4.320 ms (4 samples); real-GPU forced Raster fallback preserves VRAM.
  These are startup synthetic samples, not iPhone/whole-game measurements.
- Detailed commands/logs: ignored `ios/.tools/verification/`; root REPORT.md.

## CI must show before goals 1-4 can be accepted

1. Real clang GPU/game C + UIKit compile with warnings as errors; both locked
   Apple targets link; device IPA unsigned; simulator installed/signed/launched.
2. `simulator.png`, `importer-ocr.txt`, landscape log and no disc/native game start.
3. `game.log`: live points/density; measured IOS_GPU probe; selected backend=Metal,
   scale 1..3, stats=false/wide=false; UIImage installed; 90 renders; real audio
   callbacks >0; touch=PASS and saves=PASS. No Raster fallback/panic/GPU error.
4. `synthetic-metal.png`/`synthetic-pixels.txt`: both synthetic triangles visible.
5. Two synthetic notification recoveries with continuing audio callbacks/sample
   clock and spu_state=PASS; a NEW pause acknowledgement after Settings launch,
   same-worker resume/new rendering/audio progress; resumed pixel check passes.
6. `synthetic-resumed.png`, `resumed-pixels.txt`, logs uploaded even on failure.

## Open / director handoff

- Director must push/run GitHub CI; this lane cannot mark Apple goals passed yet.
- Simulator notification fixtures do not certify real calls/headphone changes.
  Friend must check touch ergonomics, sound/silent switch, actual interruptions,
  save/relaunch/import cases and gameplay on the phone. No full playthrough claim.
- Startup readback timing omits core/overlay cost; full-game iPhone frame time,
  memory and A/V latency remain unmeasured. UIKit currently copies scaled pixels.
- Inventory/combat/ownership/puzzles need core-owned context accessors. Shared
  worker Host initializer/prefix ABI must be reconciled after parallel integration.
