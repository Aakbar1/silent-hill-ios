// SPDX-License-Identifier: GPL-3.0-only
//! Register-level SPU on the game worker; cpal only consumes finished PCM.
//!
//! Core must call `advance_to` at sample boundaries, interleaving the original
//! libsd timer handler. One virtual 60 Hz tick is exactly 735 mixer frames.
//! Neither the device callback nor this backend runs a second sequencer.
use crate::backend::SpuBackend;
use psxspu::{Frame, SAMPLE_RATE, Spu, output::Output, xa::XaResampler};
use std::{
    collections::VecDeque,
    fs::{File, OpenOptions},
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

const BLOCK: usize = 512;
const CD_CAPACITY: usize = SAMPLE_RATE as usize * 2;
const DEVICE_CAPACITY: usize = 4096;
const PREFILL: usize = 1470;
static MODE: OnceLock<AudioMode> = OnceLock::new();
static OPENED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum AudioMode {
    #[default]
    On,
    Off,
    Wav(PathBuf),
}

impl AudioMode {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "on" => Ok(Self::On),
            "off" => Ok(Self::Off),
            value => match value.strip_prefix("wav:").filter(|v| !v.is_empty()) {
                Some(path) => Ok(Self::Wav(path.into())),
                None => Err("--audio requires on, off, or wav:PATH".into()),
            },
        }
    }
}

/// Configure before starting the native worker; open the device on that worker.
pub fn configure(mode: AudioMode) -> Result<(), String> {
    MODE.set(mode)
        .map_err(|_| "audio already configured".into())
}

/// This replaces SilentSpu construction in both windowed and headless hosts.
pub fn open_configured() -> Result<SpuCpal, String> {
    let backend = SpuCpal::open(MODE.get().cloned().unwrap_or_default())?;
    OPENED.store(true, Ordering::Relaxed);
    Ok(backend)
}

/// A selected device/WAV mode must not silently succeed through SilentSpu.
pub fn require_backend() -> Result<(), String> {
    if MODE.get() != Some(&AudioMode::Off) && !OPENED.load(Ordering::Relaxed) {
        Err("audio requested but the native host did not install SpuCpal; core3 factory/clock integration is required".into())
    } else {
        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
pub struct Statistics {
    pub rendered_frames: u64,
    pub cd_frames_received: u64,
    pub cd_frames_played: u64,
    pub cd_frames_cancelled: u64,
    pub cd_gap_frames: u64,
    pub cd_first_input_clock: Option<u64>,
    pub cd_gaps_before_last_input: u64,
    pub nonzero_samples: u64,
    pub rail_samples: u64,
    pub peak: u32,
    pub device_underrun_frames: u64,
    pub device_errors: u64,
}

enum Sink {
    Discard,
    Wav(Wave<File>),
    Device { output: Output, playing: bool },
}

pub struct SpuCpal {
    spu: Spu,
    cd: VecDeque<Frame>,
    resampler: XaResampler,
    cd_format: Option<(u32, u8)>,
    sink: Sink,
    statistics: Statistics,
    finished: bool,
    failure: Option<String>,
}

impl SpuCpal {
    pub fn open(mode: AudioMode) -> Result<Self, String> {
        #[cfg(target_os = "ios")]
        if mode == AudioMode::On {
            unsafe extern "C" {
                fn audio_ios_session_start() -> i32;
            }
            // SAFETY: C configures AVAudioSession synchronously on this worker.
            if unsafe { audio_ios_session_start() } != 0 {
                return Err("AVAudioSession playback activation failed".into());
            }
        }
        let sink = match mode {
            AudioMode::Off => Sink::Discard,
            AudioMode::On => Sink::Device {
                output: Output::open(DEVICE_CAPACITY, 256)?,
                playing: false,
            },
            AudioMode::Wav(path) => {
                check_recording_path(&path)?;
                let file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)
                    .map_err(|e| format!("audio {}: {e}", path.display()))?;
                Sink::Wav(Wave::new(file).map_err(|e| e.to_string())?)
            }
        };
        Ok(Self {
            spu: Spu::new(),
            cd: VecDeque::with_capacity(CD_CAPACITY),
            resampler: XaResampler::default(),
            cd_format: None,
            sink,
            statistics: Statistics::default(),
            finished: false,
            failure: None,
        })
    }

    pub fn statistics(&self) -> &Statistics {
        &self.statistics
    }

    /// ENVX alone cannot identify a just-keyed-on voice before rendering.
    /// Preserve immediate SDK status and -1 for an invalid voice mask.
    pub fn key_status(&self, mask: u32) -> i32 {
        self.spu.key_status(mask) as i32
    }

    /// Absolute game sample time, retained across SPU resets. Register writes
    /// between advances take effect on the very next frame. Never use wall time.
    pub fn advance_to(&mut self, sample: u64) -> Result<(), String> {
        self.check_running()?;
        if sample < self.statistics.rendered_frames {
            return Err("SPU clock cannot move backwards".into());
        }
        let result = self.render_until(sample);
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }

    fn render_until(&mut self, sample: u64) -> Result<(), String> {
        let mut frames = [[0; 2]; BLOCK];
        while self.statistics.rendered_frames < sample {
            let count = (sample - self.statistics.rendered_frames).min(BLOCK as u64) as usize;
            for frame in &mut frames[..count] {
                let cd = match self.cd.pop_front() {
                    Some(frame) => {
                        self.statistics.cd_frames_played += 1;
                        frame
                    }
                    None => {
                        if self.cd_format.is_some() {
                            self.statistics.cd_gap_frames += 1;
                        }
                        [0; 2]
                    }
                };
                *frame = self.spu.next_frame(cd, [0; 2]);
                self.statistics.rendered_frames += 1;
                for value in *frame {
                    self.statistics.nonzero_samples += u64::from(value != 0);
                    self.statistics.rail_samples += u64::from(matches!(value, i16::MIN | i16::MAX));
                    self.statistics.peak =
                        self.statistics.peak.max(u32::from(value.unsigned_abs()));
                }
            }
            match &mut self.sink {
                Sink::Discard => {}
                Sink::Wav(wave) => wave.write(&frames[..count]).map_err(|e| e.to_string())?,
                Sink::Device { output, playing } => {
                    for &frame in &frames[..count] {
                        // PORT: bounded backpressure paces an unthrottled host;
                        // no game state, synthesis, allocation or locks in cpal.
                        let deadline = Instant::now() + Duration::from_secs(2);
                        while output.producer.slots() == 0 {
                            check_device(output)?;
                            if Instant::now() >= deadline {
                                return Err("audio device stopped consuming PCM".into());
                            }
                            std::thread::sleep(Duration::from_millis(1));
                        }
                        output.producer.push(frame).map_err(|e| e.to_string())?;
                        if !*playing && DEVICE_CAPACITY - output.producer.slots() >= PREFILL {
                            output.play()?;
                            *playing = true;
                        }
                    }
                    let underruns = output.statistics.underrun_frames.load(Ordering::Relaxed);
                    if underruns != self.statistics.device_underrun_frames
                        && std::env::var_os("SH_AUDIO_TRACE").is_some()
                    {
                        println!(
                            "AUDIO_DEVICE underruns={} clock={} max_callback_frames={}",
                            underruns,
                            self.statistics.rendered_frames,
                            output
                                .statistics
                                .max_callback_frames
                                .load(Ordering::Relaxed)
                        );
                    }
                    self.statistics.device_underrun_frames = underruns;
                    check_device(output)?;
                }
            }
        }
        Ok(())
    }

    /// Finalize the RIFF sizes and surface recording/device errors before exit.
    /// Core must check this result, including when a replay ends early.
    pub fn finish(&mut self) -> Result<(), String> {
        if self.finished {
            return self.failure.clone().map_or(Ok(()), Err);
        }
        self.finished = true;
        let result = match &mut self.sink {
            Sink::Discard => Ok(()),
            Sink::Wav(wave) => wave.finish().map_err(|e| e.to_string()),
            Sink::Device { output, playing } => {
                if std::env::var_os("SH_AUDIO_TRACE").is_some() {
                    println!(
                        "AUDIO_DEVICE before_finish_underruns={} max_callback_frames={} rate={}",
                        output.statistics.underrun_frames.load(Ordering::Relaxed),
                        output
                            .statistics
                            .max_callback_frames
                            .load(Ordering::Relaxed),
                        output.sample_rate
                    );
                }
                let result = finish_device(output, *playing);
                self.statistics.device_underrun_frames =
                    output.statistics.underrun_frames.load(Ordering::Relaxed);
                self.statistics.device_errors =
                    output.statistics.device_errors.load(Ordering::Relaxed);
                result
            }
        };
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        if std::env::var_os("SH_AUDIO_TRACE").is_some() {
            println!(
                "AUDIO_PCM frames={} peak={} nonzero={} rails={} underruns={} device_errors={}",
                self.statistics.rendered_frames,
                self.statistics.peak,
                self.statistics.nonzero_samples,
                self.statistics.rail_samples,
                self.statistics.device_underrun_frames,
                self.statistics.device_errors
            );
        }
        self.failure.clone().map_or(result, Err)
    }

    fn check_running(&self) -> Result<(), String> {
        if let Some(error) = &self.failure {
            Err(error.clone())
        } else if self.finished {
            Err("audio backend already finished".into())
        } else {
            Ok(())
        }
    }
}

impl Drop for SpuCpal {
    fn drop(&mut self) {
        if !self.finished
            && let Err(error) = self.finish()
        {
            eprintln!("audio finalization failed: {error}");
        }
    }
}

fn check_device(output: &Output) -> Result<(), String> {
    let errors = output.statistics.device_errors.load(Ordering::Relaxed);
    if errors != 0 {
        Err(format!("audio device reported {errors} errors"))
    } else {
        Ok(())
    }
}

fn finish_device(output: &mut Output, playing: bool) -> Result<(), String> {
    if output.producer.slots() == DEVICE_CAPACITY {
        return Ok(());
    }
    // PORT: pad only the device tail, never the game clock/WAV. Preserve the
    // final queued game samples before pausing; a bounded timeout protects exit.
    // PORT: An unthrottled native replay can finish with a completely full
    // ring. Wait for space to append the whole silence tail; zero available
    // slots must not turn shutdown padding into zero and force an underrun.
    let padding = PREFILL;
    let deadline = Instant::now() + Duration::from_secs(2);
    for _ in 0..padding {
        while output.producer.slots() == 0 {
            check_device(output)?;
            if Instant::now() >= deadline {
                output.pause()?;
                return Err("audio device did not consume shutdown padding".into());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        output.producer.push([0; 2]).map_err(|e| e.to_string())?;
    }
    if !playing {
        output.play()?;
    }
    while DEVICE_CAPACITY - output.producer.slots() > padding / 2 {
        if let Err(error) = check_device(output) {
            let _ = output.pause();
            return Err(error);
        }
        if Instant::now() >= deadline {
            output.pause()?;
            return Err("audio device did not drain before shutdown".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    output.pause()?;
    check_device(output)
}

impl SpuBackend for SpuCpal {
    fn advance_to(&mut self, sample: u64) -> Result<(), String> {
        SpuCpal::advance_to(self, sample)
    }
    fn finish(&mut self) -> Result<(), String> {
        SpuCpal::finish(self)
    }
    fn reset(&mut self) {
        self.cd_stop();
        self.spu.reset();
        // PORT: SpuInit initializes SDK registers/dummy RAM; an absolute replay
        // clock and its WAV timeline must not rewind on a device reset.
        self.spu.init();
    }

    fn write_register(&mut self, offset: u16, value: u16) -> Result<(), String> {
        self.check_running()?;
        self.spu
            .write_register(u32::from(offset), value)
            .map_err(|e| e.to_string())
    }

    fn read_register(&self, offset: u16) -> Result<u16, String> {
        self.check_running()?;
        self.spu
            .read_register(u32::from(offset))
            .map_err(|e| e.to_string())
    }

    fn transfer_write(&mut self, address: u32, bytes: &[u8]) -> Result<(), String> {
        self.check_running()?;
        self.spu
            .upload(address as usize, bytes)
            .map_err(|e| e.to_string())
    }

    fn transfer_read(&self, address: u32, bytes: &mut [u8]) -> Result<(), String> {
        self.check_running()?;
        let start = address as usize;
        let end = start
            .checked_add(bytes.len())
            .ok_or("SPU transfer overflow")?;
        bytes.copy_from_slice(
            self.spu
                .ram()
                .get(start..end)
                .ok_or("SPU transfer outside RAM")?,
        );
        Ok(())
    }

    fn cd_input(&mut self, pcm: &[i16], rate: u32, channels: u8) -> Result<bool, String> {
        self.check_running()?;
        if !matches!(channels, 1 | 2)
            || !matches!(rate, 18_900 | 37_800 | SAMPLE_RATE)
            || !pcm.len().is_multiple_of(usize::from(channels))
        {
            return Err("invalid CD PCM format or partial frame".into());
        }
        if self
            .cd_format
            .is_some_and(|format| format != (rate, channels))
        {
            return Err("CD PCM format changed without cd_stop".into());
        }
        // Validate bounds before allocating or mutating filter state. Include
        // the retained six-sample phase so small fragmented input is bounded too.
        let inputs = pcm.len() / usize::from(channels);
        let bound = if rate == SAMPLE_RATE {
            inputs
        } else {
            inputs
                .checked_mul(if rate == 18_900 { 2 } else { 1 })
                .and_then(|n| n.checked_add(5))
                .and_then(|n| (n / 6).checked_mul(7))
                .ok_or("CD PCM length overflow")?
        };
        if bound > CD_CAPACITY - self.cd.len() {
            return Err("CD PCM queue exceeds two seconds".into());
        }
        let mut frames = Vec::with_capacity(bound);
        if rate == SAMPLE_RATE {
            frames.extend(
                pcm.chunks_exact(usize::from(channels))
                    .map(|s| [s[0], *s.get(1).unwrap_or(&s[0])]),
            );
        } else {
            self.resampler
                .process(
                    pcm,
                    psxmedia::AudioFormat {
                        sample_rate: rate,
                        channels,
                        adpcm_bits_per_sample: 4,
                    },
                    &mut frames,
                )
                .map_err(|e| e.to_string())?;
        }
        if self.cd_format.is_none() {
            self.statistics.cd_first_input_clock = Some(self.statistics.rendered_frames);
        }
        self.statistics.cd_gaps_before_last_input = self.statistics.cd_gap_frames;
        self.cd_format = Some((rate, channels));
        self.statistics.cd_frames_received += frames.len() as u64;
        self.cd.extend(frames);
        Ok(true)
    }

    fn cd_stop(&mut self) {
        self.statistics.cd_frames_cancelled += self.cd.len() as u64;
        self.cd.clear();
        self.cd_format = None;
        self.resampler = XaResampler::default();
    }
}

fn check_recording_path(path: &Path) -> Result<(), String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../private/work")
        .canonicalize()
        .map_err(|e| format!("private audio directory: {e}"))?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .canonicalize()
        .map_err(|e| format!("audio parent: {e}"))?;
    if !parent.starts_with(&root) {
        return Err("WAV output must be inside private/work/".into());
    }
    if path.file_name().is_none() || path.exists() {
        return Err("WAV output must be a new file".into());
    }
    Ok(())
}

struct Wave<W: Write + Seek> {
    writer: W,
    frames: u64,
}

impl<W: Write + Seek> Wave<W> {
    fn new(writer: W) -> std::io::Result<Self> {
        let mut wave = Self { writer, frames: 0 };
        wave.header()?;
        Ok(wave)
    }

    fn header(&mut self) -> std::io::Result<()> {
        let bytes = u32::try_from(self.frames * 4)
            .ok()
            .filter(|n| *n <= u32::MAX - 36)
            .ok_or_else(|| std::io::Error::other("recording exceeds RIFF size limit"))?;
        self.writer.seek(SeekFrom::Start(0))?;
        self.writer.write_all(b"RIFF")?;
        self.writer.write_all(&(bytes + 36).to_le_bytes())?;
        self.writer.write_all(b"WAVEfmt \x10\0\0\0\x01\0\x02\0")?;
        self.writer.write_all(&SAMPLE_RATE.to_le_bytes())?;
        self.writer.write_all(&(SAMPLE_RATE * 4).to_le_bytes())?;
        self.writer.write_all(b"\x04\0\x10\0data")?;
        self.writer.write_all(&bytes.to_le_bytes())?;
        self.writer.seek(SeekFrom::End(0))?;
        Ok(())
    }

    fn write(&mut self, frames: &[Frame]) -> std::io::Result<()> {
        if self.frames + frames.len() as u64 > u64::from((u32::MAX - 36) / 4) {
            return Err(std::io::Error::other("recording exceeds RIFF size limit"));
        }
        let mut bytes = [0; BLOCK * 4];
        for block in frames.chunks(BLOCK) {
            for (frame, output) in block.iter().zip(bytes.as_chunks_mut::<4>().0) {
                output[..2].copy_from_slice(&frame[0].to_le_bytes());
                output[2..].copy_from_slice(&frame[1].to_le_bytes());
            }
            self.writer.write_all(&bytes[..block.len() * 4])?;
            self.frames += block.len() as u64;
        }
        Ok(())
    }

    fn finish(&mut self) -> std::io::Result<()> {
        self.header()?;
        self.writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::VoiceRegisters;
    use psxspu::RAM_BYTES;
    use std::io::Cursor;

    fn cd_enabled(spu: &mut SpuCpal) {
        spu.reset();
        spu.write_register(0x180, 0x3fff).unwrap();
        spu.write_register(0x182, 0x3fff).unwrap();
        spu.write_register(0x1b0, 0x7fff).unwrap();
        spu.write_register(0x1b2, 0x7fff).unwrap();
        spu.write_register(0x1aa, 0xc001).unwrap();
    }

    #[test]
    fn register_backend_units_bounds_key_status_and_reset() {
        let mut s = SpuCpal::open(AudioMode::Off).unwrap();
        s.reset();
        s.voice(
            23,
            VoiceRegisters {
                pitch: 0x1000,
                start_address: 0x201,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(s.read_register(23 * 16 + 6).unwrap(), 0x201);
        assert!(s.write_register(1, 0).is_err());
        assert!(s.read_register(640).is_err());
        assert!(s.voice(24, VoiceRegisters::default()).is_err());
        s.write_register(0x18a, 0x80).unwrap();
        assert_eq!(s.key_status(1 << 23), 1);
        assert_eq!(s.key_status(0), -1);
        s.write_register(0x18e, 0x80).unwrap();
        assert_eq!(s.key_status(1 << 23), 0);
        s.transfer_write((RAM_BYTES - 2) as u32, &[1, 2]).unwrap();
        assert!(s.transfer_write((RAM_BYTES - 1) as u32, &[1, 2]).is_err());
        let mut bytes = [0; 2];
        s.transfer_read((RAM_BYTES - 2) as u32, &mut bytes).unwrap();
        assert_eq!(bytes, [1, 2]);
        s.transfer_write(0x2000, &[3, 4]).unwrap();
        assert!(s.transfer_read(u32::MAX, &mut bytes).is_err());
        s.advance_to(735).unwrap();
        s.reset();
        assert_eq!(s.statistics.rendered_frames, 735);
        s.transfer_read(0x2000, &mut bytes).unwrap();
        assert_eq!(bytes, [3, 4]); // SDK init clears reverb/dummy RAM, retaining banks.
        assert!(s.advance_to(734).is_err());
    }

    #[test]
    fn cd_cancel_preserves_voices_and_resets_xa_filter_history() {
        let mut s = SpuCpal::open(AudioMode::Off).unwrap();
        cd_enabled(&mut s);
        s.write_register(0x188, 8).unwrap();
        let pcm = vec![1200; 2016 * 2];
        s.cd_input(&pcm, 37_800, 2).unwrap();
        let fresh = s.cd.clone();
        s.advance_to(100).unwrap();
        s.cd_stop();
        assert!(s.cd.is_empty());
        assert!(s.spu.voice(3).unwrap().keyed);
        s.cd_input(&pcm, 37_800, 2).unwrap();
        assert_eq!(s.cd, fresh);
        assert_eq!(s.statistics.cd_frames_received, 4704);
        assert_eq!(s.statistics.cd_frames_cancelled, 2252);
    }

    #[test]
    fn fragmented_xa_has_exact_sample_clock_without_accumulated_drift() {
        for rate in [18_900, 37_800] {
            let mut s = SpuCpal::open(AudioMode::Off).unwrap();
            cd_enabled(&mut s);
            let mut received = 0;
            for tick in 1..=600 {
                // Exactly ten seconds of decoded source, delivered in unusual
                // chunks, proves the zigzag phase survives packet boundaries.
                let total = rate as u64 * tick / 60;
                let pcm = vec![1000; (total - received) as usize];
                for chunk in pcm.chunks(13) {
                    s.cd_input(chunk, rate, 1).unwrap();
                }
                received = total;
                s.advance_to(tick * 735).unwrap();
            }
            assert_eq!(s.statistics.cd_frames_received, 441_000);
            assert_eq!(s.statistics.cd_frames_played, 441_000);
            assert_eq!(s.statistics.cd_gap_frames, 0);
            assert!(s.statistics.nonzero_samples > 800_000);
            assert_eq!(s.statistics.rail_samples, 0);
        }
    }

    #[test]
    fn invalid_cd_input_does_not_mutate_queue_or_filter() {
        let mut s = SpuCpal::open(AudioMode::Off).unwrap();
        assert!(s.cd_input(&[1], 37_800, 0).is_err());
        assert!(s.cd_input(&[1], 37_800, 2).is_err());
        assert!(s.cd_input(&[1], 48_000, 1).is_err());
        s.cd_input(&[100; 12], 37_800, 2).unwrap();
        let before = s.cd.clone();
        assert!(s.cd_input(&[100; 12], 18_900, 2).is_err());
        assert!(s.cd_input(&vec![0; CD_CAPACITY * 2], 37_800, 2).is_err());
        assert_eq!(s.cd, before);
    }

    #[test]
    fn wav_contains_mixed_pcm_and_finalized_sizes() {
        let mut wave = Wave::new(Cursor::new(Vec::new())).unwrap();
        wave.write(&[[1, -2], [i16::MIN, i16::MAX]]).unwrap();
        wave.finish().unwrap();
        let bytes = wave.writer.into_inner();
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 44);
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 8);
        assert_eq!(&bytes[44..], &[1, 0, 254, 255, 0, 128, 255, 127]);
    }

    #[test]
    fn audio_modes_and_finished_backend_reject_invalid_calls() {
        assert_eq!(AudioMode::parse("on").unwrap(), AudioMode::On);
        assert_eq!(AudioMode::parse("off").unwrap(), AudioMode::Off);
        assert_eq!(
            AudioMode::parse("wav:C:/private/test.wav").unwrap(),
            AudioMode::Wav("C:/private/test.wav".into())
        );
        assert!(AudioMode::parse("wav:").is_err());
        assert!(AudioMode::parse("auto").is_err());
        let mut s = SpuCpal::open(AudioMode::Off).unwrap();
        s.advance_to(4410).unwrap();
        s.finish().unwrap();
        s.finish().unwrap();
        assert!(s.advance_to(4411).is_err());
        assert!(s.cd_input(&[], 44_100, 2).is_err());
        assert!(s.write_register(0x188, 1).is_err());
    }

    #[test]
    #[ignore = "opens the default speaker device for a quiet synthetic 3-second smoke test"]
    fn live_device_output_smoke() {
        let mut s = SpuCpal::open(AudioMode::On).unwrap();
        cd_enabled(&mut s);
        let start = Instant::now();
        for tick in 0..180u64 {
            let pcm: Vec<i16> = (tick * 735..(tick + 1) * 735)
                .map(|frame| {
                    ((frame as f64 * 440.0 * std::f64::consts::TAU / f64::from(SAMPLE_RATE)).sin()
                        * 1000.0) as i16
                })
                .collect();
            s.cd_input(&pcm, SAMPLE_RATE, 1).unwrap();
            s.advance_to((tick + 1) * 735).unwrap();
            if let Some(wait) = (start + Duration::from_secs_f64((tick + 1) as f64 / 60.0))
                .checked_duration_since(Instant::now())
            {
                std::thread::sleep(wait);
            }
        }
        s.finish().unwrap();
        println!("LIVE_OUTPUT statistics={:?}", s.statistics);
        assert_eq!(s.statistics.device_errors, 0);
        assert_eq!(s.statistics.device_underrun_frames, 0);
    }

    #[test]
    #[ignore = "requires player-owned disc and private artifact directory"]
    fn private_intro_pipeline_recording() {
        use crate::{disc::GameDisc, movie::Movie, raster::Raster};
        let disc_path = std::env::var_os("SH_SPUWIRE_DISC").expect("SH_SPUWIRE_DISC");
        let output =
            PathBuf::from(std::env::var_os("SH_SPUWIRE_OUTPUT").expect("SH_SPUWIRE_OUTPUT"));
        // This private diagnostic directly drives Movie, not the C game loop.
        // In the validation copy CARGO_MANIFEST_DIR is relocated; use the
        // player-disc's project root to enforce the same private output rule.
        let root = Path::new(&disc_path)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("work/spuwire")
            .canonicalize()
            .unwrap();
        assert!(
            output
                .parent()
                .unwrap()
                .canonicalize()
                .unwrap()
                .starts_with(root)
        );
        let mut disc = GameDisc::open(disc_path).unwrap();
        let mut movie = Movie::new(&disc, 2053, 0, 0).unwrap();
        let mut gpu = Raster::default();
        let mut s = SpuCpal::open(AudioMode::Off).unwrap();
        cd_enabled(&mut s);
        s.sink = Sink::Wav(
            Wave::new(
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&output)
                    .unwrap(),
            )
            .unwrap(),
        );
        let mut tick = 0;
        loop {
            let done = movie.tick(&mut disc, &mut gpu, &mut s, tick).unwrap();
            if done {
                break;
            }
            s.advance_to((tick + 1) * 735).unwrap();
            tick += 1;
            assert!(tick < 10_000);
        }
        let video_frames = movie.decoded_frames;
        let source_frames = movie.audio_samples;
        let offset =
            s.statistics.cd_first_input_clock.unwrap_or(0) + s.statistics.cd_gaps_before_last_input;
        movie.end(&mut s, false);
        s.finish().unwrap();
        println!(
            "INTRO_PIPELINE ticks={tick} video_frames={video_frames} source_frames={source_frames} statistics={:?}",
            s.statistics
        );
        assert_eq!(video_frames, 2060);
        assert!(s.statistics.nonzero_samples > 1_000_000);
        assert_eq!(s.statistics.rail_samples, 0);
        std::fs::write(output.with_extension("txt"), format!(
            "diagnostic=Movie+SpuCpal pipeline (not C game replay)\nticks={tick}\nvideo_frames={video_frames}\nsource_frames={source_frames}\nmaximum_cd_clock_offset_frames={offset}\nstatistics={:?}\n", s.statistics)).unwrap();
        // Only gaps before subsequent packets shift their playback. Silence
        // after the last XA sector belongs to source/video duration mismatch.
        assert!(
            offset < u64::from(SAMPLE_RATE / 15),
            "CD drift by {offset} frames"
        );
    }
}
