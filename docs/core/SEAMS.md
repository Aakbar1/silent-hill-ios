# Host seams and replay

`native::run_with_backends` accepts boxed GpuBackend, SpuBackend and PadSource. C owns OT token resolution; GPU begin/packet/end callbacks preserve traversal order and include the original 32-bit tag. GPU VRAM upload, readback, clear and copy go through the same backend. Software Raster remains the default. No psxgpu/psxspu source changes.

SPU interface is register/voice/sample-RAM level. The silent boot fallback stores state and bounds-checks transfers; it is not a sequencer or audible mixer. C SpuInit calls the selected backend. Full libsd/libspu linkage is still pending.

Core2 adds `GpuBackend::frame_rgb24`, reading packed RGB bytes from existing VRAM, and `SpuBackend::cd_input`/`cd_stop` for psxmedia-decoded XA PCM. The default CD input returns false, which the movie service logs; audible output is not claimed. No GPU/SPU lane files were changed. Original movie state handlers drive the native bounded streaming loop and Start skip.

libpad reads an eight-byte connected DualShock packet from PadSource; port 1 is disconnected. Keyboard and XInput supply the shared sh-touch PadState packet encoder. The original Joy click/release/pulse/analog processing is now linked against native pointer-free records. Full libkpad/vibration remain unlinked. Hardware controller interaction has not been manually tested.

Keyboard: arrows/WASD direction, Enter Start, Space Cross, Escape Circle, left Shift Square, Tab Select, M Triangle, Q/E L1/R1, Z/C L2/R2. Focus loss releases input. XInput face buttons map A/B/X/Y to Cross/Circle/Square/Triangle; shoulders/triggers map L1/R1/L2/R2.

Replay: `--input docs/core/replays/boot.txt`. Rows are `tick active_high_hex [rx ry lx ly]`, axes default to 128; tick zero is required, times strictly increase, and each row holds until replaced. Replays replace live input. Tick is the number of completed VBlanks; multiple reads in a tick agree.

Core3 adds the defaulted, backward-compatible `GpuBackend::packet_precise` hook. Exact integer packets remain authoritative; optional GTE coordinates travel beside polygons when built with `--features precise-vertices`. Metadata is consumed once and discarded at OT completion. The software menu Raster ignores it; a renderer must implement the hook before it affects presentation. Existing backend/pad implementations need no changes.

Current reproduction: `tools/milestones.cmd --available --repeat 2`. The runner creates private/work/core3 output folders and checks original state/menu endpoints, nonblack pixels and repeat PNG hashes. See [MILESTONES.md](MILESTONES.md). The full-brief gate fails until missing screens/maps are implemented. Earlier core/core2 captures are historical.
