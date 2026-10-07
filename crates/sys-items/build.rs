// SPDX-License-Identifier: GPL-3.0-only
use std::{env, path::PathBuf, process::Command};
fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    assert!(
        Command::new("python")
            .args(["-X", "utf8"])
            .arg(root.join("tools/prepare_items.py"))
            .arg("--out")
            .arg(&out)
            .status()
            .unwrap()
            .success()
    );
    let mut b = cc::Build::new();
    b.include(out.join("include"))
        .include(&out)
        .include(root.join("port/sys/items"))
        .define("VER_USA", None)
        .define("SKIP_ASM", None);
    if b.get_compiler().is_like_msvc() {
        b.flag("/std:c11")
            .flag("/W4")
            .flag("/WX")
            .flag("/Gy")
            .flag("/Gw");
    } else {
        b.flag("-std=c11")
            .flag("-Wall")
            .flag("-Wextra")
            .flag("-Werror");
    }
    for name in std::fs::read_to_string(out.join("units.txt"))
        .unwrap()
        .lines()
    {
        b.file(out.join(format!("{name}.c")));
    }
    // All complete units are compiled strictly, but the fixture links only
    // the original functions it exercises. Engine imports stay core-owned.
    b.cargo_metadata(false).compile("sh_items_compile_gate");
    let mut h = cc::Build::new();
    h.include(out.join("include"))
        .include(&out)
        .include(root.join("port/sys/items"))
        .define("VER_USA", None)
        .define("SKIP_ASM", None);
    if h.get_compiler().is_like_msvc() {
        h.flag("/std:c11").flag("/W4").flag("/WX");
    } else {
        h.flag("-std=c11")
            .flag("-Wall")
            .flag("-Wextra")
            .flag("-Werror");
    }
    for name in ["abi.c", "save_backend.c"] {
        h.file(root.join("port/sys/items").join(name));
    }
    h.file(root.join("crates/sys-items/probes.c"))
        .file(out.join("headless_original.c"))
        .cargo_metadata(false)
        .compile("sh_sys_items");
    // Only #[cfg(test)] explicitly links the fixture library. Production Rust
    // consumers of the decoders/storage never inherit synthetic C identities.
    println!("cargo:rustc-link-search=native={}", out.display());
    println!(
        "cargo:rerun-if-changed={}",
        root.join("tools/prepare_items.py").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        root.join("port/sys/items").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        root.join("crates/sys-items/probes.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        root.join("game/decomp").display()
    );
}
