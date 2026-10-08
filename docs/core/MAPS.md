# Objects lane checkpoint - 8 October 2026

**43/43 overlays activate; 41/43 finish the object-init stage; 15/43 pass the
world-frame warp smoke, up from 2/43. No process crashes.** The independent
flashlight fixture also passes 15/43. A submitted mesh or a flare pixel alone
is not a rendering pass. These are static world diagnostics, not gameplay.
The full brief is not passed: the reversible walking milestone remains blocked,
and MAP1_S04 cannot supply a faithful playable world at its original spawn.

## Object-init closure and groups

All 37 previous blanket init guards were replaced by the original initializer
bodies. The two encountered init leaves still guarded are Player_ItemRemove
(events ownership) and an unloaded VAB header for MAP1_S05's Sfx_Unk1478.
MAP0_S00 callback bodies and all events/player/NPC/combat/items lane files stay
unchanged. Other unlinked descriptor callbacks retain typed fatal guards.

| Original shared helper | Initializers using it (other than MAP0_S00) | Native service |
|---|---:|---|
| WorldObject_Init | 31 | Original pose/model macros |
| WorldObject_ModelNameSet | 29 | Street lane's real name registration |
| WorldObject_PlacementInit | 19 | Original placement macro |
| Math_Vector3Set | 12 | Native fixed-width stores |
| WorldObject_PoseInit | 7 | Original pose macro |
| Chara_SpawnFlagsSet | 7 | Original spawn-flag helper |
| Math_SetSVectorFast | 6 | Explicit three-component stores, no misaligned word alias |
| WorldObject_PosePositionInit | 4 | Original position/model macro |

The compiled 72-byte native pose and 64-byte placement records have layout
assertions. Every newly linked map-owned object, pose and helper workspace is
namespaced and inventoried for reset. The shared numeric boss parameters use
the original 14-byte shape, appended to boot.h; ending selection and the ground
image descriptor retain original shared ownership. Existing raw TIM scratch
holds GROUND.TIM, and the real queue captures each TV texture descriptor.

Original bounded effect setup is linked for MAP1_S05, MAP5_S00, MAP6_S04,
MAP4_S03 and MAP7_S03. The effect-slot allocator keeps the original shared
cursor. MAP2_S00's three 32-spawn variants decode owned 12-byte records to
16-byte native records before the original flag-driven selection/memcpy.
Its signed difficulty nibble and native stride have a canary fixture.
The original misnamed s_MapPoint2d table is never copied at native point stride.

## Messages, lifecycle and boundaries

One checked decoder loads the 16 pointer slots in each of MAP4_S00, MAP4_S06 and
MAP6_S05. PS1 addresses become bounded offsets in the owned overlay; strings
must terminate within 4096 bytes. Null entries remain null. Decoding completes
before copying native pointers into reset-owned arenas or publishing the map.
Synthetic tests reject pointers below/above the overlay, table overflow and
unterminated strings. All three owned-disc activation paths pass.

The existing queued activation still invalidates the active descriptor first,
resets its compiled images/BSS and ground cache, loads numeric/string records,
then publishes it. A failed read publishes no descriptor. The all-43 reset probe
passes with the enlarged inventory. This is the complete linked slice, not a
claim that guarded callbacks' unlinked local statics or all host asset caches reset.
MAP1_S03's broader unknown-size table and the finale's broader effect tables
remain guarded; only their explicitly used init footprints are allocated.

## Warp and MAP1_S04 diagnosis

Warp now restores original settings, initializes the sound driver, streams the
world, establishes camera/loading-pose coordinates and the environment, then
calls the original func_8005E650 initializer dispatcher (including its null
callback check). The original fixed-position light identity case returns before
traversing a null bone; a stack fixture verifies that service. SFX calls retain
original playback when a VAB header exists and fail explicitly when it does not.

MAP1_S04 is the pinned sound-test overlay: its only point is (0,0), its message
15 is "NO_STAGE!", and func_800CCA2C selects XA/SFX tasks. Its empty object init
and lack of geometry at that point are original data, not a missing draw helper.
See game/decomp/src/maps/map1_s04/{map_points.h,map1_s04.c,map1_s04_events_data.c}.
No alternate spawn, fake geometry or changed event flag was added to pass it.

World-frame gates require nonzero lit pixels, at least one submitted world mesh
and at least 64 native pixels differing from the dominant color. The latter
rejects a brightness rectangle plus a one-pixel flare: the private MAP3_S04-0.png
negative capture was visually reviewed. MAP2_S04-0.png shows textured room
geometry, with remaining rendering artefacts; it is not a parity certification.
The explicit --flashlight fixture initializes a diagnostic loading pose and
calls the original light toggle/effects. It grants no inventory item and does
not initialize/render Harry or advance gameplay. It exposes missing water
reflection rendering and does not cure the remaining night-scene shading.

```text
python tools/prepare_maps.py --smoke-all
python tools/prepare_maps.py --smoke-all --flashlight
python tools/prepare_maps.py --warp MAP2_S04:0 --capture
python tools/prepare_maps.py --check-clang C:/BuildTools2022/VC/Tools/Llvm/x64/bin/clang-tidy.exe
```

## Current all-43 table

Every row compiles on MSVC /W4 /WX and the arm64 frontend. Results below use
normal difficulty and point 0; a static smoke pass certifies neither NPCs,
interactions, equipped items, movement nor a transition. Each blocked row names
one measured service/frame boundary. Private logs and both result JSON files
are under private/work/objects, with map-named default and -lit logs.

| Map | Reset objects | Warp result / precise blocker |
|---|---:|---|
| MAP0_S00 | 33 | PASS: 41 colors / 13 meshes / 49573 varied pixels |
| MAP0_S01 | 37 | PASS: 60 colors / 55 meshes / 61565 varied pixels |
| MAP0_S02 | 27 | PASS: 67 colors / 48 meshes / 54968 varied pixels |
| MAP1_S00 | 29 | BLOCKED: Player_ItemRemove (events/items integration) |
| MAP1_S01 | 29 | BLOCKED: world shading, 0 varied pixels / 22 meshes (light fixture: 0) |
| MAP1_S02 | 36 | BLOCKED: world shading, 0 varied pixels / 36 meshes (light fixture: 1) |
| MAP1_S03 | 29 | BLOCKED: world shading, 0 varied pixels / 14 meshes (light fixture: 1) |
| MAP1_S04 | 19 | BLOCKED: original NO_STAGE sound-test overlay; no mesh at (0,0) |
| MAP1_S05 | 22 | BLOCKED: Sfx_Unk1478 requires a loaded VAB header |
| MAP1_S06 | 24 | PASS: 57 colors / 19 meshes / 60416 varied pixels |
| MAP2_S00 | 44 | PASS: 108 colors / 75 meshes / 59350 varied pixels |
| MAP2_S01 | 24 | PASS: 54 colors / 21 meshes / 57559 varied pixels |
| MAP2_S02 | 19 | PASS: 44 colors / 38 meshes / 53267 varied pixels |
| MAP2_S03 | 20 | PASS: 33 colors / 8 meshes / 43466 varied pixels |
| MAP2_S04 | 25 | PASS: 108 colors / 5 meshes / 41710 varied pixels |
| MAP3_S00 | 22 | PASS: 85 colors / 3 meshes / 55833 varied pixels |
| MAP3_S01 | 28 | PASS: 54 colors / 15 meshes / 50224 varied pixels |
| MAP3_S02 | 17 | PASS: 90 colors / 12 meshes / 52362 varied pixels |
| MAP3_S03 | 30 | BLOCKED: world shading, 0 varied pixels / 3 meshes (light fixture: 0) |
| MAP3_S04 | 20 | BLOCKED: world shading, 0 varied pixels / 4 meshes (light fixture: 1) |
| MAP3_S05 | 32 | BLOCKED: world shading, 0 varied pixels / 2 meshes (light fixture: 0) |
| MAP3_S06 | 21 | PASS: 79 colors / 4 meshes / 60147 varied pixels |
| MAP4_S00 | 18 | PASS: 47 colors / 4 meshes / 41193 varied pixels |
| MAP4_S01 | 25 | BLOCKED: world shading, 0 varied pixels / 25 meshes (light fixture: 0) |
| MAP4_S02 | 20 | BLOCKED: world shading, 0 varied pixels / 38 meshes (light fixture: 0) |
| MAP4_S03 | 34 | BLOCKED: world shading, 0 varied pixels / 46 meshes (light fixture: 1) |
| MAP4_S04 | 18 | BLOCKED: world shading, 0 varied pixels / 4 meshes (light fixture: 0) |
| MAP4_S05 | 20 | BLOCKED: world shading, 0 varied pixels / 55 meshes (light fixture: 1) |
| MAP4_S06 | 18 | PASS: 47 colors / 4 meshes / 41193 varied pixels |
| MAP5_S00 | 20 | BLOCKED: water reflection service in flashlight fixture |
| MAP5_S01 | 20 | BLOCKED: world shading, 0 varied pixels / 50 meshes (light fixture: 0) |
| MAP5_S02 | 28 | BLOCKED: world shading, 0 varied pixels / 60 meshes (light fixture: 1) |
| MAP5_S03 | 23 | BLOCKED: world shading, 0 varied pixels / 10 meshes (light fixture: 0) |
| MAP6_S00 | 21 | BLOCKED: world shading, 0 varied pixels / 11 meshes (light fixture: 1) |
| MAP6_S01 | 20 | BLOCKED: world shading, 0 varied pixels / 3 meshes (light fixture: 0) |
| MAP6_S02 | 22 | BLOCKED: world shading, 0 varied pixels / 19 meshes (light fixture: 0) |
| MAP6_S03 | 24 | BLOCKED: water reflection service in flashlight fixture |
| MAP6_S04 | 37 | BLOCKED: world shading, 0 varied pixels / 71 meshes (light fixture: 0) |
| MAP6_S05 | 16 | BLOCKED: world shading, 0 varied pixels / 40 meshes (light fixture: 0) |
| MAP7_S00 | 20 | BLOCKED: world shading, 0 varied pixels / 4 meshes (light fixture: 1) |
| MAP7_S01 | 58 | BLOCKED: world shading, 0 varied pixels / 4 meshes (light fixture: 1) |
| MAP7_S02 | 60 | BLOCKED: world shading, 0 varied pixels / 3 meshes (light fixture: 0) |
| MAP7_S03 | 30 | BLOCKED: world shading, 0 varied pixels / 3 meshes (light fixture: 0) |

## Reversible transition handoff

The selected pair is MAP2_S04 -> MAP2_S02 -> MAP2_S04 (police station/street).
MAP2_S04 event 15 uses trigger point 4 and destination point 9 in its source
table; MAP2_S02 event 16 uses trigger point 21 and source destination point 23.
Both are button-triggered overlay loads with no required item or story flag.
SysState_LoadArea_Update copies the destination point from the current overlay
before loading the new one; these point indices are not destination-array indices.

`python tools/prepare_maps.py --transition-candidate` verifies those source
records and writes police-return.candidate.txt/.json in the private lane tree.
This is a BLOCKED, unrun pad candidate. Its timings are uncalibrated, and it is
not a milestone pass. The events-owned load-area dispatcher is still guarded;
non-MAP0_S00 BGM, player/update callbacks and the map effect-texture startup
boundary also need integration before a real walk-through and return can run.
No positions/completion flags are injected after the explicit diagnostic warp.

## Verification and requests

Release MSVC /W4 /WX, fmt, workspace/all-target Clippy -D warnings, workspace
tests (307 unit/integration + 3 doctests passed, zero failed, 6 opt-in ignored),
all-43 reset, map arm64 frontend (47 units), core
frontend (18 units), extended renderer frontend (39 units), MSVC x86/x64 layouts
and arm64 layout positive/negative controls are the required recorded gates.
The map frontend now also compiles the real SH_NATIVE_AUDIO bank-check branch.
The release build, fmt check and Clippy pass; MSVC x86/x64 layout checks and
arm64 positive/negative controls pass. Both 43-map smokes return failure for
their 28 explicitly blocked maps, with 15 passed and zero crashes each.
Commands and logs are recorded in REPORT.md and private/work/objects.
Apple SDK/device runtime, golden image parity and a gameplay transition are untested.

Director/events: bind Player_ItemRemove and original map transition startup/
callbacks; replace the fixed MAP0_S00 event bound before other maps run gameplay.
Director/audio: load/verify the Sfx_Unk1478 VAB bank before its original init call.
Director/render: diagnose the mode-1 world shading and link water reflection;
keep the meaningful-pixel check instead of counting a flare as a map frame.
Director: record MAP1_S04 as the original NO_STAGE sound-test overlay, publish
this checkpoint into the core-owned root state, and run the actual Apple SDK CI.
No main/merge/push, system install, disc copy or extracted game data in Git.
