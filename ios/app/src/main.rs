// SPDX-License-Identifier: GPL-3.0-only
#[cfg(target_os = "ios")]
fn main() {
    silent_hill_boot::platform_ios::run();
}

#[cfg(not(target_os = "ios"))]
fn main() {
    eprintln!("Use an iOS target for this app; use the host binary on desktop.");
}
