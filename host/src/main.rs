// SPDX-License-Identifier: GPL-3.0-only
use silent_hill_boot::disc::RawDisc;
use std::path::PathBuf;
use std::process::ExitCode;

struct Options {
    disc: PathBuf,
    inspect: bool,
    frames: Option<u64>,
    screenshot: Option<PathBuf>,
}

fn options() -> Result<Options, String> {
    let mut result = Options {
        disc: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../private/disc/Silent Hill (USA).bin"),
        inspect: false,
        frames: None,
        screenshot: None,
    };
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--inspect-disc") => result.inspect = true,
            Some("--disc") => result.disc = args.next().ok_or("--disc needs a path")?.into(),
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
    if result.inspect && (result.frames.is_some() || result.screenshot.is_some()) {
        return Err("--inspect-disc cannot be combined with --frames or --screenshot".into());
    }
    Ok(result)
}

fn run() -> Result<(), String> {
    let options = options()?;
    let started = std::time::Instant::now();
    let mut disc = RawDisc::open(&options.disc).map_err(|e| format!("disc open: {e}"))?;
    println!("Raw MODE2/2352 disc: {} sectors", disc.sector_count());
    let entries = disc
        .root_directory()
        .map_err(|e| format!("ISO root: {e}"))?;
    for entry in entries.iter().filter(|e| !e.is_directory) {
        println!("{}: LBA {}, {} bytes", entry.name, entry.lba, entry.bytes);
    }
    println!(
        "Disc inspection: {:.3} ms",
        started.elapsed().as_secs_f64() * 1000.0
    );
    if options.inspect {
        return Ok(());
    }
    if let Some(path) = &options.screenshot {
        let private_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../private/work/boot")
            .canonicalize()
            .map_err(|e| format!("private output directory: {e}"))?;
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(std::path::Path::new("."))
            .canonicalize()
            .map_err(|e| format!("screenshot parent: {e}"))?;
        if !parent.starts_with(&private_root) {
            return Err("screenshots must be inside private/work/boot/".into());
        }
        if path.exists()
            && !path
                .canonicalize()
                .map_err(|e| e.to_string())?
                .starts_with(&private_root)
        {
            return Err("resolved screenshot target is outside private/work/boot/".into());
        }
    }
    silent_hill_boot::native::run(disc, options.frames.unwrap_or(600), options.screenshot)
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
