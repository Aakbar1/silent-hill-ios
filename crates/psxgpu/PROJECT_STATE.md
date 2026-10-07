# GPU lane checkpoint — 2026-10-07

## Goal and ownership

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
