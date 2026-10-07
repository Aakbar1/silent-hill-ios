# Native overlays: core4 checkpoint

Three of five screens are linked; zero of 43 map overlays are linked. The brief's first-map pass is **not achieved**.

| Overlay | Original file ID | Native implementation | Verification |
|---|---:|---|---|
| B_KONAMI | 4 | sh_b_konami_*; next-state byte reset on load | KCET replay, dirty/reset probe |
| STREAM | 2043 | sh_stream_* state handlers; psxmedia STR/XA service | RGB24 capture, 2060-frame end, Start skip and return to title |
| OPTION | 2040 | sh_option_*; 10 writable globals and 14 local statics reset from compiled initial images | Main screen, brightness, controller bindings, return/re-entry; probe compares all 24 restored objects |
| SAVELOAD | — | Explicit state guard; native file service ready but UI/backend wiring absent | No save/load UI claim |
| STF_ROLL | — | Explicit state guard | No credits claim |

Fs_QueueUpdate activates screens at queued read completion, then restores native initialized state. Code is compiled from pinned GPL source; disc overlay bytes are never executed. The namespace/reset generator is deliberately specific to OPTION, with a pinned writable-inventory assertion; it does not process maps generically. Its read-only static string pointer arrays are made explicitly const.

STREAM retains the upstream state handlers and original open_main end-frame argument. Its SDK movie loop is replaced by bounded raw-sector decoding, one video frame ahead, and a bounded XA queue. Video uses the virtual 60 Hz VBlank clock at 15 fps, writes packed RGB24 into GpuBackend VRAM, and displays that same VRAM. SpuBackend receives decoded CD PCM if supported; the default silent sink logs its absence. Skip/end clear CD samples without resetting SPU voices. Natural end and Start skip return through the original game-state transitions. Warm reset, alternate-intro semantics and audible synchronization are not verified.

The original title, glyph renderers, controller processing, RNG and GPL sine table are linked. Native menu raster support includes untextured polygons, lines and fixed-size sprites; PS1 subpixel/dither fidelity remains uncalibrated. GPU/SPU lane sources were not edited.

Core4 replaces PortMapHeader with native map/callback/data records. MAP0_S00's GPL initializers supply points, events, spawns, camera paths, collision triggers, messages and animation info. Sixteen linked writable objects reset from compiled initial/zero images; the original room callback consumes two bounded room-grid reads from the owned disc. Seventy-six callbacks remain explicit guards: one descriptor slice, zero complete gameplay maps. Shared included gameplay code and its local statics remain unlinked; the generator does not certify a generic reset for all 43 maps.

NORMAL at tick 1650 completes the retained original WorldInit, then original GameBoot_MapLoad queues the descriptor and headerless HB_M0S00 frame block. Original spawn/reset initializes (-25395,0,657408), heading 2048 and initial camera heading 2048. STREAM temporarily changes the screen namespace while the native map descriptor remains selected. The original flow enters the opening movie at VBlank 1674. The opening replay passes at tick 2200 in state 9/step 0 with 153 cumulative movie frames (22 intro, 131 opening); its private screenshot was visually reviewed.

Start at tick 2201 exits the movie to state 10 at tick 2203. The original loading-screen dispatcher stops at map0_s00/GameBoot_LoadScreen_PlayerRun at tick 2204, native code 3/process exit 1. MAP0_S00 walking, MAP0_S01, world/camera/player updates and transitions remain unverified. Captures/logs are under private/work/core4/new-game-opening and new-game-world-guard; the latter captures the transition boundary without a rendered map.

Core3 recorded the separate broad original-header failure (37 C2118 diagnostics before truncation); it was not rerun or weakened here. Native map/IPD layouts pass MSVC and arm64 checks; eight changed C units pass the installed LLVM frontend with -Werror -Wall -Wextra. Chunk/texture lifecycle, collision queries, camera/world drawing and original gameplay updates remain prerequisites. Actual Apple SDK compilation and iOS runtime were not run. See [ABI.md](ABI.md), [GTE.md](GTE.md), [MILESTONES.md](MILESTONES.md) and [SURVEY.md](../survey/SURVEY.md).
