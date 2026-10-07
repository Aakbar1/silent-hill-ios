# Renderer parity decision — 2026-10-07

**The psxgpu parity gate is GO. Flipping the current host default alone is NO-GO.**
Software and wgpu agree exactly on the reproduced rules, the complete synthetic
suite, and all 1,747 scanouts in the available private brightness submission trace.
The host fallback is wrong on the reproduced rules. Core must apply the proposed
host fix and pass its integration gates before changing the default. No raster
shader correction was justified: changing psxgpu to match the fallback would
introduce incorrect PS1 behavior.

## Independent rule reference

The software rasterizer is the executable oracle. Expected fixture words are
hand-calculated from the following specification rules, rather than obtained by
running either renderer. Both psxgpu paths share packet/plane setup; their agreement
alone is not independent hardware certification.

1. [PSX-SPX GPU misc](https://psx-spx.consoledev.net/ps1/gpu/misc/): dither adds the
   specified 4×4 offset before saturation and dropping three low bits. It applies
   to Gouraud/modulated polygons and lines, never rectangles or flat unmodulated
   polygons. Blend modes average, add, subtract, or add a quarter of foreground,
   with channel saturation. Modulation uses texel intensity × vertex color / 128;
   128 is neutral, 255 brightens and saturates. Indexed texture blending is gated
   by bit15 of the palette **value**, not by the palette index.
2. [Polygon commands](https://psx-spx.consoledev.net/ps1/gpu/render-polygon-commands/):
   quads split into vertices 0/1/2 and 1/2/3; lower/right edges are excluded.
   Affine Gouraud values must be evaluated at the PS1 integer raster position.
   A modern half-pixel center changes both coverage and interpolated color.
3. [Line commands](https://psx-spx.consoledev.net/ps1/gpu/render-line-commands/):
   final endpoints are included, coincident endpoints draw one pixel, and mono
   lines also dither. Coordinate truncation toward zero is not the PS1 DDA.
4. [Rendering attributes](https://psx-spx.consoledev.net/ps1/gpu/rendering-attributes/):
   15-bit output discards low RGB bits when dither is disabled. Transparent texture
   value 0000 is skipped; 8000 is drawable black and retains STP/mask information.

PSX-SPX's shorthand B/2+F/2 does not specify integer rounding adequately. The
independent pinned [Mednafen blend implementation](https://github.com/libretro/beetle-psx-libretro/blob/6f08abeaec4ec9f4611aa5b1bd0004fdddb4f9e4/mednafen/psx/gpu_common.h)
retains a carry when both channel values are odd: floor((B+F)/2), not
floor(B/2)+floor(F/2). Its [line DDA](https://github.com/libretro/beetle-psx-libretro/blob/6f08abeaec4ec9f4611aa5b1bd0004fdddb4f9e4/mednafen/psx/gpu_line.c)
and [polygon raster](https://github.com/libretro/beetle-psx-libretro/blob/6f08abeaec4ec9f4611aa5b1bd0004fdddb4f9e4/mednafen/psx/gpu_polygon.c)
corroborate the integer samples, interpolation bias, and line tie rules that the
high-level spec leaves unspecified. These are documentary/implementation references,
not a newly measured PS1 hardware golden. Existing GPL provenance remains in NOTICE.md.
The locally inspected PSX-SPX source revision was `6eb1dc8da721227f822f242bfbe5a68f4559c37c`.

## Reproductions and adjudication

All inputs in `tests/support/parity_cases.rs` are synthetic. Nine individually
named software tests assert expected BGR555+mask words; one GPU test compares
**all 524,288 native and scaled words** at 1× for every case. Another compares
native words and all native-coordinate scaled anchors at 4× and 6×.

| Milestone/rule | Reproduction | Legacy host result | Verdict |
|---|---|---|---|
| Title-family small RGB differences | `title_odd_channel_average`: B=F=1 | 2 words differ; max RGB8 delta 8 | psxgpu software/wgpu correct; host loses odd carry |
| Options / options re-entry, max 25 | `options_gouraud_integer_sample`: 4×4 Gouraud quad, red 0→192 | 16 pixels differ; max delta **25** | psxgpu correct; host samples x+0.5 instead of x |
| Options shading / controller small differences | `options_reentry_gouraud_dither_matrix` | 8 pixels differ; max delta 8 | psxgpu correct; host omits dither |
| Brightness high-contrast diagonal boundaries | `brightness_triangle_integer_coverage` | 8 pixels differ; max delta **255** | psxgpu correct; host triangle coverage shifts |
| Brightness diagonal lines | `brightness_line_rounding_both_slopes` | 8 pixels differ; max delta **255** | psxgpu correct; host truncates coordinates |
| Brightness / CLUT black and transparency | `brightness_clut_zero_black_stp` | 2 mask words differ, RGB equal | psxgpu correct; host discards texture bit15 |
| Brightness=255 modulation | `brightness_modulation_255_clamp` | 1 mask word differs, RGB equal | All three RGB results correct; host mask is wrong |
| Flat polygon vs rectangle truncation | `flat_polygon_and_rect_strip_low_three_bits` | 4 coverage pixels differ | All three covered-pixel RGB conversions correct; host coverage wrong |
| Mono-line dither vs modulated rectangle | `line_dither_but_modulated_rectangle_no_dither` | 2 pixels differ | psxgpu correct; host line lacks dither; rectangle RGB correct |

Thus brightness=255 is not evidence of an overflow or CLUT color bug: the
documented modulation is correct in all three. The reproduced 255 deltas arise
from high-contrast **coverage**, while the CLUT audit finds separately meaningful
mask-word differences that an RGB-only gate misses. The precise attribution of
each of the historical 87 pixels has not been measured; do not infer it from
their count. Options and re-entry share the same menu rules, not distinct GPU modes.

The old 9-screen PNG comparison is a fallback comparison, not a hardware golden.
The claimed associations above are rule reproductions supported by source inspection,
not an exhaustive per-pixel decomposition of all nine historical screenshots.
Its unchanged 6/9 tolerance result remains historical evidence in HOST_INTEGRATION.md.

## Private trace and patch evidence

Evidence root: `C:/Claude Projects/Silent Hill iOS/private/work/parity/`.
`tools/audit_host.py` compiles private source snapshots and reads gpuwire's original
kind/count/words trace. It edits neither host/ nor any game source. Its synthetic
comparison includes the actual legacy host Raster, the software oracle, and wgpu.

The available `gpuwire/brightness-commands-1.bin` trace spans boot, title, options
and brightness. `brightness-commands-1-legacy.csv` has **1,747/1,747 zero wgpu
VRAM differences**, and every software hash matches the recorded native GPU hash.
The final legacy-host result differs on 88 RGB pixels, max delta 255. This older
**unguarded** trace is not the later guarded 87-pixel milestone fixture. The
native anchors of `gpuwire/brightness-trace-1.png` match the oracle: zero pixels
different (`trace-image-check.txt`).

The proposed `host-raster.patch` replaces duplicate fallback raster math with
the same `Processor<SoftwareRenderer>` API, retains the existing four host Raster
tests, and changes the adapter gradient/line test from asserting disagreement
to asserting exact agreement. All nine synthetic cases then have zero host or
wgpu word differences. `brightness-commands-1-reference.csv` has zero host and
wgpu differences, including non-displayed VRAM, at all 1,747 scanouts. The patch
applies cleanly to this branch's host files (`git apply --check`); core5's parallel
changes may require reconciliation. This lane did not apply it to host/.

The shim also preserves texture bit15 and aligns wrapped transfers and inclusive
draw-area conversion with the existing wgpu adapter. Invalid host packets fail
explicitly rather than being silently ignored. This is a correction of the
fallback's GPU semantics, not a change to game commands. Core should review the
fail-fast policy alongside its existing adapter error handling.

## Performance on this PC

wgpu 24.0.5, D3D12, RTX 5070 Laptop GPU, driver 32.0.16.1088. Final release runs:
600 frames each, eight warmups of the **same** workload, completion waited every
frame. Includes clear, 1,200 textured/Gouraud triangles (400 semi-transparent),
framebuffer copy/blend and GPU RGBA8 scanout. Excludes setup, readback and game work.
These are completed CPU submission + GPU wait times, not GPU timestamp measurements.

| Scale | Scanout | Mean ms | p50 | p95 | p99 | Max | Mean margin to 16.667 ms |
|---|---|---:|---:|---:|---:|---:|---:|
| 4× | 1280×960 | 5.584 | 4.907 | 8.776 | 11.065 | 18.341 | 11.083 ms |
| 6× | 1920×1440 | 8.580 | 8.474 | 11.379 | 13.459 | 15.496 | 8.087 ms |

Logs: `benchmark-final-4x.log`, `benchmark-final-6x.log`. Scaled VRAM alone costs
32 MiB at 4× and 72 MiB at 6×, plus native/source buffers, scanout and host textures.
The PC 4× run has a frame over the 60 fps budget. Neither mean nor percentile
figures guarantee uninterrupted 60 fps under gameplay or thermal load.

For a phone, the available frame budget is 16.667 ms **minus** game, native
readback, presentation and other work. If everything else cost zero, the measured
completed path could slow by at most 2.98× at 4× or 1.94× at 6× on average; those
are budget ratios, not predicted phone performance. Metal and sustained target-phone
timings remain untested. Choose 4× for the PC default gate; do not promise 6×/60 fps
on a phone from these numbers.

## Core's required default-switch sequence

1. Apply/reconcile `private/work/parity/host-raster.patch`. It also updates the
   host test which currently expects the inaccurate fallback to disagree.
2. Install/review gpuwire's `brightness-upstream.patch` if still absent. The
   `BORDER_ARROWS[-1]` undefined read is an incoming geometry problem; raster
   parity cannot cure it. Integrate the existing native selection/presentation
   hooks: this branch's `native.rs` still hardcodes Raster in its headless path.
3. Run core's full tests and all current milestones against the corrected
   software oracle and wgpu at 1× with **zero** image differences. Use
   `tools/compare_host.py --tolerance 0`; include packed RGB24 and all available
   new milestones. Replay 4× twice and require identical native checks and PNG
   hashes with precision/widening disabled. This lane did not rerun those host
   milestones after the proposed patch or claim that their integration passed.
4. Once those gates pass, the production change in `host/src/gpu_wgpu.rs`,
   `Options::default`, is one line: `renderer: RendererKind::Wgpu,` (scale remains 4).
   Update its default-options test to expect Wgpu. Preserve explicit `--renderer soft`.

The documented renderer parity objection is resolved. The unchanged host default
cannot safely be flipped before steps 1–3. A phone release additionally needs
Metal/arm64 checks and sustained device measurements; cache residency, primitive
self-feedback and full gameplay hardware goldens remain the existing fidelity limits.

## Reproduce the lane gates

```powershell
git submodule update --init
cargo fmt --manifest-path crates/psxgpu/Cargo.toml --check
cargo clippy --manifest-path crates/psxgpu/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path crates/psxgpu/Cargo.toml
cargo clippy --manifest-path crates/psxgpu/Cargo.toml --no-default-features --all-targets -- -D warnings
cargo test --manifest-path crates/psxgpu/Cargo.toml --no-default-features
python crates/psxgpu/tools/audit_host.py --private-root 'C:/Claude Projects/Silent Hill iOS/private/work/parity'
python crates/psxgpu/tools/audit_host.py --private-root 'C:/Claude Projects/Silent Hill iOS/private/work/parity' --reference --trace 'C:/Claude Projects/Silent Hill iOS/private/work/gpuwire/brightness-commands-1.bin'
git apply --check 'C:/Claude Projects/Silent Hill iOS/private/work/parity/host-raster.patch'
cargo run --release --manifest-path crates/psxgpu/Cargo.toml --example benchmark -- 600 feedback 4
cargo run --release --manifest-path crates/psxgpu/Cargo.toml --example benchmark -- 600 feedback 6
```

Final manifest gates: 54 tests with wgpu, 45 software-only; zero failures.
Fmt and all-target Clippy clean with/without wgpu. Native C compiled with /W4 /WX.
The private proposed-host Raster regression harness passed its four existing tests.
See REPORT.md for the uncommitted director handoff.
