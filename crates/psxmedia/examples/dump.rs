//! Private, minimal raw-sector reader, deliberately independent of the disc lane.
use psxmedia::{Event, StreamConfig, StreamDecoder};
use std::{
    error::Error,
    fs::{self, File},
    io::{BufWriter, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const PRIVATE: &str = "C:/Claude Projects/Silent Hill iOS/private";
const INTRO_LBA: u64 = 133127;
const INTRO_END: u64 = 153797;

fn png(path: &Path, frame: &psxmedia::Frame) -> Result<(), Box<dyn Error>> {
    let mut encoder = png::Encoder::new(
        BufWriter::new(File::create(path)?),
        frame.width as u32,
        frame.height as u32,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&frame.rgba)?;
    Ok(())
}
fn wav(path: &Path, pcm: &[i16], rate: u32, channels: u16) -> Result<(), Box<dyn Error>> {
    let size = u32::try_from(pcm.len() * 2)?;
    let mut out = BufWriter::new(File::create(path)?);
    out.write_all(b"RIFF")?;
    out.write_all(&(36 + size).to_le_bytes())?;
    out.write_all(b"WAVEfmt ")?;
    out.write_all(&16u32.to_le_bytes())?;
    out.write_all(&1u16.to_le_bytes())?;
    out.write_all(&channels.to_le_bytes())?;
    out.write_all(&rate.to_le_bytes())?;
    out.write_all(&(rate * channels as u32 * 2).to_le_bytes())?;
    out.write_all(&(channels * 2).to_le_bytes())?;
    out.write_all(&16u16.to_le_bytes())?;
    out.write_all(b"data")?;
    out.write_all(&size.to_le_bytes())?;
    for &sample in pcm {
        out.write_all(&sample.to_le_bytes())?;
    }
    out.flush()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    // Only this private destination is permitted; there is no output-path argument.
    let root = PathBuf::from(PRIVATE);
    let output = root.join("work/media");
    fs::create_dir_all(output.join("frames"))?;
    let mut disc = File::open(root.join("disc/Silent Hill (USA).bin"))?;
    disc.seek(SeekFrom::Start(INTRO_LBA * 2352))?;
    let config = StreamConfig {
        audio_filter: Some((1, 1)),
        video_filter: Some((0, 1)),
        ..StreamConfig::default()
    };
    let mut stream = StreamDecoder::new(config)?;
    let mut sector = [0; 2352];
    let mut input = Vec::new();
    let mut pcm = Vec::new();
    let mut frame_count = 0;
    let mut audio_packets = 0;
    let mut rate = 0;
    let mut channels = 0;
    let mut decode_time = Duration::ZERO;
    let mut timeline = BufWriter::new(File::create(output.join("timeline.csv"))?);
    writeln!(timeline, "kind,index,sector,pts_seconds,duration_seconds")?;
    for lba in INTRO_LBA..INTRO_END {
        disc.read_exact(&mut sector)?;
        input.push(sector);
        let before = Instant::now();
        let event = stream.feed_sector(&sector, lba)?;
        decode_time += before.elapsed();
        match event {
            Some(Event::Video(frame)) => {
                frame_count += 1;
                writeln!(
                    timeline,
                    "video,{},{},{:.9},{:.9}",
                    frame.number,
                    frame.first_sector,
                    frame.pts.seconds(),
                    frame.duration.seconds()
                )?;
                png(
                    &output
                        .join("frames")
                        .join(format!("{:04}.png", frame_count)),
                    &frame,
                )?;
            }
            Some(Event::Audio(packet)) => {
                audio_packets += 1;
                rate = packet.format.sample_rate;
                channels = packet.format.channels as u16;
                writeln!(
                    timeline,
                    "audio,{audio_packets},{lba},{:.9},{:.9}",
                    packet.pts.seconds(),
                    packet.pcm.len() as f64 / channels as f64 / rate as f64
                )?;
                pcm.extend(packet.pcm);
            }
            None => {}
        }
        if frame_count == 300 && pcm.len() >= rate as usize * channels as usize * 20 {
            break;
        }
    }
    stream.finish()?;
    if frame_count != 300 || pcm.len() < rate as usize * channels as usize * 10 {
        return Err("insufficient intro output".into());
    }
    wav(
        &output.join("intro-10s.wav"),
        &pcm[..rate as usize * channels as usize * 10],
        rate,
        channels,
    )?;
    // Full audio for the 300-frame sequence, useful for checking synchronization.
    let full_samples = rate as usize * channels as usize * 20;
    wav(
        &output.join("intro-20s.wav"),
        &pcm[..full_samples],
        rate,
        channels,
    )?;
    timeline.flush()?;

    // Three in-memory passes: no file I/O, PNG compression, threads or player.
    let mut passes = Vec::new();
    for _ in 0..3 {
        let mut stream = StreamDecoder::new(config)?;
        let start = Instant::now();
        let mut frames = 0;
        let mut checksum = 0u64;
        for (i, bytes) in input.iter().enumerate() {
            if let Some(Event::Video(frame)) = stream.feed_sector(bytes, INTRO_LBA + i as u64)? {
                frames += 1;
                checksum = checksum.wrapping_add(frame.rgba.iter().map(|&v| v as u64).sum::<u64>());
            }
        }
        stream.finish()?;
        std::hint::black_box(checksum);
        assert_eq!(frames, 300);
        passes.push(300.0 / start.elapsed().as_secs_f64());
    }
    passes.sort_by(f64::total_cmp);
    let summary = format!(
        "300 frames, 320x208 at 15 fps; {audio_packets} XA packets, {rate} Hz, {channels} channels.\nSingle thread release throughput (3 passes, I/O excluded, includes XA): {:.2}, {:.2}, {:.2} frames/s; median {:.2}x realtime.\nInitial sector decode time {:.3}s; PNG writes excluded.\nOutput: {}\n",
        passes[0],
        passes[1],
        passes[2],
        passes[1] / 15.0,
        decode_time.as_secs_f64(),
        output.display()
    );
    print!("{summary}");
    fs::write(output.join("decode-results.txt"), summary)?;
    Ok(())
}
