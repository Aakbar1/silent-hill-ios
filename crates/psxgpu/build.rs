// SPDX-License-Identifier: GPL-3.0-only
fn main() {
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
