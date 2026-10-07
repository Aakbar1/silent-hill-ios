// SPDX-License-Identifier: GPL-3.0-only
//! Transactional, data-free platform boundary. The source is always read-only.
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn imported(root: &Path) -> PathBuf {
    root.join("disc.bin")
}

pub fn staging(root: &Path) -> PathBuf {
    root.join("Importing")
}

pub fn import_disc(source: &Path, root: &Path) -> Result<PathBuf, String> {
    if !source
        .extension()
        .is_some_and(|s| s.eq_ignore_ascii_case("bin"))
    {
        return Err(
            "Choose the raw Silent Hill (USA) v1.1 .bin image, not a cue, ISO or ZIP.".into(),
        );
    }
    fs::create_dir_all(root).map_err(storage_error)?;
    let pending = staging(root);
    // Only our fixed disposable staging folder. Never remove the selected file.
    if pending.exists() {
        fs::remove_dir_all(&pending).map_err(storage_error)?;
    }
    fs::create_dir(&pending).map_err(storage_error)?;
    let result = (|| {
        let stats = psxdisc::import(source, &pending, psxdisc::ImportMode::KeepImage)
            .map_err(friendly_error)?;
        // Reverify the COPY, including the executable hash and archive bounds.
        // A crash/short copy cannot become the next launch's active import.
        psxdisc::GameDisc::open(&stats.path).map_err(friendly_error)?;
        let target = imported(root);
        if target.exists() {
            return Err("An imported disc already exists. Restart the app to use it.".into());
        }
        fs::rename(&stats.path, &target).map_err(storage_error)?;
        Ok(target)
    })();
    let _ = fs::remove_dir_all(&pending);
    result
}

fn storage_error(error: std::io::Error) -> String {
    format!("Could not store the disc copy. Check free space and try again. ({error})")
}

fn friendly_error(error: psxdisc::Error) -> String {
    match error {
        psxdisc::Error::WrongRelease { .. } =>
            "This disc is a different release. This app requires Silent Hill (USA), US v1.1. Your file was not changed.".into(),
        psxdisc::Error::Io(e) => storage_error(e),
        _ => "This is not a complete supported Silent Hill (USA) v1.1 raw .bin disc. Finish copying or downloading it, then try again. Your file was not changed.".into(),
    }
}

/// Watch only regular BIN files directly in Documents, never saves or imports.
pub fn candidates(documents: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<_> = fs::read_dir(documents)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|s| s.eq_ignore_ascii_case("bin")))
        .collect();
    paths.sort();
    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static ID: AtomicU64 = AtomicU64::new(0);
    #[test]
    fn failed_import_keeps_source_and_previous_import_and_cleans_staging() {
        let root = std::env::temp_dir().join(format!(
            "ios-import-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        let support = root.join("support");
        fs::create_dir_all(staging(&support)).unwrap();
        fs::write(
            staging(&support).join("disc.bin"),
            b"interrupted synthetic copy",
        )
        .unwrap();
        fs::write(imported(&support), b"previous synthetic import").unwrap();
        let source = root.join("player.BIN");
        let fixture = vec![0u8; 2352]; // Synthetic invalid sector, no game data.
        fs::write(&source, &fixture).unwrap();
        assert!(
            import_disc(&source, &support)
                .unwrap_err()
                .contains("not a complete")
        );
        assert_eq!(fs::read(&source).unwrap(), fixture);
        assert_eq!(
            fs::read(imported(&support)).unwrap(),
            b"previous synthetic import"
        );
        assert!(!staging(&support).exists());
        fs::write(root.join("ignore.cue"), b"synthetic").unwrap();
        fs::create_dir(root.join("directory.bin")).unwrap();
        assert_eq!(candidates(&root), [source]);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn wrong_extension_is_rejected_before_creating_storage() {
        let root = std::env::temp_dir().join(format!(
            "ios-extension-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        assert!(
            import_disc(Path::new("disc.cue"), &root)
                .unwrap_err()
                .contains("raw")
        );
        assert!(!root.exists());
    }

    #[test]
    #[ignore = "player-owned image in private/work/ios2 only; never run in CI"]
    fn private_disc_import_is_byte_exact_and_reopens_as_us11() {
        use std::io::Read;
        let source =
            PathBuf::from(std::env::var_os("SH_IOS_TEST_DISC").expect("private source path"));
        let work = PathBuf::from("C:/Claude Projects/Silent Hill iOS/private/work/ios2");
        fs::create_dir_all(&work).unwrap();
        let root = work.join(format!("import-test-{}", std::process::id()));
        assert!(!root.exists());
        let copied = import_disc(&source, &root).unwrap();
        assert_eq!(
            psxdisc::GameDisc::open(&copied).unwrap().release(),
            psxdisc::Release::Us11
        );
        let mut original = fs::File::open(source).unwrap();
        let mut imported = fs::File::open(copied).unwrap();
        let mut a = [0u8; 65536];
        let mut b = [0u8; 65536];
        assert_eq!(
            original.metadata().unwrap().len(),
            imported.metadata().unwrap().len()
        );
        loop {
            let count = original.read(&mut a).unwrap();
            if count == 0 {
                break;
            }
            imported.read_exact(&mut b[..count]).unwrap();
            assert_eq!(&a[..count], &b[..count]);
        }
        drop(imported);
        fs::remove_dir_all(root).unwrap();
    }
}
