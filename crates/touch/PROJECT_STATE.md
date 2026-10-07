# Touchwire checkpoint - 2026-10-07

Goal: real native-game touch input, live read-only context, original overlay and
touch-only menu replay. Branch lane/touchwire. Owned scope: pad_touch.rs, touch
crate, existing host dependency and minimal registration. No merge/push/main or
system installs. Source notices retained; game bytes/captures stay private.

Implemented: Engine-backed PadSource cached on actual Joy reads/VBlank clock;
queued native contacts and Windows mouse, DPI bounds, focus/scene release, live
native bindings/run/weapon config. Read-only prefix matches current port/boot.h.
Boot/title/options/movie/inventory/map/dialogue/exploring are identified; unknown
ownership/combat/event availability is closed. No guessed aiming or state writes.
GpuBackend decorator draws original low-opacity chips/rows/stick on display
copies; VRAM/readbacks unchanged, all overlay hidden during movies.

Real menu path: intro skip -> title -> New Game difficulty selector -> Back ->
title -> options. Separate replay directly taps brightness through ordinary C
navigation/feedback/guards. Each passed twice: 4 release runs, 14 exact state
checks, 2 identical PNG hashes. Overlay captures visually reviewed.
Evidence: private/work/touchwire/milestones/20261007T130053971314Z/results.json.
Windows own-process mouse probe passed difficulty/back/options at 1.75 DPI:
private/work/touchwire/mouse-20261007T1300539178553Z.log/png. First probe failed
because CursorLeft cancelled captured taps; focus/scene now cancel while mouse
leave preserves capture/native fingers. Wrong-state negative control exits 1.
Initial debug menu run passed slowly; use release for windowed replay.

Private selected workspace (host/psxdisc/psxmedia/touch): fmt, all-target clippy
with warnings denied, 246 tests + 3 doctests pass; linked C /W4 /WX. Root
fmt/clippy/test stop at nested psxgpu/psxspu workspace declarations. No root
manifest/gates changed; private validation uses identical owned Rust sources
and pinned read-only decomp d9e28f8.

Director/core3 requests: root workspace repair; typed native snapshot, live
title visibility/cursor, difficulty, aiming, owned/allowed chips, puzzle IDs/rows
and enabled guards/original handlers; headless backend injection and active
GPU/SPU factory; root touch milestone registration. Title/difficulty currently
use labelled previous/next/OK/back. No difficulty confirmation/opening world,
physical touchscreen, iOS ergonomics or whole-game puzzle coverage is claimed.
See README.md, COVERAGE.md, milestones.json and ignored root REPORT.md.

## Previous standalone evidence (historical touch lane)

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
