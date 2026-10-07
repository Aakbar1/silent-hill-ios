# Native Windows boot spike

The release build runs the decomp's C boot path to a recognisable Konami logo. The measured final run executed **600 native VBlank ticks in 9.997 seconds**, entered B_KONAMI at tick 95, and exited with code 0. The window presented 598 times (redraw events can coalesce), including the final frame. The screenshot is 640×448 and was visually inspected. Its grey background is the original fade-out at tick 600.

This establishes the native C boot path. It does not establish whole-game portability, complete PsyQ compatibility, iOS readiness, or 3D performance. The renderer currently rasterizes boot GPU packets in Rust and presents them through winit/softbuffer; wgpu is not implemented.

## Reproduce

From the `boot` worktree:

```powershell
.\tools\dev-cargo.cmd run --release -- --frames 600 --screenshot "C:\Claude Projects\Silent Hill iOS\private\work\boot\konami.png"
.\tools\dev-cargo.cmd fmt -- --check
.\tools\dev-cargo.cmd clippy --workspace -- -D warnings
.\tools\dev-cargo.cmd test --workspace
.\tools\dev-cargo.cmd run --release -- --inspect-disc
```

The wrapper finds **all Visual Studio products**, including Build Tools, and enters the x64 developer environment. The verified compiler is `C:\BuildTools2022\VC\Tools\MSVC\14.44.35207\bin\HostX64\x64\cl.exe`, called by `cc` 1.6.0. Rust is stable 1.99.0, target `x86_64-pc-windows-msvc`. No system tools were installed. Looking only under the Visual Studio 18 Community installation missed the separately installed Build Tools; the successful C/Rust linkage test corrected that initial diagnosis.

The default disc resolves to `../../private/disc/Silent Hill (USA).bin` from the worktree root. `--disc PATH` overrides it. The raw reader checks 2352-byte sector framing, MODE2 duplicated subheaders, Form 1 payloads, ISO block/record bounds, and little/big endian pairs. It reads sectors directly, with no extraction step or emulator. EDC/ECC validation and Form 2/XA decoding are not implemented. ISO field definitions follow [ECMA-119](https://www.ecma-international.org/wp-content/uploads/ECMA-119_4th_edition_june_2019.pdf); CD framing is documented in [ECMA-130](https://ecma-international.org/publications-and-standards/standards/ecma-130/).

Screenshots are restricted to existing parent directories under `private/work/boot/`, including a check of an existing target's resolved path. No game data or screenshots are in this repository. Closing the window before N ticks returns an error after the C worker exits. Runs extending past the Konami state stop at the first unimplemented state guard; this is a boot spike, not a title-screen implementation.

## What actually executes

`host/build.rs` reads the external GPL-3.0 decomp at commit `d9e28f8315c7938117224f21516786d9d149a145`, generates adapted C under `private/work/boot/native-source/`, and compiles it with MSVC `/std:c11 /W4 /WX`. The reference checkout is unchanged. `SH_DECOMP_DIR` may point to another checkout of that same revision.

- `src/main/main.c`: real entry sequence, warning-image fade, BODYPROG/B_KONAMI reads and decryption, font load, call to `MainLoop`.
- `src/bodyprog/sys/game_main.c`: real BODYPROG `MainLoop` and `GameState_Init_Update`.
- `src/screens/b_konami/b_konami.c`: real `GameState_KonamiLogo_Update`, `BootScreen_ImageSegmentDraw` and `BootScreen_KonamiScreenDraw`.
- `src/bodyprog/screen/screen_fade.c`, `screen_draw.c`, `src/bodyprog/sys/vsync.c`: real fade, screen setup and VBlank counter callback.
- `src/bodyprog/screen/background_draw.c`: real warning-background sprite emitter.

The native C functions are statically linked. The original overlay files are still read and decrypted into separate data storage, but their MIPS instructions are never executed. Boot-only runtime declarations do not reinterpret decrypted overlay structures as native objects. The file table and file enum remain external source includes. Header narrowing and all source transforms are listed in [PORT_FIXES.md](PORT_FIXES.md).

The C shims implement 24-bit OT tokens backed by a native pointer registry. Actual emitted packets reach the Rust renderer through `DrawPrim`/`GsDrawOt`. Supported boot primitives are SPRT, TILE, DR_TPAGE and DR_MODE, with 4/8/16-bit texture reads, CLUT lookup, modulation, transparency, clipping, offsets and blend modes. Unknown commands log once per opcode. GTE, polygon rendering, dithering accuracy, texture-window effects and full GPU fidelity remain unverified or unsupported.

## Results and evidence

All evidence is under `C:\Claude Projects\Silent Hill iOS\private\work\boot\`:

- `konami.png`: frame 600, visibly recognisable red Konami mark and black text; final frame was presented in the native window.
- `boot-600.log`: successful release build/run, state transition at VBlank 94, first Konami tick 95, 3,413 rectangle/sprite primitives, final state 1/step 3, 600 ticks, 9.997 seconds; full command including build/environment startup took 14.728 seconds.
- `fmt.log`, `clippy.log`, `test.log`: formatting/lint success; eight tests pass (five raw-disc/ISO, two rasterizer, one C/Rust x64 linkage).
- `decomp-compile.log`: the deliberately separate baseline probe retains the original headers/assertions and fails at 43 structure-size assertions, producing 86 errors and 29 warnings. It is not the runnable boot configuration.
- `beyond-boot.log`: requested 800 ticks; clean nonzero exit after 731 ticks at `GameState_KcetLogo_Update` (state transition at VBlank 729), confirming the next unimplemented boundary.
- The live disc SHA-1 matches PLAN.md: `34278d31d9b9b12b3b5db5e45bcbe548991ecbc7`; the release host's PE machine is `0x8664` (AMD64). An outside-private screenshot request was rejected and created no file.

The baseline remains reproducible with `tools\dev-cargo.cmd build -p native-toolchain-probe --features decomp`; expected exit 101. No valid PS1 file-layout checks were disabled to make that gate pass. The boot configuration uses separate native runtime declarations and preserves the packet/image wire-size checks it consumes.

## Beyond the logo

1. **Native versus wire layouts:** the original headers fail 43 x64 size assertions. `s_ModelAnim`, `s_FsAnmDesc`, model/material/map structs and pointer-bearing engine records need explicit wire views plus relocation/native storage. macOS arm64 also needs 32-bit replacements for PsyQ `long` fields.
2. **Overlay linkage and data:** KCET/title/stream/map code needs native namespaces and entry tables, original global initializers, data relocation, and missing ASM/RODATA definitions. The first state after Konami is currently an explicit stop guard.
3. **3D/GTE/rendering:** native fixed-point GTE operations, polygon/mesh emitters, ordering, fog and full GPU semantics need validation. A wgpu backend is still required to assess the proposed performance target.
4. **Media:** MDEC FMVs, XA streaming, VAB/SPU mixing and audio scheduling are missing. The title path references movie states and cannot proceed faithfully with the current media stubs.
5. **Interactive systems/timing:** controller input, memory cards/native saves, demos, warm reset, CD retry/stream scheduling and full timing behavior are stubbed or simplified. They need gameplay and transition tests, not just another logo run.

Whole-game native execution looks technically plausible, with moderate confidence that the architecture can be made to work. It requires substantial ABI, relocation, rendering and media work. This is a **go for the native boot approach**, conditional evidence for a whole-game port; it supplies no measured estimate of gameplay performance or iOS completion time.
