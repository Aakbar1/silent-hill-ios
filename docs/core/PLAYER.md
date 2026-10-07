# Move lane - in progress

Goal 1 is not passed. Own branch `lane/move`, pinned submodule initialized.
The opt-in runner `python tools/milestones.py --first-map --sound-stub-bgm`
sets a test-only BGM bypass at the generated gameplay call site; the public
`Bgm_Update` and its default path remain unchanged.

The original player delegate closure, wall response and production ray queries
are generated from pinned GPL source. Original empty-group NPC scheduling,
gameplay timer and unarmed combat setup/cleanup are linked. Damage reads the
owned encrypted BODYPROG attack table through the combat lane's decoder.
Current verification is provisional: the last completed release build passed
before the added effect scheduler; its warning fixes are still being checked.
First-map replays reached the original player loop at state 11 / step 2 /
VBlank 2266, then the timer guard, then unarmed combat setup's dependency.
Latest measured evidence: `private/work/move/milestones/20261007T190242941190Z/`.
No walking, screenshot, opening DMS/Cheryl, attacks or transition pass is claimed.
Provisional arm64 check was 31/33, with compile-only CRT declarations being fixed.
Next: finish effect scheduling, remeasure the opening guard, link its native DMS
and character publication prerequisites, then run the real movement gate.
Generated game data stays in `private/work/move`; no merge/push/main changes.

---

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
