# Core integration

For the current parity/default-switch decision and 4x/6x measurements, see
[PARITY.md](PARITY.md). The host fallback is not the independent PS1 oracle.

`psxgpu` is standalone. Its `Renderer` trait is an internal raster target, not a replacement
for the host-owned `GpuBackend` trait. Implement the host trait in `host/` by forwarding to
`Processor<SoftwareRenderer>` or `Processor<WgpuRenderer>`. The host trait did not exist in
this worktree at handoff, so its adapter is a core-lane responsibility.

## Cargo and platforms

1. This crate has no nested `[workspace]` table; keep it that way. The gpuwire
   baseline includes `crates/*`, while current main excludes GPU/SPU from its member
   list. Run this crate's manifest gates explicitly when it is excluded. The old
   gpuwire baseline still has psxspu's table, blocking root gates; main has removed it.
   See `HOST_INTEGRATION.md` for the merge handoff; do not rebase or restore tables.
2. The host dependency is `psxgpu = { path = "../crates/psxgpu" }`. The crate re-exports
   `wgpu` and `block_on` so the host shares the exact pinned GPU API without another dependency.
3. The GPU dependency is pinned to **wgpu 24.0.5**, with D3D12/Metal/WGSL enabled.
   Use that version in host to share devices/textures. `default-features = false` provides
   the software/GTE/capture paths without wgpu. Rust minimum is 1.88; verified stable is 1.99.
4. `cc` builds and links the native GTE static library automatically. Windows needs the
   existing MSVC environment; Apple CI needs its usual C compiler. No system installations
   were made. Little-endian x64 and arm64 are the intended platforms.

## Exact calls

```rust
use psxgpu::{Processor, SoftwareRenderer, Renderer, Rect};

let mut gpu = Processor::new(SoftwareRenderer::new());
gpu.gp1(0x0000_0000)?;                         // Reset state; VRAM retained.
gpu.gp0_words(&[0xe300_0000, 0xe407_ffff])?;   // Drawing area: full VRAM.
gpu.gp0_words(&[0x6800_00ff, 0x000a_000a])?;   // Red 1x1 tile at (10,10).
gpu.renderer_mut().flush()?;
let words = gpu.renderer_mut().read(Rect::new(0, 0, 1024, 512))?;
let rgba = psxgpu::scanout(&words, gpu.display, false)?;
```

| Host operation | Call / contract |
|---|---|
| GP0 port write / DMA words | `gpu.gp0(word)?` / `gpu.gp0_words(&words)?`; splits at any word boundary are accepted. |
| GP1 control write | `gpu.gp1(word)?`; reset command buffer aborts image input, retaining supplied pixels. |
| GPUSTAT / GPUREAD | `gpu.status()` / `gpu.read_gp0()`; downloads consume two halfwords per read. |
| OT submission | `gpu.walk_ot(&ram, head, max_packets)?`; returns visited packet count. |
| LoadImage / StoreImage | `renderer_mut().upload(rect, &[u16], mask)?` / `.read(rect)?`; exact word count required. |
| MoveImage / ClearImage | `.copy(src_rect, dest_x, dest_y, mask)?` / `.fill(rect, bgr555)?`. |
| DrawSync / frame boundary | `.flush()?`; for GPU completion explicitly call `.wait_idle()` on WgpuRenderer. |
| Interlace field state | `gpu.set_field(field)`; use the same field for scanout. |
| Software inspection | `gpu.renderer().vram()` is an authoritative 524,288-word slice. |

GP0 packet arrays contain **command words only**, without PsyQ/DMA tags. `walk_ot` consumes
the 32-bit tag itself: high byte = payload word count, low 24 bits = next **byte offset** in
the supplied arena; `0x00ffffff` terminates. It checks alignment, arena bounds, cycles and
the caller's budget. Never encode or truncate a host pointer into that tag. For host-side
native OTs, core should flatten packets in OT order or generate checked arena offsets.
Earlier packets remain executed if a later link fails validation.

Transfers wrap X/Y independently at 1024/512. Public direct transfers accept literal
dimensions 0..1024 / 0..512; zero is empty. GP0 image packets implement the hardware's
zero-as-maximum size encoding. `mask` is E6's low two bits; fill ignores masks and clears
bit15. Upload data is BGR555 plus bit15, or opaque halfwords for packed 24-bit MDEC.
Complete image input before bypassing the Processor to access its Renderer.

## GPU output without CPU readback

```rust
use psxgpu::{Processor, WgpuRenderer};
let renderer = WgpuRenderer::new(4).await?; // D3D12 on Windows, Metal on Apple.
let mut gpu = Processor::new(renderer);
// Submit the game's GP1 display state and GP0 draw stream here.
let display = gpu.display;
let frame = gpu.renderer().create_scanout_texture(display)?;
gpu.renderer_mut().render_scanout(display, false, &frame)?;
// Bind frame.create_view(...) in the host's surface blit.
```

Reuse the frame texture until `Display::dimensions()` changes. `render_scanout` flushes
pending draws, supports disabled display, packed 24-bit video, horizontal reverse and
interlace field selection, and writes RGBA8Unorm at `native_width*scale` by `native_height*scale`.
Use a **nearest** sampler and an unorm surface view (or explicitly compensate sRGB
conversion) to preserve the encoded PS1 RGB values. Interlaced output blacks the opposite
field; core may weave fields. Empty display ranges return an error; host should skip them.

To share the surface device, call
`WgpuRenderer::from_device(device.clone(), queue.clone(), adapter.get_info(), scale).await?`.
The device needs at least **6 storage buffers per shader stage** and a storage-buffer
binding limit of `1024*512*4*scale*scale` bytes (128 MiB at 8x). Scale is fixed at creation,
1..=8. Return validation errors to core instead of silently falling back. Explicit backend
selection for diagnostics is `WgpuRenderer::with_backends(scale, backends).await?`.

`native_buffer()` and `scaled_buffer()` expose GPU buffers after `flush`. Both store
one little-endian **u32 per BGR555+mask pixel**, not packed pairs: native stride 1024,
scaled stride 1024*scale. `read_vram()` / `read_scaled()` are synchronous debug/StoreImage
readbacks, not the presentation path. Each dependency batch samples a native VRAM
snapshot; potential texture-page/CLUT feedback forces an ordered batch boundary.
Non-overlapping copies retain sharp subpixels on the GPU; wrapped/overlapping copies use
the 128-halfword burst model with readback and retain sharp subpixels too, but cost more.

## Native GTE and optional precision

`native/psxgpu_gte.h` exports a 256-byte context (`data[32]`, `control[32]`) and five
namespaced functions: `psxgpu_gte_read_data`, `psxgpu_gte_write_data`,
`psxgpu_gte_read_control`, `psxgpu_gte_write_control`, `psxgpu_gte_execute`.
Zero-initialize one context per original COP2 execution context. The C dispatcher accepts
the COP2 word or its low 25 bits, decodes `sf/mx/v/cv/lm`, returns 0 on success and -1 on an
undefined operation without changing state. Register APIs implement sign/zero extension,
SXY/RGB FIFOs, IRGB/ORGB, LZCS/LZCR and FLAG semantics. No CPU emulation or global CPU
registers are involved. Cycle stalls are the host scheduler's responsibility.

The Rust `gte::Gte` wraps the same C context with `read_data`, `write_data`, `read_control`,
`write_control`, `execute`. All 22 documented commands are implemented: RTPS/RTPT, NCLIP,
AVSZ3/4, MVMVA (including reserved matrix/far-color quirks), OP, SQR, DPCS/DPCT, INTPL,
NCS/NCT, NCCS/NCCT, NCDS/NCDT, CC/CDP/DCPL and GPF/GPL.

Opt-in precision: call `gte.project_precise(vertex_index)` for indices 0, 1 or 2 **before** RTPS/RTPT, keep the returned
positions associated with that vertex, and submit the original complete GP0 polygon plus
these positions through `gpu.polygon_with_positions(&words, &positions)?`. Positions are
before drawing offset; their difference from the wire coordinate must be less than one
pixel. Preserve integer results for OT depth, culling, game logic, colors and fog. Native
VRAM stays on the integer path; only scaled presentation uses precise positions. Textures
remain nearest and affine. The software reference always renders integer geometry.
Already-projected GP0 coordinates alone cannot recover lost subpixel information.

Optional widening: `gpu.set_widescreen(true)` expands E3/E4 clipping to approximately
4/3 of its width while staying inside VRAM. **Core must supply wider-view geometry,
projection center/drawing offset, non-overlapping wider framebuffers and GP1 scanout
dimensions.** This switch alone does not make a correctly composed 16:9 game view.
Precision and widening are marked `// PORT:` and disabled by default.

## Capture/replay and verification

Capture path: `C:/Claude Projects/Silent Hill iOS/private/work/gpu/*.psxcap`, never Git.
Use `capture::create_private(private_root, "name.psxcap")?` for guarded, non-overwriting
creation. `CaptureWriter::new(Write)` also supports in-memory synthetic fixtures.
Record `.gp0(&words)`, `.gp1(&words)`, `.frame()`; start from reset and include initial
VRAM as normal GP0 uploads. Core should record at submission, before batching. This v1
format captures integer GP0/GP1 streams; it does not capture GTE or precise sidecars.

Wire format: eight-byte `PSXCAP1\0` magic, then records of little-endian u32 tag and u32
payload byte length. Tag 0 = GP0 words, 1 = GP1 words, 2 = frame boundary with zero length.
Words are little-endian. Records are bounded to 4 MiB; replay takes a cumulative byte
budget and rejects truncation, invalid lengths/tags, unfinished packets and mid-packet
frame boundaries. `capture::replay(reader, &mut gpu, byte_budget)?` returns word/frame counts.
`capture::compare_replay(reader, &mut software, &mut accelerated, byte_budget)?` checks
all VRAM words at **every** frame boundary and EOF, with bounded memory. The replay CLI
uses that comparison and returns nonzero on the first mismatch with frame/pixel details.

```powershell
cargo fmt --manifest-path crates/psxgpu/Cargo.toml --check
cargo clippy --manifest-path crates/psxgpu/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path crates/psxgpu/Cargo.toml
cargo test --manifest-path crates/psxgpu/Cargo.toml --no-default-features
cargo run --release --manifest-path crates/psxgpu/Cargo.toml --example benchmark -- 600 feedback
cargo run --release --manifest-path crates/psxgpu/Cargo.toml --example benchmark -- 600 feedback 6
cargo run --release --manifest-path crates/psxgpu/Cargo.toml --example replay -- 'C:/Claude Projects/Silent Hill iOS/private/work/gpu/map.psxcap'
```

The 1x comparisons require **zero differing VRAM words**, including masks. GPU tests fail
if hardware initialization fails; they are not silently skipped. The performance test
waits for completion every frame, excludes setup/warmup/readback, and includes 1,200
textured/Gouraud triangles, 400 transparent triangles, clear, optional framebuffer copy
and blend, and 1280x960 GPU scanout. It measures a synthetic workload, not game FPS.

## Remaining fidelity limits

Hardware texture-cache/CLUT residency and within-primitive self-feedback are not simulated;
sampling uses ordered uncached primitive snapshots. GPUSTAT readiness is functional, not
cycle/FIFO-accurate. In 480i with DFE=0, draws/fills skip the displayed line parity supplied
by `set_field`; core must drive the original field schedule. The burst overlap model follows Mednafen,
not an independently measured PS1 transfer trace. Real Silent Hill map/fog/blur/noise,
item TMD and hardware-golden capture comparisons are still required. Metal/arm64 builds
and phone throughput have not been tested on this Windows machine. See PROJECT_STATE.md.
