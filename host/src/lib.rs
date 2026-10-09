// SPDX-License-Identifier: GPL-3.0-only
pub mod asset_store;
pub mod assets;
pub mod backend;
pub mod gameplay;
pub mod gpu_wgpu;
pub mod gte;
pub mod maps;
pub mod pad;
pub mod pad_touch;
pub use psxdisc as disc;
pub mod movie;
#[cfg(not(target_os = "ios"))]
pub mod native;
#[cfg(target_os = "ios")]
pub mod native {
    include!(concat!(env!("OUT_DIR"), "/native_ios.rs"));
}
#[cfg(target_os = "ios")]
pub mod platform_ios;
pub mod raster;
pub mod saves;
pub mod spu_cpal;

#[cfg(all(test, not(target_os = "ios")))]
#[path = "../../ios/importer.rs"]
mod ios_importer_tests;

// PORT: UIKit's renderer policy is platform-neutral so Windows exercises it too.
#[path = "../../ios/renderer.rs"]
pub mod ios_renderer;

/// The player's disc image under the project's `private/disc/`, found by walking up from
/// this crate so it works from the main checkout and from worker worktrees alike.
pub fn default_disc_path() -> std::path::PathBuf {
    let rel = std::path::Path::new("private/disc/Silent Hill (USA).bin");
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .map(|dir| dir.join(rel))
        .find(|candidate| candidate.exists())
        .unwrap_or_else(|| manifest.join("../../../").join(rel))
}

/// The project's `private/work/` scratch directory, found the same way as the disc.
pub fn default_private_work() -> std::path::PathBuf {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .map(|dir| dir.join("private"))
        .find(|candidate| candidate.is_dir())
        .map(|private| private.join("work"))
        .unwrap_or_else(|| manifest.join("../../../private/work"))
}
