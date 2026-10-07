# Core build

Initialize reference source with `git submodule update --init game/decomp` (nested decomp tooling is unnecessary). The pin is d9e28f8315c7938117224f21516786d9d149a145. Set SH_DECOMP_DIR only for a checkout at that revision.

Use `tools/dev-cargo.cmd` on Windows to enter the installed MSVC environment. Run fmt --all -- --check, clippy --workspace --all-targets --locked -- -D warnings, test --workspace --locked and build --workspace --locked. CI requires no game data. psxdisc's three owned-disc tests return early with SKIP when the optional private image is absent; synthetic fixtures still execute.

The opt-in compiler-probe decomp feature intentionally retains the original full-header ABI failures. Do not use --all-features for the passing native workspace gate. No assertions are disabled.
