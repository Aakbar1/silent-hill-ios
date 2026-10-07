# psxspu

Standalone GPL-3.0 native sound backend for Silent Hill USA 1.1. The mixer
produces 44.1 kHz stereo i16 frames from 24 hardware-style voices and 512 KiB
sound RAM. It uses the read-only sibling `psxmedia` for SPU/XA ADPCM decoding.
`output` (default) adds cpal and a bounded lock-free rtrb transport; disabling
it leaves a platform-independent mixer and file parsers with no unsafe code.

Implemented: Q12 pitch and canonical Gaussian interpolation, block loop flags
and ENDX, ADSR, signed direct/sweep volumes, noise, pitch modulation, CD/external
mixing and capture, manual/DMA transfers, the PS1 reverb network with 39-tap
resampling and all ten PsyQ presets. `sdk` maps the game's used voice/common
attributes, key state, reverb and transfers. Allocation records are optional;
the game already owns its bank allocator and sequencer.

`sequence` supplies checked KDT/KDT1, SEQ and MIDI event parsing and the original
integer event timing. **Production retains game-owned libsd.** `preview` is a
developer audition helper, with unsupported synthesis controls reported rather
than a claim to replace the game's voice allocation/modulation logic.

From this directory:

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --no-default-features
cargo run --release --example bench
cargo run --release --example device
cargo run --release --example disc
```

`device` plays a quiet synthetic 440 Hz tone for three seconds. `disc` verifies
all 92 VAB banks and steps all 40 KDT tracks, then renders six BASE-bank effects,
30 seconds of A2 music, and XA item 1. It uses the local survey inventory and
the owner's disc; `--disc`, `--inventory`, and `--seconds` override inputs.
Every generated WAV stays under `private/work/spu/`, outside this repository.
The music audition activates all layers through the equivalent of the game's
external MIDI-volume control at 0.1 s, using sequence volume 40. Actual gameplay
selects/fades these layers according to world state.

See [INTEGRATION.md](INTEGRATION.md), [TIMING.md](TIMING.md),
[PROJECT_STATE.md](PROJECT_STATE.md) and [NOTICE.md](NOTICE.md). The current
handoff distinguishes numerical checks from listening and hardware validation.
