# sh-touch

Standalone GPL-3.0 Rust library and Windows example for the touch lane. No game
data, platform dependencies or clock APIs in the library. Input semantics were
derived from the Silent Hill decompilation project's US source at commit
`d9e28f8315c7938117224f21516786d9d149a145` (GPL-3.0). The GPL licence text is retained
in `LICENSE`. Source evidence and limitations are in `COVERAGE.md` and
`INPUT_AUDIT.md`; `PROJECT_STATE.md` is the lane checkpoint.

## Run from the worktree root

```powershell
cargo run --manifest-path crates/touch/Cargo.toml --example harness
cargo run --manifest-path crates/touch/Cargo.toml --example harness -- --record crates/touch/target/session.jsonl
cargo run --manifest-path crates/touch/Cargo.toml --example harness -- --smoke --replay crates/touch/tests/replays/coverage.jsonl
```

Mouse left button is one finger. Native `WindowEvent::Touch` preserves finger IDs;
synthetic mouse delivery is suppressed while native contacts are active. Coordinates
are converted from physical pixels using the window scale factor. Tap MODE NEXT
or press Tab to cycle gameplay, aiming, menus, inventory, map, puzzles, cutscene,
dialogue. Escape releases all input. Demo item/element IDs and original shapes
exercise the contract; the example is not a game renderer or a playable game.
The window shows active-high pad bits, both axes, raw packet and UI requests.
`--smoke` exits after at least ten successful presents and a complete replay.
`--snapshot path.ppm` saves original harness shapes on the tenth frame.

## Host integration contract

```rust
use sh_touch::*;

let mut input = Engine::new(Config::default(), Viewport::default(),
    GameContext { mode: Mode::Exploring, available: Availability::all() })?;
input.touch(TouchEvent { id: 1, phase: Phase::Down,
    x: 650.0, y: 200.0, time_ms: 0 })?;
input.touch(TouchEvent { id: 1, phase: Phase::Up,
    x: 650.0, y: 200.0, time_ms: 10 })?;
let output = input.frame(33)?; // once per game logic tick
assert_eq!(output.pad.buttons, Buttons::CROSS);
let packet: [u8; 8] = output.pad.ps1_packet(); // libpad host buffer
# Ok::<(), InputError>(())
```

1. Supply **live** logical bounds and safe-area insets, game mode, owned/allowed
   chips, movement/combat/skip availability. `Availability::all()` is for demos.
   Install context before registering targets or receiving events for that scene.
2. Feed timestamped Down/Move/Up/Cancel events in monotonic order. Do not drop
   events between logic ticks. Poll `frame()` at the original C game's tick rate,
   independently of interpolated rendering. Each queued tap occupies one tick;
   repeated identical button pulses get a neutral tick for original clicked flags.
3. Pass `pad.ps1_packet()` through the platform libpad replacement. It is the
   original eight-byte connected DualShock buffer (0, 0x73, active-low buttons,
   right X/Y, left X/Y). `joy.c` derives held/clicked/released/pulsed states and
   signed/normalised analog values; don't inject its derived 32-bit flags too.
4. Supply enabled `UiTarget`s in **points** for current menu rows, inventory items
   and commands, and puzzle elements. IDs are opaque host IDs; the library never
   loads assets, recognises a puzzle image or contains solutions. Reserved native
   chips win hit testing. Targets must fit the safe content rectangle. If an old
   panel needs pad input, a labelled `TargetAction::Pad` target can pulse/hold any
   of the 16 buttons and both axes. No raw pad fallback is displayed by default.
5. Route `ui_actions` to a host-side command adapter, then call the existing core
   handler at its normal tick. Don't translate the same direct action into another
   pad click. Selection must honour actual visible/enabled entries, item ownership,
   animation guards, current puzzle phase and the game's original consequences.
   `Confirm/Cancel/Continue/Skip` use the relevant original handler and bindings.
   `MapPan` deltas are points; convert them to original map coordinates in the host.
   `ToggleRadio` goes through the existing inventory toggle path (there is no
   independent radio pad bit). `ReturnToTitle` opens native confirmation and then
   invokes the original return/reset path. This crate performs neither operation.
6. Read actual `controllerConfig`, `extraWalkRunCtrl` and `extraWeaponCtrl`. Default
   bindings represent USA preset 0; USA presets 1 and 2 are also provided. Use
   `set_config` when options change. Arbitrary rebinding is supported via `Bindings`;
   choose one accepted bit for each action, avoiding alternative enter/cancel combos.
   `run_inverted` matches the game's run inversion and `weapon_toggle` matches
   `extraWeaponCtrl == 0`. Auto-target selection and auto-aim options remain in C.
7. `cancel_all` on focus loss, suspension or interruption, and deliver the next
   logic frame so release/aim-off reaches the core. Viewport/config/target changes
   invalidate captures. Scene/availability changes flush old input and hotspots;
   Exploring <-> Aiming alone preserves contacts. Stable snapshots are no-ops.

The director must add the workspace entry and provide libpad/core/UI adapters;
those files are outside this lane. The library has no C FFI dependency. Typed
`PadState`/`UiAction` are the boundary, so the game core never receives touch data.
Physical touchscreen and iPhone ergonomics are unverified on this PC. An end-to-end
playthrough is required before claiming every game action/puzzle works in the port.

## Point profile and gesture details

Defaults use a provisional 956 x 440 landscape profile for a 6.9-inch phone. The
62pt side and 21pt bottom insets are examples, not measured device safe areas.
Sizes do not scale with pixels or window DPI. Alternate orientation/bounds and
left/right chip reach are configurable; all chips fit within the preferred bottom
176pt region, with 48pt targets and 8pt gaps (minimum accepted size 44pt).

| Parameter | Default |
| --- | --- |
| Floating stick radius / dead zone | 64pt / 12pt |
| Run threshold / hysteresis | 46pt / 4pt |
| Tap slop | 12pt maximum drift (including out-and-back) |
| Hold-to-aim | 280ms |
| Quick-turn swipe | Down at least 72pt within 220ms, sideways at most 28pt |

The stick retains the original tank movement: up = forward, down = retreat,
left/right = turn. Slow outer downward push also sends the original run modifier
(the core may select its backward-hop state); fast downward swipe sends both step
inputs and suppresses stick motion for that contact. Beyond the touch dead zone,
analog magnitude ranges from 0.2 to 1.0, clearing the C pad dead zone even diagonally.
Run changes only its pad modifier and respects inversion; the core selects speed.

Hold right to aim; the original core selects a target. Another finger can tap/hold
to attack while aim is held. Default `AimRelease::Latch` retains aim on release for
one-finger use: subsequent right tap/hold attacks, AIM chip exits. Configure
`WhileHeld` for release-to-stop aim. These deliberate input affordances are marked
`// PORT:`; combat rules, target choice, item checks and puzzle logic are unchanged.
LOOK / STEP< / STEP> chips hold the original camera/sidestep input, including the
ordinary run/aim combinations. State-dependent kicks, stomps and struggle inputs
remain ordinary action taps. Chip presses are captured, so dragging a stick across
them cannot activate them.

## Replay format and checks

JSON lines, one tagged record per nonempty line; no comments or trailing commas:

```jsonl
{"type":"marker","id":"action_example"}
{"type":"touch","id":1,"phase":"down","x":650,"y":200,"time_ms":0}
{"type":"touch","id":1,"phase":"up","x":650,"y":200,"time_ms":10}
{"type":"frame","time_ms":33,"expect":{"buttons":16384}}
{"type":"frame","time_ms":66,"expect":{"buttons":0}}
```

Other records: `context`, `config`, `viewport`, `targets`, `cancel_all`. `expect`
contains exact active-high `buttons`, optional `left`/`right` (default zero), and
optional `ui_actions` (default empty). Axes use a 0.0001 comparison tolerance;
UI actions are exact. Unknown top-level record/event fields, malformed records,
backwards time and frame mismatches fail with line numbers. Marker records label coverage cases; they do
not alter input. The recorder writes context/targets/events and expected frames.

```powershell
cargo fmt --manifest-path crates/touch/Cargo.toml --check
cargo clippy --manifest-path crates/touch/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path crates/touch/Cargo.toml
& ./crates/touch/tools/verify_windows_harness.ps1
python crates/touch/tools/audit_reference.py 'C:/Claude Projects/Silent Hill iOS/reference/silent-hill-decomp' --check
```

The Windows probe posts messages only to its own harness window and verifies live
action, held aim, latched attack and direct menu output from recorded JSONL. It
doesn't move the global cursor or inject input into another app. Logs and original
overlay renders are ignored under this crate's `target/`. Dependencies use the
[winit 0.30 application API](https://docs.rs/winit/0.30.13/winit/application/trait.ApplicationHandler.html)
and [softbuffer surface API](https://docs.rs/softbuffer/0.4.8/softbuffer/struct.Surface.html).
Regenerate coverage after editing the independent expected-case specification with
`python crates/touch/tools/build_coverage.py`, then run `cargo fmt` and tests.

## Native game integration (touchwire)

`host/src/pad_touch.rs` implements the real host's `PadSource`. Run the release
host with `--input touch` for mouse/native touch, or `--input PATH.jsonl` for a
native touch replay. Native JSONL is separate from the standalone engine format:
it contains `touch` records with a completed-VBlank `tick`, contact ID, phase and
logical-point x/y; `check` records assert exact tick/state/step and optional
menu/option-entry. It cannot inject pad buttons, context, bindings or targets.
Coordinates in the supplied native replays use 640x448 points with zero insets;
live input reads window DPI/bounds. Keyboard/controller packets are not sampled
in either touch mode. Focus loss and scene changes release input. Mouse capture
can leave the window without cancelling native fingers or losing its release.

```powershell
python crates/touch/tools/verify_native.py --exe target/release/silent-hill-boot.exe --disc 'C:/Claude Projects/Silent Hill iOS/private/disc/Silent Hill (USA).bin' --repeat 2
& crates/touch/tools/verify_native_mouse.ps1 -Exe target/release/silent-hill-boot.exe -Disc 'C:/Claude Projects/Silent Hill iOS/private/disc/Silent Hill (USA).bin'
```

The native milestone manifest is `milestones.json`. The menu replay skips the
intro, enters New Game's difficulty selector, backs out, then opens options.
It does not confirm a difficulty/start the unported world. The second replay
directly taps the original brightness row; normal C navigation/highlight guards
remain in charge. Full-screen movie taps use the live skip binding; movies show
no overlay. Low-opacity original chip/row/stick shapes are composited on display
copies through `GpuBackend`, leaving VRAM/readbacks unchanged. Visibility checks
count game pixels before compositing; captures/logs/hashes stay under
`private/work/touchwire/`.

Current limitations: title/difficulty previous/next/OK/back are labelled pad
fallbacks because their cursors are private C statics. OPTION row taps use its
existing selected-entry getter and ordinary pad navigation. A read-only native
prefix accessor supplies boot/title/options/movie/inventory/map/dialogue/exploring
mode and live bindings/run/weapon options; aiming, ownership and puzzle targets
need core-owned accessors. Unavailable gameplay chips stay hidden. The accessor
matches `port/boot.h` in this checkout and must migrate with core3's records.
Windowed replay is verified; `--headless` explicitly fails until core3 exposes
backend injection. Root milestone registration also belongs to the director.

UIKit now uses the same `PlatformTouch` mailbox, engine and live context/bindings
provider. Its game-view point coordinates and safe areas match the scaled display
overlay. Exploring state enables walking only while the core permits player
control. Suspension/interruption releases input, including a repeated pad read
in the same tick. See `ios/README.md` for data-free CI checks and device limits.
