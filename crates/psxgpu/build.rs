// SPDX-License-Identifier: GPL-3.0-only
fn main() {
    // PORT: Match the host's explicit Windows Rust-only diagnostic mode. Real
    // Apple builds still compile this C; never accept the bypass on macOS/CI.
    println!("cargo:rerun-if-env-changed=SH_IOS_RUST_CHECK_ONLY");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("ios")
        && std::env::var("SH_IOS_RUST_CHECK_ONLY").as_deref() == Ok("1")
    {
        assert_eq!(
            std::env::consts::OS,
            "windows",
            "Apple CI must compile GPU C"
        );
        println!("cargo:warning=Rust type-check ONLY: GPU C is NOT compiled");
        return;
    }
    for file in [
        "native/gte.c",
        "native/gte_divider.c",
        "native/gte_divider.h",
        "native/portable.h",
        "native/psxgpu_gte.h",
    ] {
        println!("cargo:rerun-if-changed={file}");
    }
    let mut c = cc::Build::new();
    c.files(["native/gte.c", "native/gte_divider.c"])
        .include("native")
        .warnings(true)
        .extra_warnings(true)
        .warnings_into_errors(true);
    if c.get_compiler().is_like_msvc() {
        c.flag("/std:c11").flag("/W4");
    } else {
        c.flag("-std=c11")
            .flag("-fno-strict-aliasing")
            .flag("-fwrapv");
    }
    c.compile("psxgpu_gte");
}
