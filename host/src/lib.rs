// SPDX-License-Identifier: GPL-3.0-only
pub mod asset_store;
pub mod assets;
pub mod backend;
pub mod pad;
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
