# Host seams and replay

`native::run_with_backends` accepts boxed GpuBackend, SpuBackend and PadSource. C owns OT token resolution; GPU begin/packet/end callbacks preserve traversal order and include the original 32-bit tag. GPU VRAM upload, readback, clear and copy go through the same backend. Software Raster remains the default. No psxgpu/psxspu source changes.

SPU interface is register/voice/sample-RAM level. The silent boot fallback stores state and bounds-checks transfers; it is not a sequencer or audible mixer. C SpuInit calls the selected backend. Full libsd/libspu linkage is still pending.

libpad reads an eight-byte connected DualShock packet from PadSource; port 1 is disconnected. Keyboard and XInput supply the shared sh-touch PadState packet encoder. Boot Joy glue reads held bits; full game-owned joy/libkpad processing and vibration remain unlinked. Hardware controller interaction has not been manually tested.

Keyboard: arrows/WASD direction, Enter Start, Space Cross, Escape Circle, left Shift Square, Tab Select, M Triangle, Q/E L1/R1, Z/C L2/R2. Focus loss releases input. XInput face buttons map A/B/X/Y to Cross/Circle/Square/Triangle; shoulders/triggers map L1/R1/L2/R2.

Replay: `--input docs/core/replays/boot.txt`. Rows are `tick active_high_hex [rx ry lx ly]`, axes default to 128; tick zero is required, times strictly increase, and each row holds until replaced. Replays replace live input. Tick is the number of completed VBlanks; multiple reads in a tick agree.

Reproduce:
```
tools/dev-cargo.cmd run --release -- --frames 600 --input docs/core/replays/boot.txt --screenshot "C:/Claude Projects/Silent Hill iOS/private/work/core/step2-konami.png"
tools/dev-cargo.cmd run --release -- --frames 800 --input docs/core/replays/boot.txt --screenshot "C:/Claude Projects/Silent Hill iOS/private/work/core/step2-stop.png"
```
600 ticks succeeds (state 1/step 3, 3,413 primitives). 800 requested stops at the explicit KCET guard after 731 (exit 1, C code 3, 4,201 primitives). Guard exit saves/presents the last actual frame, which is black after the logo fade; it does not claim a drawn KCET screen. Captures/logs are private.
