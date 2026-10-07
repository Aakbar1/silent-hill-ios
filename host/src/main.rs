// SPDX-License-Identifier: GPL-3.0-only
use silent_hill_boot::disc::{GameDisc, IsoFileSystem, SectorReader};
use std::path::PathBuf;
use std::process::ExitCode;

struct Options {
    disc: PathBuf,
    inspect: bool,
    inspect_assets: bool,
    frames: Option<u64>,
    screenshot: Option<PathBuf>,
    input: Option<PathBuf>,
    headless: bool,
    audio: silent_hill_boot::spu_cpal::AudioMode,
    check: silent_hill_boot::native::ReplayCheck,
}

fn options() -> Result<Options, String> {
    let mut result = Options {
        disc: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../private/disc/Silent Hill (USA).bin"),
        inspect: false,
        inspect_assets: false,
        frames: None,
        screenshot: None,
        input: None,
        headless: false,
        audio: Default::default(),
        check: Default::default(),
    };
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--inspect-disc") => result.inspect = true,
            Some("--headless") => result.headless = true,
            Some("--audio") => {
                result.audio = silent_hill_boot::spu_cpal::AudioMode::parse(
                    args.next()
                        .ok_or("--audio needs on, off, or wav:PATH")?
                        .to_str()
                        .ok_or("invalid audio mode")?,
                )?;
            }
            Some("--min-lit-pixels") => {
                result.check.min_lit_pixels = args
                    .next()
                    .ok_or("--min-lit-pixels needs a number")?
                    .to_str()
                    .ok_or("invalid pixel count")?
                    .parse()
                    .map_err(|_| "invalid pixel count")?
            }
            Some("--expect-option-entry") => {
                result.check.option_entry = Some(
                    args.next()
                        .ok_or("--expect-option-entry needs a number")?
                        .to_str()
                        .ok_or("invalid entry")?
                        .parse()
                        .map_err(|_| "invalid entry")?,
                )
            }
            Some("--expect-menu") => {
                result.check.menu_state = Some(
                    args.next()
                        .ok_or("--expect-menu needs a number")?
                        .to_str()
                        .ok_or("invalid menu")?
                        .parse()
                        .map_err(|_| "invalid menu")?,
                )
            }
            Some("--expect-state") => {
                result.check.state = Some(
                    args.next()
                        .ok_or("--expect-state needs a number")?
                        .to_str()
                        .ok_or("invalid state")?
                        .parse()
                        .map_err(|_| "invalid state")?,
                )
            }
            Some("--expect-step") => {
                result.check.step = Some(
                    args.next()
                        .ok_or("--expect-step needs a number")?
                        .to_str()
                        .ok_or("invalid step")?
                        .parse()
                        .map_err(|_| "invalid step")?,
                )
            }
            Some("--min-movie-frames") => {
                result.check.min_movie_frames = args
                    .next()
                    .ok_or("--min-movie-frames needs a number")?
                    .to_str()
                    .ok_or("invalid count")?
                    .parse()
                    .map_err(|_| "invalid count")?
            }
            Some("--expect-movie-skips") => {
                result.check.movie_skips = Some(
                    args.next()
                        .ok_or("--expect-movie-skips needs a number")?
                        .to_str()
                        .ok_or("invalid count")?
                        .parse()
                        .map_err(|_| "invalid count")?,
                )
            }
            Some("--inspect-assets") => {
                result.inspect = true;
                result.inspect_assets = true;
            }
            Some("--disc") => result.disc = args.next().ok_or("--disc needs a path")?.into(),
            Some("--input") => {
                result.input = Some(args.next().ok_or("--input needs a path")?.into())
            }
            Some("--screenshot") => {
                result.screenshot = Some(args.next().ok_or("--screenshot needs a path")?.into());
            }
            Some("--frames") => {
                result.frames = Some(
                    args.next()
                        .ok_or("--frames needs a number")?
                        .to_str()
                        .ok_or("invalid frame count")?
                        .parse::<u64>()
                        .map_err(|_| "invalid frame count")?,
                );
                if result.frames == Some(0) {
                    return Err("--frames must be greater than zero".into());
                }
            }
            _ => return Err(format!("unknown argument: {}", arg.to_string_lossy())),
        }
    }
    if result.inspect
        && (result.frames.is_some() || result.screenshot.is_some() || result.input.is_some())
    {
        return Err("--inspect-disc cannot be combined with run arguments".into());
    }
    Ok(result)
}

fn run() -> Result<(), String> {
    let options = options()?;
    let started = std::time::Instant::now();
    let mut disc = GameDisc::open(&options.disc)
        .map_err(|e| format!("disc open/release verification: {e}"))?;
    println!(
        "Verified {:?} disc: {} sectors, {} archive entries",
        disc.release(),
        disc.source_mut().sector_count(),
        disc.entries().len()
    );
    let iso = IsoFileSystem::open(disc.source_mut()).map_err(|e| e.to_string())?;
    let entries = iso
        .read_directory(disc.source_mut(), &iso.root)
        .map_err(|e| format!("ISO root: {e}"))?;
    for entry in entries.iter().filter(|e| !e.is_directory) {
        println!("{}: LBA {}, {} bytes", entry.name, entry.extent, entry.size);
    }
    println!(
        "Disc inspection: {:.3} ms",
        started.elapsed().as_secs_f64() * 1000.0
    );
    if options.inspect {
        if options.inspect_assets {
            silent_hill_boot::assets::inspect_disc(&mut disc)?;
        }
        return Ok(());
    }
    if let Some(path) = &options.screenshot {
        let private_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../private/work/core2")
            .canonicalize()
            .map_err(|e| format!("private output directory: {e}"))?;
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(std::path::Path::new("."))
            .canonicalize()
            .map_err(|e| format!("screenshot parent: {e}"))?;
        if !parent.starts_with(&private_root) {
            return Err("screenshots must be inside private/work/core2/".into());
        }
        if path.exists()
            && !path
                .canonicalize()
                .map_err(|e| e.to_string())?
                .starts_with(&private_root)
        {
            return Err("resolved screenshot target is outside private/work/core2/".into());
        }
    }
    let replay = options
        .input
        .map(|path| {
            std::fs::read_to_string(&path)
                .map_err(|e| format!("input {}: {e}", path.display()))
                .and_then(|text| silent_hill_boot::pad::ReplayPad::parse(&text))
        })
        .transpose()?;
    silent_hill_boot::spu_cpal::configure(options.audio)?;
    if options.headless {
        return silent_hill_boot::native::run_headless(
            disc,
            options.frames.unwrap_or(600),
            options.screenshot,
            replay.ok_or("--headless requires --input")?,
            options.check,
        )
        .and_then(|()| silent_hill_boot::spu_cpal::require_backend());
    }
    if options.check.state.is_some()
        || options.check.step.is_some()
        || options.check.min_movie_frames != 0
        || options.check.movie_skips.is_some()
        || options.check.menu_state.is_some()
        || options.check.option_entry.is_some()
        || options.check.min_lit_pixels != 0
    {
        return Err("milestone expectations require --headless".into());
    }
    silent_hill_boot::native::run(
        disc,
        options.frames.unwrap_or(600),
        options.screenshot,
        replay,
    )?;
    silent_hill_boot::spu_cpal::require_backend()
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("STOP: {error}");
            ExitCode::FAILURE
        }
    }
}
