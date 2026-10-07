# Native overlays: core2 checkpoint

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

The deepest New Game replay confirms NORMAL at tick 1650, executes original 636-byte save initialization, then deliberately stops at GameBoot_WorldInit's missing native model/animation/collision consumers: state 7, step 1, menu 3, native code 3, process exit 1. Opening movie, MAP0_S00 walking, MAP0_S01 cafe and door transitions are not reached. The declaration-only PortMapHeader is a guarded seam, not a loaded map descriptor.

The opt-in broad original-header probe still fails with 43 C2118 ABI assertions; no assertions were disabled. Native records/asset adapters, GTE and scratch/packet contracts must precede first-map integration. See [ABI.md](ABI.md), [MILESTONES.md](MILESTONES.md) and [SURVEY.md](../survey/SURVEY.md).
