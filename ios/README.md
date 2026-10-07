# iPhone game host and disc importer

The thin `ios/app` binary depends on the real `host` crate (`silent-hill-boot`),
including its pinned GPL decomp C with Apple's clang. UIKit owns the application loop and the importer;
the existing native worker owns the game. The app bundles no game data.

`ios/shell` is retained as a diagnostic fallback build target. It is no longer
packaged or launched by the default iOS workflow. Its original test-pattern code
and Windows smoke script are unchanged. Build it explicitly with
`cargo build -p silenthill-shell`; the shared bundle metadata now describes the host.

## First launch and storage

The original screen says **Import your Silent Hill (USA) disc image (.bin)**.
**Choose .bin in Files** opens `UIDocumentPickerViewController`. External files
are read under security-scoped access and `NSFileCoordinator` on a serial background
queue, including iCloud downloads. Files Open In URLs enter the same importer.
Regular `.bin` files copied into Documents are detected after three stable polls;
partial copies are rejected, retried when their size/mtime changes, or can be
selected manually. The source file is always read-only and is never deleted.

`psxdisc::import(KeepImage)` checks US v1.1, copies into disposable staging storage,
flushes/syncs, and `GameDisc::open` verifies the **copy** before an atomic rename
makes it active. Wrong releases have a friendly message. Failed/crashed staging
is cleaned on the next attempt. The stored copy is reverified on each launch.
A damaged saved copy is preserved as `Documents/RejectedImport-*.invalid`, and
another import is offered. Progress shows actual copied MiB and a progress bar;
verification uses a spinner. Keep at least one disc image's size of free space,
in addition to any source copy already stored on the phone.

- Imported image: `Library/Application Support/SilentHillPort/disc.bin`.
- Native save slots: `Documents/saves/slot-*.shs` (original 636-byte records).
- Diagnostics: `Documents/game.log`, rotated to `game.previous.log` above 2 MiB.

Files exposes Documents through `UIFileSharingEnabled` and
`LSSupportsOpeningDocumentsInPlace`. The actual Documents URL comes from Foundation.
The playback audio session is configured without background-audio entitlement.
Apple's [playback category](https://developer.apple.com/documentation/avfaudio/avaudiosession/category-swift.struct/playback)
continues audio with Ring/Silent set to silent; this is the requested category's
behaviour. The current host still uses its existing silent SPU fallback: session
configuration alone does not implement game sound.

## Build and CI

`.github/workflows/ios.yml` checks out the decomp submodule and builds device and
simulator from `ios/app/Cargo.toml` with `--locked`, iOS 15 minimum, explicit Apple
clang/SDK/ar, and warnings as errors. `package.py` compiles icons/storyboard using
Xcode, strips the device executable's ad-hoc signature, and uploads the unsigned
`SilentHillPort-unsigned.ipa`. Simulator signing is local ad-hoc only. No Apple
account, provisioning profile, game image or extracted asset is used in CI.

The smoke test creates a **new** simulator (never reuses or clones an existing
container), installs the real host with no disc, waits for the importer, captures
`simulator.png`, checks landscape view dimensions, and uses macOS Vision OCR to
require the importer title and choose button in the screenshot pixels. It uploads
`game.log`, `simulator-system.log`, and `importer-ocr.txt`, then deletes only that
new simulator. No game screenshots are captured or uploaded by this workflow.

Local data-free checks:

```powershell
python ios/scripts/package.py --dry-run
python ios/scripts/test_pipeline.py
python ios/scripts/check_workflows.py
python -m compileall -q ios/scripts
```

`ios/app` has its own committed lockfile and builds the actual host source as a
path dependency. This avoids the inherited root workspace discovery failure
(psxgpu/psxspu still have nested `[workspace]` tables despite `crates/*`). It does
not copy or change host sources/dependency declarations. The director still needs
to fix root membership for whole-workspace Windows checks; iOS CI can proceed
independently with the actual host dependency.

Windows verification commands (existing MSVC and installed Rust iOS targets):

```powershell
$env:CARGO_TARGET_DIR = Join-Path (Get-Location) 'ios/.tools/host-target'
cargo fmt --manifest-path ios/app/Cargo.toml -- --check
rustfmt --edition 2024 --check host/build.rs host/src/lib.rs host/src/platform_ios.rs ios/importer.rs ios/host_bridge.rs
./tools/dev-cargo.cmd clippy --manifest-path ios/app/Cargo.toml -p silent-hill-boot --all-targets --locked -- -D warnings
./tools/dev-cargo.cmd test --manifest-path ios/app/Cargo.toml -p silent-hill-boot --lib --locked
$env:SH_IOS_RUST_CHECK_ONLY = '1'
cargo clippy --manifest-path ios/app/Cargo.toml -p silent-hill-boot --lib --locked --target aarch64-apple-ios -- -D warnings
cargo clippy --manifest-path ios/app/Cargo.toml --locked --target aarch64-apple-ios -- -D warnings
cargo clippy --manifest-path ios/app/Cargo.toml -p silent-hill-boot --lib --locked --target aarch64-apple-ios-sim -- -D warnings
cargo clippy --manifest-path ios/app/Cargo.toml --locked --target aarch64-apple-ios-sim -- -D warnings
Remove-Item Env:SH_IOS_RUST_CHECK_ONLY
```

`SH_IOS_RUST_CHECK_ONLY=1` bypasses C/UIKit compilation for Windows Rust checking;
it emits an explicit warning and is forbidden on macOS or in real packaging.
These checks are **not Apple C builds or iOS linking tests**. CI omits this flag
and compiles both C archives with Apple's clang, with warnings as errors.

The default host tests use synthetic importer fixtures. The ignored
`private_disc_import_is_byte_exact_and_reopens_as_us11` test is local-only: it
requires `SH_IOS_TEST_DISC`, writes exclusively under `private/work/ios2`, compares
every copied byte, reopens through psxdisc, and removes its private test output.
Never run it in CI or upload the player's disc.

## Integration handoff

See [PROJECT_STATE.md](PROJECT_STATE.md) and the ignored root REPORT.md for measured
results and requests. A small Apple-only generated module appends
`ios/host_bridge.rs` to current `host/src/native.rs`, preserving the existing C
callbacks without editing the parallel core lane. Its Host initializer must stay
in sync when core3 changes the worker fields; replace it with a public worker seam
when the director integrates that API. UIKit presents real raster/movie frames,
with bounded pending frame storage and background pause. Native GPU/audio/touch
backend integration belongs with the parallel lanes; current boot logic, silent
SPU and keyboard PadSource are preserved. This does not establish full-game,
audio, or touch playability.

After integration, CI must prove both complete Apple C builds and links, resource
compilation, unsigned device IPA inspection, simulator signing/launch, landscape
importer screenshot and OCR. Then the friend's phone must verify picker/iCloud,
USB/Files import, wrong release, interrupted copy, relaunch, saves/log sharing,
background/resume and the integrated gameplay/audio/touch backends. Neither a
Windows type-check nor the no-disc simulator smoke proves those device checks.

Installation and PC-to-Files transfer: [INSTALL_FOR_FRIEND.md](INSTALL_FOR_FRIEND.md).
API references: [document picker/security scope](https://developer.apple.com/documentation/uikit/uidocumentpickerviewcontroller),
[Apple Devices file sharing](https://support.apple.com/en-gb/120402),
[iTunes file sharing](https://support.apple.com/en-gb/120403).
