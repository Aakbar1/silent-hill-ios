# Story map checkpoint - 9 October 2026

MAP0_S00 now reaches active alley combat through the original no-skip route,
with real Harry damage and a clean VBlank-19650 endpoint. No enemy kill is
claimed. See [STORY.md](replays/STORY.md). The all-43 static warp inventory below
is historical and unchanged; static warp still does not execute gameplay.

---

# Transit map checkpoint - 9 October 2026

**43/43 overlays activate; 42/43 finish object initialization; 15/43 pass
the world-frame warp smoke. Zero crashes.** Item removal unblocks MAP1_S00's
initializer, but its world shading still fails the meaningful-pixel gate.
The pass count has not risen. Remaining static failures concern shading,
the original NO_STAGE sound-test map and MAP1_S05's unloaded SFX bank.

The original generic load-area dispatcher, player-position transfer, door SFX,
startup/fade services and effect-atlas selection are linked. Each overlay now
supplies its own event and callback bounds. Across the other 42 overlays, 833
shared loading/player-control/BGM descriptor bindings replace fatal wrappers;
event-specific and enemy callbacks remain guarded. New numeric tables participate
in destructive/restoring reset probes. Compilation is not an all-map gameplay pass.

The no-skip story route crosses the first gate at 9552, restores enabled control
in room 22 at 9615, and crosses the second door at 12901. Its next measured
boundary is the wheel positional-SFX guard at state 10/step 11/VBlank 12961.
The police walking replay fails before gameplay because the unowned host
accepts only HB_M0S00.ANM. See [transit replay notes](replays/TRANSIT.md).

## Object-init closure and groups

All 37 previous blanket init guards were replaced by the original initializer
bodies. The remaining encountered initializer leaf is an unloaded VAB header for
MAP1_S05's Sfx_Unk1478. Player_ItemRemove now uses its original implementation
and inventory sort/merge helper, with a six-case isolated fixture.
Transit adds the original MAP0_S00 post-gate environment ramp and numeric
waypoints. Combat/fx and the other lanes' source files stay unchanged. Other unlinked descriptor callbacks retain typed fatal guards.

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
| MAP0_S00 | 93 | PASS: 41 colors / 13 meshes / 49573 varied pixels |
| MAP0_S01 | 43 | PASS: 60 colors / 55 meshes / 61565 varied pixels |
| MAP0_S02 | 42 | PASS: 67 colors / 48 meshes / 54968 varied pixels |
| MAP1_S00 | 37 | BLOCKED: world shading, 0 varied pixels / 30 meshes; item-removal init now passes |
| MAP1_S01 | 37 | BLOCKED: world shading, 0 varied pixels / 22 meshes (light fixture: 0) |
| MAP1_S02 | 44 | BLOCKED: world shading, 0 varied pixels / 36 meshes (light fixture: 1) |
| MAP1_S03 | 37 | BLOCKED: world shading, 0 varied pixels / 14 meshes (light fixture: 1) |
| MAP1_S04 | 26 | BLOCKED: original NO_STAGE sound-test overlay; no mesh at (0,0) |
| MAP1_S05 | 29 | BLOCKED: Sfx_Unk1478 requires a loaded VAB header |
| MAP1_S06 | 31 | PASS: 57 colors / 19 meshes / 60416 varied pixels |
| MAP2_S00 | 51 | PASS: 108 colors / 75 meshes / 59350 varied pixels |
| MAP2_S01 | 38 | PASS: 54 colors / 21 meshes / 57559 varied pixels |
| MAP2_S02 | 26 | PASS: 44 colors / 38 meshes / 53267 varied pixels |
| MAP2_S03 | 26 | PASS: 33 colors / 8 meshes / 43466 varied pixels |
| MAP2_S04 | 39 | PASS: 108 colors / 5 meshes / 41710 varied pixels |
| MAP3_S00 | 29 | PASS: 85 colors / 3 meshes / 55833 varied pixels |
| MAP3_S01 | 35 | PASS: 54 colors / 15 meshes / 50224 varied pixels |
| MAP3_S02 | 25 | PASS: 90 colors / 12 meshes / 52362 varied pixels |
| MAP3_S03 | 38 | BLOCKED: world shading, 0 varied pixels / 3 meshes (light fixture: 0) |
| MAP3_S04 | 28 | BLOCKED: world shading, 0 varied pixels / 4 meshes (light fixture: 1) |
| MAP3_S05 | 40 | BLOCKED: world shading, 0 varied pixels / 2 meshes (light fixture: 0) |
| MAP3_S06 | 28 | PASS: 79 colors / 4 meshes / 60147 varied pixels |
| MAP4_S00 | 25 | PASS: 47 colors / 4 meshes / 41193 varied pixels |
| MAP4_S01 | 40 | BLOCKED: world shading, 0 varied pixels / 25 meshes (light fixture: 0) |
| MAP4_S02 | 31 | BLOCKED: world shading, 0 varied pixels / 38 meshes (light fixture: 0) |
| MAP4_S03 | 45 | BLOCKED: world shading, 0 varied pixels / 46 meshes (light fixture: 1) |
| MAP4_S04 | 26 | BLOCKED: world shading, 0 varied pixels / 4 meshes (light fixture: 0) |
| MAP4_S05 | 31 | BLOCKED: world shading, 0 varied pixels / 55 meshes (light fixture: 1) |
| MAP4_S06 | 25 | PASS: 47 colors / 4 meshes / 41193 varied pixels |
| MAP5_S00 | 26 | BLOCKED: water reflection service in flashlight fixture |
| MAP5_S01 | 26 | BLOCKED: world shading, 0 varied pixels / 50 meshes (light fixture: 0) |
| MAP5_S02 | 43 | BLOCKED: world shading, 0 varied pixels / 60 meshes (light fixture: 1) |
| MAP5_S03 | 38 | BLOCKED: world shading, 0 varied pixels / 10 meshes (light fixture: 0) |
| MAP6_S00 | 31 | BLOCKED: world shading, 0 varied pixels / 11 meshes (light fixture: 1) |
| MAP6_S01 | 35 | BLOCKED: world shading, 0 varied pixels / 3 meshes (light fixture: 0) |
| MAP6_S02 | 32 | BLOCKED: world shading, 0 varied pixels / 19 meshes (light fixture: 0) |
| MAP6_S03 | 30 | BLOCKED: water reflection service in flashlight fixture |
| MAP6_S04 | 48 | BLOCKED: world shading, 0 varied pixels / 71 meshes (light fixture: 0) |
| MAP6_S05 | 27 | BLOCKED: world shading, 0 varied pixels / 40 meshes (light fixture: 0) |
| MAP7_S00 | 26 | BLOCKED: world shading, 0 varied pixels / 4 meshes (light fixture: 1) |
| MAP7_S01 | 64 | BLOCKED: world shading, 0 varied pixels / 4 meshes (light fixture: 1) |
| MAP7_S02 | 66 | BLOCKED: world shading, 0 varied pixels / 3 meshes (light fixture: 0) |
| MAP7_S03 | 36 | BLOCKED: world shading, 0 varied pixels / 3 meshes (light fixture: 0) |

## Reversible transition handoff

MAP2_S04 -> MAP2_S02 -> MAP2_S04 remains the source-checked pair. Event 15 in
MAP2_S04 uses trigger point 4 and destination point 9 from its **source** overlay.
Event 16 in MAP2_S02 uses trigger point 21 and source destination point 23.
Neither requires an item or story flag. The destination point is copied before
queued overlay replacement, as in the original dispatcher.

`docs/core/replays/police_return.txt` is a failed candidate, not a passed milestone.
The opt-in `maps::warp_tests::transit_walking_replay` starts original boot and
the main loop with one explicit initial MAP2_S04:4 warp. It injects no later
position, flag, inventory, damage or successful callback result. At frame 1651
the host returns `map animation fragment identity is not linked`, before walking.
The new exact `maps::is_player_map_fragment` predicate is ready for the director's
one-line change in unowned host/src/native.rs; the existing bounded arena decoder
and ownership remain authoritative. Timings and both transitions are unverified.

## Verification and requests

The 43-map normal smoke is 15 passed, 28 blocked, zero crashes. MAP1_S00's new
shading boundary is recorded in private/work/transit/MAP1_S00.log. The all-43
reset probe includes the new callback data and validates each event terminator.
Release MSVC /W4 /WX and the 87-unit arm64 production frontend pass; the latter
uses the actual prepared production units. MSVC x86/x64 layouts and arm64 layout
positive/negative controls pass. See REPORT.md for the final Cargo/replay gates.
No Apple SDK/device runtime or render parity is certified.

Director: wire the exact animation identity predicate in host/src/native.rs,
then re-run the police candidate to expose its next service boundary. Complete
remaining map event/enemy providers in their owning lanes. Audio: bind the wheel
falloff/pitch service and load MAP1_S05's VAB bank. Render: resolve the existing
mode-1 shading and water reflection; retain the 64-pixel/world-mesh checks.
Publish the checkpoint to core-owned PROJECT_STATE.md and run Apple SDK CI.
No main, merge, push, system install, disc copy or extracted game data in Git.
