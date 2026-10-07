// SPDX-License-Identifier: GPL-3.0-only
//! Run explicitly with a private output directory, never the repository.
use psxdisc::{Archive, GameDisc, ImportMode, SectorReader, import};
use sha2::{Digest, Sha256};
use std::{
    io::{self, Write},
    path::PathBuf,
    time::Instant,
};

struct HashSink(Sha256);
impl Write for HashSink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn measure<S: SectorReader>(game: &mut GameDisc<S>) -> psxdisc::Result<(String, f64, f64)> {
    let start = Instant::now();
    let entries = game.entries().to_vec();
    let mut sink = HashSink(Sha256::new());
    for entry in &entries {
        game.copy_entry(entry.id, &mut sink)?;
    }
    let all_ms = start.elapsed().as_secs_f64() * 1000.;
    let digest = format!("{:x}", sink.0.finalize());
    let ids: Vec<_> = entries
        .iter()
        .filter(|e| e.archive == Archive::Silent && e.size > 0)
        .map(|e| e.id)
        .collect();
    let start = Instant::now();
    let mut total = 0;
    for id in &ids {
        let bytes = game.read_entry(*id)?;
        total += bytes.len();
        std::hint::black_box(bytes);
    }
    assert!(total > 0);
    let data_ms = start.elapsed().as_secs_f64() * 1000.;
    Ok((digest, all_ms, data_ms))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 3 {
        return Err("usage: import_bench <owned BIN/CUE> <PRIVATE output directory>".into());
    }
    let source = PathBuf::from(&args[1]);
    let output = PathBuf::from(&args[2]);
    let mut expected = None;
    println!("mode,run,bytes,import_ms,open_ms,all_entries_ms,data_entries_ms");
    for run in 0..3 {
        let modes = if run % 2 == 0 {
            [ImportMode::KeepImage, ImportMode::Packed]
        } else {
            [ImportMode::Packed, ImportMode::KeepImage]
        };
        for mode in modes {
            let folder = output.join(format!("{mode:?}-{run}"));
            let stats = import(&source, &folder, mode)?;
            let start = Instant::now();
            let (digest, all_ms, data_ms, open_ms) = match mode {
                ImportMode::KeepImage => {
                    let mut game = GameDisc::open(&stats.path)?;
                    let open_ms = start.elapsed().as_secs_f64() * 1000.;
                    let (digest, all_ms, data_ms) = measure(&mut game)?;
                    (digest, all_ms, data_ms, open_ms)
                }
                ImportMode::Packed => {
                    let mut game = GameDisc::open_packed(&stats.path)?;
                    let open_ms = start.elapsed().as_secs_f64() * 1000.;
                    let (digest, all_ms, data_ms) = measure(&mut game)?;
                    (digest, all_ms, data_ms, open_ms)
                }
            };
            if let Some(ref baseline) = expected {
                assert_eq!(&digest, baseline, "import modes changed entry bytes");
            } else {
                expected = Some(digest);
            }
            // Prove the importer refuses overwrites, preserving the successful file.
            assert!(import(&source, &folder, mode).is_err());
            assert_eq!(std::fs::metadata(&stats.path)?.len(), stats.bytes_written);
            println!(
                "{mode:?},{run},{},{:.3},{open_ms:.3},{all_ms:.3},{data_ms:.3}",
                stats.bytes_written,
                stats.elapsed.as_secs_f64() * 1000.
            );
        }
    }
    println!("PASS: all six imports have identical entry-stream hashes; overwrite checks passed");
    Ok(())
}
