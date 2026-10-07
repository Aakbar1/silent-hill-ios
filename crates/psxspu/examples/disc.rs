//! Private developer renders; never writes imported content inside the repo.
use psxspu::{
    Error, Frame, Spu,
    preview::PreviewSynth,
    reverb::Preset,
    sequence::{Sequence, SequenceClock},
    vab::{ToneRequest, Vab},
    xa::XaStream,
};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

struct Asset {
    name: String,
    lba: u64,
    size: usize,
    sectors: usize,
}
fn inventory(path: &Path) -> Result<Vec<Asset>, Box<dyn std::error::Error>> {
    fs::read_to_string(path)?
        .lines()
        .skip(1)
        .map(|line| {
            let c: Vec<_> = line.split(',').collect();
            if c.len() != 11 {
                return Err("unexpected inventory CSV schema".into());
            }
            Ok(Asset {
                name: c[2].to_string(),
                lba: c[7].parse()?,
                size: c[4].parse()?,
                sectors: c[8].parse()?,
            })
        })
        .collect()
}
fn asset(
    disc: &mut File,
    assets: &[Asset],
    name: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let a = assets
        .iter()
        .find(|a| a.name == name)
        .ok_or("asset absent from inventory")?;
    disc.seek(SeekFrom::Start(a.lba * 2352))?;
    let mut output = Vec::with_capacity(a.size);
    let mut sector = [0; 2352];
    for _ in 0..a.sectors {
        disc.read_exact(&mut sector)?;
        let s = psxmedia::Sector::parse(&sector)?;
        output.extend_from_slice(s.payload.get(..2048).ok_or("not a data sector")?);
    }
    if output.len() < a.size {
        return Err("truncated asset extent".into());
    }
    output.truncate(a.size);
    Ok(output)
}
fn wav(path: &Path, pcm: &[Frame], rate: u32) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    let bytes = pcm.len() as u32 * 4;
    file.write_all(b"RIFF")?;
    file.write_all(&(bytes + 36).to_le_bytes())?;
    file.write_all(b"WAVEfmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&2u16.to_le_bytes())?;
    file.write_all(&rate.to_le_bytes())?;
    file.write_all(&(rate * 4).to_le_bytes())?;
    file.write_all(&4u16.to_le_bytes())?;
    file.write_all(&16u16.to_le_bytes())?;
    file.write_all(b"data")?;
    file.write_all(&bytes.to_le_bytes())?;
    let mut writer = std::io::BufWriter::new(file);
    for frame in pcm {
        for sample in frame {
            writer.write_all(&sample.to_le_bytes())?;
        }
    }
    writer.flush()
}
fn setup() -> Spu {
    let mut s = Spu::new();
    s.init();
    s.set_reverb_preset(Preset::Room, true);
    s.set_reverb(true);
    s.reverb.volume = [20 << 8; 2];
    s
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .ok_or("expected worktree directory layout")?;
    let private = root.join("private");
    let out = private.join("work/spu");
    fs::create_dir_all(&out)?;
    let mut disc_path = private.join("disc/Silent Hill (USA).bin");
    let mut inventory_path = private.join("work/survey/archive_inventory.csv");
    let mut seconds = 30usize;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--disc" => disc_path = PathBuf::from(args.next().ok_or("missing disc path")?),
            "--inventory" => {
                inventory_path = PathBuf::from(args.next().ok_or("missing inventory path")?)
            }
            "--seconds" => seconds = args.next().ok_or("missing duration")?.parse()?,
            _ => return Err(format!("unknown argument {arg}").into()),
        }
    }
    if !(1..=600).contains(&seconds) {
        return Err("duration must be 1..600 seconds".into());
    }
    let assets = inventory(&inventory_path)?;
    let mut disc = File::open(&disc_path)?;
    let mut banks = 0;
    let mut tracks = 0;
    for a in &assets {
        if a.name.ends_with(".VAB") {
            let bytes = asset(&mut disc, &assets, &a.name)?;
            Vab::parse(&bytes).map_err(|e| format!("{}: {e}", a.name))?;
            banks += 1;
        }
        if a.name.ends_with(".KDT") {
            let bytes = asset(&mut disc, &assets, &a.name)?;
            let mut s = Sequence::parse(&bytes).map_err(|e| format!("{}: {e}", a.name))?;
            for _ in 0..(5778 * seconds / 10) {
                s.tick(|_| {}).map_err(|e| format!("{}: {e}", a.name))?;
            }
            tracks += 1;
        }
    }
    println!(
        "disc format checks: {banks} VAB banks, {tracks} KDT tracks parsed and stepped for {seconds}s each"
    );
    let bytes = asset(&mut disc, &assets, "1ST/BASE.VAB")?;
    let bank = Vab::parse(&bytes)?;
    let mut s = setup();
    bank.upload(&mut s, 0x1010)?;
    let mut sfx = vec![[0; 2]; 12 * 44100];
    let mut count = 0;
    for (program, p) in bank.programs.iter().enumerate() {
        for (tone, t) in p.tones.iter().enumerate() {
            if count == 6 {
                break;
            }
            if t.sample == 0 {
                continue;
            }
            let a = bank.tone_attr(ToneRequest {
                program,
                tone,
                note: t.center.clamp(t.min_note, t.max_note),
                fine: 0,
                voice: 0,
                base: 0x1010,
                volume: [0x1600; 2],
            })?;
            s.key_on_with_attr(&a)?;
            s.set_reverb_voice(t.mode & 4 != 0, 1);
            let start = count * 2 * 44100;
            s.render(&mut sfx[start..start + 44100]);
            s.key_off(1);
            s.render(&mut sfx[start + 44100..start + 2 * 44100]);
            count += 1;
        }
        if count == 6 {
            break;
        }
    }
    wav(&out.join("vab-sfx.wav"), &sfx, 44100)?;
    println!(
        "VAB: {count} tones auditioned; decode_errors={}",
        s.decode_errors()
    );
    let bytes = asset(&mut disc, &assets, "SND/A2.VAB")?;
    let bank = Vab::parse(&bytes)?;
    let track_bytes = asset(&mut disc, &assets, "SND/A2.KDT")?;
    let mut sequence = Sequence::parse(&track_bytes)?;
    let mut clock = SequenceClock::default();
    let mut preview = PreviewSynth::default();
    let mut s = setup();
    // SD_Start BGM uses a nominal sequence volume of 40.
    preview.set_sequence_volume([40; 2], &mut s, &bank)?;
    bank.upload(&mut s, 0x1010)?;
    let mut music = vec![[0; 2]; seconds * 44100];
    let mut event_error: Option<Error> = None;
    let mut controls = [0u32; 128];
    for (frame_index, frame) in music.iter_mut().enumerate() {
        // PORT: developer audition activates all layers through SdSetMidiVol;
        // production uses the game world state to choose/fade those layers.
        if frame_index == 4410 {
            for channel in 0..16 {
                preview.set_channel_volume(channel, 127, &mut s, &bank)?;
            }
        }
        for _ in 0..clock.advance_frame() {
            sequence.tick(|event| {
                if let psxspu::sequence::Event::Control { controller, .. } = event {
                    controls[usize::from(controller)] += 1;
                }
                if let Err(e) = preview.event(event, &mut s, &bank, 0x1010) {
                    event_error = Some(e);
                }
            })?;
        }
        if let Some(error) = event_error.take() {
            return Err(error.into());
        }
        *frame = s.next_frame([0; 2], [0; 2]);
    }
    wav(&out.join("kdt-music.wav"), &music, 44100)?;
    println!(
        "KDT preview: ticks={} started_notes={} unsupported_controls={} dropped_notes={} decode_errors={}",
        clock.ticks,
        preview.started_notes,
        preview.unsupported_events,
        preview.dropped_notes,
        s.decode_errors()
    );
    println!(
        "KDT controls: {:?}",
        controls
            .iter()
            .enumerate()
            .filter(|(_, n)| **n != 0)
            .collect::<Vec<_>>()
    );
    // gSDXATable[1]: bank 1, sector 0, CD file 1/channel 0, length 362 virtual
    // 60 Hz frames. Disc LBA 39359 excludes the MSF 150-sector pregap.
    let mut xa = XaStream::new(1, 0);
    let mut pcm = Vec::new();
    let mut native_pcm = Vec::new();
    let mut native_decoder = psxmedia::XaDecoder::default();
    let mut raw = [0; 2352];
    disc.seek(SeekFrom::Start(39359 * 2352))?;
    let frames = 362 * 735;
    for _ in 0..2152 {
        disc.read_exact(&mut raw)?;
        let sector = psxmedia::Sector::parse(&raw)?;
        if sector.is_audio() && (sector.file, sector.channel) == (1, 0) {
            let format = psxmedia::AudioFormat::from_xa_coding(sector.coding)?;
            if format.sample_rate != 37800 || format.channels != 2 {
                return Err("unexpected demo XA coding".into());
            }
            let decoded = native_decoder.decode(sector.payload, format)?;
            for pair in decoded.as_chunks::<2>().0 {
                native_pcm.push([pair[0], pair[1]]);
            }
        }
        xa.feed_sector(&raw, &mut pcm)?;
        if pcm.len() >= frames || xa.ended {
            break;
        }
    }
    if pcm.is_empty() || pcm.len() < frames && !xa.ended {
        return Err("XA item did not yield enough PCM".into());
    }
    pcm.truncate(frames.min(pcm.len()));
    let mut s = Spu::new();
    s.init();
    s.write_register(0x1aa, 0xc001)?;
    s.write_register(0x1b0, 0x7fff)?;
    s.write_register(0x1b2, 0x7fff)?;
    let mut output = vec![[0; 2]; pcm.len()];
    s.render_cd(&pcm, &mut output)?;
    wav(&out.join("xa-voice.wav"), &output, 44100)?;
    wav(
        &out.join("xa-native-reference-input.wav"),
        &native_pcm,
        37800,
    )?;
    println!(
        "XA: file=1 channel=0 frames={} duration={:.6}s; outputs={}",
        output.len(),
        output.len() as f64 / 44100.0,
        out.display()
    );
    Ok(())
}
