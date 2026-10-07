# Native overlays: current integration

B_KONAMI is the only integrated screen overlay (one of five); no map overlay is integrated (zero of 43). Step 4 is partial. Each of its six exported functions now uses sh_b_konami_* symbols. The release object symbol table was inspected with dumpbin. BODYPROG's existing state table binds to these native names; no MIPS entry is executed.

Its sole writable static, the KCET next-state byte, is a separate native object with a compiled constant initial image. Fs_QueueUpdate activates B_KONAMI on its original file ID 4 and restores that image. A C/Rust test dirties and resets this state and checks repeat activation/unknown-ID rejection. Other overlays still need their own namespaces, native entry descriptors and complete initialized-data/BSS/reset inventories. The current activation function is deliberately limited to B_KONAMI.

The full original Konami/KCET source now compiles, together with original USA settings/control presets, selected screen FS functions and the BG_ETC texture loader. Native pointer arithmetic replaces the KCET packet-base truncation; the unused NTSC-J local is excluded in USA. The host reports two absent memory cards because a native card backend is not implemented. It does not synthesize a save or claim a successful card operation. Demo/media/audio helpers remain logged stubs; movies have not been skipped yet.

Replay evidence (all captures/logs under ../../private/work/core/):
```
tools/dev-cargo.cmd run --release -- --frames 850 --input docs/core/replays/boot.txt --screenshot "C:/Claude Projects/Silent Hill iOS/private/work/core/kcet-850.png"
tools/dev-cargo.cmd run --release -- --frames 1600 --input docs/core/replays/boot.txt --screenshot "C:/Claude Projects/Silent Hill iOS/private/work/core/final-stop.png"
tools/dev-cargo.cmd run --release -- --frames 1600 --input docs/core/replays/kcet-skip.txt --screenshot "C:/Claude Projects/Silent Hill iOS/private/work/core/kcet-skip-stop.png"
```
850 ticks succeeds in state 2/step 6 with 4,628 primitives; the visible KCET image was inspected. Neutral replay reaches state 3 at VBlank 987, then stops after 989 completed ticks at GameState_MovieIntroFadeIn_Update (C code 3, process exit 1, 5,125 primitives). The actual last frame is saved/presented; it is black after the KCET fade.

The input replay presses Start at tick 850 and releases at 851. C logs both edges; the game takes its original early logo-exit branch and reaches the same guard after 929 ticks (60 earlier). This verifies PadSource-to-libpad-to-native-game input. Physical keyboard/XInput hardware has not been manually tested, and no walking claim is made.

The broad original-header probe was rerun and exits 101 with 43 negative-array size assertion diagnostics. It is separate from the passing native subset and wire schema checks. The full resident/map C clients still need native records/decoder adapters and GTE/scratch/packet migration; merely linking and namespacing all their source is not sufficient. Title, New Game and first-map walking remain unimplemented/unproven. No remaining overlay is represented as successfully loaded native code.
