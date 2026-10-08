# Events lane checkpoint - 8 October 2026

**The required events pass is incomplete.** Native story text and the initial
unskipped opening movie/dialogue pass. The progression replay reaches the first
alley door, then stops at `SysState_LoadArea_Update`, state 11 / step 2 /
VBlank 9552. Combat damage both ways, pickups, inventory and saving are unproved.

The original text parser, glyph packets, choice input and timed page rollout use
the shared font state. `--opening-noskip` requires all 338 opening movie frames,
all 15 opening callback steps, gradual rollout/completion of messages 15-19,
and restored gameplay/Cheryl. No input follows New Game in that replay. The
private VBlank 3300 capture was visually inspected and shows rolling story text.

MAP0_S00 adds 18 original descriptor callbacks (including Stalker), four Cheryl
companion handlers, waypoint/camera/tween helpers and source-driven in-game FMV,
read-message and flag/sound states. Its original opening, footsteps, spotted and
into-the-alley sequences execute in the deeper replay. 52 additional mutable
objects have a destructive/restoring reset fixture; descriptor size/offsets
remain unchanged. `charaAnimReset` now has its implementation's typed prototype.

The 21-function combat production slice and all 21 Stalker functions are linked
with real native ray/LOS, movement, animation and collision providers. Stalker's
96 animation entries and 279 collision keyframes are decoded from owned map data.
Blood/impact/death effects retain explicit guards. NPC damage, equipped weapon
fragments and Air Screamer/Groaner gameplay are not certified by compilation.

See [events replay notes](replays/EVENTS.md) and the uncommitted root REPORT.md
for verification, the exact handoff boundary and director requests. The root
PROJECT_STATE.md belongs to core; publish this checkpoint there through core.

---

The following move checkpoint is historical context for this events handoff.

# Move lane checkpoint - 8 October 2026

**Goal 1 passes.** `python tools/milestones.py --first-map` completes the original
New Game/movie/cutscene-skip path through state 11 / step 2 / VBlank 3720 with
real BGM logic, original player delegates, wall/ray queries and NPC scheduling.
The BGM test bypass has been removed. No position or completion flag is injected.

The 422 consecutive gameplay samples verify 9173 Q12 units of straight walking,
1711 angle units of right turning, running at 2.86 times the sampled walking
speed, and camera character/look/position follow. The numeric replay uses wgpu
at 1x for efficiency; the application default remains wgpu at 4x. `mid-walk.png`
is a real replay capture at VBlank 3380 and was visually reviewed.
Evidence: `private/work/move/milestones/20261008T191556888153Z/`.

The original opening callback, freeze/unfreeze, map animation states, DMS
interpolation/owned graph publication and Cheryl update/draw are integrated.
Cheryl's eight missing animation records are decoded from the owned map at
runtime with checked function identities; no extracted bytes are in Git.
MAP0_S00 now has 16 original callbacks, one existing no-draw particle bridge and
56 guarded callbacks. Its 33 writable objects participate in the reset probe.
Unskipped opening reaches the **map-message text-rollout guard**, state 11 /
step 2 / VBlank 2443; see `private/work/move/opening-no-skip.log`. Goal 2 remains
partial. Actual weapon attacks/enemy AI (Goal 3), MAP0_S01 and generic overlay
transition/back (Goal 4) remain open. Unarmed setup/cleanup and the attack-table
decoder are integrated; their presence does not certify attacks or damage play.

Two runtime defects were repaired: retire frame-local OT tokens before building
new OTs; reserve the complete bounded ray-cell tail rather than a two-cell stack
buffer. The native fixture covers 32768 packet identities across four epochs and
a ten-cell diagonal ray with surrounding canaries. Movie checks now require the
measured start ticks/exact counts; see [replay notes](replays/MOVE.md).

Verification: release MSVC /W4 /WX; fmt; workspace/all-target Clippy -D warnings;
workspace tests 300 passed, zero failed, four opt-in tests ignored; static
`first_map_view` separately 1/1 passed with default 4x wgpu (94 colors).
Available boot/menu/movie suite 10/10 passed, without a repeat/hash claim.
Captures retain distant geometry/texture artefacts; full render parity is not certified.
Arm64 frontend 39/39 C units, MSVC x86/x64 layouts and arm64 layout positive/
negative controls pass. Actual Apple SDK/device runtime remains untested.
Logs: `private/work/move/{build,fmt,clippy,tests,clang,layouts,ios-layouts,
first-map-view,milestones,trace-controls}.log`. Scratch is below 200 MB.

Director: publish this checkpoint into the core-owned root state; add the new
player/NPC/DMS/effect units to permanent arm64 CI; bind the native map-message
rollout before claiming the unskipped opening/footsteps path. No main, merge,
push, system install, disc copy or audio-owned source edit.

---

The following player-lane checkpoint is historical and superseded by the move result above.

# Player lane checkpoint

**The brief is not passed: Harry does not yet move.** The owned-disc opening now completes the original New Game startup and `GameBoot_InGameInit`, enters state 11, and executes the original gameplay dispatcher. The deepest measured guard is `Bgm_Update/original layer controller`, state **11 / step 2 / VBlank 2266**, before `Player_Update`. The final host CHECK still reports step 0 because that is the last presented frame, before the failing update. Do not treat loader animation, input fixtures or the available boot/menu suite as a movement pass.

Reference remains `d9e28f8315c7938117224f21516786d9d149a145`, GPL-3.0-only, Copyright (C) 2026 shdecompilations. Generation reads pinned source, never owned executable bytes. Extracted data and replay captures remain outside Git in `../../private/work/player/`. No renderer/world lane files, dependencies, main branch or system installation were changed.

World/player merge reconciliation: [RECONCILE.md](RECONCILE.md). Shared declarations now come from generated `shared_types.h`, including the original two-byte `s_MapEnvPresetIdxs`. Rendering and startup use the same appended `gameplayEnvironment` and one copy of the original environment functions/preset tables. World services replace the no-draw player rendering bridges. The owned-disc opening still reaches the exact BGM guard above; the rendering test requires that guard before its separate static capture. This does not complete movement or the BGM controller.

## Implemented

- Original NPC clear/init, InGameInit, empty-group model-load selection, bone-info setup, spawn/persistence flag handling and ground-height initialization. MAP0_S00 starts with empty character groups. Actual NPC asset publication still stops at missing character file/model ownership; full enemy/cutscene character setup is not claimed.
- Original BGM/ambient startup state machines and task tables run against the existing silent task sink. Its MIDI channels are idle; audible BGM, sequencing and positional SFX are not established. PS1 vibration requests are logged because the native host has no PS1 motors.
- Environment presets and checked native environment records, original effect-list resets, inventory texture loading and the exact unequipped weapon-info branch. Equipped weapon ANM fragments remain guarded. Native environment state is appended as `PortSysWork.gameplayEnvironment`; existing flashlight storage and preceding FFI fields remain in place.
- Original `GameState_InGame_Update`, gameplay/event system dispatch, event trigger selection and three trigger geometry routines, plus `Player_Update`'s original call order. Movement/damage/animation delegates remain named guards. Original cutscene-border timing and camera projection execute through a no-draw border bridge.
- Thirteen original controller, turn, animation-duration/init and reset helpers. Native duration callbacks explicitly receive the model instead of relying on the PS1 argument register; the union uses its pointer member. A 256-entry native animation table replaces reliance on base-table/BSS adjacency, with a checked map-animation boundary.
- MAP0_S00 links the original particle-environment selection, wheel initialization/update and Grey Child spawn callback. It resets 26 writable objects, including the now-mutable descriptor and new environment/object scratch. Six original callbacks are linked; one particle-system callback is a marked no-draw bridge; 66 callbacks remain guarded. Opening cutscene callbacks remain guarded.
- Two combat production helpers use the combat lane's pinned extractor and `sh_combat_*` namespace, with no harness doubles. Flag8Clear is referenced in the original visible-Harry branch; damaged-flag handling is compiled and fixture-tested. The 21-function weapon slice, real attacks, enemy AI and NPC scheduling are not integrated.

## Movement gate and verification

Run `python tools/milestones.py --first-map` after a release build. `first_map.txt` is a **candidate** replay, not a verified playable path. It skips the movie/cutscene, walks, turns right and runs. `SH_PLAYER_TRACE=1` samples numeric player position/heading, pad flags and camera character/look/position targets on every presented frame. The gate requires 422 consecutive gameplay samples, checks movement relative to measured heading, right-turn direction, faster running, and camera follow. It rejects missing/malformed/static evidence and native guard exits. No first-map screenshot is requested or used. Replay timings and camera thresholds still need calibration against the completed original control/cutscene path.

Current first-map result: **FAIL**, exit 1, C guard code 3 at VBlank 2266. Evidence: `../../private/work/player/milestones/20261007T175215305631Z/{first-map.log,results.json}`. Earlier guard snapshots are superseded by this result. Available boot/movie/menu/option suite: 10/10 passed, with captures kept in the player private tree using `SH_MILESTONE_LANE=player`; no repeat/hash-determinism claim.

Rust workspace tests: 299 passed including doctests, zero failed, three existing ignored. The single native-global test also checks nine original input-history/run/turn/combat-flag/duration cases and restores touched state. These are fixtures, not gameplay. The movement evaluator separately passed a synthetic positive and eight rejection controls; they do not prove Harry moves. MSVC release C uses /W4 /WX; fmt and all-target workspace Clippy with warnings denied pass. Disk layout checks pass on MSVC x86/x64. The arm64 frontend checks all 26 C units, including the eight added units, with warnings/errors denied. Actual Apple SDK compilation/runtime is untested; the broad original-header ABI gate was not changed or claimed green.

## Remaining work and integration requests

1. Link the original runtime BGM layer controller and its real services; it is the current measured boundary. Preserve the Stalker-related BGM status bit rather than discarding all BGM logic.
2. Migrate the damage, upper/lower-body movement, position/wall-collision, animation and bone-transform delegates; run the original player input in its original update position. Nearby trigger classification and the full NPC update remain guarded.
3. Implement the opening callback's native DMS/character loading and freeze/unfreeze/reset state, then unguard encountered footsteps/Cheryl callbacks in order. Do not set completion flags or inject positions to make the movement gate pass.
4. Continue the combat integration recipe in `../sys/combat.md`: real attack/LOS/collision providers, stable native NPC graphs and complete AI are still required. Reconcile combat's `D_800297B8[]` declaration with upstream's const pointer and this lane's `HARRY_BASE_ANIM_INFOS` storage before linking the full slice.
5. Reconciliation connected loading effects, character/world drawing and motion blur, and shares the appended environment record. Object registration, particles and black borders remain pending; retain the original logic/timing when linking them.
6. Director: update the core-owned root checkpoint and add the eight new C units to the permanent arm64 CI source list. The private `check-player-clang.py` exercised them locally; the unchanged baseline checker alone still covers only 18 units.
