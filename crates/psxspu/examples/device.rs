use psxspu::output::Output;
use std::{
    sync::atomic::Ordering,
    thread,
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut output = Output::open(2048, 256)?;
    // Quiet synthetic tone: this example does not access game content.
    let frame = |i: u64| {
        let sample = ((i as f64 * 440.0 * std::f64::consts::TAU / 44100.0).sin() * 1600.0) as i16;
        [sample; 2]
    };
    let mut written = 0;
    // WASAPI shared mode may ask for a larger first callback than the requested
    // quantum. Prime 1024 frames, then maintain only 512 frames of steady lead.
    while written < 1024 {
        output.producer.push(frame(written))?;
        written += 1;
    }
    output.play()?;
    let start = Instant::now();
    let stop = Duration::from_secs(3);
    while start.elapsed() < stop {
        while output.producer.slots() > 1536 {
            output.producer.push(frame(written))?;
            written += 1;
        }
        thread::sleep(Duration::from_millis(2));
    }
    output.pause()?;
    println!(
        "rate={} channels={} requested_buffer={:?} max_callback_frames={} callbacks={} underrun_frames={} device_errors={}",
        output.sample_rate,
        output.channels,
        output.requested_buffer_frames,
        output
            .statistics
            .max_callback_frames
            .load(Ordering::Relaxed),
        output.statistics.callbacks.load(Ordering::Relaxed),
        output.statistics.underrun_frames.load(Ordering::Relaxed),
        output.statistics.device_errors.load(Ordering::Relaxed)
    );
    Ok(())
}
