// SPDX-License-Identifier: GPL-3.0-only
//! Read player-owned PS1 images without emulation or platform-specific libraries.
//! Archive bytes remain encrypted/compressed exactly as they are on the disc.
#![doc = include_str!("../README.md")]
mod cue;
mod disc;
mod error;
mod game;
mod import;
mod iso;

pub use disc::{DiscImage, ImageFormat, Sector, SectorKind, SectorReader};
pub use error::{Error, Result};
pub use game::{Archive, Entry, GameDisc, Release, US_11_SHA256, verify};
pub use import::{ImportMode, ImportStats, PackedImage, import};
pub use iso::{IsoEntry, IsoFileSystem};
