# SPU lane checkpoint — 2026-10-07

Goal: a standalone native backend inside crates/psxspu; core owns workspace,
SpuBackend/C bindings and the existing game's libsd. No other lane files changed.
All disc/extracted/rendered content remains private/work/spu. No system installs,
merge or push. GPL notices retained; explicit native deviations marked PORT.

Implemented: 24-voice/512 KiB register model, Gaussian/pitch/ADPCM loops, ADSR,
signed sweep volumes, noise/pitch modulation, PS1 reverb/presets/FIR, CD/ext
mix/capture, synchronous transfer services, SDK attribute adapters, allocation
records, checked VAB/KDT/SEQ/MIDI parsers, XA zigzag conversion and cpal transport.
INTEGRATION.md maps every surveyed libspu call; TIMING.md resolves mode 0x258,
nominal 577.729257642 Hz and the first-12/recurring-11 housekeeping cadence.

Verification:

- Rust stable 1.99 on Windows x64: 45 tests pass with output; 43 without output.
  fmt and clippy -D warnings, including all targets, pass.
- Read-only disc SHA-1 reconfirmed: 34278d31d9b9b12b3b5db5e45bcbe548991ecbc7.
  92 VAB banks parse; all 40 KDT tracks step for 30 seconds without error.
- Original-C comparison: 40/40 KDT files, 20,000 interrupts each; all 21,296
  events and interrupt timestamps match. Private C17 /W3 /WX build: no warnings.
- XA native decode vs independent FFmpeg 7.1: 455,616 samples, zero differences.
- WAV checks: stereo/44.1 kHz, non-silent and zero output-rail samples for
  12 s SFX, 30 s layered KDT audition, 6.026667 s XA voice item.
- Mixer and callback allocation instrumentation: zero allocations. Windows
  WASAPI: 48 kHz, 300 callbacks/3 s, 1056-frame startup burst, zero underruns/errors
  with 1024-frame startup prefill and 512-frame steady producer lead.
- i7-14700HX release single-thread benchmark: 2.065–2.105% of one core normally,
  2.695–2.724% with all voices at max pitch, noise/PMON/Room reverb. Wall-time
  measurement excludes I/O and is a conservative single-thread CPU bound.

Decisions/evidence: authored KDT layer volume zero is deliberate; the audition
uses external MIDI-volume control and sequence volume 40. Keep original libsd
for production controller/voice behavior. A smaller 512-frame startup prefill
failed with 460 missing frames; requested buffer size does not bound WASAPI's
startup callback. Native service timing is synchronous; idle bus/IRQ timing is
not cycle-exact. Precise silicon reverb rounding and timer reset-edge latency
remain unmeasured. SEQ/MIDI were synthetic-tested; this disc uses KDT1.

Open: human listening/PS1 audio-reference acceptance, iOS build/output and host
integration are not verified. Preview reports unsupported advanced controls;
it is not a substitute for libsd. PsyQ NOTE/SAMPLE_NOTE and convenience reverb
delay/feedback masks are outside inspected game usage and return Unsupported.

Director actions: wire host trait/C callbacks; retain libsd and layer/fade state;
normalize incompatible VAB size-pointer casts and fix Boolean SpuGetKeyStatus calls in
INTEGRATION.md; run Mac/iOS AVAudioSession/device checks and review private WAVs.
Type review rejected the initial byte-read hypothesis: the C pointers are u16*.
Private evidence: sequence-comparison.json, xa-comparison.json, wave-checks.json,
benchmark-results.json, reference-sequence.c/exe and event traces under
C:/Claude Projects/Silent Hill iOS/private/work/spu/.
