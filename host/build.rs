// SPDX-License-Identifier: GPL-3.0-only
use std::fs;
use std::path::{Path, PathBuf};

fn between<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
    let first = text.find(start).expect("pinned upstream start marker");
    let last = text[first..].find(end).expect("pinned upstream end marker") + first;
    &text[first..last]
}

// PORT: SDK long means a PS1 word. Preserve the reference notices and every size
// assertion while replacing this scalar in generated SDK declarations for LP64.
fn fixed_long(text: &str) -> String {
    text.split_inclusive(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .map(|token| {
            let end = token
                .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .unwrap_or(token.len());
            if &token[..end] == "long" {
                format!("int32_t{}", &token[end..])
            } else {
                token.to_owned()
            }
        })
        .collect()
}

fn main() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let decomp = std::env::var_os("SH_DECOMP_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("game/decomp"));
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
    println!("cargo:rerun-if-changed={}", decomp.join(".git").display());
    println!(
        "cargo:rerun-if-changed={}",
        decomp.join("include").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        decomp.join("src/main/filetable.c.USA.inc").display()
    );
    // PORT: Generated reference C contains no game bytes; OUT_DIR permits data-free CI builds.
    let generated =
        PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo OUT_DIR")).join("native-source");
    fs::create_dir_all(&generated).expect("generated-source directory");
    fs::create_dir_all(generated.join("psyq")).expect("generated SDK directory");
    for header in ["libgte.h", "libgpu.h"] {
        let source = decomp.join("include/psyq").join(header);
        println!("cargo:rerun-if-changed={}", source.display());
        let text = fs::read_to_string(source).expect("SDK header");
        fs::write(generated.join("psyq").join(header), fixed_long(&text))
            .expect("fixed-width SDK header");
    }
    let mut build = cc::Build::new();
    build
        .include(repo.join("port"))
        .include(repo.join("port/include"))
        .include(&generated)
        .include(decomp.join("include"))
        .include(decomp.join("src/main"))
        .warnings_into_errors(true);
    if build.get_compiler().is_like_msvc() {
        build.flag("/std:c11").flag("/W4");
    } else {
        build.flag("-std=c11").flag("-Wall").flag("-Wextra");
    }
    let inputs = [
        ("main", "src/main/main.c"),
        ("game_main", "src/bodyprog/sys/game_main.c"),
        ("konami", "src/screens/b_konami/b_konami.c"),
        ("fade", "src/bodyprog/screen/screen_fade.c"),
        ("screen", "src/bodyprog/screen/screen_draw.c"),
        ("vsync", "src/bodyprog/sys/vsync.c"),
        ("background", "src/bodyprog/screen/background_draw.c"),
        ("settings", "src/bodyprog/sys/settings_reset.c"),
        ("fs_screens", "src/bodyprog/sys/fs_screens.c"),
        ("bg_etc", "src/bodyprog/world/world_draw.c"),
    ];
    for (name, relative) in inputs {
        let source = decomp.join(relative);
        println!("cargo:rerun-if-changed={}", source.display());
        let original = fs::read_to_string(&source).expect("read upstream C");
        // PORT: Compile boot functions against the native interface, excluding unrelated headers/code.
        // Function bodies are read from the pinned decomp, not reimplemented game state machines.
        let selected = match name {
            "konami" => original.clone(),
            "settings" => between(
                &original,
                "void Settings_ScreenAndVolUpdate",
                "const s32 __pad_rodata",
            )
            .to_owned(),
            "fs_screens" => between(
                &original,
                "void GameFs_TitleGfxSeek",
                "void GameFs_OptionBinLoad",
            )
            .to_owned(),
            "bg_etc" => between(
                &original,
                "void GameFs_BgEtcGfxLoad",
                "void GameFs_BgItemLoad",
            )
            .to_owned(),
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
        if name == "konami" {
            // PORT: The sole writable overlay static is separate native data,
            // restored from its compiled initial image on every native load.
            native = native
                .replace(
                    "    static u8 nextGameState = GameState_Init; // 0x800CA4F0",
                    "",
                )
                .replace("nextGameState", "sh_b_konami_data.next_state")
                // PORT: Both logo emitters use the same unsigned 32-bit OT tag view.
                .replace("s32*  ptr;", "u32*  ptr;")
                // PORT: USA excludes the only consumer of this NTSC-J local.
                .replace("                    s32 curTime;", "")
                // PORT: Packet arithmetic uses native pointers, never u32.
                .replace(
                    "(g_ActiveBufferIdx << 0xF) + (u32)TEMP_MEMORY_ADDR",
                    "(PACKET*)(TEMP_MEMORY_ADDR + (g_ActiveBufferIdx << 15))",
                );
            native = format!(
                "typedef struct {{ u8 next_state; }} ShBootOverlayData;\nstatic const ShBootOverlayData sh_b_konami_initial={{GameState_Init}};\nstatic ShBootOverlayData sh_b_konami_data={{GameState_Init}};\nvoid sh_b_konami_reset(void) {{ sh_b_konami_data=sh_b_konami_initial; }}\nint sh_b_konami_reset_probe(void) {{ sh_b_konami_data.next_state=255; sh_b_konami_reset(); return sh_b_konami_data.next_state==GameState_Init; }}\n{native}"
            );
        }
        if name == "settings" {
            // PORT: Name the first field of the contiguous native binding array.
            native = native.replace(
                "ptr = &g_GameWorkPtr->config.controllerConfig;",
                "ptr = &g_GameWorkPtr->config.controllerConfig.enter;",
            );
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
                "ptr = (u32*)&g_OtTags0[g_ActiveBufferIdx][15]",
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
        "port/overlay.h",
        "port/include/common.h",
        "port/include/types.h",
        "port/runtime.c",
        "port/disk32.h",
        "port/layout_check.c",
    ] {
        println!("cargo:rerun-if-changed={}", repo.join(file).display());
    }
    build
        .file(repo.join("port/runtime.c"))
        .file(repo.join("port/layout_check.c"))
        .define("SH_CHECK_BOOT_LAYOUT", None)
        .compile("sh_native_boot");
}
