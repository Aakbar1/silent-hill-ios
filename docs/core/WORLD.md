# Street checkpoint - 8 October 2026

`first_street_full` passes on `lane/street`. The 1280x896 private capture is
`private/work/street/first_street_full.png`: Harry is mid-walk, building fronts
are visible on the left, with roadside trees/canopy, pavement, lamp poles,
textured road and the original grey-purple fog. The white diamond patch is gone.
PNG SHA-256: `e63aa99e6cd031cab94dfa025ad38903826595410c40d97eccc7ba143772ec84`.
The scene is produced by the original New Game/movie/skip/gameplay path, with
no injected positions or completion flags. At VBlank 3650 it has 162 native
colours, 17 world-model draw calls, move speed 5724 and held walking input 0x10.
This capture uses the existing `first_map.txt` replay with the final run input
changed to walking (`3540 8010` -> `3540 0010`) to show the building fronts.
The independent movement gate still uses the unchanged walk/turn/run replay.

## Fixes and services

- The distant white diamonds came from reversed `SetPriority` arguments:
  PsyQ takes **check mask, force mask**. The fog background must carry bit 15;
  transparent billboard texels then retain that background during the fog pass.
  A native packet regression checks both software and wgpu against the expected
  masked-background/textured-pixel/fog result. Before and after captures at
  VBlank 3380 match between backends at 1x, pixel for pixel. The TIM palette,
  billboard texture page/CLUT and original OT linkage were inspected; the shared
  mask shim was the fault. Evidence: `before-wgpu.png`, `after-soft.png`,
  `after-wgpu.png`, `parity.json`, `test-mask.log` and `sdk-mask.txt` in the street private tree.
- World projection now uses bounded native triples, retaining unsigned SZ for
  fog lookup and original signed mesh culling/far limits. The regression checks
  a nonzero offset, final two vertices, behind/far depths and untouched tails.
  The static view uses framebuffer y=32, preserving the original CLUT rows.
- Original model lookup, common-item material setup, object registration,
  deduplication, 29-slot limit, transforms and drawing are linked. Native
  sidecars preserve preceding shared workspace offsets. MAP0_S00's local no-draw
  registration helpers are rebound to `port_world_object_name_set` and
  `port_world_object_add`. The existing sole native-global fixture checks a
  nonempty list, duplicate rejection, geometry/rotation packing, capacity and reset.
- Character rendering uses the original `field_6` and optional `field_8` metadata
  for all registered character IDs. The Harry-only renderer guard is removed.
- Original held-item selection/loading, hand attachment/mesh swap and model draw,
  lighter texture load/flame draw, overlap light/tint and lens-flare occlusion,
  smoothing and drawing are linked. Generated gameplay calls use these services;
  shared compatibility stubs remain untouched. Lighting shares the player environment.

## Verification and reproduction

Release MSVC `/W4 /WX`, fmt, workspace/all-target Clippy with `precise-vertices`
/ `-D warnings`, and workspace tests pass: 302 passed, zero failed, five opt-in
tests ignored. Available milestones pass 10/10; `first_map`, `first_map_view`
and `first_street_full` pass separately. The movement gate retains 422 samples,
9173 Q12 units walked, right turn 1711, and running at 2.86 times walking speed.
`first_map_view` has 170 native colours. Arm64 clang frontend: 39/39 C units,
with warnings denied; this is not an Apple SDK/device runtime result.
Logs and milestone results are under `private/work/street/`. The private
`run-milestones.py` changes only evidence destinations in the unchanged evaluator.
The all-game manifest still has unrelated pending requirements and is unchanged.

```text
tools/dev-cargo.cmd test --release -p silent-hill-boot first_street_full -- --ignored --nocapture --test-threads=1
python tools/prepare_render.py --check-clang "C:/BuildTools2022/VC/Tools/Llvm/x64/bin/clang-tidy.exe"
```

`SH_RENDER_TEST_BACKEND=soft|wgpu` and `SH_RENDER_TEST_SCALE=1..8` select
independent diagnostics. `SH_RENDER_TEST_LANE=street` keeps `first_map_view`
evidence in this lane. Cargo and the extended arm64 checker bind map callers
**after** map generation. Standalone SDK generation should do the same before
runtime use; its independent compile check retains the map-local compatibility helpers.

Open coverage: the daytime street has empty hands and no active flashlight.
Equipped weapons, visible flame/overlap/flare in a real night scene, NPC variants,
water reflections, PS1 golden imagery and actual iOS/device execution are not
certified by this capture. Water-reflection rendering remains a named guard.
Opening events/messages and transitions remain the other lanes' work.
Director: register this opt-in milestone in the shared manifest/state and use the
39-unit arm64 checker in CI. No main/merge/push, system installs or assets in Git.

---

## Historical world checkpoint (superseded by the street result above)

# World rendering checkpoint

`first_map_view` draws MAP0_S00 after the real New Game/opening/skip path, with
Harry, road geometry and the original fog preset. It is a **static rendering
milestone**, not a walking/gameplay pass. After the world/player reconciliation,
the player guard is state 11 / step 2 / VBlank 2266 (`Bgm_Update`). The separate rendering
capture returns without advancing or replacing that dispatcher.

The private 1280x896 capture is `private/work/world/first_map_view.png`.
It shows textured asphalt, a white road marking, Harry in the loading animation's
running pose and grey-purple distance fog. No buildings, movement, NPCs, events
or map transitions are certified by this view.

## Linked rendering

- The parity lane's `host-raster.patch` is applied. Software now delegates to
  psxgpu's reference processor; Gouraud, coverage, dither and lines share its rules.
- Desktop/headless default is wgpu at 4x. `--renderer soft` retains the reference
  backend. All ten available replays have identical decoded pixels between
  software and wgpu at 1x (software captures were reduced from nearest-scaled 4x).
- Original chunk/subcell/model traversal, mesh drawing, material UV/CLUT data,
  skeleton drawing, billboards, fog/lighting transitions, fog background,
  brightness overlay and loading-screen motion blur compile natively.
- Generation reads the pinned GPL decomp only. Owned IPD/ILM/TIM/ANM graphs are
  consumed at runtime; no assets, game executable bytes or captures enter Git.

## Native boundaries

`prepare_world.py` invokes `prepare_render.py`, including in standalone SDK
preparation. Rendering prep rebinds four explicit rendering guards in generated
`gameplay_consumers.c` to `port_render_*` entry points. The player lane's source
files are unchanged. `WorldGfx_Draw` remains the original public world entry point.
The generated rendering tests are included by a small append to `native.rs`.

The native 3732-byte scratch record separates the byte-indexed vertex, depth,
fog, normal-index and color arenas into 256 slots each. The upstream nominal
arrays overlap: real HERO has a 57-vertex/57-normal mesh and an opening road mesh
has 94 vertices. Native bounds are checked before drawing; final SDK triples use
bounded local inputs and store only live results. Numeric matrix unions remain
intact. Synthetic tests verify projection, untouched tail slots, normal colors
and the original custom light-matrix register order.

Environment-transition work now shares the player's appended 400-byte
`PortSysWork.gameplayEnvironment`; the independent render sidecar was removed.
The earlier `field_2388` flashlight flag is synchronized on each effects update,
preserving preceding FFI offsets. `shared_types.h` is generated by one helper in
`prepare_gameplay.py` from pinned `game.h` and `bodyprog.h`. Pointer-free records
keep their original sizes (44/52 bytes); the native pointer-bearing workspace
has size/offset assertions. Preset indices use the original two `u8` fields,
with a 2-byte assertion. During loading, water-zone data comes
from the already active map descriptor until the streaming workspace publishes
its `mapInfo`. Both native record sizes/offsets have explicit assertions.

## Reproduce and merge

Build with `tools/dev-cargo.cmd build --release`. Run the capture with:

```text
tools/dev-cargo.cmd test --release -p silent-hill-boot first_map_view -- --ignored --nocapture --test-threads=1
```

The test requires the verified owned US 1.1 disc at the established private path.
`SH_RENDER_TEST_BACKEND=soft|wgpu` and `SH_RENDER_TEST_SCALE=1..8` select isolated
diagnostic variants. Captures always go to private/work/world. The test verifies
the original movie counters and exact retained startup boundary before rendering.
It also checks the exact BGM guard name/live state/step/VBlank in C. The last
presented host frame is state 11 / step 0; this precedes the guarded update.

Rendering owns the shared environment functions and preset tables; player
startup calls those same functions. Public rendering bridges forward to the
world services, and the player's duplicate no-draw `WorldGfx_Draw` was removed.
Include `render_services.h` for new native rendering callers.
World-object registration, held items, non-Harry metadata, volumetric overlap,
lens flare and lighter flame remain explicit guards. They were not exercised by
MAP0_S00's static view. UIKit's separate backend selection and an actual Apple
SDK/device build remain outside this lane.

## Verification

Current merged verification is recorded in [RECONCILE.md](RECONCILE.md).
`first_map_view` passes at the BGM guard with 96 colors, fog near/far 3456/3712,
RGB (108,100,116), and an empty queue. Its private capture was visually reviewed
(Harry, road marking and fog); no PS1 golden/device comparison was run.
The changed pixels reflect the later original startup boundary, not a claim of
visual parity with the earlier lane-only frame. SHA-256:
`3788872ec1d1c08110cef24957570e72eac0e34d0b8d10ceac52c7b7c89aaeb2`.

The following verification is historical, before the player merge:

Release `/W4 /WX` build, fmt check and workspace/all-target clippy with
`precise-vertices` / `-D warnings` pass. Workspace tests pass: 297 unit/integration
and 3 doctests; 4 opt-in tests are ignored in the default run. The existing
MSVC x86/x64 and arm64 layout gates pass, including the negative pointer control.
All 20 native C units pass the installed arm64 frontend with `-Werror`.

The default wgpu available suite passes 20/20 runs with 10 matching repeated PNG
hashes (`private/work/core5/milestones/20261007T171034132315Z/results.json`).
`first_map_view` passes four isolated runs (wgpu 4x twice, wgpu 1x, software 1x):
the 1x pixels match exactly and the two 4x PNG hashes match. The final view has
106 native colors, fog near/far 3456/3712 in Q8, fog RGB (107,99,115), and an empty
file queue. Its PNG SHA-256 is
`ea37c8852fbc165a8e722c4c610b6af71c5ee68e3c4b12193bb89bb787d2756d`.
The original three-leg `--probe-world --audio off` streaming/camera/reset test
also passes using the default backend. No PS1 golden-image or device comparison
was run.

Evidence is under `private/work/world/`: build-final.log, clippy.log, tests.log,
clang-render.log, layouts.log, layouts-arm64.log, first_map_view.log and
milestones-default.log. The rendering arm64 command extends the unchanged
18-unit gate with both new C units:

```text
python tools/prepare_render.py --check-clang "C:/BuildTools2022/VC/Tools/Llvm/x64/bin/clang-tidy.exe"
```

The all-game milestone manifest still lists unrelated pending gameplay/screens.
The available suite and this rendering milestone must be distinguished from that
full-game gate; no pending requirement has been removed or weakened.
