# World/player reconciliation checkpoint

Goal: restore the combined MSVC host build with one declaration owner and retain
both lanes' measured boundaries. This is a merge fix, not a movement feature.
Pinned decomp: `d9e28f8315c7938117224f21516786d9d149a145`; GPL notices retained.

`prepare_gameplay.prepare_shared_types` generates guarded `shared_types.h` from
`include/game.h` and `include/bodyprog/bodyprog.h`. Gameplay records, map constants
and render declarations include it. Original environment enums, 44-byte
`s_MapEffectsInfo`, 52-byte `s_StructUnk3`, and two-byte `s_MapEnvPresetIdxs` have
one declaration. The latter is two `u8` fields in the decomp, not the player
shim's two `s16` fields. The original pointer-bearing environment workspace is
392 bytes on PS1 and 400 bytes natively; size and pointer/preset/live offsets
remain asserted. No assertion was disabled to resolve the collision.

Rendering owns the original shared environment functions and preset tables.
Both lanes use `PortSysWork.gameplayEnvironment`, preserving preceding FFI
offsets and the existing flashlight flag synchronization. Separate rendering
state would let gameplay initialization and drawing diverge, so it was removed.
Player rendering bridges forward to world services; the duplicate no-draw
`WorldGfx_Draw` was removed. No guards for unavailable game logic were bypassed.

Verification: release MSVC `/W4 /WX` build, fmt check, workspace/all-target Clippy
with `precise-vertices` and `-D warnings`, and workspace tests with that feature
pass (300 tests including doctests, zero failed, four opt-in tests ignored).
Available milestones pass 10/10; no repeat/hash comparison was requested.
`first_map_view` separately passes 1/1 and checks the exact original BGM guard:
state 11 / step 2 / VBlank 2266, 154 movie frames, two skips. Last presented host
frame is state 11 / step 0. Capture details are in [WORLD.md](WORLD.md).

MSVC x86/x64 wire checks pass. Arm64 layout positive/negative controls pass;
`prepare_render.py --check-clang` passes 20/20 C units, and the existing private
`private/work/player/check-player-clang.py` passes 26/26. Actual Apple SDK/device
build and runtime remain untested. Evidence logs: `private/work/reconcile/`.
Available replay evidence: `private/work/core5/milestones/20261007T180025058847Z/`.

Open: BGM layer controller, player movement and the other pending full-game
milestones remain as documented in [PLAYER.md](PLAYER.md). Director should update
the root checkpoint and permanent arm64 CI source list, which are outside this
brief's ownership. No main changes, merge, push, assets or system installs.
