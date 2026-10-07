// SPDX-License-Identifier: GPL-3.0-only
// Original metadata and options live beside, never inside, 636-byte slots.
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static TEMP_ID: AtomicU64 = AtomicU64::new(0);
pub struct SidecarStore {
    root: PathBuf,
}
impl SidecarStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    #[cfg(test)]
    pub fn root(&self) -> &std::path::Path {
        &self.root
    }
    pub fn valid(kind: u32, id: u32, count: u32) -> bool {
        (kind == 0 && id < 330 && count == 12) || (kind == 1 && id < 30 && count == 56)
    }
    fn path(&self, kind: u32, id: u32) -> Result<(PathBuf, usize), String> {
        match kind {
            0 if id < 330 => Ok((self.root.join(format!("slot-{id:03}.shmi")), 12)),
            1 if id < 30 => Ok((self.root.join(format!("options-{id:02}.shoc")), 56)),
            _ => Err("invalid items sidecar identity".into()),
        }
    }
    pub fn read(&self, kind: u32, id: u32) -> Result<Option<Vec<u8>>, String> {
        let (path, size) = self.path(kind, id)?;
        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };
        if file.metadata().map_err(|e| e.to_string())?.len() != size as u64 {
            return Err("items sidecar length mismatch".into());
        }
        let mut bytes = vec![0; size];
        file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
        Ok(Some(bytes))
    }
    pub fn write(&self, kind: u32, id: u32, bytes: &[u8]) -> Result<(), String> {
        let (path, size) = self.path(kind, id)?;
        if bytes.len() != size {
            return Err("items sidecar length mismatch".into());
        }
        fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        let temp = self.root.join(format!(
            ".items-{kind}-{id}-{}-{}.tmp",
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
    #[test]
    fn sidecars_are_bounded_atomic_and_independent_of_payloads() {
        let store = SidecarStore::new(std::env::temp_dir().join(format!(
            "sh-items-sidecars-{}-{}-{}",
            std::any::type_name::<SidecarStore>().replace("::", "-"),
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::Relaxed)
        )));
        assert!(store.read(0, 0).unwrap().is_none());
        for (kind, id, size) in [(0, 329, 12), (1, 29, 56)] {
            store.write(kind, id, &vec![0xa5; size]).unwrap();
            store.write(kind, id, &vec![0x5a; size]).unwrap();
            assert_eq!(store.read(kind, id).unwrap().unwrap(), vec![0x5a; size]);
            assert!(store.write(kind, id, &vec![0; size + 1]).is_err());
            fs::write(store.path(kind, id).unwrap().0, vec![0; size - 1]).unwrap();
            assert!(store.read(kind, id).is_err());
            fs::remove_file(store.path(kind, id).unwrap().0).unwrap();
        }
        assert!(!SidecarStore::valid(0, 330, 12));
        assert!(store.read(2, 0).is_err());
        assert_eq!(fs::read_dir(store.root()).unwrap().count(), 0);
        fs::remove_dir(store.root()).unwrap();
    }
}
