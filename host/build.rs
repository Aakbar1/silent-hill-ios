// SPDX-License-Identifier: GPL-3.0-only
use std::fs;
use std::path::{Path, PathBuf};

fn between<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
    let first = text.find(start).expect("pinned upstream start marker");
    let last = text[first..].find(end).expect("pinned upstream end marker") + first;
    &text[first..last]
}

fn main() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let decomp = std::env::var_os("SH_DECOMP_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("../../reference/silent-hill-decomp"));
    println!("cargo:rerun-if-env-changed=SH_DECOMP_DIR");
    let head = std::process::Command::new("git")
        .arg("-C")
        .arg(&decomp)
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("read pinned reference revision");
    assert!(
        head.status.success(),
        "SH_DECOMP_DIR must be the reference Git checkout"
    );
    assert_eq!(
        String::from_utf8_lossy(&head.stdout).trim(),
        "d9e28f8315c7938117224f21516786d9d149a145",
        "unexpected decomp revision: review the boot transforms before updating the pin"
    );
    println!(
        "cargo:rerun-if-changed={}",
        decomp.join(".git/HEAD").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        decomp.join("include").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        decomp.join("src/main/filetable.c.USA.inc").display()
    );
    let generated = repo.join("../../private/work/boot/native-source");
    fs::create_dir_all(&generated).expect("private generated-source directory");
    let mut build = cc::Build::new();
    build
        .include(repo.join("port"))
        .include(repo.join("port/include"))
        .include(&generated)
        .include(decomp.join("include"))
        .include(decomp.join("src/main"))
        .flag("/std:c11")
        .warnings_into_errors(true);
    let inputs = [
        ("main", "src/main/main.c"),
        ("game_main", "src/bodyprog/sys/game_main.c"),
        ("konami", "src/screens/b_konami/b_konami.c"),
        ("fade", "src/bodyprog/screen/screen_fade.c"),
        ("screen", "src/bodyprog/screen/screen_draw.c"),
        ("vsync", "src/bodyprog/sys/vsync.c"),
        ("background", "src/bodyprog/screen/background_draw.c"),
    ];
    for (name, relative) in inputs {
        let source = decomp.join(relative);
        println!("cargo:rerun-if-changed={}", source.display());
        let original = fs::read_to_string(&source).expect("read upstream C");
        // PORT: Compile boot functions against the native interface, excluding unrelated headers/code.
        // Function bodies are read from the pinned decomp, not reimplemented game state machines.
        let selected = match name {
            "konami" => format!(
                "{}\n{}",
                between(
                    &original,
                    "void GameState_KonamiLogo_Update",
                    "s32 GameState_KcetLogo_MemCardCheck"
                ),
                between(
                    &original,
                    "void BootScreen_ImageSegmentDraw",
                    "void BootScreen_KcetScreenDraw"
                )
            ),
            "background" => format!(
                "q0_8 g_Screen_BackgroundImgGamma = Q8(0.5f);\n{}",
                between(
                    &original,
                    "void Screen_BackgroundImgDraw(",
                    "void Screen_BackgroundImgTransition("
                )
            ),
            _ => original.clone(),
        };
        let mut native = selected
            .lines()
            .map(|line| {
                if line.trim_start().starts_with("#include") {
                    ""
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        // PORT: Bind overlay data destinations to native data storage, not executable PS1 addresses.
        native = native
            .replace("(void*)0x800C9578", "(void*)port_overlay_dynamic")
            .replace("(void*)0x80024B60", "(void*)port_overlay_body");
        if name == "main" {
            native = native
                .replace("int main(void)", "int sh_main(void)")
                // PORT: Make original 16-bit packet/display narrowing explicit for MSVC.
                .replace(
                    "disp.y = 256 - (g_MainFbIdx * 224)",
                    "disp.y = (s16)(256 - (g_MainFbIdx * 224))",
                )
                .replace(
                    "offsetY              = 256 - (fbNext * 224)",
                    "offsetY              = (s16)(256 - (fbNext * 224))",
                );
        }
        // PORT: Remove an upstream unused local so the native compile stays warning-free.
        if name == "game_main" {
            native = native.replace("    s32 gameState;", "");
        }
        // PORT: The native OT is a separate allocation; name its tail instead of PS1 BSS adjacency.
        native = native.replace(
            "(GsOT*)&g_OtTags1[g_ActiveBufferIdx + 1][0]",
            "(GsOT*)&g_OtTags1[g_ActiveBufferIdx][ORDERING_TABLE_SIZE - 1]",
        );
        // PORT: Preserve negative coordinate scaling without undefined signed left shifts.
        native = native.replace(
            "-120 << (g_GameWork.gsScreenHeight >> 8)",
            "-120 * (1 << (g_GameWork.gsScreenHeight >> 8))",
        );
        // PORT: GsOT_TAG is a wire-compatible 32-bit record; make aliases explicit.
        native = native
            .replace(
                "= &g_OtTags0[g_ActiveBufferIdx][otz]",
                "= (u32*)&g_OtTags0[g_ActiveBufferIdx][otz]",
            )
            .replace(
                "ptr = &g_OtTags0[g_ActiveBufferIdx][15]",
                "ptr = (s32*)&g_OtTags0[g_ActiveBufferIdx][15]",
            )
            .replace(
                "ot = &g_OtTags0[g_ActiveBufferIdx][5]",
                "ot = (GsOT*)&g_OtTags0[g_ActiveBufferIdx][5]",
            );
        if name == "fade" {
            for field in ["r0", "g0", "b0"] {
                native = native.replace(
                    &format!("tile->{field} = Q12_TO_Q8("),
                    &format!("tile->{field} = (u8)Q12_TO_Q8("),
                );
            }
        }
        if name == "screen" {
            native = native.replace("GsInitGraph2(g_GameWork.gsScreenWidth, g_GameWork.gsScreenHeight,", "GsInitGraph2((u16)g_GameWork.gsScreenWidth, (u16)g_GameWork.gsScreenHeight,")
                .replace("displayEnv->screen.x = g_GameWorkConst->config.screenPositionX", "displayEnv->screen.x = (s16)g_GameWorkConst->config.screenPositionX")
                .replace("displayEnv->screen.y = g_GameWorkConst->config.screenPositionY + RANGE_Y", "displayEnv->screen.y = (s16)(g_GameWorkConst->config.screenPositionY + RANGE_Y)")
                ;
        }
        let output = generated.join(format!("{name}.c"));
        fs::write(&output, format!("/* SPDX-License-Identifier: GPL-3.0-only; derived from silent-hill-decomp. */\n#include \"boot.h\"\n#line 1 \"{}\"\n{native}\n", source.display().to_string().replace('\\', "/"))).expect("generate boot C");
        build.file(output);
    }
    for file in [
        "port/boot.h",
        "port/include/common.h",
        "port/include/types.h",
        "port/runtime.c",
    ] {
        println!("cargo:rerun-if-changed={}", repo.join(file).display());
    }
    build
        .file(repo.join("port/runtime.c"))
        .compile("sh_native_boot");
}
