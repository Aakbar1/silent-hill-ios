# Fight lane checkpoint

**The full brief is not passed: goal 5 remains blocked.** Effects and weapon
publication are demonstrated through isolated rendering fixtures; these fixtures
explicitly grant equipment and emit test blood, and never count as combat play.
The original opening progression still stops at `SysState_LoadArea_Update` /
VBlank 9552. The existing warp entry renders a static world without a gameplay
loop. Transit-owned source files, core replays and milestones.py are unchanged.

## Implemented

- Sixteen original effect functions now execute natively: allocation, shared
  scheduler/work arena, blood/impact publication, ground/death blood, muzzle
  effects, original packet drawing and effect texture loading. `func_8005F6B0`,
  `func_800622B8` and `func_8006342C` no longer stop at publication guards.
  The native work union preserves the scheduler prefix across draw consumers;
  original packed byte/halfword stores remain explicit. Native SVector stores
  replace signed-shift packing, and the original zero-register macro remains zero.
- Original `GameFs_WeaponInfoUpdate`, weapon animation/collision tables and weapon
  bank selection are linked. Headerless WEP fragments patch the stable HB_BASE
  frame arena at 0x15bb4, bounded before map frames at 0x19d84. Numeric reads and
  decoded native descriptors remain separate; no raw pointer relocation is used.
  Held PLMs retain their original material/bone-assignment readiness step after
  native graph publication. The unused HyperBlaster table tail has explicit zero
  storage instead of an out-of-bounds source-table read (`// PORT:`).
- BIRD/NightFlutter ANMs use a bounded native decoder with forward-parent support,
  channel/count/cycle checks, stable owned leaves and failure-atomic publication.
  Both C and Rust decoders reach the original C model/bone consumer for all 39
  owned-disc model/animation pairs, with matching results. Air Screamer/Groaner
  AI and other map enemy callbacks remain guarded; their gameplay is unproved.
- Original enemy kill statistics and death flag handling replace the final NPC
  death-statistics guard. The no-attack sentinel cannot index before the table.
- Cold map object SFX loads the original current ambient VAB task through libsd.
  MAP1_S05 reports `FIGHT_SFX sound=1478 bank=2 header=loaded body=loaded`; it then
  fails the existing static warp's varied-pixel gate. No audible SFX claim is made.

## Evidence and verification

Private evidence: `C:/Claude Projects/Silent Hill iOS/private/work/fight/`.
`handgun.png` and `pipe.png` were visually reviewed: held weapons/attack poses,
handgun flash and ground blood through the real wgpu renderer at 1x. Both runs
finish 3650 frames in original gameplay with no guards. Test setup at tick 3500
is explicitly logged as `FIGHT_FIXTURE`; blood is emitted at tick 3600, without
changing health. The real input subsequently drives aim and attack animation.

| Isolated fixture | Samples | Aim | Attack | Distinct keyframes | Held draws | Effect packet bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Handgun | 151 | 129 | 67 | 36 | 67 | 24672 |
| Steel pipe | 151 | 129 | 63 | 38 | 66 | 18720 |

`python tools/check_fight.py --render-fixtures`: 2/2 pass (`render-results.json`).
`--self-test`: one positive and eleven rejection controls pass. The actual combat
check rejects both the static cafe warp and the guarded school warp: zero player,
NPC or production-hit samples. `combat.log`, `combat-results.json` and
`opening-combat-boundary.log` retain those failures. `first_fight.txt` is a
candidate, not a completed combat replay. Inventory equip/reload UI, other weapon
variants and real NPC damage/death remain unverified.

Release MSVC `/W4 /WX` build, root/standalone fmt and all-target Clippy with warnings
denied pass. Workspace tests with `precise-vertices`: 311 passed, zero failed,
seven opt-in tests ignored. Standalone combat: 19 passed with `SH_REQUIRE_DISC=1`,
including 39 C-decoded owned model/ANM pairs and 128 THR collision graphs (3978
surface reads). Unchanged movement evaluator: 422 samples, 9173 Q12 walked,
1711 right turn, running 2.86 times walking speed. Arm64 frontend: 90 playable C
units plus nine standalone combat units passed, zero failed. Apple SDK/device,
PS1 visual goldens and broader enemy gameplay are untested. Logs: `build.log`,
`tests.log`, `clippy.log`, `harness-tests.log`, `harness-clippy.log`, `clang.log`,
`harness-clang.log`, `first-map.log` and `first-map-results.json`.

## Reproduction and requests

Build with `tools/dev-cargo.cmd build --release`. For isolated rendering set
`SH_FIGHT_DIAGNOSTIC=handgun` or `pipe`, then run the host with `--headless --audio
off --scale 1 --frames 3650 --input docs/sys/replays/weapon_render.txt --screenshot
<private/work/fight/handgun.png or pipe.png>`. Remove that environment setting
before any real combat proof. `python tools/check_fight.py --run --warp
MAP0_S01:0 --frames 1300` drives the opt-in native replay test and deliberately
fails until warp can execute actual gameplay. New replays live in `docs/sys/replays/`.

Arm64: `python tools/prepare_fight.py --decomp game/decomp --out
<the release build's OUT_DIR/native-source> --check-clang
C:/BuildTools2022/VC/Tools/Llvm/x64/bin/clang-tidy.exe`. The checker consumes the
actual Cargo-generated closure, including all 43 map units and native audio.

Director/transit: provide a playable warp using original player/NPC loading and
update flow, and complete the area-load boundary. Then calibrate the candidate
against real enemies and require Harry damage, an enemy hit and death, and a
private screenshot through `tools/check_fight.py`. Register the 90-unit arm64
check and publish this checkpoint to the core-owned PROJECT_STATE.md. No system
installs, main/merge/push, copied disc, assets, disassembly or screenshots in Git.

---

## Historical standalone combat checkpoint (superseded for fight integration)

The following inventory documents the older isolated harness; it does not claim
current playable AI linkage or replace the measured fight results above.

# Combat lane checkpoint

**The full combat brief is not passed.** This is a tested native combat slice and an exhaustive conservative source inventory, not a complete enemy/gameplay plug-in. Reference: `d9e28f8315c7938117224f21516786d9d149a145`, GPL-3.0-only, Copyright (C) 2026 shdecompilations. No core-owned file was changed. `PROJECT_STATE.md` remains core-owned; this document is the combat checkpoint.

## Implemented and verified

- Complete `bodyprog_combat_8008A058.c`: 21 original functions, attack timing/history, firearm dispatch, melee sweeps, hit masks, damage accumulation and weapon SFX dispatch. NPC identity uses equality against six native slots, replacing PS1 pointer truncation and inverse multiplication by the 296-byte stride. These functions are `sh_combat_*` and cannot override core's existing guards accidentally.
- Original damaged/dead/persistence helpers, reload calculation, animation-lock/reset helpers and collision-keyframe interpolation. Four original Stalker functions: initialization, damage/stagger handling, scripted state 13 and SFX selection. Complete Cat, Parasite and Flauros source units compile. Remaining Stalker control states and its full update loop do not.
- The 24-byte attack wire record becomes a checked 32-byte native record with an explicit resolved auxiliary pointer; all 70 records match the owned US 1.1 disc. Unaligned reads, truncation, wrong identity and failure atomicity have tests.
- LM/ILM and native graphs reuse the current core decoders/builders read-only. Enemy ANM decoding additionally permits bounded forward hierarchy references and validates cycles. BIRD bones 2/3/6 reference bone 16; Floatstinger has 44 bones. Core's current decoder rejects both. All 39 distinct non-dummy character model/animation pairs, including unused Chicken and cutscene variants, decode and reach original C bone-initialization/model-assignment consumers. Serialized bytes remain unchanged. DummyNurse/DummyDoctor use dummy slots and are excluded from nonempty model graph checks.
- Keyframe collision leaves decode as ten signed little-endian halfwords into aligned native records; original interpolation updates character shapes. All 128 THR IPD collision graphs decode to core's aligned native allocations; C reads 3,978 signed surface heights with exact wire-value agreement and rejects invalid indices without changing the output. This establishes the record/consumer boundary, not real map ray/LOS queries or full animated collision scheduling.

Wire assertions retain `ShDiskMesh=24`, `ShDiskModel=16`, `ShDiskLm=20`, `ShDiskCollision=308`, `ShDiskIpd=392` through core's disk32 header. Combat asserts attack wire/native 24/32, keyframe 20, native collision 40, character health offset 192, properties offset 256 and player-extra offset 328. Core's named native character/model/ANM assertions remain enabled; the broad upstream gameplay gate is not suppressed. Native model/ANM allocations must outlive every C pointer. File offsets/PS1 addresses are never host pointers. MIPS division-by-zero/overflow results have explicit adapters and tests. Original BODYPROG static scratch retains its lifetime and has an explicit warm-boot reset.

## Verification commands

From the worktree root:

```powershell
git submodule update --init
tools/dev-cargo.cmd fmt --manifest-path crates/sys-combat/Cargo.toml -- --check
tools/dev-cargo.cmd clippy --manifest-path crates/sys-combat/Cargo.toml --all-targets -- -D warnings
tools/dev-cargo.cmd test --manifest-path crates/sys-combat/Cargo.toml -- --nocapture
python tools/prepare_combat.py --check-clang C:/BuildTools2022/VC/Tools/Llvm/x64/bin/clang-tidy.exe
```

The standalone harness uses `cc`, MSVC `/std:c11 /W4 /WX`. LLVM checks eight production C units for arm64/iOS with `-Wall -Wextra -Werror` and the same additional divide-by-zero analyzer as core's `check-native-clang.py`. The latter script is also run unchanged for core; it does not discover this lane automatically. Minimal CRT declarations are compile-only. Apple SDK compilation, iOS runtime, real collision/LOS, gameplay replay, sound playback and all-AI compilation remain unverified.

Final local checks: 18 combat tests passed (3 decoder unit, 3 owned-disc integration, 12 C system fixtures), zero failed; fmt and all-target Clippy passed with warnings denied. Real-disc assertions were enforced with `SH_REQUIRE_DISC=1`. Missing-disc skip and required-disc failure controls were separately verified. MSVC builds all ten harness C units without warnings; production arm64 frontend 8/8 and unchanged core arm64 frontend 8/8 passed. The existing workspace passed 275 unit/integration tests plus 4 doctests, with 3 existing ignored tests. Root fmt/Clippy and x86/x64 disk layout checks passed. These passes do not cover the uncompiled inventory rows.

Disc tests run automatically when the default owned image exists or `SH_DISC` is set. Missing images print `SKIP`; `SH_REQUIRE_DISC=1` makes missing data a failure. Invalid existing images always fail. Do not interpret a skip as a real-disc pass. The native harness has isolated test doubles for RNG, ray/LOS, matrix transforms, effects, audio and atan/sqrt; those doubles are in `harness.c` and must never enter a playable build. The firearm test verifies dispatch and damage through an injected hit service, not geometric ray correctness. The model probe uses the original bone initializer and model assignment; it does not draw an enemy or advance bone keyframes.

## Character and weapon coverage

The inventory includes all 32 shared character sources, every map instantiation, map spawn/header/animation data and all shared gameplay function headers. It is deliberately conservative and includes cutscene/BGM/dependency sources; it is not a reachability or active-preprocessor census. `port/sys/combat/inventory.json` records exact functions, core-linked subsets, migrated subsets and wrapper paths. Regenerate it with `python tools/prepare_combat.py --export-inventory port/sys/combat/inventory.json`.

Enemy aliases: AirScreamer/NightFlutter use `air_screamer.c`; Groaner/Wormhead use `groaner.c`; Stalker/GreyChild/Mumbler use `stalker.c`; nurse/doctor/dummy variants use `puppet_nurse.c`. LarvalStalker, HangedScratcher, Creeper, Romper, SplitHead, Floatstinger, Twinfeeler, Bloodsucker, Incubus, Unknown23 and MonsterCybil each have their named source. Chicken is marked unused upstream and has no dedicated AI file. Harry has both shared/map player logic and BODYPROG `player_control.c`. IDs 25–43 have the corresponding cutscene sources, with ending variants sharing Cybil/Dahlia/Kaufmann. Padlock is handled by map/event/attack logic and has no dedicated character source. `CHARA_FILE_INFOS` has 45 entries, including None; this is not a claim of 45 distinct AI implementations.

Weapons: KitchenKnife, SteelPipe, RockDrill, Hammer, Chainsaw, Katana, Axe, Kick, Stomp, Handgun, HuntingRifle, Shotgun and HyperBlaster. The source retains unknown attack IDs and the original `weapon + input_type*10` packing. Reload capacities are original 15/6/6. Full input handling, ammo consumption on firing, weapon animation selection, aim/auto-target, player damage/death transitions, inventory equip/reload UI and held-item drawing remain in uncompiled portions of `player_control.c`, map player/shared headers and item sources. The existing empty-hand/bootstrap guard is not full weapon linkage. Hit effects/particles, radio and complete enemy animation/collision updates are inventoried but mostly uncompiled.

## Exact integration recipe for the migrated slice

This recipe can integrate the listed helpers once the required real services exist. It does **not** authorize replacing missing AI or world services with the harness doubles.

1. Generate into core's chosen build output using `python tools/prepare_combat.py --decomp game/decomp --out <generated>`. It verifies the reference revision. Add `port/sys/combat` to the existing C include list. Compile `combat_original.c`, `combat_helpers.c`, `combat_assets.c` and `port/sys/combat/combat.c` with the existing generated/native includes. Compile desired `combat_ai_cat.c`, `combat_ai_parasite.c`, `combat_ai_flauros.c`, `combat_ai_stalker.c` only after supplying the named tables/services below. Do not compile `combat_test_math.c` or `crates/sys-combat/harness.c` in core. Do not link the harness crate into the playable executable.
2. Use the same `PortGameWork`, `PortSysWork`, native character and animation records as core. Shared globals read/written directly: `g_GameWork`/savegame, `g_SysWork.playerWork`, six `npcs`, `sysState`, `targetNpcIdx`, `field_275C/2760/2764`, and `g_DeltaTime`. No synchronization copy is needed for these records. Before each simulation tick publish `sh_combat_dark_environment` from the active environment. At queued map activation publish `sh_combat_harry_map_anims` from that map's native descriptor; `D_800297B8` is the native base Harry animation table, not file bytes.
3. Provide the exact SDK/GTE/game services declared in `combat.h`: `SQRT` (192 signed halfwords, original SDK lookup table), `Math_Sin/Math_Cos`, `SquareRoot12`, `ratan2`, `Math_Vector2MagCalc`, `Math_Distance2dGet`, `func_80080540`, original RNG/`Rng_RandQ12`, `func_80044918`, `func_8007FD2C`, `Game_HyperBlasterBeamColorGet`, real `Ray_TraceQuery`/`Ray_CharaTraceQuery`, `func_800892A4`, `func_80089314`, `func_8009151C`, kill-stat accessors `func_8009146C/914C4`, blood/impact `func_8005F6B0`, SFX play/update/stop and `GsInitCoordinate2`. These are unresolved production dependencies, not all currently linked. `SQRT` is data at BODYPROG VA `0x800AFFCC` (file offset VA minus `0x80024B60`); use `sh_combat_sqrt_decode` on that 384-byte range from decrypted owned data into native storage. The real-disc test checks its original combat square-root consumer on zero and three exact powers. Never cast a PS1 address or copy the table into this repo.
4. At WorldInit/warm boot call `sh_combat_reset_scratch()`, reset `sh_combat_attack_aux`, clear the three weapon sound fields through the original `sh_combat_func_8008B398()`, and reset the dark/map-animation publication as the engine initializes its map. Do not clear scratch each frame. Keep original VBlank/delta ordering and approximately 30 Hz gameplay; do not step AI again for interpolated render frames.
5. Preserve the order in `events/npc_main.c`: set `AnimFlag_Unlocked`; select its animation slot/bone coordinates; `sh_combat_Chara_Flag8Clear`; `sh_combat_Chara_DamagedFlagUpdate`; `Collision_FlagsLocationUpdate`; dispatch the active map's original character update; `Collision_FlagsUpdate`; `sh_combat_func_80037E78`; `sh_combat_func_8008A3AC`; then original visible draw. Complete `Game_NpcUpdate` and spawn/reload/draw services are not implemented here. Player control calls `sh_combat_func_8008A0E4` in its original animation/update position; it must still consume ammo and perform original aim/input transitions in player code. `func_8008B714` only queues damage; each character's original damage handler applies health/stagger rules.
6. For the Stalker slice, supply `sh_map0_s00_STALKER_ANIM_INFOS`, `sh_map0_s00_sharedData_800E3A20/24/28/2C_0_s00` from the original map and its difficulty initialization, plus `sh_combat_chara_group_flags[4]` bound/synchronized with the native character-group state. Original `Stalker_Update` sets the difficulty constants before Init; omitting it changes health/regeneration/stagger. Supply real angle/magnitude/matrix services in `combat_ai.h`. The full Stalker Update/Control/animation/collision functions must be migrated before wiring the actual map callback. Cat/Parasite/Flauros require their `sh_combat_ai_<kind>_*_ANIM_INFOS` arrays and real playback/matrix/SFX services. They are single active-descriptor adapters; generate per-map prefixes and per-map reset images before compiling multiple resident instantiations. Preserve original map-specific animation tables, not the test callbacks.
7. Publish stable native enemy LM and ANM graphs from core's AssetStore, run the original model/animation initialization and retain their handles/ownership until no skeleton/animation/collision consumer can refer to them. Apply the forward-parent/44-bone decoder correction from `crates/sys-combat/src/animation.rs` (the private patch proposes the matching core correction). Validate every hierarchy and collision leaf before publishing. Native bone initialization accepts forward pointers but all coordinates must be initialized before any recursive transform traversal.
8. Run the harness, workspace checks and real first-map replay after wiring. Confirm real enemy movement/attack, player health/death, ammo depletion, LOS obstruction and overlay reentry; none of those full integration milestones is established here.

The root workspace in this checkout uses an explicit member list; the brief's `crates/*` glob is absent. The standalone crate therefore has its own `[workspace]`. `private/work/combat/` contains a proposed registration patch, a decoder correction patch and a production C build fragment. Registration requires the director to apply the root-member change and remove the standalone workspace stanza, then refresh the root lockfile. No patch has been applied to core-owned files.

Regenerate those private proposals with `python tools/prepare_combat.py --handoff-private`. Both `.patch` files passed `git apply --check` against this checkout. They are separate changes; the C fragment is a build recipe and still needs the real runtime providers listed above. Test logs are in the same private folder (`combat-tests.log`, `workspace-test.log`, `clang-combat.log`, `clang-core.log`, `require-disc-negative.log`).

## Open problems and requests

- Full native compilation of the other 29 shared character sources and all map-specific instantiations, effects, collision/ray/LOS and player weapon control. The strict full-source gate is not claimed; only the migrated subsets below were compiled. The recipe is complete for those subsets and records every required provider, but the whole plug-in is not ready.
- Core must register the harness, migrate/link the actual world services and apply/review the ANM correction. Combat must continue the missing AI and map namespaces/reset tables. Preserve original guards until each callback and provider is real.
- No complete weapon firing/ammo/aim replay, complete enemy state-machine coverage, iOS execution, audiovisual result or map transition/reentry verification.

<!-- COMBAT SOURCE INVENTORY -->

| Source | Core baseline | Combat harness |
|---|---|---|
| `include/maps/shared/Event_CutsceneTimerAdvance.h` | unlinked | uncompiled |
| `include/maps/shared/MapEvent_DoorJammed.h` | unlinked | uncompiled |
| `include/maps/shared/MapEvent_DoorLocked.h` | unlinked | uncompiled |
| `include/maps/shared/MapEvent_DoorUnlocked.h` | unlinked | uncompiled |
| `include/maps/shared/Map_RoomBgmInit_0_s02_CondFalse.h` | unlinked | uncompiled |
| `include/maps/shared/Map_RoomBgmInit_0_s02_CondTrue.h` | unlinked | uncompiled |
| `include/maps/shared/Map_RoomBgmInit_1_s02.h` | unlinked | uncompiled |
| `include/maps/shared/Map_RoomBgmInit_2_s00.h` | unlinked | uncompiled |
| `include/maps/shared/Map_RoomBgmInit_2_s02.h` | unlinked | uncompiled |
| `include/maps/shared/Map_RoomBgmInit_3_s00_CondFalse.h` | unlinked | uncompiled |
| `include/maps/shared/Map_RoomBgmInit_3_s00_CondTrue.h` | unlinked | uncompiled |
| `include/maps/shared/Map_RoomBgmInit_3_s02_CondFalse.h` | unlinked | uncompiled |
| `include/maps/shared/Map_RoomBgmInit_3_s02_CondTrue.h` | unlinked | uncompiled |
| `include/maps/shared/Map_RoomBgmInit_4_s02.h` | unlinked | uncompiled |
| `include/maps/shared/Map_RoomBgmInit_6_s00.h` | unlinked | uncompiled |
| `include/maps/shared/Map_RoomBgmInit_6_s04.h` | unlinked | uncompiled |
| `include/maps/shared/Map_RoomBgmInit_6_s04_CondFalse.h` | unlinked | uncompiled |
| `include/maps/shared/Map_RoomBgmInit_6_s04_CondTrue.h` | unlinked | uncompiled |
| `include/maps/shared/Map_RoomBgmInit_CheckCond.h` | unlinked | uncompiled |
| `include/maps/shared/map_msg_common.h` | unlinked | uncompiled |
| `include/maps/shared/sharedFunc_800CDAA8_0_s02.h` | unlinked | uncompiled |
| `include/maps/shared/sharedFunc_800D0110_7_s00.h` | unlinked | uncompiled |
| `include/maps/shared/sharedFunc_800D15F0_3_s01.h` | unlinked | uncompiled |
| `include/maps/shared/sharedFunc_800DB60C_7_s01.h` | unlinked | uncompiled |
| `src/bodyprog/bodyprog_80089090.c` | unlinked | uncompiled |
| `src/bodyprog/bodyprog_combat_8008A058.c` | unlinked | full unit |
| `src/bodyprog/bodyprog_data_80028B94.c` | unlinked | uncompiled |
| `src/bodyprog/bodyprog_math_8005BF38.c` | unlinked | uncompiled |
| `src/bodyprog/bodyprog_npc_8005BF38.c` | unlinked | uncompiled |
| `src/bodyprog/collision/chara.c` | unlinked | Collision_CharaCollisionSet, Chara_ModelBoneScaleSet |
| `src/bodyprog/collision/collision.c` | Collision_Init, Collision_FlagsSet | uncompiled |
| `src/bodyprog/collision/los.c` | unlinked | uncompiled |
| `src/bodyprog/collision/ray.c` | unlinked | uncompiled |
| `src/bodyprog/events/bodyprog_data_800A99B4.c` | unlinked | uncompiled |
| `src/bodyprog/events/chara_spawn.c` | unlinked | uncompiled |
| `src/bodyprog/events/npc_main.c` | unlinked | Savegame_EnemyStateUpdate, Chara_DamagedFlagUpdate, func_80037E78 |
| `src/bodyprog/events/player_pos_update.c` | Chara_PositionSet | uncompiled |
| `src/bodyprog/events/radio.c` | unlinked | uncompiled |
| `src/bodyprog/game_boot/fs_chara_anim.c` | unlinked | uncompiled |
| `src/bodyprog/game_boot/game_boot.c` | GameBoot_WorldInit, GameBoot_MapLoad, GameState_LoadScreen_Update, GameBoot_LoadingScreen, GameBoot_SavegameInitialize | uncompiled |
| `src/bodyprog/gfx/bodyprog_effects_8005E0DC.c` | unlinked | uncompiled |
| `src/bodyprog/gfx/materials.c` | Lm_MaterialFileIdxApply, Lm_MaterialFsImageApply, Material_FsImageApply, Lm_MaterialFlagsApply, Model_MaterialFlagsApply, LmHeader_ModelCountGet, Bone_ModelAssign | Bone_ModelAssign |
| `src/bodyprog/items/item_screens_2.c` | unlinked | uncompiled |
| `src/bodyprog/items/item_screens_3.c` | unlinked | Items_AmmoReloadCompute |
| `src/bodyprog/items/item_unk_data.c` | unlinked | uncompiled |
| `src/bodyprog/items/item_utils.c` | unlinked | uncompiled |
| `src/bodyprog/player_control.c` | Game_PlayerInfoInit, GameFs_PlayerMapAnimLoad, Game_SavegameResetPlayer, func_8007E9C4 | uncompiled |
| `src/bodyprog/sound/sfx.c` | unlinked | uncompiled |
| `src/bodyprog/sys/chara_data_info.c` | unlinked | uncompiled |
| `src/bodyprog/sys/npc_anims_clear.c` | unlinked | uncompiled |
| `src/bodyprog/world/bodyprog_anim_800445A4.c` | Anim_BoneInit | Anim_BoneInit |
| `src/bodyprog/world/bodyprog_bone_80044F14.c` | Bone_ModelIdxGet, Skeleton_Init, func_80045014, func_8004506C, func_80045108, Skeleton_BoneModelAssign, func_80045258, func_800452EC, func_800453E8, func_80045468 | uncompiled |
| `src/bodyprog/world/collision_trigger.c` | unlinked | uncompiled |
| `src/bodyprog/world/world_draw.c` | WorldGfx_HarryCharaLoad, Chara_FsImageCalc, WorldGfx_PlayerModelProcessLoad, WorldGfx_CharaModelProcessLoad, World_Init, WorldGfx_HeldItemModelFree, WorldGfx_CharaModelsFree, Chara_ModelFree, WorldObjects_Clear, WorldGfx_HarryMeshSwap | uncompiled |
| `src/bodyprog/world/world_effects.c` | Game_FlashlightAttributesFix, Game_TurnFlashlightOn, Game_TurnFlashlightOff | uncompiled |
| `src/maps/chara_util.c` | unlinked | Chara_MovementReset, Chara_AnimReset, Chara_AnimStateSet, Chara_ControlStateReset, Chara_AnimLock, Chara_AnimIsLocked, Chara_AnimUnlock |
| `src/maps/characters/air_screamer.c` | unlinked | uncompiled |
| `src/maps/characters/alessa.c` | unlinked | uncompiled |
| `src/maps/characters/bloodsucker.c` | unlinked | uncompiled |
| `src/maps/characters/bloody_incubator.c` | unlinked | uncompiled |
| `src/maps/characters/bloody_lisa.c` | unlinked | uncompiled |
| `src/maps/characters/cat.c` | unlinked | full unit |
| `src/maps/characters/cheryl.c` | unlinked | uncompiled |
| `src/maps/characters/creeper.c` | unlinked | uncompiled |
| `src/maps/characters/cybil.c` | unlinked | uncompiled |
| `src/maps/characters/dahlia.c` | unlinked | uncompiled |
| `src/maps/characters/flauros.c` | unlinked | full unit |
| `src/maps/characters/floatstinger.c` | unlinked | uncompiled |
| `src/maps/characters/ghost_child_alessa.c` | unlinked | uncompiled |
| `src/maps/characters/ghost_doctor.c` | unlinked | uncompiled |
| `src/maps/characters/groaner.c` | unlinked | uncompiled |
| `src/maps/characters/hanged_scratcher.c` | unlinked | uncompiled |
| `src/maps/characters/incubator.c` | unlinked | uncompiled |
| `src/maps/characters/incubus.c` | unlinked | uncompiled |
| `src/maps/characters/kaufmann.c` | unlinked | uncompiled |
| `src/maps/characters/larval_stalker.c` | unlinked | uncompiled |
| `src/maps/characters/lisa.c` | unlinked | uncompiled |
| `src/maps/characters/little_incubus.c` | unlinked | uncompiled |
| `src/maps/characters/locker_dead_body.c` | unlinked | uncompiled |
| `src/maps/characters/monster_cybil.c` | unlinked | uncompiled |
| `src/maps/characters/parasite.c` | unlinked | full unit |
| `src/maps/characters/player.c` | unlinked | uncompiled |
| `src/maps/characters/puppet_nurse.c` | unlinked | uncompiled |
| `src/maps/characters/romper.c` | unlinked | uncompiled |
| `src/maps/characters/split_head.c` | unlinked | uncompiled |
| `src/maps/characters/stalker.c` | unlinked | Stalker_Init, sharedFunc_800D3308_0_s00, Stalker_Control_13, sharedFunc_800D7E04_0_s00 |
| `src/maps/characters/twinfeeler.c` | unlinked | uncompiled |
| `src/maps/characters/unknown23.c` | unlinked | uncompiled |
| `src/maps/map0_s00/Chara_Cheryl.c` | unlinked | uncompiled |
| `src/maps/map0_s00/Chara_Stalker.c` | unlinked | uncompiled |
| `src/maps/map0_s00/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map0_s00/map0_s00_anim_info.c` | unlinked | uncompiled |
| `src/maps/map0_s00/map0_s00_header.c` | unlinked | uncompiled |
| `src/maps/map0_s00/particle.c` | unlinked | uncompiled |
| `src/maps/map0_s00/player.c` | unlinked | uncompiled |
| `src/maps/map0_s01/Chara_AirScreamer.c` | unlinked | uncompiled |
| `src/maps/map0_s01/Chara_Cybil.c` | unlinked | uncompiled |
| `src/maps/map0_s01/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map0_s01/map0_s01_anim_info.c` | unlinked | uncompiled |
| `src/maps/map0_s01/map0_s01_header.c` | unlinked | uncompiled |
| `src/maps/map0_s01/particle.c` | unlinked | uncompiled |
| `src/maps/map0_s01/particle_glass.c` | unlinked | uncompiled |
| `src/maps/map0_s01/player.c` | unlinked | uncompiled |
| `src/maps/map0_s02/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map0_s02/map0_s02_anim_info.c` | unlinked | uncompiled |
| `src/maps/map0_s02/map0_s02_header.c` | unlinked | uncompiled |
| `src/maps/map0_s02/particle.c` | unlinked | uncompiled |
| `src/maps/map0_s02/player.c` | unlinked | uncompiled |
| `src/maps/map1_s00/Chara_LarvalStalker.c` | unlinked | uncompiled |
| `src/maps/map1_s00/Chara_Stalker.c` | unlinked | uncompiled |
| `src/maps/map1_s00/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map1_s00/map1_s00_anim_info.c` | unlinked | uncompiled |
| `src/maps/map1_s00/map1_s00_header.c` | unlinked | uncompiled |
| `src/maps/map1_s00/particle.c` | unlinked | uncompiled |
| `src/maps/map1_s00/player.c` | unlinked | uncompiled |
| `src/maps/map1_s01/Chara_Cat.c` | unlinked | uncompiled |
| `src/maps/map1_s01/Chara_LarvalStalker.c` | unlinked | uncompiled |
| `src/maps/map1_s01/Chara_Stalker.c` | unlinked | uncompiled |
| `src/maps/map1_s01/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map1_s01/map1_s01_anim_info.c` | unlinked | uncompiled |
| `src/maps/map1_s01/map1_s01_header.c` | unlinked | uncompiled |
| `src/maps/map1_s01/particle.c` | unlinked | uncompiled |
| `src/maps/map1_s01/player.c` | unlinked | uncompiled |
| `src/maps/map1_s02/Chara_Creeper.c` | unlinked | uncompiled |
| `src/maps/map1_s02/Chara_Stalker.c` | unlinked | uncompiled |
| `src/maps/map1_s02/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map1_s02/map1_s02_anim_info.c` | unlinked | uncompiled |
| `src/maps/map1_s02/map1_s02_header.c` | unlinked | uncompiled |
| `src/maps/map1_s02/particle.c` | unlinked | uncompiled |
| `src/maps/map1_s02/particle_water.c` | unlinked | uncompiled |
| `src/maps/map1_s02/player.c` | unlinked | uncompiled |
| `src/maps/map1_s03/Chara_Creeper.c` | unlinked | uncompiled |
| `src/maps/map1_s03/Chara_Stalker.c` | unlinked | uncompiled |
| `src/maps/map1_s03/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map1_s03/map1_s03_anim_info.c` | unlinked | uncompiled |
| `src/maps/map1_s03/map1_s03_header.c` | unlinked | uncompiled |
| `src/maps/map1_s03/particle.c` | unlinked | uncompiled |
| `src/maps/map1_s03/particle_water.c` | unlinked | uncompiled |
| `src/maps/map1_s03/player.c` | unlinked | uncompiled |
| `src/maps/map1_s04/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map1_s04/map1_s04_anim_info.c` | unlinked | uncompiled |
| `src/maps/map1_s04/map1_s04_header.c` | unlinked | uncompiled |
| `src/maps/map1_s04/particle.c` | unlinked | uncompiled |
| `src/maps/map1_s04/player.c` | unlinked | uncompiled |
| `src/maps/map1_s05/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map1_s05/map1_s05_anim_info.c` | unlinked | uncompiled |
| `src/maps/map1_s05/map1_s05_header.c` | unlinked | uncompiled |
| `src/maps/map1_s05/particle.c` | unlinked | uncompiled |
| `src/maps/map1_s05/player.c` | unlinked | uncompiled |
| `src/maps/map1_s06/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map1_s06/map1_s06_anim_info.c` | unlinked | uncompiled |
| `src/maps/map1_s06/map1_s06_header.c` | unlinked | uncompiled |
| `src/maps/map1_s06/particle.c` | unlinked | uncompiled |
| `src/maps/map1_s06/player.c` | unlinked | uncompiled |
| `src/maps/map2_s00/Chara_AirScreamer.c` | unlinked | uncompiled |
| `src/maps/map2_s00/Chara_Groaner.c` | unlinked | uncompiled |
| `src/maps/map2_s00/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map2_s00/map2_s00_anim_info.c` | unlinked | uncompiled |
| `src/maps/map2_s00/map2_s00_header.c` | unlinked | uncompiled |
| `src/maps/map2_s00/particle.c` | unlinked | uncompiled |
| `src/maps/map2_s00/player.c` | unlinked | uncompiled |
| `src/maps/map2_s01/Chara_Dahlia.c` | unlinked | uncompiled |
| `src/maps/map2_s01/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map2_s01/map2_s01_anim_info.c` | unlinked | uncompiled |
| `src/maps/map2_s01/map2_s01_header.c` | unlinked | uncompiled |
| `src/maps/map2_s01/particle.c` | unlinked | uncompiled |
| `src/maps/map2_s01/player.c` | unlinked | uncompiled |
| `src/maps/map2_s02/Chara_AirScreamer.c` | unlinked | uncompiled |
| `src/maps/map2_s02/Chara_Groaner.c` | unlinked | uncompiled |
| `src/maps/map2_s02/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map2_s02/map2_s02_anim_info.c` | unlinked | uncompiled |
| `src/maps/map2_s02/map2_s02_header.c` | unlinked | uncompiled |
| `src/maps/map2_s02/particle.c` | unlinked | uncompiled |
| `src/maps/map2_s02/player.c` | unlinked | uncompiled |
| `src/maps/map2_s03/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map2_s03/map2_s03_anim_info.c` | unlinked | uncompiled |
| `src/maps/map2_s03/map2_s03_header.c` | unlinked | uncompiled |
| `src/maps/map2_s03/particle.c` | unlinked | uncompiled |
| `src/maps/map2_s03/player.c` | unlinked | uncompiled |
| `src/maps/map2_s04/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map2_s04/map2_s04_anim_info.c` | unlinked | uncompiled |
| `src/maps/map2_s04/map2_s04_header.c` | unlinked | uncompiled |
| `src/maps/map2_s04/particle.c` | unlinked | uncompiled |
| `src/maps/map2_s04/player.c` | unlinked | uncompiled |
| `src/maps/map3_s00/Chara_Kaufmann.c` | unlinked | uncompiled |
| `src/maps/map3_s00/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map3_s00/map3_s00_anim_info.c` | unlinked | uncompiled |
| `src/maps/map3_s00/map3_s00_header.c` | unlinked | uncompiled |
| `src/maps/map3_s00/particle.c` | unlinked | uncompiled |
| `src/maps/map3_s00/player.c` | unlinked | uncompiled |
| `src/maps/map3_s01/Chara_Creeper.c` | unlinked | uncompiled |
| `src/maps/map3_s01/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map3_s01/map3_s01_anim_info.c` | unlinked | uncompiled |
| `src/maps/map3_s01/map3_s01_header.c` | unlinked | uncompiled |
| `src/maps/map3_s01/particle.c` | unlinked | uncompiled |
| `src/maps/map3_s01/player.c` | unlinked | uncompiled |
| `src/maps/map3_s02/Chara_Alessa.c` | unlinked | uncompiled |
| `src/maps/map3_s02/Chara_Creeper.c` | unlinked | uncompiled |
| `src/maps/map3_s02/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map3_s02/map3_s02_anim_info.c` | unlinked | uncompiled |
| `src/maps/map3_s02/map3_s02_header.c` | unlinked | uncompiled |
| `src/maps/map3_s02/particle.c` | unlinked | uncompiled |
| `src/maps/map3_s02/player.c` | unlinked | uncompiled |
| `src/maps/map3_s03/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map3_s03/map3_s03_anim_info.c` | unlinked | uncompiled |
| `src/maps/map3_s03/map3_s03_header.c` | unlinked | uncompiled |
| `src/maps/map3_s03/particle.c` | unlinked | uncompiled |
| `src/maps/map3_s03/player.c` | unlinked | uncompiled |
| `src/maps/map3_s04/Chara_Lisa.c` | unlinked | uncompiled |
| `src/maps/map3_s04/Chara_PuppetNurse.c` | unlinked | uncompiled |
| `src/maps/map3_s04/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map3_s04/map3_s04_anim_info.c` | unlinked | uncompiled |
| `src/maps/map3_s04/map3_s04_header.c` | unlinked | uncompiled |
| `src/maps/map3_s04/particle.c` | unlinked | uncompiled |
| `src/maps/map3_s04/player.c` | unlinked | uncompiled |
| `src/maps/map3_s05/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map3_s05/map3_s05_anim_info.c` | unlinked | uncompiled |
| `src/maps/map3_s05/map3_s05_header.c` | unlinked | uncompiled |
| `src/maps/map3_s05/particle.c` | unlinked | uncompiled |
| `src/maps/map3_s06/Chara_Dahlia.c` | unlinked | uncompiled |
| `src/maps/map3_s06/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map3_s06/map3_s06_anim_info.c` | unlinked | uncompiled |
| `src/maps/map3_s06/map3_s06_header.c` | unlinked | uncompiled |
| `src/maps/map3_s06/particle.c` | unlinked | uncompiled |
| `src/maps/map3_s06/player.c` | unlinked | uncompiled |
| `src/maps/map4_s00/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map4_s00/map4_s00_anim_info.c` | unlinked | uncompiled |
| `src/maps/map4_s00/map4_s00_header.c` | unlinked | uncompiled |
| `src/maps/map4_s00/particle.c` | unlinked | uncompiled |
| `src/maps/map4_s00/player.c` | unlinked | uncompiled |
| `src/maps/map4_s01/Chara_Cybil.c` | unlinked | uncompiled |
| `src/maps/map4_s01/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map4_s01/map4_s01_anim_info.c` | unlinked | uncompiled |
| `src/maps/map4_s01/map4_s01_header.c` | unlinked | uncompiled |
| `src/maps/map4_s01/particle.c` | unlinked | uncompiled |
| `src/maps/map4_s01/player.c` | unlinked | uncompiled |
| `src/maps/map4_s02/Chara_AirScreamer.c` | unlinked | uncompiled |
| `src/maps/map4_s02/Chara_Groaner.c` | unlinked | uncompiled |
| `src/maps/map4_s02/Chara_Romper.c` | unlinked | uncompiled |
| `src/maps/map4_s02/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map4_s02/map4_s02_anim_info.c` | unlinked | uncompiled |
| `src/maps/map4_s02/map4_s02_header.c` | unlinked | uncompiled |
| `src/maps/map4_s02/particle.c` | unlinked | uncompiled |
| `src/maps/map4_s02/player.c` | unlinked | uncompiled |
| `src/maps/map4_s03/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map4_s03/map4_s03_anim_info.c` | unlinked | uncompiled |
| `src/maps/map4_s03/map4_s03_header.c` | unlinked | uncompiled |
| `src/maps/map4_s03/particle.c` | unlinked | uncompiled |
| `src/maps/map4_s03/particle_acid.c` | unlinked | uncompiled |
| `src/maps/map4_s03/player.c` | unlinked | uncompiled |
| `src/maps/map4_s04/Chara_Lisa.c` | unlinked | uncompiled |
| `src/maps/map4_s04/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map4_s04/map4_s04_anim_info.c` | unlinked | uncompiled |
| `src/maps/map4_s04/map4_s04_header.c` | unlinked | uncompiled |
| `src/maps/map4_s04/particle.c` | unlinked | uncompiled |
| `src/maps/map4_s04/player.c` | unlinked | uncompiled |
| `src/maps/map4_s05/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map4_s05/map4_s05_anim_info.c` | unlinked | uncompiled |
| `src/maps/map4_s05/map4_s05_header.c` | unlinked | uncompiled |
| `src/maps/map4_s05/particle.c` | unlinked | uncompiled |
| `src/maps/map4_s05/particle_acid.c` | unlinked | uncompiled |
| `src/maps/map4_s05/player.c` | unlinked | uncompiled |
| `src/maps/map4_s06/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map4_s06/map4_s06_anim_info.c` | unlinked | uncompiled |
| `src/maps/map4_s06/map4_s06_header.c` | unlinked | uncompiled |
| `src/maps/map4_s06/particle.c` | unlinked | uncompiled |
| `src/maps/map4_s06/player.c` | unlinked | uncompiled |
| `src/maps/map5_s00/Chara_Creeper.c` | unlinked | uncompiled |
| `src/maps/map5_s00/Chara_HangedScratcher.c` | unlinked | uncompiled |
| `src/maps/map5_s00/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map5_s00/map5_s00_anim_info.c` | unlinked | uncompiled |
| `src/maps/map5_s00/map5_s00_header.c` | unlinked | uncompiled |
| `src/maps/map5_s00/particle.c` | unlinked | uncompiled |
| `src/maps/map5_s00/player.c` | unlinked | uncompiled |
| `src/maps/map5_s01/Chara_AirScreamer.c` | unlinked | uncompiled |
| `src/maps/map5_s01/Chara_Groaner.c` | unlinked | uncompiled |
| `src/maps/map5_s01/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map5_s01/map5_s01_anim_info.c` | unlinked | uncompiled |
| `src/maps/map5_s01/map5_s01_header.c` | unlinked | uncompiled |
| `src/maps/map5_s01/particle.c` | unlinked | uncompiled |
| `src/maps/map5_s01/player.c` | unlinked | uncompiled |
| `src/maps/map5_s02/Chara_Kaufmann.c` | unlinked | uncompiled |
| `src/maps/map5_s02/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map5_s02/map5_s02_anim_info.c` | unlinked | uncompiled |
| `src/maps/map5_s02/map5_s02_header.c` | unlinked | uncompiled |
| `src/maps/map5_s02/particle.c` | unlinked | uncompiled |
| `src/maps/map5_s03/Chara_Kaufmann.c` | unlinked | uncompiled |
| `src/maps/map5_s03/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map5_s03/map5_s03_anim_info.c` | unlinked | uncompiled |
| `src/maps/map5_s03/map5_s03_header.c` | unlinked | uncompiled |
| `src/maps/map5_s03/particle.c` | unlinked | uncompiled |
| `src/maps/map5_s03/player.c` | unlinked | uncompiled |
| `src/maps/map6_s00/Chara_AirScreamer.c` | unlinked | uncompiled |
| `src/maps/map6_s00/Chara_Groaner.c` | unlinked | uncompiled |
| `src/maps/map6_s00/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map6_s00/map6_s00_anim_info.c` | unlinked | uncompiled |
| `src/maps/map6_s00/map6_s00_header.c` | unlinked | uncompiled |
| `src/maps/map6_s00/particle.c` | unlinked | uncompiled |
| `src/maps/map6_s00/player.c` | unlinked | uncompiled |
| `src/maps/map6_s01/Chara_Cybil.c` | unlinked | uncompiled |
| `src/maps/map6_s01/Chara_Dahlia.c` | unlinked | uncompiled |
| `src/maps/map6_s01/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map6_s01/map6_s01_anim_info.c` | unlinked | uncompiled |
| `src/maps/map6_s01/map6_s01_header.c` | unlinked | uncompiled |
| `src/maps/map6_s01/particle.c` | unlinked | uncompiled |
| `src/maps/map6_s01/player.c` | unlinked | uncompiled |
| `src/maps/map6_s02/Chara_Alessa.c` | unlinked | uncompiled |
| `src/maps/map6_s02/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map6_s02/map6_s02_anim_info.c` | unlinked | uncompiled |
| `src/maps/map6_s02/map6_s02_header.c` | unlinked | uncompiled |
| `src/maps/map6_s02/particle.c` | unlinked | uncompiled |
| `src/maps/map6_s02/player.c` | unlinked | uncompiled |
| `src/maps/map6_s03/Chara_HangedScratcher.c` | unlinked | uncompiled |
| `src/maps/map6_s03/Chara_Stalker.c` | unlinked | uncompiled |
| `src/maps/map6_s03/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map6_s03/map6_s03_anim_info.c` | unlinked | uncompiled |
| `src/maps/map6_s03/map6_s03_header.c` | unlinked | uncompiled |
| `src/maps/map6_s03/particle.c` | unlinked | uncompiled |
| `src/maps/map6_s03/player.c` | unlinked | uncompiled |
| `src/maps/map6_s04/Chara_AlessaDahlia.c` | unlinked | uncompiled |
| `src/maps/map6_s04/Chara_LarvalStalker.c` | unlinked | uncompiled |
| `src/maps/map6_s04/Chara_MonsterCybil.c` | unlinked | uncompiled |
| `src/maps/map6_s04/Chara_Stalker.c` | unlinked | uncompiled |
| `src/maps/map6_s04/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map6_s04/map6_s04_anim_info.c` | unlinked | uncompiled |
| `src/maps/map6_s04/map6_s04_header.c` | unlinked | uncompiled |
| `src/maps/map6_s04/particle.c` | unlinked | uncompiled |
| `src/maps/map6_s04/player.c` | unlinked | uncompiled |
| `src/maps/map6_s05/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map6_s05/map6_s05_anim_info.c` | unlinked | uncompiled |
| `src/maps/map6_s05/map6_s05_header.c` | unlinked | uncompiled |
| `src/maps/map6_s05/particle.c` | unlinked | uncompiled |
| `src/maps/map6_s05/player.c` | unlinked | uncompiled |
| `src/maps/map7_s00/Chara_Lisa.c` | unlinked | uncompiled |
| `src/maps/map7_s00/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map7_s00/map7_s00_anim_info.c` | unlinked | uncompiled |
| `src/maps/map7_s00/map7_s00_header.c` | unlinked | uncompiled |
| `src/maps/map7_s00/particle.c` | unlinked | uncompiled |
| `src/maps/map7_s00/player.c` | unlinked | uncompiled |
| `src/maps/map7_s01/Chara_BloodyLisa.c` | unlinked | uncompiled |
| `src/maps/map7_s01/Chara_GhostChildAlessa.c` | unlinked | uncompiled |
| `src/maps/map7_s01/Chara_Lisa.c` | unlinked | uncompiled |
| `src/maps/map7_s01/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map7_s01/map7_s01_anim_info.c` | unlinked | uncompiled |
| `src/maps/map7_s01/map7_s01_header.c` | unlinked | uncompiled |
| `src/maps/map7_s01/particle.c` | unlinked | uncompiled |
| `src/maps/map7_s01/particle_glass.c` | unlinked | uncompiled |
| `src/maps/map7_s02/Chara_Bloodsucker.c` | unlinked | uncompiled |
| `src/maps/map7_s02/Chara_BloodyLisa.c` | unlinked | uncompiled |
| `src/maps/map7_s02/Chara_Dahlia.c` | unlinked | uncompiled |
| `src/maps/map7_s02/Chara_GhostChildAlessa.c` | unlinked | uncompiled |
| `src/maps/map7_s02/Chara_GhostDoctor.c` | unlinked | uncompiled |
| `src/maps/map7_s02/Chara_Kaufmann.c` | unlinked | uncompiled |
| `src/maps/map7_s02/Chara_Lisa.c` | unlinked | uncompiled |
| `src/maps/map7_s02/Chara_Stalker.c` | unlinked | uncompiled |
| `src/maps/map7_s02/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map7_s02/map7_s02_anim_info.c` | unlinked | uncompiled |
| `src/maps/map7_s02/map7_s02_header.c` | unlinked | uncompiled |
| `src/maps/map7_s02/particle.c` | unlinked | uncompiled |
| `src/maps/map7_s02/player.c` | unlinked | uncompiled |
| `src/maps/map7_s03/Chara_Alessa.c` | unlinked | uncompiled |
| `src/maps/map7_s03/Chara_BloodyIncubator.c` | unlinked | uncompiled |
| `src/maps/map7_s03/Chara_BloodyLisa.c` | unlinked | uncompiled |
| `src/maps/map7_s03/Chara_Cybil.c` | unlinked | uncompiled |
| `src/maps/map7_s03/Chara_Dahlia.c` | unlinked | uncompiled |
| `src/maps/map7_s03/Chara_Incubator.c` | unlinked | uncompiled |
| `src/maps/map7_s03/Chara_Incubus.c` | unlinked | uncompiled |
| `src/maps/map7_s03/Chara_Kaufmann.c` | unlinked | uncompiled |
| `src/maps/map7_s03/Chara_LittleIncubus.c` | unlinked | uncompiled |
| `src/maps/map7_s03/Chara_Unknown23.c` | unlinked | uncompiled |
| `src/maps/map7_s03/chara_spawns.h` | unlinked | uncompiled |
| `src/maps/map7_s03/chara_util.c` | unlinked | uncompiled |
| `src/maps/map7_s03/map7_s03_anim_info.c` | unlinked | uncompiled |
| `src/maps/map7_s03/map7_s03_header.c` | unlinked | uncompiled |
| `src/maps/map7_s03/particle.c` | unlinked | uncompiled |
| `src/maps/map7_s03/player.c` | unlinked | uncompiled |
| `src/maps/map_util.c` | unlinked | uncompiled |
| `src/maps/particle.c` | unlinked | uncompiled |
| `src/maps/particle_acid.c` | unlinked | uncompiled |
| `src/maps/particle_glass.c` | unlinked | uncompiled |
| `src/maps/particle_water.c` | unlinked | uncompiled |
