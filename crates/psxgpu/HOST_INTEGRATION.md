# gpuwire host handoff

`host/src/gpu_wgpu.rs` implements the existing `GpuBackend` using
`Processor<WgpuRenderer>`. It strips PsyQ tags, retains GP0 state across ordering
tables, translates host clip/offset rectangles, preserves packed RGB24 transfers,
and provides synchronous VRAM reads. Initialization fails explicitly if the GPU
is unavailable. GPU errors are retained for the native host to stop at a frame
boundary. No fallback to `Raster` occurs after selecting wgpu.

The new module also owns nearest, unorm window presentation, scaled screenshots,
and argument handling for `--renderer wgpu|soft`, `--scale 1..8` (default 4),
`--stats`, and `--wide` / `--16:9`. Widescreen is disabled by default and stretches
presentation; it does not add camera geometry or change drawing clips. The normal
window preserves the previous 640x448 aspect across framebuffer resolutions.
Each queued GPU frame owns a separate texture to prevent the worker overwriting
a frame while the UI presents it. RGB24 scanout comes from the same native VRAM.

Native pixels are still read each frame because `GpuBackend::frame` and the
existing host's visibility/replay checks require them. Presentation binds the
scaled GPU texture directly. Statistics measure frame intervals, including
pacing, game work, decoding and native readback; they are not isolated GPU timings.
Screenshots use scaled scanout storage dimensions: the title is 1280x1792 at 4x,
while its correctly aspect-adjusted window is 1280x896.

## Ownership boundary and required hooks

The working branch leaves core3's `native.rs`, `psxspu/Cargo.toml` and the root
lockfile untouched. **The parsed renderer flags are not connected to the game
until core3 installs the hooks. This branch is not a completed integration.**

The exact reviewable patch and a compiled private integration copy are under:
`C:/Claude Projects/Silent Hill iOS/private/work/gpuwire/`.
Core3 must replace both hardcoded Raster factories with `gpu_wgpu::backend()`,
send `frame_texture(display_enabled)` alongside each native Frame, use
`Presentation` / `window_size()`, and call `frame_error()`, `record_frame()` and
`screenshot()`. Window ownership changes from Rc to Arc for safe wgpu surfaces.
Main's screenshot guard must accept `private/work/` so gpuwire output is allowed.
Main already removed the nested workspace tables; do not restore them. Root gates
in this older branch still stop at psxspu's table. No rebase/merge was performed.
`core-hooks-main.patch` replaces the old `core-hooks.patch`: it targets main
`a08d10e91c01ac407de261a861cb27bcb5011cad`, retains its audio/touch/GTE additions,
and includes the three-line newer-trait override forwarding `packet_precise` to
the adapter's inherent method. Apply after merging gpuwire; check with
`git apply --ignore-space-change --check` for Windows line endings. Its native/main
hunks were checked against main and its adapter hunk against this branch.
Keep host's psxgpu dependency wgpu-enabled when resolving core3's
`default-features = false` dependency line. These are reviewable proposals, not
committed edits to core3-owned files.

## Measured parity and default decision

The private integration copy of main a08d10e plus the proposed hooks/brightness
guard was compared against unchanged host Raster at 1x on all nine entries in
`docs/core/replays/milestones.json`. Both renderers passed all nine state/visibility/
movie expectations with identical checkpoints. Tests explicitly use `--audio off`:
main's unconnected SPU factory otherwise rejects its default audio request.
The chosen image gate allows **at most 9 RGB8 levels per channel at every pixel**,
one expanded native 5-bit quantization step. It exempts no pixels or coverage.

| Replay | Different pixels | Maximum channel delta | Image gate |
|---|---:|---:|---|
| kcet | 0 | 0 | pass |
| movie-intro | 0 | 0 | pass |
| movie-end-title | 26255 | 9 | pass |
| title-skip | 25964 | 9 | pass |
| new-game-menu | 25919 | 9 | pass |
| options | 3331 | 25 | fail |
| options-reentry | 3331 | 25 | fail |
| options-brightness | 87 | 255 | fail |
| options-controller | 407 | 9 | pass |

Evidence: `private/work/gpuwire/parity/20261007T134617420313Z/results.json`.
The fallback uses half-pixel polygon sampling, omits dithering, truncates line
coordinates, and separately truncates blend halves. psxgpu uses its PS1 integer
rules. The host adapter's synthetic gradient/diagonal-line test matches psxgpu's
software VRAM exactly while differing from the fallback. Earlier unguarded
brightness analysis found opposing one-pixel diagonal coverage shifts.
Main-options differences include its small Gouraud bullet quads.
These observations explain candidate causes; they are not hardware certification.

## Repeated 4x gate and upstream defect

The first private `tools/milestones.cmd --available --repeat 2` run selected wgpu
explicitly at scale 4. It stopped on brightness's second PNG hash: six scaled
pixels differed although both state checks passed. Two private submission traces
then differed in 116 G3 packets; native VRAM checksums matched at every scanout.
The changed packets contain a cyan, nearly flat triangle with a varying first X
coordinate. This is incoming geometry variation, not evidence of a GPU race.

Pinned `options.c` uses `dir = 0` when neither arrow is held, then iterates
`i = dir - 1; i < dir`, reading `BORDER_ARROWS[-1]`. A three-line proposal in
`brightness-upstream.patch` guards this generated-source loop with `dir != 0` and
marks the correction `// PORT:`. It changes only the private verification copy of
core3's `tools/prepare_option.py`. Core3 must review/install this undefined-read
correction; gpuwire must not discard or rewrite legitimate submitted triangles.

With that proposal applied only in the private main copy, all 18 executions of
the nine replays at 4x pass, with nine matching repeated PNG hashes. Evidence:
`private/work/gpuwire/milestones/20261007T134619383215Z/results.json`.
The runner originally printed a nested private route; the artifacts were relocated
here without changing any PNG, and all 18 recorded hashes were rechecked.

## Final verification on the resumed run

- Private main a08d10e + proposed hooks: fmt check, workspace/all-target Clippy
  `-D warnings`, release build and C /W4 /WX pass; workspace tests: 274 passed,
  0 failed, 3 existing ignored tests. This main excludes GPU/SPU
  from workspace members, so psxgpu was also checked independently.
- psxgpu manifest: fmt/Clippy pass with wgpu; tests 43 passed, 0 failed. Without
  default features: Clippy passes and 36 tests pass, 0 fail.
- Eight host adapter tests include packed RGB24 at 1/4/8x, immutable queued frames,
  native VRAM unchanged by precise sidecars, and invalid-frame rejection. A device
  teardown crash found by the precision test was fixed by releasing the cached
  frame when its backend drops; retained UI frames remain readable.
- Wgpu title: 1280x896 window, final frame presented, 16.672 ms mean / 41.408 ms
  max interval / 59.98 fps. These include pacing/decode/readback, not GPU-only time.
  Private PNG: `title-wgpu-core3-final-4x.png`, 1280x1792 storage dimensions.
  Wide 1592x896 and soft 320x224 windows also present their final frames.
- Local branch root fmt/Clippy/test and `tools/milestones.cmd` stop before checking
  code at its old nested psxspu workspace. Private-copy results do not make those
  root gates green. The hooks and brightness correction remain director requests.

**Keep soft as default.** Do not relax the image gate or alter the PS1 renderer
merely to pass it. Core3/director must reconcile the fallback's raster rules or
establish an independently justified reference/tolerance before the default switch.
Full gameplay, independent PS1 goldens and Metal/iPhone runs remain unverified.
