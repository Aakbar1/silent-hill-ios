fn main() {
    println!("cargo:rerun-if-changed=probe.c");
    let mut build = cc::Build::new();
    build
        .file("probe.c")
        .warnings(true)
        .warnings_into_errors(true);
    println!(
        "cargo:metadata=c_compiler={}",
        build.get_compiler().path().display()
    );
    build.compile("native_toolchain_probe");

    if std::env::var_os("CARGO_FEATURE_DECOMP").is_some() {
        let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let decomp = std::env::var_os("SH_DECOMP_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| repo.join("../../reference/silent-hill-decomp"));
        println!("cargo:rerun-if-env-changed=SH_DECOMP_DIR");
        let prelude = repo.join("port/msvc_prelude.h");
        println!("cargo:rerun-if-changed={}", prelude.display());
        // This is a compile gate, not linked game code. Keep upstream ABI assertions active.
        let mut source = cc::Build::new();
        source
            .file(decomp.join("src/main/main.c"))
            .include(decomp.join("include"))
            .include(decomp.join("include/decomp"))
            .include(decomp.join("include/psyq"))
            .define("VER_USA", None)
            // PORT: Omit MIPS assembly inclusion markers for this native compile-only gate.
            // Missing assembly/rodata still needs native definitions before a runnable port.
            .define("SKIP_ASM", None)
            // PORT: Reserve the process entry point for Rust; preserve the C entry as sh_main.
            .define("main", "sh_main")
            .flag("/std:c11")
            .flag("/W4")
            .flag(format!("/FI{}", prelude.display()))
            .warnings_into_errors(true)
            .cargo_metadata(false)
            .compile("decomp_main_compile_gate");
    }
}
