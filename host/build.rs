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
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("ios") {
        // PORT: Extend the existing worker inside its module without editing the
        // parallel core lane. No copied game bytes or alternate game logic.
        let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo OUT_DIR"));
        let native = repo.join("host/src/native.rs");
        let bridge = repo.join("ios/host_bridge.rs");
        println!("cargo:rerun-if-changed={}", native.display());
        println!("cargo:rerun-if-changed={}", bridge.display());
        let native_source = fs::read_to_string(native)
            .expect("shared native worker")
            .replace("\r\n", "\n");
        let clock_marker = "        if host.proxy.is_some() {\n";
        assert_eq!(
            native_source.matches(clock_marker).count(),
            1,
            "review iOS clock seam after core changes"
        );
        // PORT: Use the same 60Hz clock for every UIKit tick, including blank
        // display frames. The desktop proxy/pacing branch stays unchanged.
        let native_source = native_source.replace(
            clock_marker,
            "        if host.proxy.is_none() { ios_clock(); }\n        if host.proxy.is_some() {\n",
        );
        fs::write(
            out.join("native_ios.rs"),
            format!(
                "{}\n{}",
                native_source,
                fs::read_to_string(bridge).expect("iOS worker bridge")
            ),
        )
        .expect("iOS worker extension");
        println!("cargo:rerun-if-env-changed=SH_IOS_RUST_CHECK_ONLY");
        if std::env::var("SH_IOS_RUST_CHECK_ONLY").as_deref() == Ok("1") {
            assert_ne!(std::env::consts::OS, "macos", "CI must compile native C");
            println!("cargo:warning=Rust type-check ONLY: native C and UIKit are NOT compiled");
            return;
        }
        println!(
            "cargo:rerun-if-changed={}",
            repo.join("ios/app.m").display()
        );
        cc::Build::new()
            .file(repo.join("ios/app.m"))
            .flag("-fobjc-arc")
            .flag("-fblocks")
            .flag("-Wall")
            .flag("-Wextra")
            .warnings_into_errors(true)
            .compile("sh_ios_app");
        for framework in [
            "UIKit",
            "Foundation",
            "CoreGraphics",
            "AVFoundation",
            "UniformTypeIdentifiers",
        ] {
            println!("cargo:rustc-link-lib=framework={framework}");
        }
        println!("cargo:rustc-link-lib=objc");
    }
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
    println!(
        "cargo:rerun-if-changed={}",
        repo.join("tools/prepare_gte.py").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        decomp.join("include/psyq/inline_c.h").display()
    );
    let gte = std::process::Command::new("python")
        .arg(repo.join("tools/prepare_gte.py"))
        .arg("--decomp")
        .arg(&decomp)
        .arg("--out")
        .arg(&generated)
        .status()
        .expect("generate native PsyQ GTE");
    assert!(gte.success(), "native GTE generation failed");
    println!(
        "cargo:rerun-if-changed={}",
        repo.join("tools/prepare_gameplay.py").display()
    );
    for file in [
        "tools/prepare_camera.py",
        "tools/prepare_math.py",
        "tools/prepare_world.py",
        "tools/prepare_collision.py",
    ] {
        println!("cargo:rerun-if-changed={}", repo.join(file).display());
    }
    let records = std::process::Command::new("python")
        .arg(repo.join("tools/prepare_gameplay.py"))
        .arg("--decomp")
        .arg(&decomp)
        .arg("--out")
        .arg(&generated)
        .status()
        .expect("generate native gameplay records");
    assert!(records.success(), "native gameplay generation failed");
    println!(
        "cargo:rerun-if-changed={}",
        repo.join("tools/prepare_maps.py").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        decomp.join("src/maps").display()
    );
    let maps = std::process::Command::new(
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("ios") {
            "python3"
        } else {
            "python"
        },
    )
    .arg(repo.join("tools/prepare_maps.py"))
    .arg("--decomp")
    .arg(&decomp)
    .arg("--out")
    .arg(&generated)
    .status()
    .expect("generate native maps");
    assert!(maps.success(), "native map generation failed");
    println!(
        "cargo:rerun-if-changed={}",
        repo.join("tools/prepare_option.py").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        decomp.join("src/screens/options/options.c").display()
    );
    let python = if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("ios") {
        "python3"
    } else {
        "python"
    };
    let option = std::process::Command::new(python)
        .arg(repo.join("tools/prepare_option.py"))
        .arg("--decomp")
        .arg(&decomp)
        .arg("--out")
        .arg(&generated)
        .status()
        .expect("generate OPTION native state");
    assert!(option.success(), "OPTION state generation failed");
    // PORT: Reuse pointer-free upstream save/input records, with their original
    // size assertions. Do not include unrelated pointer-bearing gameplay records.
    let joy_header =
        fs::read_to_string(decomp.join("include/bodyprog/sys/joy.h")).expect("joy records");
    let joy_records = between(
        &joy_header,
        "/** @brief PSX controller input flags.",
        "extern s_ControllerData* const g_Controller0;",
    );
    let save_header =
        fs::read_to_string(decomp.join("include/bodyprog/savegame.h")).expect("save records");
    let save_records = between(
        &save_header,
        "/** @brief Savegame data.",
        "/** @brief User options configuration.",
    );
    let text_header = fs::read_to_string(decomp.join("include/bodyprog/text/text_draw.h"))
        .expect("font constants");
    let text_constants = between(
        &text_header,
        "#define MAP_MSG_CODE_MARKER",
        "// ====================",
    );
    fs::write(generated.join("game_records.h"), format!("/* SPDX-License-Identifier: GPL-3.0-only; derived from silent-hill-decomp. */\n#define INV_ITEM_COUNT_MAX 40\n#define Chara_Count 45\ntypedef u32 q20_12;\ntypedef struct {{u8 id,count,command,field_3;}} s_InventoryItem;\n{joy_records}\n{save_records}\n{text_constants}\n")).expect("native pointer-free records");
    let gpu_header = fs::read_to_string(decomp.join("include/gpu.h")).expect("2D GPU records");
    fs::write(generated.join("gpu_records.h"), format!("/* SPDX-License-Identifier: GPL-3.0-only; derived from silent-hill-decomp. */\n#define RECT_VERT_COUNT 4\n{}",between(&gpu_header,"/** @brief 2D screen-space line.","/** @brief Primitive color."))).expect("2D GPU records");
    let sine_source = decomp.join("src/bodyprog/libkmath/libkmath.s");
    println!("cargo:rerun-if-changed={}", sine_source.display());
    let sine_text = fs::read_to_string(sine_source).expect("reference sine table");
    let sine: Vec<_> = sine_text
        .split("dlabel g_SineTable")
        .nth(1)
        .expect("sine label")
        .lines()
        .filter_map(|line| line.split(".short ").nth(1))
        .map(|value| {
            (u16::from_str_radix(value.trim().trim_start_matches("0x"), 16)
                .expect("reference sine word") as i16)
                .to_string()
        })
        .collect();
    assert_eq!(sine.len(), 5120);
    fs::write(generated.join("sine.c"), format!("/* SPDX-License-Identifier: GPL-3.0-only; derived from silent-hill-decomp libkmath.s. */\n#include \"boot.h\"\nstatic const s16 sine[5120]={{{}}};\ns32 Math_Sin(s32 angle) {{return sine[(u32)angle&4095];}}\ns32 Math_Cos(s32 angle) {{return sine[((u32)angle&4095)+1024];}}\n", sine.join(","))).expect("native sine lookups");
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
        // PsyQ's libgte declares integer ccos/csin/csqrt/catan, which clang otherwise
        // treats as the C99 complex-math builtins and rejects as redeclarations.
        for name in ["ccos", "csin", "csqrt", "catan"] {
            build.flag(format!("-fno-builtin-{name}"));
        }
        // PORT: Apple clang flags warnings in original decomp code (e.g. -Wsign-compare)
        // that MSVC /W4 does not. MSVC CI still builds with /WX, so new port code stays
        // warning-clean there; on Apple targets, report the decomp's warnings without
        // stopping the build so each CI run surfaces all of them at once.
        if std::env::var("CARGO_CFG_TARGET_VENDOR").as_deref() == Ok("apple") {
            build.warnings_into_errors(false);
        }
    }
    let inputs = [
        ("main", "src/main/main.c"),
        ("game_main", "src/bodyprog/sys/game_main.c"),
        ("konami", "src/screens/b_konami/b_konami.c"),
        ("stream", "src/screens/stream/stream.c"),
        ("title", "src/bodyprog/events/title.c"),
        ("text", "src/bodyprog/text/text_draw.c"),
        ("text_debug", "src/bodyprog/text/text_debug_draw.c"),
        ("texture_mode", "src/bodyprog/items/item_screens_3.c"),
        ("brightness", "src/bodyprog/gfx/option_brightness_line.c"),
        ("joy", "src/bodyprog/sys/joy.c"),
        ("rng", "src/main/rng.c"),
        ("save_init", "src/bodyprog/game_boot/game_boot.c"),
        ("reset_player", "src/bodyprog/player_control.c"),
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
            "title" | "rng" => original.clone(),
            "texture_mode" => between(
                &original,
                "void Gfx_Primitive2dTextureSet",
                "void Gfx_Results_ItemsDisplay",
            )
            .to_owned(),
            "text_debug" => between(
                &original,
                "static DVECTOR g_Text_Debug_PositionSet0",
                "char* Text_Debug_IntToString",
            )
            .to_owned(),
            "save_init" => between(
                &original,
                "void GameBoot_SavegameInitialize",
                "void GameBoot_WorldInit",
            )
            .to_owned(),
            "reset_player" => between(
                &original,
                "void Game_SavegameResetPlayer",
                "void Game_PlayerInfoInit",
            )
            .to_owned(),
            "joy" => between(&original, "void Joy_Init", "bool func_8003483C").to_owned(),
            "text" => format!(
                "{}\nDVECTOR g_StringPosition;\ns32 g_StringPositionX1;\n{}",
                between(
                    &original,
                    "static const u8 FONT_12X16_GLYPH_WIDTHS",
                    "// ========================================\n// GLOBAL VARIABLES"
                ),
                between(
                    &original,
                    "void Gfx_StringPositionSet",
                    "s32 Gfx_MapMsg_WidthsCompute"
                )
            ),
            "stream" => between(
                &original,
                "void GameState_MovieIntroFadeIn_Update",
                "s32 max_frame =",
            )
            .to_owned(),
            "settings" => between(
                &original,
                "void Settings_ScreenAndVolUpdate",
                "const s32 __pad_rodata",
            )
            .to_owned(),
            "fs_screens" => between(
                &original,
                "void GameFs_TitleGfxSeek",
                "void GameFs_LoadingTextDraw",
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
        let selected = if name == "text" {
            format!(
                "{selected}\nstatic struct {{s32 x;}} g_MapMsg_GlyphSprite;\n{}",
                between(
                    &original,
                    "void Gfx_StringDrawInt",
                    "const s32 unused_Rodata_80025E88"
                )
            )
        } else {
            selected
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
        if name == "stream" {
            // PORT: STREAM's PS1 image symbol aliases the native image descriptor.
            native = native.replace(
                "Screen_BackgroundImgDraw(g_MemCardWarningImg)",
                "Screen_BackgroundImgDraw(&g_MemCardWarningImg)",
            );
            native = native
                .replace("static s32  g_Debug_MoviePlayerIdx = 0;", "")
                .replace("static s32 g_Debug_MoviePlayerIdx = 0;", "");
            native = format!(
                "static s32 g_Debug_MoviePlayerIdx;\nvoid sh_stream_reset(void) {{ g_Debug_MoviePlayerIdx=0; }}\n{native}"
            );
        }
        if name == "joy" {
            native = native
                .replace(
                    "PadInitDirect(&g_GameWork.rawController, g_Controller1)",
                    "PadInitDirect((u8*)&g_GameWork.rawController, (u8*)g_Controller1)",
                )
                .replace(
                    "cont->buttonFlags.held << 20",
                    "(u32)cont->buttonFlags.held << 20",
                )
                .replace(
                    "cont->buttonFlags.held << 8",
                    "(u32)cont->buttonFlags.held << 8",
                )
                .replace(
                    "    cont = &g_GameWork.controllers[0];",
                    "    port_pad_refresh();\n    cont = &g_GameWork.controllers[0];",
                )
                .replace(
                    "cont->rawSticks.rawData_0 = signedRawAnalog",
                    "cont->rawSticks.rawData_0 = (u32)signedRawAnalog",
                )
                .replace(
                    "normalizedAnalogData <<= 8",
                    "normalizedAnalogData = (s32)((u32)normalizedAnalogData << 8)",
                )
                .replace(
                    "xorShiftedRawAnalog  <<= 8",
                    "xorShiftedRawAnalog = (s32)((u32)xorShiftedRawAnalog << 8)",
                )
                .replace(
                    "heldButFlags         |= 1 <<",
                    "heldButFlags         |= (s32)(1u <<",
                )
                .replace(
                    "(negDirBitIdx - (axisIdx * 2));",
                    "(negDirBitIdx - (axisIdx * 2)));",
                )
                .replace(
                    "(posDirBitIdx - ((axisIdx >> 1) * 4));",
                    "(posDirBitIdx - ((axisIdx >> 1) * 4)));",
                );
        }
        if name == "save_init" {
            native = native.replace(
                "bzero(g_SavegamePtr, sizeof(s_Savegame))",
                "memset(g_SavegamePtr, 0, sizeof(s_Savegame))",
            );
            // PORT: Avoid forming a pointer before the array on the final iteration.
            native = native
                .replace("    s32* mapEnemyStatesPtr;", "")
                .replace("    mapEnemyStatesPtr = g_SavegamePtr->mapEnemyStates;", "")
                .replace(
                    "        mapEnemyStatesPtr[44] = NO_VALUE;\n        mapEnemyStatesPtr--;",
                    "        g_SavegamePtr->mapEnemyStates[44-i] = NO_VALUE;",
                );
        }
        if name == "reset_player" {
            native = native.replace(
                "g_SavegamePtr->items[i].id    = NO_VALUE",
                "g_SavegamePtr->items[i].id    = (u8)NO_VALUE",
            );
        }
        if name == "title" {
            // PORT: Replace fixed-address title fog and scratch writes with native
            // storage, retaining the original neighbour padding and update order.
            native = native
                .replace("s8* s0 = 0x801E2432", "s8* s0 = port_title_fog + 1")
                .replace("*(s32*)0x1F800000", "*(s32*)port_scratch")
                .replace("*(s32*)0x1F800004", "*(s32*)(port_scratch+4)")
                .replace("(RECT*)0x1F800000", "(RECT*)port_scratch")
                .replace(
                    "MainMenu_FogPacketGet(GsOT* ot",
                    "MainMenu_FogPacketGet(u32* ot",
                )
                .replace("poly = packet", "poly = (POLY_G4*)packet")
                .replace("ptr1  = ptr + 441", "ptr1  = (u8*)ptr + 441")
                .replace("ptr = &D_800BCDE0[", "ptr = (u8*)&D_800BCDE0[")
                .replace("*ptr2-- = val", "*ptr2-- = (s8)val")
                .replace("ptr1[idx] = NO_VALUE", "ptr1[idx] = (u8)NO_VALUE")
                .replace("ptr[j] = val", "ptr[j] = (u8)val");
            native = native.replace("    e_GameState prevState;", "").replace(
                "    #define COLUMN_POS_Y                    204",
                "    #undef COLUMN_POS_Y\n    #define COLUMN_POS_Y                    204",
            );
            native = format!(
                "static void MainMenu_MainTextDraw(void);\nstatic void MainMenu_DifficultyTextDraw(s32 selected);\nstatic void MainMenu_BackgroundDraw(void);\nstatic void func_8003BCF4(void);\nvoid func_8003B560(void);\nstatic s8 port_title_fog[464];\n{native}\nint sh_title_menu_state(void) {{ return g_MainMenuState; }}\n"
            );
        }
        if name == "text" {
            native = native
                .replace("Gfx_StringDraw(char* str", "Gfx_StringDraw(const char* str")
                .replace(
                    "g_StringPosition.vx = x -",
                    "g_StringPosition.vx = (s16)(x -",
                )
                .replace(
                    "g_StringPosition.vy = y -",
                    "g_StringPosition.vy = (s16)(y -",
                )
                .replace("OFFSET_FROM_CENTER_X;", "OFFSET_FROM_CENTER_X);")
                .replace("OFFSET_FROM_CENTER_Y;", "OFFSET_FROM_CENTER_Y);")
                .replace("ot         = &g_OtTags0", "ot         = (GsOT*)&g_OtTags0")
                .replace("strCpy  = str", "strCpy  = (u8*)str")
                .replace("(posY << 16)", "((u32)(u16)posY << 16)")
                .replace(
                    "*((u16*)&glyphPoly->u2) = u0 - 0xFF4",
                    "*((u16*)&glyphPoly->u2) = (u16)(u0 - 0xFF4)",
                )
                .replace(
                    "*((u16*)&glyphPoly->u3) = u0 - 0xF4",
                    "*((u16*)&glyphPoly->u3) = (u16)(u0 - 0xF4)",
                )
                .replace(
                    "g_StringColorId = charCode",
                    "g_StringColorId = (s16)charCode",
                )
                // PORT: MIPS masks a variable shift by 32 to zero. C's shift
                // is undefined; the observed quotient is the division result.
                .replace(
                    "(val / ATLAS_COLUMN_COUNT) >> 32",
                    "val / ATLAS_COLUMN_COUNT",
                )
                .replace(
                    "*str     = (val - (quotient * ATLAS_COLUMN_COUNT)) + '0'",
                    "*str     = (char)((val - (quotient * ATLAS_COLUMN_COUNT)) + '0')",
                )
                .replace("*str = val + '0'", "*str = (char)(val + '0')");
        }
        if name == "text_debug" {
            native = native
                .replace(
                    "Text_Debug_Draw(char* str)",
                    "Text_Debug_Draw(const char* str)",
                )
                .replace("strCpy = str", "strCpy = (u8*)str")
                .replace("= x - OFFSET_X", "= (s16)(x - OFFSET_X)")
                .replace("= y - OFFSET_Y", "= (s16)(y - OFFSET_Y)");
            native = format!("#include <ctype.h>\n{native}");
        }
        if name == "brightness" {
            native = native
                .replace(
                    "line->x1 = ((g_GameWork.gsScreenWidth - 64) / 20) * i",
                    "line->x1 = (s16)(((g_GameWork.gsScreenWidth - 64) / 20) * i)",
                )
                .replace(
                    "line->y1 = (g_GameWork.gsScreenHeight / 2) - 45",
                    "line->y1 = (s16)((g_GameWork.gsScreenHeight / 2) - 45)",
                )
                .replace(
                    "color    = (brightness * 8) + 4",
                    "color    = (u8)((brightness * 8) + 4)",
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
        "port/title_services.c",
        "port/gte_native.h",
        "port/gte_services.c",
        "port/gameplay.h",
        "port/gameplay.c",
        "port/map.h",
        "port/map.c",
        "port/camera_services.c",
        "port/world_services.c",
    ] {
        println!("cargo:rerun-if-changed={}", repo.join(file).display());
    }
    build
        .file(repo.join("port/runtime.c"))
        .file(repo.join("port/layout_check.c"))
        .file(repo.join("port/title_services.c"))
        .file(repo.join("port/gte_services.c"))
        .file(generated.join("gte_command_probe.c"))
        .file(generated.join("gameplay_consumers.c"))
        .file(generated.join("native_math.c"))
        .file(generated.join("vc_main.c"))
        .file(generated.join("vc_util.c"))
        .file(generated.join("vw_main.c"))
        .file(generated.join("vw_calc.c"))
        .file(generated.join("camera_globals.c"))
        .file(repo.join("port/camera_services.c"))
        .file(generated.join("world_consumers.c"))
        .file(generated.join("collision_consumers.c"))
        .file(repo.join("port/world_services.c"))
        .file(repo.join("port/gameplay.c"))
        .file(repo.join("port/map.c"))
        .file(generated.join("map0_s00.c"))
        .file(generated.join("player_spawn.c"))
        .file(generated.join("map_info.c"))
        .file(generated.join("sine.c"))
        .file(generated.join("option.c"))
        .define("SH_CHECK_BOOT_LAYOUT", None)
        .compile("sh_native_boot");
}
