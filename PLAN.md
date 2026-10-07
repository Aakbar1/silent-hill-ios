# Silent Hill iOS: native port plan

Goal: Silent Hill (PS1, US v1.1) running **natively** on an iPhone 17 Pro Max, sharp and smooth,
with the **whole game playable by native touch**. No emulator and no on-screen gamepad overlay.

## Fixed facts
- Game: `Silent Hill (USA).bin`, raw MODE2/2352, single track, no .cue.
  Disc SHA-1 `34278d31d9b9b12b3b5db5e45bcbe548991ecbc7`. Boot exe `SLUS_007.07`,
  SHA-256 `e73859cc…0f202398`, which **matches the decomp's USA target exactly**.
  Disc files: `SYSTEM.CNF`, `SLUS_007.07`, `SILENT.` (80 MB), `HILL.` (456 MB).
- Local copies: `../private/disc/` and `../private/SLUS_007.07`. This is **outside the repo** and must never be committed.
- Base code: `../reference/silent-hill-decomp` (GPL-3.0, read-only; commit d9e28f8, 2026-10-06).
  In the USA source only about 11 `INCLUDE_ASM` functions remain, plus 8 `NON_MATCHING` and 95 `INCLUDE_RODATA`.
  The decomp says it will never be a port. Making it one is our job.
- Our code is GPL-3.0 because it builds on the decomp.
- Dev PC: Windows 11, Rust stable, MSVC (VS 18), Python 3.14, git. Not installed: clang, cmake, gcc, Docker, a WSL distro, gh.
- No Mac and no paid Apple account. iOS builds come out **unsigned** from GitHub Actions macOS runners.
  The friend (Northern Ireland, remote) signs them with a free Apple ID through **SideStore**: 7-day re-sign done on the phone, max 3 apps.
- The friend owns the game. **The app ships no game data.** On first launch it imports the player's
  own disc image (via Files) and extracts what it needs on the device.

## Target experience
- "Faithful but sharp": original art, fog and camera. Native resolution, 60 fps (original logic
  runs at its own tick rate, with rendering interpolated where safe), optional 16:9 that keeps fog distance correct.
- Loads feel instant. Saves are native files, with save-anywhere-at-save-points as in the original.
- Touch controls (design to be finalized in Phase 3):
  - Left half: floating stick to walk/run (push distance = speed). Quick swipe down = 180° turn.
  - Right half: tap = interact/examine; hold = aim (auto-lock nearest enemy, as the original does); tap while aiming = fire/swing.
  - Context chips appear only when relevant: flashlight, radio, map, inventory, pause.
  - Menus, inventory, maps and puzzles (piano, codes, etc.) become direct taps on the item.
  - Every PS1 pad input must have a touch path. A checklist covers every puzzle, boss and menu.

## Architecture (proposed; Phase 1 confirms or rejects)
- **Game logic**: the decomp's C, compiled natively for x86-64 and arm64.
- **Host shell in Rust** (per the video's advice and our existing toolchain): window/touch via winit,
  rendering via wgpu (D3D12 on Windows, Metal on iPhone), audio via cpal, C built through the `cc` crate.
- **PsyQ replacement layer**: our own implementations of the libraries the game calls (libgpu/libgs
  → GPU command list into our renderer; libgte → native math; libspu → software mixer;
  libcd → reads from the imported disc; libpad → the input layer that touch feeds). The game code stays unchanged where possible.
- Main known risks: 32-bit pointer and fixed-address assumptions on a 64-bit CPU, overlays loaded
  at fixed addresses, timing tied to PS1 vsync, MDEC video (FMVs), and XA audio streams.

## Phases (Abdullah gives the go before each)
1. **Go/no-go on Windows** (now): survey the code (lane `survey`) and boot natively to the Konami logo (lane `boot`).
2. **Full game on Windows with a controller/keyboard**: renderer, audio, FMV, saves; a scripted playthrough of every area.
3. **Touch controls**: built and tested on Windows (mouse or touchscreen as touch), with the input checklist completed.
4. **iOS build**: arm64 target, disc importer, unsigned IPA from GitHub Actions, automated runs on the iOS simulator with screenshots.
5. **Friend play-tests via SideStore**: screen recordings plus the auto log come back, and we fix what they find.

## Testing without an iPhone
Same code on Windows and iOS. Each phase adds headless replay tests (recorded inputs → expected
frame hashes or screenshots). GitHub's macOS runner builds and runs the simulator on every push.
The friend is the last check, not the first.
