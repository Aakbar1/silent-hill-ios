// SPDX-License-Identifier: GPL-3.0-only
// Included inside spu_cpal: only the physical sink is rebuilt, never voices,
// sequence time, sample RAM, CD queue or resampler state.
impl SpuCpal {
    pub(crate) fn ios_callbacks(&self) -> u64 {
        match &self.sink {
            Sink::Device { output, .. } => output.statistics.callbacks.load(Ordering::Relaxed),
            _ => 0,
        }
    }
    pub(crate) fn ios_suspend(&mut self) -> Result<(), String> {
        if let Sink::Device { output, .. } = &self.sink {
            if let Err(error) = output.pause() {
                // A reset/interrupted AudioUnit may already be invalid. Drop it
                // regardless; recovery opens a new physical device stream.
                eprintln!("IOS_AUDIO old stream pause failed; discarding it: {error}");
            }
        }
        // PORT: Discard only queued physical-device PCM on suspension. Keeping
        // it would replay stale sound after a long interruption/route change.
        self.sink = Sink::Discard;
        Ok(())
    }

    pub(crate) fn ios_recover(&mut self) -> Result<(), String> {
        unsafe extern "C" {
            fn audio_ios_session_start() -> i32;
        }
        // SAFETY: Called on the sole audio/game worker, just as initial open.
        if unsafe { audio_ios_session_start() } != 0 {
            return Err("AVAudioSession reactivation failed".into());
        }
        self.sink = Sink::Device {
            output: Output::open(DEVICE_CAPACITY, 256)?,
            playing: false,
        };
        // PORT: A notified OS interruption may have timed out the old device
        // producer. It is safe to retry from the retained absolute mixer clock.
        self.failure = None;
        Ok(())
    }
}
