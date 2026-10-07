# Audio lane checkpoint

Original libsd (`smf_main`, `smf_mid`, `smf_io`, `smf_snd`), the complete `sd_call` task/voice/bank driver, its sound tables, and the BGM layer controller are linked. `Bgm_Update(false)` now executes the original MAP0_S00 callback and layer logic. The next measured guard is **World_NearbyPlayerCollisionTriggersGet/native nearby trigger classification**, state **11 / step 2 / VBlank 2274**, before camera/player updates. The host's final CHECK shows step 0, the last presented frame. Movement is not established.

Reference: `d9e28f8315c7938117224f21516786d9d149a145`, GPL-3.0-only, Copyright (C) 2026 shdecompilations. `git submodule update --init` completed. No game bytes, extracted assets, disassembly, or captures are committed. Audio evidence is in `../../private/work/audio/`.

## Implementation

- `tools/prepare_audio.py` applies `crates/psxspu/port/prepare_libsd.py`'s strict five fixes before native transformations. Generated SDK longs are 32-bit words. Native VAB/SMF pointer records retain explicit size assertions. PS1 scalar narrowing is explicit and source-pinned. Allocator lookahead gets a zero sentinel; a missing tone returns failure instead of an uninitialized voice. The BGM channel-task temporary retains upstream enum-bool's 32-bit value until its predicate conversion: native `_Bool` would lose the `0xffff` stop sentinel. An original-controller fixture checks that stopped channels mute and restores the touched fields.
- Original VAB/KDT loading state machines use bounded native buffers and synchronous owned-disc logical-sector reads. Original libsd owns voice selection, note-to-pitch, MIDI/NRPN, sequencing, fades and bank addresses. MAP0_S00's missing scalar layer/flag tables are read at runtime from its owned overlay; disk pointer slots are never widened.
- The sole C/game worker interleaves original `smf_timer` calls with SPU sample advances at `33868800 / (8 * 7328)` Hz. Integer phase survives each VBlank; one VBlank is 735 frames. C callbacks run outside the Rust HOST borrow. The cpal callback consumes PCM only; no standalone second sequencer runs.
- PsyQ SPU calls translate to the same SpuCpal hardware registers/RAM. Key-on polling observes pending key-on immediately. Attribute reads select the lowest voice-mask bit. Original CD mix/volume writes surround native movie playback. Original XA task commands select/seek/pause raw sectors through XaStream and SpuCpal, with PsyQ matrix order converted explicitly and EOF terminating reads.
- Desktop's existing default SpuCpal factory is retained; `--audio off` still runs the sound engine with discarded PCM, and `--audio wav:PATH` writes a new file under `private/work/`. Generated UIKit worker code replaces its SilentSpu injection with SpuCpal and checks finalization. `port/audio_session.c` sets playback category, 44.1kHz/256-frame preferences and activation before cpal. [Apple's playback category documentation](https://developer.apple.com/documentation/avfaudio/avaudiosession/category-swift.struct/playback) specifies playback with the Ring/Silent switch set to silent. Actual Apple SDK/device playback and interruption/route recovery are **unverified**.
- Move-owned source files and generators are unchanged. The audio preparation runs after gameplay/map/render preparation, adapts only obsolete sound definitions in generated caller copies, and supplies the original map BGM callback. Shared build/runtime/worker additions are separate hunks.

## Verification and replay timing

MSVC release C builds with `/W4 /WX`; fmt, workspace/all-target Clippy `--features precise-vertices -- -D warnings`, workspace tests, separate psxspu tests and the two pinned source-fix tests pass. Arm64 audio frontend: 8/8; existing rendering frontend: 20/20; player frontend: 26/26; LP64 layout positive/negative controls pass. These are frontend checks, not an Apple SDK/device build.

Run `python tools/prepare_audio.py --milestones` after a release build. It retains every existing state/step/menu/movie-frame/skip/visible-pixel assertion, shifts nonzero replay input ticks and endpoints by the **measured 24-VBlank BASE/FIRST bank-load delay**, and adds WAV checks. Result: **11/11**. The unchanged `tools/milestones.py --available` uses the old silent-sink absolute timings and fails its movie-frame threshold (141 versus required 147 at frame 1900); that failure was not hidden or its threshold lowered.

| Recording | Checked window | Peak | Nonzero samples | Output rails |
|---|---|---:|---:|---:|
| Title movie soundtrack | Complete 2060-frame intro through title entry | 18435 | 12049053 | 0 |
| Menu SFX | Confirm/menu window after movie audio is muted | 11706 | 24384 | 0 |
| First-map BGM/ambience | 220500 stereo frames after the genuine next guard | 1894 | 439026 | 0 |

The verified title-entry replay issues no music start task after the movie; the title recording is the title movie's soundtrack, followed by title entry. First-map audio uses `SH_AUDIO_AMBIENCE_PROBE=1`: it first reaches the real gameplay guard, then runs only the original BGM/task/timer services for five seconds. The process retains the guard exit code. This checks loaded first-map music, not five seconds of functioning gameplay. Zero output rails are the numerical clipping check; no PS1 golden recording is claimed.

`first_map_view` passes separately at the new exact guard/frame/movie counts, with 95 colors and the original world/fog/queue assertions retained. The generated test writes its capture under `private/work/audio/`; it never replaces a successful gameplay update.

Final WAV/controller-fixture evidence: `private/work/audio/milestones/20261007T190926771517Z/results.json`. Build, fmt/clippy/tests (`gates-final.json`), SPU tests, frontend/layout and rendering evidence logs are in `private/work/audio/`.

Default desktop device playback also completes the native 1680-VBlank menu replay: 1234800 mixer frames, peak 11706, 114071 nonzero samples, zero output rails, zero device errors and zero underruns (`device-game-final.log`). An earlier repeatable 314-frame **shutdown-only** underrun came from a full producer ring leaving no room for the old silence padding. Finalization now waits for space and appends the complete device-only silence tail; WAVs and the game clock do not gain padding. The separate three-second device smoke test also passes with zero underruns/errors. Speaker latency and perceptual A/V sync remain unmeasured.

## Handoff to move/director

1. Move: implement the nearby-player collision-trigger classification at the guard above, then resume original camera/player dispatch. BGM is already running; do not remove or stub the controller. Use the unshifted `docs/core/replays/first_map.txt` to reproduce VBlank 2274.
2. Director: reconcile the audio preparation seam with move's later generated map/guard changes; do not duplicate libsd/BGM symbols. Update the global milestone runner to account for the verified real bank-load startup delay and update the root checkpoint/permanent arm64 source list.
3. Mac lane: build/run the actual iOS SDK target and verify playback, silent-switch behavior, interruptions and route changes. Cold XA dialogue paths beyond this opening replay and positional `Sfx_*` gameplay helpers are not certified by these milestones.
