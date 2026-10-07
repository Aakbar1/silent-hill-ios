# GPU lane checkpoint — 2026-10-07

## gpuwire host integration checkpoint

- New host adapter/presenter/CLI module is implemented; actual native.rs hooks,
  psxspu workspace/parent lockfile edits stay outside this lane's ownership.
  Parsed flags alone do not select the GPU in this working branch. Full pass is pending.
- Resumed private main a08d10e plus proposed hooks/brightness guard: fmt, Clippy,
  release C /W4 /WX pass; workspace 274 tests pass, 0 fail, 3 existing ignored.
  Main excludes GPU/SPU members; explicit GPU gates pass 43 tests with wgpu and
  36 without it, plus Clippy both ways. Eight host adapter tests pass.
- 1x game state checks pass 9/9 with each renderer; RGB24 movie PNG is exact.
  Image comparison still passes only 6/9 at <=9 channel delta; keep soft default.
- Final 4x repeated suite: 18/18 executions, 9/9 identical PNG pairs. Evidence:
  private/work/gpuwire/milestones/20261007T134619383215Z/results.json. Both suites
  explicitly disable audio because main's SPU factory is still unconnected.
- Windowed wgpu title at 4x presents its final frame in 1280x896: mean 16.672 ms,
  max 41.408 ms, 59.98 paced fps (includes decode/readback). GPU PNG is 1280x1792:
  private/work/gpuwire/title-wgpu-core3-final-4x.png. Wide 1592x896 and soft 1x
  320x224 window checks also present final frames.
- Initial repeated 4x suite found six varying brightness subpixels. Private
  traces show 116 different incoming G3 packets but identical native VRAM
  checksums. options.c reads BORDER_ARROWS[-1] with no direction held. A marked
  generated-source guard is proposed for core3; corrected private repeat gate passes.
- Do not hide malformed incoming triangles, relax image gates or change PS1
  raster semantics to match the fallback. Core3 must reconcile its calibration.
- Hardened invalid-frame/presentation checks and initial surface configuration.
  Fixed cached-device teardown crash; queued frames survive backend destruction.
  Precise sidecars preserve native VRAM and change only scaled geometry; director
  must add the newer trait override from core-hooks-main.patch after branch merge.
- Evidence and exact proposals: private/work/gpuwire/. core-hooks-main.patch targets
  main a08d10e and supersedes old lockfile/workspace hunks; choose wgpu-enabled host
  dependency at merge. See HOST_INTEGRATION.md. No rebase/merge/push/main edits.
  No Metal/iPhone verification. Local root gates still stop at old psxspu table;
  main has removed nested tables. Do not restore them or claim the full pass.

## Standalone GPU goal and ownership

Standalone native GPU/GTE backend for core's host-owned GpuBackend adapter. Own only
`crates/psxgpu/`; root REPORT.md is the explicitly required uncommitted handoff.
No game content, system installations, workspace edits, merges or pushes.

## Implemented

- Streaming GP0/GP1, checked disk32 OT traversal, all polygon/line/rectangle opcode
  families, pages/CLUTs, texture windows/flips, four blend modes, dithering, masks,
  draw area/offset, transfers, aborted uploads, 480i DFE/field behavior and scanout.
- Software 1x reference; tiled wgpu D3D12/Metal path at 1..8x; separate integer VRAM
  and sharp subpixels; GPU RGBA scanout including packed 24-bit video.
- Native C GTE: all 22 commands, flags/FIFOs/register semantics, hardware UNR divider,
  reserved MVMVA quirks. GPL/BSD provenance retained in NOTICE.md.
- Opt-in presentation precision via vertex sidecars and wider clipping. Integer
  GTE/game/VRAM results remain authoritative. Widescreen needs core geometry/buffers.
- PSXCAP1 bounded capture/replay; frame-by-frame full-VRAM comparator and CLI.

## Verified

- Stable Rust 1.99/MSVC 14.44: fmt check, all-target Clippy -D warnings (with and
  without wgpu), C /W4 /WX. Debug and release tests: 43 passed, 0 failed each; software-only: 36.
- Zero differing VRAM words at 1x for 96 primitive opcodes, texture depths, blend,
  dither/mask combinations, feedback/transfers and randomized clipping/windows.
- 17 GTE tests include independent UNR checks for all 65,535 nonzero denominators.
- D3D12 RTX 5070 Laptop GPU, driver 32.0.16.1088: 600 completed synthetic 4x frames,
  1,200 textured/Gouraud triangles (400 transparent), clear,
  framebuffer copy/blend and 1280x960 GPU scanout: **205.19 fps / 4.874 ms per frame**.
- Private four-frame synthetic capture CLI: 34,684 GP0 words, 3 GP1 words, zero
  differences at all frame boundaries and EOF. No actual game capture was available.
- Evidence outside Git: `C:/Claude Projects/Silent Hill iOS/private/work/gpu/`;
  `benchmark-final.log`, `replay-final.log`, `synthetic-validation.psxcap`.

## Decisions and remaining work

Integer ordered rendering was chosen over float GTE, conventional alpha blending or
a depth buffer. CPU/GPU agreement shares geometry setup and is **not independent PS1
hardware certification**. Primitive texture snapshots deliberately omit hardware
texture-cache residency/self-feedback; GPUSTAT is functional, not cycle accurate.
The 128-word overlap model follows Mednafen; exact hardware overlap traces are pending.

Core must add workspace membership (remove this crate's [workspace]), use wgpu 24.0.5
for shared devices, implement its trait adapter and wire C COP2/PsyQ helpers and precise
vertex identities. Then capture real map fog/blur/noise and item TMD cases privately,
compare against a hardware/emulator golden, and run Metal/arm64 CI and phone throughput.
See INTEGRATION.md for exact calls, format and limits. No global state documents changed.
