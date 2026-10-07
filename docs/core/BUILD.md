# Core build

Initialize reference source with `git submodule update --init game/decomp` (nested decomp tooling is unnecessary). The pin is d9e28f8315c7938117224f21516786d9d149a145. Set SH_DECOMP_DIR only for a checkout at that revision.

Use `tools/dev-cargo.cmd` on Windows to enter the installed MSVC environment. Run fmt --all -- --check, clippy --workspace --all-targets --locked -- -D warnings, test --workspace --locked and build --workspace --locked. CI requires no game data. psxdisc's three owned-disc tests return early with SKIP when the optional private image is absent; synthetic fixtures still execute.

The opt-in compiler-probe decomp feature intentionally retains the original full-header ABI failures. Do not use --all-features for the passing native workspace gate. No assertions are disabled.

Python is required at build time for the pinned OPTION namespace/state generator. It reads only GPL reference source and writes into Cargo OUT_DIR. Its inventory gate requires 10 writable globals and 14 local statics; initialized packet images are retained. tools/prepare_sdk.py produces the same native record declarations for the iOS layout gate.

Run `tools/milestones.cmd --available --repeat 2` with a player-owned disc to check completed milestones, state expectations, visible output and deterministic PNG hashes. The default command exits 2 until the required screens/maps/transitions are implemented; it does not certify the brief's pass. See [MILESTONES.md](MILESTONES.md).
