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
