# Maps lane checkpoint

The lane links **43 native descriptor/data slices**, not 43 complete gameplay
overlays. MAP0_S00 retains the events lane's original provider. The other 42
descriptors retain their original points, events, spawns, camera paths, collision
triggers and source animation data. The common original room-index function is
linked across the maps, with checked disc-backed grids and its original ground
height/cache helpers. Four original empty object initializers are retained;
unavailable callbacks are typed fatal guards.
No map initializer is skipped to claim a smoke pass.

## Lifecycle and inventory

`tools/prepare_maps.py` discovers the pinned `src/maps/map*/*_header.c` inventory
and requires exactly 43 entries. `maps_registry.c` and `maps_inventory.json` are
generated in Cargo's native-source directory. The inventory lists every mutable
object compiled into each new data slice and every guarded callback. Initialized
objects reset by copying compiled const images; BSS resets to zero. Pointer fields
refer to that map's namespaced native objects. Callback-local state remains outside
the slice until its callback is migrated; this is not an exhaustive reset of all
unlinked upstream code.

Pinned symbol intervals bound missing byte/scalar tables, which are read from the
owned overlay at runtime. Pointer-bearing disc messages are not cast to native
pointers: MAP4_S00, MAP4_S06 and MAP6_S05 currently fail activation at a named
message-decoder guard. The unknown-size MAP1_S03 object table has only its source's
constant-index footprint allocated, with its consumers guarded. No asset bytes,
disassembly or game captures are generated into tracked files.

`port/maps_runtime.c` replaces the old single-map activation/descriptor exports.
The existing `Fs_QueueUpdate` completion path selects the registry entry, invalidates
the old active descriptor, resets the new image and loads bounded numeric data
before publishing it. A failed load leaves no active descriptor. The original
room-query ground cache is invalidated on activation and covered by the reset
fixture. Its broader integration with future MAP6 gameplay is not verified. The
original startup dispatcher still owns calling map init in production; activation alone
does not certify that dispatcher or a transition. The reset fixture dirties and
restores all 43 slices and checks distinct descriptor, point, event and animation
storage. `port/map.c` stays unchanged; the shared build compiles a generated copy
with its superseded lifecycle exports renamed.

## Debug warp

```text
python tools/prepare_maps.py --warp MAP0_S00:0
python tools/prepare_maps.py --smoke-all
python tools/prepare_maps.py --check-clang C:/BuildTools2022/VC/Tools/Llvm/x64/bin/clang-tidy.exe
```

The test-only command builds the release library test and runs each warp in an
isolated headless process against the verified owned disc. `--disc PATH` overrides
the private default. Spawn is an index in the original map-point array and is
checked against that map's compiled count. The warp establishes the original
screen/world setup and nonzero virtual timestep, queues the original overlay file
read, calls map init, uses `Chara_PositionSet`, streams chunks and attempts a world
frame. It does not initialize/render Harry or advance gameplay. Backend errors,
native guards and missing frames remain failures. Host application CLI parsing is
outside the lane; the `--warp` option is currently on this preparation/test tool.

## Transition constraint and requests

The requested MAP0_S00 walk-through exit and return is absent from the pinned game
data. Its sole overlay transition is event 33: `TriggerType_Tick`, requiring
`EventFlag_26`, destination MAP0_S01, point 4. MAP0_S01 event 6 exits to MAP2_S00,
requiring the pocket radio and button interaction; it has no return to MAP0_S00.
See `game/decomp/src/maps/map0_s00/map0_s00_events_data.c:304` and
`game/decomp/src/maps/map0_s01/map0_s01_events_data.c:57`. A faithful reversible
pair elsewhere, or the original story transition, needs the director's selection.
No flags or positions are injected to invent a gameplay transition.

Core/events requests: link the original `SysState_LoadArea_Update` system-state
dispatcher (currently guarded in `prepare_gameplay.py`), the destination object,
BGM/NPC/loading callbacks, and map effect texture services. Replace the current MAP0_S00-only
72-event traversal bound with each descriptor's event count before general
gameplay. MAP0_S00 callback bodies and `prepare_gameplay.py` were not edited.
Actual Apple SDK/device runtime still needs the iOS lane's verification.

## Verification and smoke results

Private evidence lives in `private/work/maps/`; smoke logs are named
by map and `smoke-results.json` retains native exit/guard results. A guard is a
precise blocker, not a successful map load/render or a gameplay pass.

| Map | MSVC / arm64 | Reset objects | Warp result |
|---|---|---:|---|
| MAP0_S00 | PASS / PASS | 33 | PASS: world frame, 48 colors / 369 primitives |
| MAP0_S01 | PASS / PASS | 24 | BLOCKED: map0_s01/Map_WorldObjectsInit |
| MAP0_S02 | PASS / PASS | 16 | BLOCKED: map0_s02/Map_WorldObjectsInit |
| MAP1_S00 | PASS / PASS | 16 | BLOCKED: map1_s00/Map_WorldObjectsInit |
| MAP1_S01 | PASS / PASS | 17 | BLOCKED: map1_s01/Map_WorldObjectsInit |
| MAP1_S02 | PASS / PASS | 17 | BLOCKED: map1_s02/Map_WorldObjectsInit |
| MAP1_S03 | PASS / PASS | 18 | BLOCKED: map1_s03/Map_WorldObjectsInit |
| MAP1_S04 | PASS / PASS | 19 | BLOCKED: black frame: lit_pixels=0, colors=1, primitives=2 |
| MAP1_S05 | PASS / PASS | 17 | BLOCKED: map1_s05/Map_WorldObjectsInit |
| MAP1_S06 | PASS / PASS | 16 | BLOCKED: map1_s06/Map_WorldObjectsInit |
| MAP2_S00 | PASS / PASS | 17 | BLOCKED: map2_s00/Map_WorldObjectsInit |
| MAP2_S01 | PASS / PASS | 19 | BLOCKED: map2_s01/Map_WorldObjectsInit |
| MAP2_S02 | PASS / PASS | 17 | BLOCKED: map2_s02/Map_WorldObjectsInit |
| MAP2_S03 | PASS / PASS | 20 | PASS: world frame, 37 colors / 211 primitives |
| MAP2_S04 | PASS / PASS | 19 | BLOCKED: map2_s04/Map_WorldObjectsInit |
| MAP3_S00 | PASS / PASS | 16 | BLOCKED: map3_s00/Map_WorldObjectsInit |
| MAP3_S01 | PASS / PASS | 16 | BLOCKED: map3_s01/Map_WorldObjectsInit |
| MAP3_S02 | PASS / PASS | 16 | BLOCKED: map3_s02/Map_WorldObjectsInit |
| MAP3_S03 | PASS / PASS | 16 | BLOCKED: map3_s03/Map_WorldObjectsInit |
| MAP3_S04 | PASS / PASS | 16 | BLOCKED: map3_s04/Map_WorldObjectsInit |
| MAP3_S05 | PASS / PASS | 17 | BLOCKED: map3_s05/Map_WorldObjectsInit |
| MAP3_S06 | PASS / PASS | 16 | BLOCKED: map3_s06/Map_WorldObjectsInit |
| MAP4_S00 | PASS / PASS | 17 | BLOCKED: map4_s00/message pointer decoder |
| MAP4_S01 | PASS / PASS | 17 | BLOCKED: map4_s01/Map_WorldObjectsInit |
| MAP4_S02 | PASS / PASS | 17 | BLOCKED: map4_s02/Map_WorldObjectsInit |
| MAP4_S03 | PASS / PASS | 17 | BLOCKED: map4_s03/Map_WorldObjectsInit |
| MAP4_S04 | PASS / PASS | 16 | BLOCKED: map4_s04/Map_WorldObjectsInit |
| MAP4_S05 | PASS / PASS | 17 | BLOCKED: map4_s05/Map_WorldObjectsInit |
| MAP4_S06 | PASS / PASS | 17 | BLOCKED: map4_s06/message pointer decoder |
| MAP5_S00 | PASS / PASS | 16 | BLOCKED: map5_s00/Map_WorldObjectsInit |
| MAP5_S01 | PASS / PASS | 17 | BLOCKED: map5_s01/Map_WorldObjectsInit |
| MAP5_S02 | PASS / PASS | 16 | BLOCKED: map5_s02/Map_WorldObjectsInit |
| MAP5_S03 | PASS / PASS | 18 | BLOCKED: map5_s03/Map_WorldObjectsInit |
| MAP6_S00 | PASS / PASS | 16 | BLOCKED: map6_s00/Map_WorldObjectsInit |
| MAP6_S01 | PASS / PASS | 16 | BLOCKED: map6_s01/Map_WorldObjectsInit |
| MAP6_S02 | PASS / PASS | 16 | BLOCKED: map6_s02/Map_WorldObjectsInit |
| MAP6_S03 | PASS / PASS | 15 | BLOCKED: map6_s03/Map_WorldObjectsInit |
| MAP6_S04 | PASS / PASS | 16 | BLOCKED: map6_s04/Map_WorldObjectsInit |
| MAP6_S05 | PASS / PASS | 15 | BLOCKED: map6_s05/message pointer decoder |
| MAP7_S00 | PASS / PASS | 16 | BLOCKED: map7_s00/Map_WorldObjectsInit |
| MAP7_S01 | PASS / PASS | 17 | BLOCKED: map7_s01/Map_WorldObjectsInit |
| MAP7_S02 | PASS / PASS | 16 | BLOCKED: map7_s02/Map_WorldObjectsInit |
| MAP7_S03 | PASS / PASS | 16 | BLOCKED: map7_s03/Map_WorldObjectsInit |

Smoke: **2 passed, 41 blocked, zero native process crashes**; 40 descriptors
activate successfully. MAP1_S04 completes its original empty init and streaming
but produces a black frame (zero lit pixels, two primitives), so the visibility
check remains failed. The 37 init guards and three message-decoder guards remain
fatal. No Harry/NPC rendering, gameplay or map-transition pass is claimed by the
warp test. The ignored test exits 101 for guard/visibility failures; logs preserve
the underlying native code 3 for guards. `--smoke-all` consequently exits 1.

The existing original first-map input replay also passes numerically: 422 gameplay
samples, 9173 Q12 units walked, 1711 angle units turned right, and running at
2.86 times sampled walking speed, with camera follow checks. It was invoked
against the owned disc at scale 1 without captures; this verifies preservation
of MAP0_S00 movement, not a transition. See `first-map-regression.json` / `.log`.

Final gates: `tools/dev-cargo.cmd build --release` passes MSVC `/W4 /WX`;
`cargo fmt --all --check` and workspace/all-target release Clippy with
`-D warnings` pass. `tools/dev-cargo.cmd test --release --workspace` passes
298 unit/integration tests plus 3 doctests, zero failures, 5 opt-in tests ignored.
The maps arm64 frontend passes 47/47 units, and the existing core frontend passes
18/18. MSVC x86/x64 layouts and arm64 positive/negative layout controls pass.
Apple SDK/device compilation and runtime were not run. Logs: `build.log`,
`fmt.log`, `clippy.log`, `tests.log`, `clang.log`, `clang-core.log`, `layouts.log`,
`ios-layouts.log`, `smoke.log`, `smoke-results.json`, and map-named warp logs.
Private scratch contains logs only and is below 1 MB; no bulky output is retained.
