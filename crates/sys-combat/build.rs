// SPDX-License-Identifier: GPL-3.0-only
use std::{env, fs, path::PathBuf, process::Command};
fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    assert!(
        Command::new("python")
            .arg(root.join("tools/prepare_combat.py"))
            .arg("--decomp")
            .arg(root.join("game/decomp"))
            .arg("--out")
            .arg(&out)
            .status()
            .expect("Python combat generator")
            .success()
    );
    for name in ["assets", "gameplay", "maps"] {
        let source = fs::read_to_string(root.join(format!("host/src/{name}.rs"))).unwrap();
        // PORT: Reuse the core decoders verbatim without pulling the native host
        // executable or its unrelated test fixtures into this standalone harness.
        let source = source
            .split("#[cfg(test)]")
            .next()
            .unwrap()
            .replace("//!", "//");
        fs::write(out.join(format!("{name}.rs")), source).unwrap();
        println!(
            "cargo:rerun-if-changed={}",
            root.join(format!("host/src/{name}.rs")).display()
        );
    }
    let mut build = cc::Build::new();
    build
        .include(root.join("port/sys/combat"))
        .include(root.join("port"))
        .include(root.join("port/include"))
        .include(&out)
        .include(root.join("game/decomp/include"))
        .file(out.join("combat_original.c"))
        .file(out.join("combat_helpers.c"))
        .file(root.join("port/sys/combat/combat.c"))
        .file(root.join("crates/sys-combat/harness.c"))
        .file(out.join("combat_test_math.c"))
        .file(out.join("combat_assets.c"))
        .warnings_into_errors(true);
    for kind in ["cat", "parasite", "flauros", "stalker"] {
        build.file(out.join(format!("combat_ai_{kind}.c")));
    }
    if build.get_compiler().is_like_msvc() {
        build.flag("/std:c11").flag("/W4").flag("/WX");
    } else {
        build
            .flag("-std=c11")
            .flag("-Wall")
            .flag("-Wextra")
            .flag("-Werror");
    }
    build.compile("sys_combat_native");
    for path in [
        "tools/prepare_combat.py",
        "port/sys/combat",
        "crates/sys-combat/harness.c",
        "game/decomp/src/bodyprog/bodyprog_combat_8008A058.c",
        "game/decomp/include",
        "game/decomp/src/maps/characters",
        "game/decomp/src/bodyprog/items/item_screens_3.c",
        "game/decomp/src/bodyprog/events/npc_main.c",
        "game/decomp/src/bodyprog/collision/chara.c",
        "game/decomp/src/maps/chara_util.c",
        "tools/prepare_sdk.py",
        "tools/prepare_gameplay.py",
        "tools/prepare_maps.py",
    ] {
        println!("cargo:rerun-if-changed={}", root.join(path).display());
    }
}
