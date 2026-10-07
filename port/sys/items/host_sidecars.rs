// SPDX-License-Identifier: GPL-3.0-only
// Included inside the host native worker module; also exercised by the harness.
#[path = "../../../crates/sys-items/src/sidecars.rs"]
pub mod storage;
use std::path::Path;

/// # Safety
/// The destination must be writable for count bytes when the identity is valid.
pub unsafe fn read(root: &Path, kind: u32, id: u32, destination: *mut u8, count: u32) -> i32 {
    if destination.is_null() || !storage::SidecarStore::valid(kind, id, count) {
        return 2;
    }
    match storage::SidecarStore::new(root.to_path_buf()).read(kind, id) {
        Ok(Some(bytes)) => {
            // SAFETY: the C callback promises a writable validated-size record.
            unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), destination, bytes.len()) };
            0
        }
        Ok(None) => 1,
        Err(_) => 2,
    }
}
/// # Safety
/// The source must be readable for count bytes when the identity is valid.
pub unsafe fn write(root: &Path, kind: u32, id: u32, source: *const u8, count: u32) -> i32 {
    if source.is_null() || !storage::SidecarStore::valid(kind, id, count) {
        return 2;
    }
    // SAFETY: the C callback promises one readable validated-size record.
    let bytes = unsafe { std::slice::from_raw_parts(source, count as usize) };
    i32::from(
        storage::SidecarStore::new(root.to_path_buf())
            .write(kind, id, bytes)
            .is_err(),
    ) * 2
}
