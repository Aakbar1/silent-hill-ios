# Silent Hill USA 1.1 sound timing

Evidence is the read-only decomp at
[d9e28f8315c7938117224f21516786d9d149a145](https://github.com/Vatuu/silent-hill-decomp/tree/d9e28f8315c7938117224f21516786d9d149a145).

- [sd_call.c SD_Init](https://github.com/Vatuu/silent-hill-decomp/blob/d9e28f8315c7938117224f21516786d9d149a145/src/bodyprog/sound/sd_call.c#L315)
  selects tick mode 1 and calls SdStart.
- [smf_snd.c SdStart](https://github.com/Vatuu/silent-hill-decomp/blob/d9e28f8315c7938117224f21516786d9d149a145/src/bodyprog/libsd/smf_snd.c#L351)
  starts timer mode for tick modes 1..3.
- [smf_main.c smf_timer_set](https://github.com/Vatuu/silent-hill-decomp/blob/d9e28f8315c7938117224f21516786d9d149a145/src/bodyprog/libsd/smf_main.c#L84)
  opens the timer-2 interrupt event and calls SetRCnt(RCntCNT2, 7328, RCntMdINTR).
  The source's `~30Hz?` comment is not the clock contract.
- include/psyq/kernel.h defines RCntMdINTR=0x1000, RCntMdSC=1, RCntMdSP=0.
  Inspection of the matching local lib/libapi/counter.o confirms its timer-2
  special branch selects mode 0x248 without SC, then enables target IRQ to
  produce 0x258. This differs from applying BIOS init_timer's generic flag
  translation; SetRCnt is a direct SDK register wrapper.
- [Timer MODE specification](https://psx-spx.consoledev.net/ps1/system/timers/)
  assigns bit 9 to timer 2's system-clock /8 prescaler and bits 3/4/6 to
  target reset, target IRQ and repeating interrupts.

The nominal clock is therefore `33,868,800 / (8 * 7328) = 577.729257641 Hz`.
`SequenceClock` accumulates integer system cycles against `period * 44100`;
no float timer, host VSync or integer milliseconds participates. At 44.1 kHz,
there are 96 prescaled timer ticks per output frame. Thirty seconds advances
17,331 interrupts, with the fractional remainder preserved.

[smf_mid.c midi_smf_main](https://github.com/Vatuu/silent-hill-decomp/blob/d9e28f8315c7938117224f21516786d9d149a145/src/bodyprog/libsd/smf_mid.c#L866)
adds each track's mf_tempo2 to a u16 accumulator and advances a converted delta
only on overflow of its low eight bits. KDT starts at 104, with C7 assigning
`(value & 127) * 2 + 2`. Its 480 division converts deltas by /4, retaining the
two-bit remainder. Other supported divisions preserve the original integer
conversion and u16 wrap. MIDI/SEQ tempo uses the game's integer BPM conversion,
100/115 correction and division-specific adjustments; a generic MIDI wall clock
would change this timing. KDT chained events and loop snapshots also retain
their original interrupt ordering.

smf_timer calls midi_vsync/auto keyoff when sd_timer_sync>=11, resets the value
to zero, and then increments it. Consequently the first housekeeping call is
interrupt 12 and subsequent calls recur every 11 interrupts (23, 34, ...).
At the nominal rate the recurring housekeeping frequency is 52.5208416 Hz.
Core should schedule the original smf_timer handler, which retains its guard
and this ordering. The separate manual smf_vsync path performs ten sequencer
calls per virtual VSync; it is not the selected mode-1 path.

Verified comparison: a private native-C harness compiled the original
smf_mid.c parser/scheduler with event-output stubs. The Rust parser matched
all 21,296 note/program/controller/bend events and their interrupt numbers
across all 40 disc KDT files, 20,000 interrupts each. The C harness keeps native
pointers separate from disk bytes and makes signed/unsigned comparisons explicit;
the sequence algorithm is unchanged. It compiled with MSVC C17 /W3 /WX.
Private traces and comparison results are in private/work/spu/sequence-comparison.json;
`examples/trace.rs` regenerates the native trace from a private KDT input.

Unverified physical effects: timer reset-edge hold cycles, IRQ delivery latency
and interruptions suppressed by real critical sections have not been measured
on a PS1. The hardware specification documents reset hold cycles; its exact
interaction with the prescaler cannot be established from this decomp alone.
The default is the nominal software target period, not a claim of a measured
silicon interrupt waveform. SequenceClock::new accepts a CPU-cycle period so
core can supply its verified timer model without retuning music or losing phase.
