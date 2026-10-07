# spuwire integration checkpoint

Status: backend implemented; the requested game-audio pass is **not achieved**.
Only the owned host module, dependency and CLI registration are changed. The
rest of host/ and port/ remains core3's responsibility. The decomp is unchanged.

`host/src/spu_cpal.rs` contains `SpuCpal: SpuBackend`, `AudioMode`,
`configure`, and `open_configured`. `--audio on|off|wav:PATH` defaults to on.
The current native host still constructs SilentSpu, so on/WAV runs report an
explicit integration error at completion; off retains the silent run path.
It cannot yet produce game sound or a game WAV. Do not report CLI recording as
working until the factory and clock are wired. WAV paths must be new files
under private/work/spuwire; the parent directory must already exist.

Core3 must provide these seams:

1. Construct `open_configured()?` on the sole C/game worker in both runtime paths.
2. Expose `advance_to(absolute_sample: u64) -> Result<(), String>` and `finish`
   through SpuBackend. Interleave timer events with sample advances, then advance
   to `completed_vblanks * 735` at each VBlank; check every result.
3. Call the original `smf_timer`, preserving TIMING.md's fractional clock and
   interrupt ordering. Do not call the standalone Sequence as well. Invoke C
   timer handlers outside the HOST RefCell borrow: their SDK calls re-enter
   Rust and need a fresh backend borrow. Advance, release the borrow, call the
   C handler, then advance again to the next boundary.
4. Preserve immediate `key_status(mask)` including invalid mask -1. ENVX alone
   misclassifies pending KON before the first sample. Convert SDK attributes to
   register operations; retain game libsd, bank loading and note-to-pitch.
5. Retain CD/common volume/control writes in the original sound/movie services.
   `cd_input` never enables CD mixing itself. Send decoded mono/stereo PCM at
   18.9/37.8/44.1 kHz; call cd_stop on skip/end/seek/reset. Queue limit: two seconds.
6. Run `port/prepare_libsd.py --decomp PATH --out GENERATED_DIR` before native
   transformations of smf_io.c/smf_snd.c. It fixes two cast expressions and three
   Boolean polling arguments, with strict pinned occurrence checks and PORT notices.
7. Regenerate the root Cargo.lock and remove psxgpu's nested workspace declaration.

Mixer commands are synchronous on the game worker; cpal consumes a lock-free
PCM ring. CD stop clears queued input and filter history without resetting voices.
Already mixed/device-submitted PCM has physical output latency and cannot be
recalled by cd_stop; live skip latency is not measured. SDK reset preserves bank
RAM while initializing registers/dummy/reverb state and retaining the host clock.
Device startup prefill is 1470 frames; finish drains the tail with device-only
silence. WAVs contain exactly rendered game frames, with no device padding.

Verified in a private relocated workspace after removing nested workspace
tables in that copy: fmt; workspace/all-targets Clippy -D warnings; 334 tests +
3 doctests; C build /W4 /WX. Two Python source-fix tests and the generated C
expression probe pass (/std:c11 /W4 /WX). Latest backend changes are checked
again at handoff. Root gates are still blocked by psxgpu's workspace metadata.

Private diagnostics: full Movie+SpuCpal capture `intro-pipeline-02.wav` (not the C
game loop), 2060 video frames, 6,053,460 output frames, peak 29,726, zero output
rails. First CD input is at sample zero. Inserted gaps before later XA packets
are 588 samples = 13.33 ms, less than a 15 fps frame (66.67 ms); remaining gap
count is trailing silence after source audio ends. This is a virtual pipeline
clock bound, not measured speaker/video latency. Quiet 3-second WASAPI smoke:
132,300 frames, zero underruns/device errors. Game title/menu WAVs and replay
sync remain unverified until the above core3 changes. No game bytes are committed.
