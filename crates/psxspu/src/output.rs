//! cpal device layer (WASAPI / CoreAudio). Mixer work belongs on the producer
//! thread. The callback only pops a lock-free bounded ring and converts PCM.
use crate::{Frame, SAMPLE_RATE};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

#[derive(Default)]
pub struct Statistics {
    pub underrun_frames: AtomicU64,
    pub callbacks: AtomicU64,
    pub device_errors: AtomicU64,
    pub max_callback_frames: AtomicU64,
}
pub struct Output {
    pub stream: cpal::Stream,
    pub producer: Producer<Frame>,
    pub statistics: Arc<Statistics>,
    pub sample_rate: u32,
    pub channels: u16,
    pub requested_buffer_frames: Option<u32>,
}

/// Callback state can also be driven without a device for allocation tests.
pub struct Callback {
    consumer: Consumer<Frame>,
    stats: Arc<Statistics>,
    rate: u32,
    phase: u64,
    left: Frame,
    right: Frame,
    primed: bool,
}
impl Callback {
    pub fn new(
        consumer: Consumer<Frame>,
        statistics: Arc<Statistics>,
        rate: u32,
    ) -> Result<Self, String> {
        if rate == 0 {
            return Err("zero output sample rate".into());
        }
        Ok(Self {
            consumer,
            stats: statistics,
            rate,
            phase: 0,
            left: [0; 2],
            right: [0; 2],
            primed: false,
        })
    }
    fn pop(&mut self) -> Frame {
        self.consumer.pop().unwrap_or_else(|_| {
            self.stats.underrun_frames.fetch_add(1, Ordering::Relaxed);
            [0; 2]
        })
    }
    pub fn next_frame(&mut self) -> Frame {
        if self.rate == SAMPLE_RATE {
            return self.pop();
        }
        // PORT: when a physical device lacks 44.1 kHz, linear device-rate
        // conversion runs here; the SPU / sequence clock stays at 44.1 kHz.
        if !self.primed {
            self.left = self.pop();
            self.right = self.pop();
            self.primed = true;
        }
        let output = std::array::from_fn(|ch| {
            let a = i64::from(self.left[ch]);
            let b = i64::from(self.right[ch]);
            (a + (b - a) * self.phase as i64 / i64::from(self.rate)) as i16
        });
        self.phase += u64::from(SAMPLE_RATE);
        while self.phase >= u64::from(self.rate) {
            self.phase -= u64::from(self.rate);
            self.left = self.right;
            self.right = self.pop();
        }
        output
    }
    pub fn fill<T: cpal::SizedSample + cpal::FromSample<f32>>(
        &mut self,
        output: &mut [T],
        channels: usize,
    ) {
        self.stats.callbacks.fetch_add(1, Ordering::Relaxed);
        if channels == 0 {
            return;
        }
        self.stats
            .max_callback_frames
            .fetch_max((output.len() / channels) as u64, Ordering::Relaxed);
        for frame in output.chunks_mut(channels) {
            let sample = self.next_frame();
            for (ch, value) in frame.iter_mut().enumerate() {
                let pcm = if channels == 1 {
                    (i32::from(sample[0]) + i32::from(sample[1])) as f32 / 65536.0
                } else {
                    sample.get(ch).copied().unwrap_or(0) as f32 / 32768.0
                };
                *value = T::from_sample(pcm);
            }
        }
    }
}

impl Output {
    /// Construct paused. Prime enough for the device's initial burst before
    /// play (1024 native frames on the tested WASAPI device). Ring capacity
    /// and buffer request are supplied explicitly in native 44.1 kHz frames.
    pub fn open(capacity: usize, buffer_frames: u32) -> Result<Self, String> {
        if capacity == 0 {
            return Err("zero audio ring capacity".into());
        }
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("no audio output device")?;
        let preferred = device
            .supported_output_configs()
            .map_err(|e| e.to_string())?
            .find(|c| {
                c.channels() == 2
                    && c.min_sample_rate().0 <= SAMPLE_RATE
                    && c.max_sample_rate().0 >= SAMPLE_RATE
                    && matches!(
                        c.sample_format(),
                        cpal::SampleFormat::F32 | cpal::SampleFormat::I16 | cpal::SampleFormat::U16
                    )
            })
            .map(|c| c.with_sample_rate(cpal::SampleRate(SAMPLE_RATE)));
        let config = match preferred {
            Some(c) => c,
            None => device.default_output_config().map_err(|e| e.to_string())?,
        };
        let format = config.sample_format();
        let mut stream_config = config.config();
        let requested = match config.buffer_size() {
            cpal::SupportedBufferSize::Range { min, max } => Some(buffer_frames.clamp(*min, *max)),
            cpal::SupportedBufferSize::Unknown => None,
        };
        if let Some(size) = requested {
            stream_config.buffer_size = cpal::BufferSize::Fixed(size);
        }
        let (producer, consumer) = RingBuffer::new(capacity);
        let stats = Arc::new(Statistics::default());
        let mut callback =
            Callback::new(consumer, Arc::clone(&stats), stream_config.sample_rate.0)?;
        let error_stats = Arc::clone(&stats);
        let errors = move |_e| {
            error_stats.device_errors.fetch_add(1, Ordering::Relaxed);
        };
        let channels = usize::from(stream_config.channels);
        let stream = match format {
            cpal::SampleFormat::F32 => device.build_output_stream(
                &stream_config,
                move |data: &mut [f32], _| callback.fill(data, channels),
                errors,
                None,
            ),
            cpal::SampleFormat::I16 => device.build_output_stream(
                &stream_config,
                move |data: &mut [i16], _| callback.fill(data, channels),
                errors,
                None,
            ),
            cpal::SampleFormat::U16 => device.build_output_stream(
                &stream_config,
                move |data: &mut [u16], _| callback.fill(data, channels),
                errors,
                None,
            ),
            _ => return Err(format!("unsupported device format {format:?}")),
        }
        .map_err(|e| e.to_string())?;
        Ok(Self {
            stream,
            producer,
            statistics: stats,
            sample_rate: stream_config.sample_rate.0,
            channels: stream_config.channels,
            requested_buffer_frames: requested,
        })
    }
    pub fn play(&self) -> Result<(), String> {
        self.stream.play().map_err(|e| e.to_string())
    }
    pub fn pause(&self) -> Result<(), String> {
        self.stream.pause().map_err(|e| e.to_string())
    }
}
