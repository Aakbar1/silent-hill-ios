# iPhone shell and unsigned CI builds

This lane is a standalone **test shell**, not a playable game. It ships no game data.
`ios/shell` uses winit 0.30 + wgpu 24, with a committed Cargo.lock. iPhone uses
Metal; Windows uses D3D12. It draws color bars, a checkerboard, a mint safe-area
border and orange circles for all active touches (left mouse button on Windows).
Escape exits on Windows. iPhone orientation is restricted to both landscapes in
the plist and the view controller. Insets are read from UIKit's `safeAreaInsets`
each frame and converted from points to physical pixels. Pattern content stays
inside them; touch feedback deliberately follows the actual contact even at an edge.

The iOS view controller requests home-indicator auto-hide and defers gestures at
all edges using winit's UIKit extensions. These are OS preferences, not a promise
that iOS will permanently remove the indicator or prevent leaving the app.
Logs append to `Documents/shell.log` in the iPhone sandbox. Windows defaults to
`%USERPROFILE%/Documents/SilentHillPort/shell.log`. On startup a previous log over
2 MiB rotates to `shell.previous.log`. Both are shareable from Files on iPhone.
Touch coordinates, filenames and Apple credentials are not logged by the shell.

## Check on this Windows PC

Run from the repository root in PowerShell (Rust stable + existing MSVC needed):

```powershell
cargo fmt --manifest-path ios/shell/Cargo.toml --check
cargo clippy --manifest-path ios/shell/Cargo.toml --locked -- -D warnings
cargo test --manifest-path ios/shell/Cargo.toml --locked
cargo build --manifest-path ios/shell/Cargo.toml --locked
python ios/scripts/package.py --dry-run
python ios/scripts/test_pipeline.py
python ios/scripts/check_workflows.py
powershell -NoProfile -File ios/scripts/windows-smoke.ps1
cargo run --manifest-path ios/shell/Cargo.toml --locked
```

The smoke run creates a native window, validates the shader through wgpu, presents
8 frames, checks the file log and exits within 60 seconds. It does not synthesize
touches or prove iPhone behaviour. `SHELL_DOCUMENTS` overrides the Windows log
directory; `SHELL_SMOKE_FRAMES` enables the bounded launch mode.

`check_workflows.py` fetches portable actionlint **1.7.12**, verifies its pinned
SHA-256, and stores it only in ignored `ios/.tools`. It checks both workflow YAML
files, action inputs, expressions and runner labels. Optional shellcheck/pyflakes
are disabled; Python scripts are covered by the dry run and unit tests.
The dry run parses bundle metadata and launch-screen XML, generates 9 original
RGB icons, and tests ZIP layout, executable permissions and signing-file exclusions
with disposable fixtures. **It does not compile Apple resources, create a real
IPA, emulate Xcode or verify a device signature.** No Mac is needed for these checks.

## Why a plain Python bundler

`scripts/package.py` uses Cargo, Python's standard library and Xcode's installed
`xcrun actool` / `ibtool`. No cargo-bundle, xcodegen, package manager or Xcode
project is required. It explicitly controls the SDK, deployment floor (iOS 15),
platform metadata, executable permissions and signing. The placeholder icon is
an original geometric ring/cross generated from `scripts/assets.py`; the launch
screen is `bundle/LaunchScreen.storyboard`. It contains no game art.

Device pipeline: build `aarch64-apple-ios --release --locked`, compile resources,
assemble `Payload/SilentHillPort.app`, remove any linker-generated ad-hoc signature,
confirm the executable is unsigned, then ZIP to `SilentHillPort-unsigned.ipa`.
There is no provisioning profile, signing identity, Apple ID or secret in CI.
SideStore signs the device IPA later using the player's free Apple Account.

Simulator pipeline: build `aarch64-apple-ios-sim --release --locked`, assemble a
separate iPhoneSimulator app, apply a **local ad-hoc** signature (needed on arm64;
no Apple account), select the newest already-installed available iPhone runtime,
boot, install and launch. It waits for a fresh `first frame presented` log entry,
takes a screenshot, verifies its landscape dimensions, collects logs and shuts
down the simulator. A missing runtime, failed launch or portrait screenshot fails
the job. It never downloads a simulator runtime to hide missing runner support.

One Apple Silicon `macos-15` job builds both targets, shares the Cargo cache,
uploads the device IPA before the simulator test, and cancels superseded pushes.
Both workflows run on **every push**, including lane branches, and support manual
dispatch. Windows checks formatting, Clippy, tests, build, packaging dry run,
pipeline tests, actionlint and the GPU smoke run. Neither workflow builds other lanes.
[GitHub runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
lists `macos-15` as arm64. Xcode/runtime contents can change; the job prints them.

## Director: once the GitHub repository exists

1. On GitHub click **+ → New repository**. Choose a name and public/private visibility.
   Leave README, .gitignore and licence initialization unchecked: this is an existing
   Git history. Do not upload the sibling `private/` or `reference/` trees.
2. Integrate commit(s) from `lane/ios` into the director's chosen integration branch.
   This worker commits locally only; it does not merge, create a remote or push.
3. From the director's integration checkout, replace `OWNER/REPO` below with the
   new repository and `INTEGRATION_BRANCH` with that checkout's branch:

   ```powershell
   git remote add origin https://github.com/OWNER/REPO.git
   git push -u origin INTEGRATION_BRANCH
   ```

   If `origin` already exists, check `git remote -v` and use the correct existing
   remote instead of blindly changing it. Authenticate with GitHub when prompted.
4. Open **Settings → Actions → General** and allow Actions plus the referenced
   checkout/upload-artifact, dtolnay/rust-toolchain and Swatinem/rust-cache actions.
   No Apple certificates, secrets or paid developer account are required.
5. Click **Actions → iOS unsigned IPA and simulator → latest run**. Under
   **Artifacts**, download **SilentHillPort-unsigned-ipa**. GitHub gives you a ZIP;
   extract it in Windows Explorer to get **SilentHillPort-unsigned.ipa**. Give the
   friend that `.ipa`, not the outer ZIP or a simulator `.app`.
6. Download **SilentHillPort-simulator-evidence** and inspect `simulator.png` plus
   `shell.log`. Check **Windows shell** is green too. The device IPA is uploaded
   independently before the simulator test, but a red simulator job still needs
   investigation. Artifacts expire after 14 days; rerun the job to replace them.
7. For another build, push a commit. Once this workflow is on the default branch,
   **Actions → workflow → Run workflow** also lets you select a branch manually.
   Share [INSTALL_FOR_FRIEND.md](INSTALL_FOR_FRIEND.md) with the friend.

Standard public runners are currently free; private repos consume the account's
Actions allowance and can incur charges. Set the account's Actions spending cap
before running a private repository if zero spend is required.
[GitHub runner costs](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).

## Handoff and limits

See [PROJECT_STATE.md](PROJECT_STATE.md) for measured local results. Until the
first GitHub macOS run, iOS linking, actool/ibtool, simulator Metal, the unsigned
IPA and SideStore installation remain **unverified**. Hardware checks still need
the friend's phone: multitouch, landscape both ways, safe-area alignment around
the notch, home-indicator fade, double-swipe exit, suspend/resume, Files import and
log sharing. Simulator smoke is evidence of launch/render only, not those checks.

`UIFileSharingEnabled`, `LSSupportsOpeningDocumentsInPlace` and `.bin`/`.cue` UTIs
are provided now. Files can store the player's disc in Documents. This shell does
not parse/extract/play it or handle an Open In URL; connect the disc importer in
the later game lane. Keep any future real-game screenshots and extracts in the
private work tree, never in Git or publicly uploaded artifacts. The current CI
screenshot contains only the original test pattern.

API sources: [winit 0.30.13 iOS source](https://github.com/rust-windowing/winit/blob/v0.30.13/src/platform/ios.rs),
[UIKit safeAreaInsets](https://developer.apple.com/documentation/uikit/uiview/safeareainsets),
[Apple file-sharing key](https://developer.apple.com/documentation/bundleresources/information-property-list/uifilesharingenabled),
[Apple in-place document key](https://developer.apple.com/documentation/bundleresources/information-property-list/lssupportsopeningdocumentsinplace).
