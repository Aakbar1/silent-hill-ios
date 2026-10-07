# Rules for Codex workers

You are one worker on Silent Hill iOS, a native port of Silent Hill (PS1, US v1.1) to iPhone with touch controls.
Read PLAN.md once. Your task is in the brief you were given.

- Only edit the files and folders your brief says you own. Need something elsewhere? Write it under "Requests" in REPORT.md.
- Reference decomp: `C:/Claude Projects/Silent Hill iOS/reference/silent-hill-decomp` (GPL-3.0, read-only).
  Copying from it is allowed; keep its licence notices. Our repo is GPL-3.0.
- Game files: `C:/Claude Projects/Silent Hill iOS/private/`. You may read them. **Never** commit or copy
  game bytes, extracted assets, disassembly generated from the game, or screenshots of game content into the repo.
  Put generated or extracted material in `C:/Claude Projects/Silent Hill iOS/private/work/<lane>/`.
- No system-wide installs (LLVM, CMake, WSL distros, Docker, VS components). Cargo crates and pip packages
  in a local venv are fine. If you need a system tool, stop and ask under "Requests".
- Rust stable: `cargo fmt`, `cargo clippy -- -D warnings` and `cargo test` must pass before you stop. C code must compile without new warnings.
- Do not change game behaviour on purpose; we are porting, not redesigning. Mark every intentional deviation with `// PORT:` and a reason.
- Commit to your own branch with clear messages. Do not merge, push, or touch main.

Finish by writing `REPORT.md` in the repo root (do not commit it; it is gitignored), 30 lines max:
1. Done (bullets)
2. Test results (command + pass/fail counts)
3. Open problems
4. Requests to the director
