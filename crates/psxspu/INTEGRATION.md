# Core integration

## Ownership and types

Core adds `crates/psxspu` to the parent workspace and removes this crate's empty
`[workspace]` stanza. Add a path dependency in host. Implement the host-owned
`SpuBackend` trait with a wrapper containing `psxspu::Spu`; this lane deliberately
does not own host's trait or exported C symbols. No host integration was compiled
in this worktree: its checkout does not yet contain that trait.

`Spu::new()` allocates sound RAM once. `init()` initializes the SDK state.
`reset()` resets device state while retaining sound RAM; `quit()` disables it.
RAM addresses are **u32 byte offsets**, registers are **u16**. Voice masks are
u32 with bits 0..23. Native C pointers only identify the host buffer to copy.
Convert C fields explicitly: do not transmute PsyQ structs, C `long`, disc32
pointers, or VAB file bytes into Rust structures. Initialize a Rust attribute
record and populate only fields selected by the C mask; other fields in the
game's stack-allocated C attributes can be uninitialized. For GetVoiceAttr, select the
lowest set bit of the incoming C voice mask and copy ENVX/current volumes from
`voice(index)` in addition to `get_voice_attr(index)`.

## Used PsyQ services

| Game SDK call | Native operation |
|---|---|
| SpuInit / SpuQuit | init / quit |
| SpuInitMalloc | Allocator::new(record_count), or retain game SdSpu allocator |
| SpuSetVoiceAttr | set_voice_attr(&sdk::VoiceAttr), same numeric mask bits |
| SpuGetVoiceAttr | get_voice_attr(index) plus voice(index) for current volumes/ENVX |
| SpuSetKeyOnWithAttr | key_on_with_attr |
| SpuSetKey | key_on(mask) / key_off(mask) |
| SpuGetKeyStatus | key_status(mask), enum values 0/1/2/3; invalid mask -1 |
| SpuSetCommonAttr | set_common_attr(&sdk::CommonAttr), same mask bits |
| SpuSetReverbModeParam | set_reverb_attr(&sdk::ReverbAttr) |
| SpuSetReverb | set_reverb(bool) |
| SpuSetReverbVoice / SpuGetReverbVoice | set_reverb_voice / reverb_voice |
| SpuReserveReverbWorkArea | Allocator::reserve_reverb(preset, bool) before uploads |
| SpuClearReverbWorkArea | clear_reverb_work_area(preset) |
| SpuSetTransferMode | set_transfer_mode(manual: mode == 1) |
| SpuSetTransferStartAddr | set_transfer_address(byte_offset), returns rounded address |
| SpuWrite | dma_write(host_bytes), return byte count after the copy |
| SpuIsTransferCompleted | transfer_completed(), true for synchronous native copy |
| SsSetSerialAttr / SsSetSerialVol | retain the game's wrappers, which set common CD/ext attributes |

SDK errors map to SPU_INVALID_ARGS / SPU_ERROR as appropriate; never return a
successful no-op for an unsupported request. Transfer addresses round up to
eight bytes; register addresses are sound-RAM addresses divided by eight.
`upload` is bounded, while hardware-style DMA/manual transfers wrap at 512 KiB.
The `VoiceAttr` volume fields use PsyQ's signed 15-bit direct setting, whereas
CD/ext and reverb depth fields are signed 16-bit Q15. `VoiceState.volume`
contains the doubled current direct volume or live sweep level.

The survey's used masks are implemented. NOTE/SAMPLE_NOTE attributes and PsyQ's
convenience reverb delay/feedback masks return Unsupported; neither is used by
this game's inspected calls. Keep the game Note2Pitch and use PITCH. Arbitrary
delay/feedback coefficients remain programmable through reverb registers.
The allocator reserves the capture area, the dummy block at 0x1000, and the
selected reverb region; Off reservation consumes the SDK's 128 bytes.

## Mixing and device output

`next_frame(cd: [i16;2], external: [i16;2]) -> [i16;2]` advances exactly one
44.1 kHz frame; `render(&mut [Frame])` renders a block without inputs.
`render_cd(input, output)` mixes a matching block of resampled CD frames.
`sample_clock()` counts produced frames. Apply timestamped register/SDK commands
at sample boundaries on the single owning mixer thread. Do not race game libsd
globals from a cpal callback. Key on/off status must be observable synchronously
to the game's polling loops.

Keep original bank loading, voice selection, MIDI controllers, fades and NRPN
modulation in game-owned libsd. Its timer handler must run using [TIMING.md](TIMING.md).
Use either that original handler or the standalone `Sequence`, never both.
`Sequence::tick` emits events without allocation; `HousekeepingClock` models
the original first-12/recurring-11 callback cadence for standalone use.

`Output::open(capacity, requested_buffer_frames)` returns a producer and paused
cpal stream. Push rendered frames into `producer`, then call `play()`.
The callback only pops/linearly converts PCM with no allocation or locks.
If the device supports 44.1 kHz it consumes PCM directly; otherwise conversion
keeps the mixer clock at 44.1 kHz. Extra device channels are zero; mono devices
receive the stereo average. Monitor statistics for underruns and device errors.

On this Windows shared-mode device, a 256-frame request produced an initial
1056-frame callback and steady callbacks near 480 frames at 48 kHz. Prefill at
least 1024 native frames; maintaining 512 native frames of lead passed the live
smoke test. A 512-frame startup prefill caused 460 missing frames, so buffer
request size alone must not determine startup fill. There is no measured
end-to-end latency guarantee. Tune lead from observed callback/scheduling data.

On iOS, core must activate/configure AVAudioSession (playback category, preferred
rate and I/O duration) before cpal, and handle interruptions/route changes.
cpal selects CoreAudio there. iOS compilation/device output needs the Mac lane;
this lane verified Windows WASAPI only.

## XA and imported banks

`Vab::parse(&imported_bytes)` validates headers, packed tone blocks, little-endian
16-bit sample sizes and body extents. `Vab::upload` checks capture/reverb overlap.
The fixed 128-program table may retain stale nonzero entries after the declared
packed program count; these have no tone blocks and are unavailable to playback.
Standalone audition uses ToneRequest and PreviewSynth; production retains libsd.

`XaStream::new(file, channel)` accepts raw sectors through `feed_sector`, using
psxmedia predictor histories and the PS1 seven-phase zigzag resampler. It emits
44.1 kHz frames off the callback. Matrix order is LL, LR, RL, RR; 0x80 is unity.
Pause stops sector consumption. `seek` resets predictor/filter state, clock,
format and EOF; set the disc seek position separately. Feed only the selected
item's sectors until its EOF or game playback limit, and schedule decoded frames
by sample count rather than read-ahead arrival. The CD lane owns commands,
seek/play/pause completion and disc scheduling; this is an audio decoder/mixer.

## Fidelity boundaries and requests

`// PORT:` marks synchronous transfer/key-status service timing, skipping idle
voice ADPCM/IRQ activity, physical device-rate conversion, negative audition
fine-index normalization, and the audition's external layer activation.
This is a native hardware-style mixer, not cycle-exact silicon. Active voice,
transfer and capture IRQs are latched; reverb/idle-voice bus IRQ timing and FIFO
contention are not modeled. IRQ status is polled via irq_pending/read_register.
Reverb follows the documented network, Q15 saturation and alternating/FIR
processing; exact undocumented silicon rounding requires a hardware recording.

Core should audit these existing decomp issues before wiring retained libsd:

- smf_snd.c SdUtKeyOnV and smf_io.c key_on declare u16 size-table pointers but
  initialize them with incompatible `(u8*)` expressions. The reads already use
  16-bit values; normalize the casts for native C compilation without changing
  the element stride. The initial byte-read concern was rejected after type review.
- Several loops call `SpuGetKeyStatus(spu_ch_tbl[vo] == 1)`, passing a Boolean
  instead of the voice mask. For voices above zero this queries invalid mask zero
  and bypasses the intended voice-status check. The correct comparison must be
  outside the call. The matching SDK s_gks.o returns -1 for invalid masks, which
  this backend preserves; do not alter it to fabricate a successful voice query.

No files outside this lane were patched. REPORT.md records these requests.
