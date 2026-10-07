// SPDX-License-Identifier: GPL-3.0-only
//! Native slots preserve the original 636-byte save payload, including reserved bits.
use crate::assets::decode_save;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const SAVE_SIZE: usize = 636;
/// Two original cards, each with 15 files of 11 saves. Native identity is stable.
pub const SLOT_COUNT: u32 = 330;
static TEMP_ID: AtomicU64 = AtomicU64::new(0);

pub struct SaveStore {
    root: PathBuf,
}
impl SaveStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    pub fn app_data() -> Result<Self, String> {
        let base = if cfg!(windows) {
            std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .ok_or("LOCALAPPDATA is unavailable")?
        } else {
            std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")))
                .ok_or("app data folder is unavailable")?
        };
        Ok(Self::new(base.join("SilentHillIOS/saves")))
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    fn path(&self, slot: u32) -> Result<PathBuf, String> {
        if slot >= SLOT_COUNT {
            return Err("native save slot is outside 0..330".into());
        }
        Ok(self.root.join(format!("slot-{slot:03}.shs")))
    }
    pub fn read(&self, slot: u32) -> Result<Option<[u8; SAVE_SIZE]>, String> {
        let path = self.path(slot)?;
        let mut file = match File::open(&path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        if file.metadata().map_err(|e| e.to_string())?.len() != SAVE_SIZE as u64 {
            return Err(format!("save slot {slot} is not exactly 636 bytes"));
        }
        let mut bytes = [0; SAVE_SIZE];
        file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
        decode_save(&bytes).map_err(|e| e.to_string())?;
        Ok(Some(bytes))
    }
    pub fn write(&self, slot: u32, bytes: &[u8]) -> Result<(), String> {
        let path = self.path(slot)?;
        decode_save(bytes).map_err(|e| e.to_string())?;
        fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        let temp = self.root.join(format!(
            ".slot-{slot:03}-{}-{}.tmp",
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        let result = (|| {
            file.write_all(bytes)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temp, &path)
        })()
        .map_err(|e: std::io::Error| e.to_string());
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn store() -> SaveStore {
        let root = std::env::temp_dir().join(format!(
            "sh-save-test-{}-{}",
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        SaveStore::new(root)
    }
    #[test]
    fn atomic_replacement_preserves_all_original_bytes_and_slot_identity() {
        let store = store();
        assert_eq!(store.read(0).unwrap(), None);
        let bytes: Vec<_> = (0..SAVE_SIZE).map(|i| (i * 37) as u8).collect();
        store.write(0, &bytes).unwrap();
        assert_eq!(store.read(0).unwrap().unwrap().as_slice(), bytes);
        let mut replacement = bytes.clone();
        replacement[0x260..0x264].copy_from_slice(&0xf0123456u32.to_le_bytes());
        store.write(0, &replacement).unwrap();
        store.write(329, &bytes).unwrap();
        assert_eq!(store.read(0).unwrap().unwrap().as_slice(), replacement);
        assert_eq!(store.read(329).unwrap().unwrap().as_slice(), bytes);
        assert!(store.write(0, &bytes[..635]).is_err());
        assert_eq!(store.read(0).unwrap().unwrap().as_slice(), replacement);
        assert!(store.write(330, &bytes).is_err());
        assert!(store.read(330).is_err());
        assert_eq!(fs::read_dir(store.root()).unwrap().count(), 2);
        fs::remove_file(store.path(0).unwrap()).unwrap();
        fs::remove_file(store.path(329).unwrap()).unwrap();
        fs::remove_dir(store.root()).unwrap();
    }
    #[test]
    fn corrupted_or_card_container_files_fail_without_being_overwritten() {
        let store = store();
        fs::create_dir_all(store.root()).unwrap();
        for size in [0, 635, 637, 8192] {
            fs::write(store.path(1).unwrap(), vec![0; size]).unwrap();
            assert!(store.read(1).is_err());
            assert_eq!(
                fs::metadata(store.path(1).unwrap()).unwrap().len(),
                size as u64
            );
        }
        fs::remove_file(store.path(1).unwrap()).unwrap();
        fs::remove_dir(store.root()).unwrap();
    }
}
