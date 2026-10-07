# Touch lane checkpoint — 2026-10-07

Goal: standalone platform-neutral native touch -> PS1 pad + host UI requests;
Windows harness and deterministic gesture coverage. Owned scope is `crates/touch/`;
root `REPORT.md` is the explicitly requested, ignored handoff. Branch: `lane/touch`.

Implemented: floating tank-control stick with analog speed/run hysteresis; down
swipe quick-turn (both sidestep inputs); tap action, hold aim, tap/hold/repeat
attack; owned/allowed context chips; camera/sidestep hold paths; direct menu,
inventory/map/puzzle/dialogue actions; live bindings/options, safe insets/hand
reach, interruption handling; eight-byte active-low PS1 packet; JSONL replay.
Winit/softbuffer example accepts mouse/native touch and draws original shapes.

Decisions: source-derived USA defaults, including libkpad/joy; target selection
stays in C. Default aim latches on release for one-finger attack; WhileHeld is
available. Every deliberate input affordance is `// PORT:`. UI adapters must
invoke existing handlers and preserve prerequisites/guards; no solutions/assets.
Rejected: down+run for quick-turn (source requires stepLeft+stepRight); separate
radio pad button (source radio toggle belongs to inventory); camera-relative
movement and a face-button gamepad overlay (not the agreed port scheme).

Verified: fmt; clippy all targets with warnings denied; 183 tests (145 coverage
including row parity, 37 edge/demo tests, 1 API doctest), zero failures.
All 144 COVERAGE rows have literal expected gesture replays. Source inventory
matches d9e28f8: 675 C/header files, 134 reader entries, 43 tables/456 callbacks.
Windows full replay presented 307 frames; demo presented 12 frames and original
overlay was visually inspected. Live Windows mouse-message probe passed action,
aim, attack and one direct menu action at 1.75 DPI scale (latest: 36 recorded frames).

Open: core/libpad and native UI adapters/workspace entry are outside lane scope;
phone/touchscreen hardware, runtime scene identities, puzzle hit IDs and whole-game
playthrough remain unverified. No claim of end-to-end game playability.
Next: director integrates per README, resolves COVERAGE uncertainties and performs
full game/device testing. Details: README.md, COVERAGE.md, INPUT_AUDIT.md, root REPORT.
