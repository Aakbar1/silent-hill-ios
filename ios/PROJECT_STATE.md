# ios2 checkpoint — 7 October 2026

Goal: ship the real host with a first-launch US v1.1 disc importer, unsigned device
IPA, and a no-disc simulator screenshot check. Branch: `lane/ios2`. Owned: `ios/`,
`.github/workflows/ios.yml`, `host/src/platform_ios.rs`, minimal host registration,
and Apple-only build glue. No core3 source, root manifests, main, merges or pushes.

## Decisions and implementation

- `ios/app` is a thin, independently locked entry point depending on the **actual**
  host library. This avoids unrelated nested root workspaces without source copies
  or manifest edits outside scope. Rejected a temporary copied host workspace once
  the direct dependency build passed. `ios/shell` remains a diagnostic fallback
  target, excluded from the default IPA/simulator pipeline.
- UIKit scene lifecycle owns the only app loop. Document picker/Open In URLs and
  Files/USB Documents copies enter a serial, coordinated read-only import.
  External security-scoped URLs stay held until import completion.
- psxdisc verifies US v1.1, copies into Application Support staging, syncs, then
  reopens/verifies the copy before atomic activation. Crash staging is discarded
  on retry. Existing imports are reverified at launch; damaged copies are preserved
  as Documents/*.invalid before another import is offered. The user's source stays.
- Actual copied bytes drive progress. Saves/logs are in Foundation's Documents URL;
  log rotation at 2 MiB. AVAudioSession playback category; no background audio.
- Apple-only build glue appends `ios/host_bridge.rs` to the current native worker
  in OUT_DIR and inserts a guarded iOS pacing call at its existing clock seam.
  Every tick, including blank display frames, uses the original 60Hz host rate.
  Background pause resets the clock on resume. Source CRLF is normalized first.
  Game callbacks/assets/save payloads stay shared; UIKit's latest-frame mailbox
  bounds pending presentation memory. No game bytes or screenshots enter the repo.
- CI: submodule checkout, explicit Apple clang/ar/SDK, both real C archives with
  warnings as errors, both locked Apple targets, host/app Clippy, resource tools,
  unsigned device IPA, local ad-hoc simulator signature. A fresh simulator prevents
  retained disc content; screenshot pixels must contain importer text via Vision.

## Verified locally on Windows

- `git submodule update --init`: pass; pinned d9e28f8315c7938117224f21516786d9d149a145.
- Actual host library + thin app Clippy for **both** aarch64-apple-ios and
  aarch64-apple-ios-sim: 4 checks pass / 0 Clippy diagnostics. Explicit
  SH_IOS_RUST_CHECK_ONLY=1; C/UIKit compilation and Apple linking excluded.
- App cargo fmt + rustfmt checks for owned host/bridge/importer files: pass.
- Actual host `clippy --all-targets --locked -- -D warnings`: pass on MSVC.
- Actual host `test --lib --locked`: 24 passed / 0 failed / 1 private test ignored.
- Local-only private importer test explicitly run: 1 passed / 0 failed. All
  616,494,480 copied bytes match and the copy reopens as Us11. Source SHA-256 before
  and after is CF975C962D8ECD7B57ED43127FB5CE41A693C713AE4917C14E79DABDE5250794.
  Test outputs were created only in private/work/ios2 and removed. No game run.
- Thin app Windows build links the real host C: pass, warnings treated as errors.
- Packaging fixtures/dry run: pass; pipeline Python tests: 4 passed / 0 failed.
  actionlint 1.7.12: 2 workflows, 0 diagnostics. Python compileall and diff check pass.

## Open integration and required checks

1. After integrating core3, reconcile bridge Host fields/pacing seam and regenerate
   ios/app/Cargo.lock if host dependencies changed. Prefer a core-owned public
   worker/backends/save-root seam to eventually remove generated module extension.
2. iOS currently preserves the base host's Raster/SilentSpu/keyboard PadSource.
   Director must connect GPU/audio/touch lane backends and UIKit touch delivery.
   This is a real native host entry; full-game/audio/touch playability is unproven.
3. First macOS CI run must verify decomp C + app.m clang compilation without
   warnings, both Apple links, actool/ibtool, device signature removal/IPA layout,
   simulator signature/install/launch, landscape importer screenshot and OCR.
   Inspect simulator.png, game.log, simulator-system.log and importer-ocr.txt.
   Apple C, UIKit, Swift Vision, signing and simulator execution were NOT run here.
4. Device tests still need picker/iCloud, USB/Files auto import, wrong releases,
   interrupted copies, source preservation, relaunch, saves/log sharing, landscape,
   safe areas, background/resume and integrated sound/touch gameplay.
5. Whole-root Cargo gates remain blocked by inherited nested psxgpu/psxspu
   workspaces; director/owning lanes must fix them. The iOS app path builds directly
   without this blocker. See README.md for reproduction and INSTALL_FOR_FRIEND.md.
