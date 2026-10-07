# iOS lane checkpoint — 7 October 2026

Goal: standalone Windows/iPhone Rust test shell and unsigned GitHub macOS build
pipeline, with a friend installation guide. Owned: `ios/`, `.github/workflows/`;
root `REPORT.md` is the explicitly requested ignored handoff. Branch: `lane/ios`.

## Decisions and completed work

- winit 0.30.13 / wgpu 24.0.5; standalone Cargo workspace + locked dependencies.
- Metal on iOS; D3D12 on Windows. Test pattern, live touch circles, fullscreen,
  both landscapes, UIKit safe areas, deferred edges and home-indicator preference.
- Documents log, previous-session rotation, panic and GPU diagnostics. No game bytes.
- Plain stdlib Python packaging using installed Xcode actool/ibtool; rejected an
  additional bundler/Xcode-project generator because signing/resource steps can
  be expressed directly and checked locally. Device signature removed; simulator
  alone receives a local ad-hoc signature. No account/certificate/profile in CI.
- One cached Apple Silicon macOS job builds device + simulator, uploads device IPA
  before simulator smoke, then captures screenshot/logs. Every push triggers both
  workflows; newer pushes cancel superseded jobs. Windows job includes GPU smoke.
- Original generated geometric icon, launch storyboard, Files keys and bin/cue UTIs.
- Live SideStore English documentation checked 7 October 2026: iloader + LocalDevVPN.
  Historical WireGuard/StosVPN instructions were excluded from the main guide.

## Verified locally

- `cargo fmt --check`, `cargo clippy --locked -- -D warnings`, `cargo build --locked`: pass.
- `cargo test --locked`: 1 passed / 0 failed (circle count/pixel mapping).
- Windows GPU smoke: 8 presented D3D12 frames, first-frame + completion log, exit 0.
- `cargo clippy --locked --target aarch64-apple-ios -- -D warnings`: pass, type-check only.
- `cargo check --locked --target aarch64-apple-ios-sim`: pass, type-check only.
- `python ios/scripts/test_pipeline.py`: 4 passed / 0 failed.
- `package.py --dry-run`: pass (XML/plist/icons/fixture ZIP; not a real iOS build).
- `check_workflows.py`: actionlint 1.7.12, 2 workflows passed / 0 diagnostics;
  checksum verified portable download. Optional shellcheck/pyflakes disabled.
- `python -m compileall -q ios/scripts`: pass. `git diff --check`: pass.
  Reproduction commands and artifact instructions: [README.md](README.md).

## Open problems and next actions

1. Director creates remote/integrates/pushes as specified in README. Worker does not push.
2. First macOS run must verify Apple linking, actool/ibtool, signature removal,
   simulator Metal + landscape PNG; then inspect both uploaded artifacts.
3. Friend follows [INSTALL_FOR_FRIEND.md](INSTALL_FOR_FRIEND.md) and checks actual
   device signing, multitouch, safe areas in both landscapes, indicator/edge gestures,
   background/resume, Files copy and log sharing. None is claimed device-tested.
4. Disc parsing/extraction, Open In callbacks and game integration are later work.
   Only storing the player's files in Documents is implemented here.
5. Apple-target checks emit a dependency future-compatibility notice for `block 0.1.6`.
   Stable 1.99.0 passes today; watch it when updating Rust/wgpu. Local Windows GPU
   also reports reduced WebGPU conformance; this simple pipeline renders successfully.
6. Root GPL-3.0 licence text is absent in this checkout; director owns adding it
   before distributing source/binaries. No read-only decomp code was copied.
