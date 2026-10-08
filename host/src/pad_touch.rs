// SPDX-License-Identifier: GPL-3.0-only
//! Native touch lane. C globals are read only on the game worker (or after join).
//! Overlay composition occurs on the display copy, never in game VRAM.
use crate::{
    backend::{GpuBackend, SilentSpu},
    disc::{DiscImage, GameDisc},
    native::{self, Backends, ReplayCheck},
    pad::{LiveInput, PadSource},
    raster::Raster,
};
use sh_touch::{
    game_replay::{self, Record},
    *,
};
use std::{
    collections::{HashSet, VecDeque},
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use winit::{
    event::{MouseButton, TouchPhase, WindowEvent},
    window::Window,
};

// Exact native prefixes from port/boot.h, NOT the original PS1 wire records.
// Keep these small; a core-owned typed accessor is needed when core3 migrates them.
#[repr(C)]
#[derive(Clone, Copy)]
struct GamePrefix {
    state: i32,
    previous: i32,
    steps: [i32; 3],
    background: [u8; 3],
    width: i32,
    height: i32,
    bindings: [u16; 14],
    options: [i32; 17],
}
#[repr(C)]
#[derive(Clone, Copy)]
struct SysPrefix {
    counters: [i32; 3],
    state: i32,
}
const _: () = assert!(std::mem::size_of::<GamePrefix>() == 128);
const _: () = assert!(std::mem::offset_of!(GamePrefix, bindings) == 32);
const _: () = assert!(std::mem::offset_of!(GamePrefix, options) == 60);
unsafe extern "C" {
    static g_GameWork: GamePrefix;
    static g_SysWork: SysPrefix;
    fn sh_title_menu_state() -> i32;
    fn sh_option_selected_entry() -> i32;
    fn VSync(mode: i32) -> i32;
    static g_Player_DisableControl: u8;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Snapshot {
    tick: u64,
    state: i32,
    step: i32,
    sys: i32,
    menu: i32,
    option_entry: i32,
}

/// Read-only accessor for the CURRENT boot ABI. Caller must be the sole C worker
/// or have joined it. No winit/render UI thread is allowed to inspect C globals.
unsafe fn context_snapshot() -> (Snapshot, Config, GameContext) {
    // SAFETY: Caller guarantees C is not concurrently writing; boot.h defines
    // these aligned native prefixes and VSync(-1) only reads the VBlank counter.
    let (game, sys, tick, menu, option_entry) = unsafe {
        (
            std::ptr::addr_of!(g_GameWork).read(),
            std::ptr::addr_of!(g_SysWork).read(),
            VSync(-1),
            sh_title_menu_state(),
            sh_option_selected_entry(),
        )
    };
    let snapshot = Snapshot {
        tick: tick.max(0) as u64,
        state: game.state,
        step: game.steps[0],
        sys: sys.state,
        menu,
        option_entry,
    };
    let bindings = native_bindings(game.bindings);
    let config = Config {
        bindings,
        weapon_toggle: game.options[5] == 0,
        run_inverted: game.options[14] != 0,
        ..Config::default()
    };
    let mut context = scene(snapshot);
    // SAFETY: Same worker-only prefix/accessor contract as the snapshot above.
    context.available.movement &= unsafe { g_Player_DisableControl == 0 };
    (snapshot, config, context)
}

fn native_bindings(bits: [u16; 14]) -> Bindings {
    let defaults = Bindings::default();
    let bit = |index: usize, fallback: Buttons| {
        let mask = bits[index];
        if mask == 0 {
            fallback
        } else {
            Buttons(mask.isolate_lowest_one())
        }
    };
    Bindings {
        enter: bit(0, defaults.enter),
        cancel: bit(1, defaults.cancel),
        skip: bit(2, defaults.skip),
        action: bit(3, defaults.action),
        aim: bit(4, defaults.aim),
        light: bit(5, defaults.light),
        run: bit(6, defaults.run),
        view: bit(7, defaults.view),
        step_left: bit(8, defaults.step_left),
        step_right: bit(9, defaults.step_right),
        pause: bit(10, defaults.pause),
        item: bit(11, defaults.item),
        map: bit(12, defaults.map),
    }
}

fn movie(snapshot: Snapshot) -> bool {
    matches!(snapshot.state, 3 | 5 | 6 | 9) || snapshot.sys == 4
}
fn scene(snapshot: Snapshot) -> GameContext {
    // Fail closed for ownership/combat/event availability. These are not yet
    // represented in PortSysWork; guessing them would expose invalid chips.
    let mode = if movie(snapshot) || matches!(snapshot.state, 0..=2 | 10 | 13) {
        Mode::Cutscene
    } else if snapshot.state == 14 || (snapshot.state == 11 && snapshot.sys == 2) {
        Mode::Inventory
    } else if snapshot.state == 15 || (snapshot.state == 11 && snapshot.sys == 3) {
        Mode::Map
    } else if snapshot.state == 11 && snapshot.sys == 7 {
        Mode::Dialogue
    } else if snapshot.state == 11 && snapshot.sys == 0 {
        Mode::Exploring
    } else {
        Mode::Menu
    };
    GameContext {
        mode,
        available: Availability {
            skip: matches!(snapshot.state, 1 | 2 | 5 | 6 | 9),
            movement: mode == Mode::Exploring,
            ..Availability::default()
        },
    }
}

#[derive(Clone)]
struct Overlay {
    view: Viewport,
    hidden: bool,
    regions: Vec<(Rect, &'static str)>,
    stick: Option<(Point, Point)>,
}
enum Input {
    Touch(TouchEvent),
    View(Viewport),
    Cancel,
}
struct Shared {
    live: bool,
    clock_ms: u64,
    queue: VecDeque<Input>,
    native_ids: HashSet<u64>,
    mouse: Point,
    mouse_down: bool,
    overlay: Option<Overlay>,
    checks: Vec<Record>,
    checked: usize,
    error: Option<String>,
    base_lit: usize,
}
impl Shared {
    fn fail(&mut self, error: impl ToString) {
        if self.error.is_none() {
            self.error = Some(error.to_string());
        }
    }
    fn observe(&mut self, snapshot: Snapshot) {
        while let Some(Record::Check {
            tick,
            id,
            state,
            step,
            menu,
            option_entry,
        }) = self.checks.get(self.checked)
        {
            if *tick > snapshot.tick {
                break;
            }
            let ok = *tick == snapshot.tick
                && *state == snapshot.state
                && *step == snapshot.step
                && menu.is_none_or(|m| m == snapshot.menu)
                && option_entry.is_none_or(|e| e == snapshot.option_entry);
            let message = format!(
                "TOUCH_CHECK {id} {} tick={} state={} step={} menu={} option_entry={}",
                if ok { "PASS" } else { "FAIL" },
                snapshot.tick,
                snapshot.state,
                snapshot.step,
                snapshot.menu,
                snapshot.option_entry
            );
            println!("{message}");
            if !ok {
                self.fail(message);
            }
            self.checked += 1;
        }
    }
}
static ACTIVE: Mutex<Option<Arc<Mutex<Shared>>>> = Mutex::new(None);
const MOUSE: u64 = u64::MAX;

/// The only event-loop registration needed. It never reads the game's globals.
pub fn window_event(event: &WindowEvent, window: Option<&Window>) {
    let Some(shared) = ACTIVE.lock().expect("touch registration").clone() else {
        return;
    };
    let Some(window) = window else {
        return;
    };
    let mut shared = shared.lock().expect("touch queue");
    if !shared.live {
        return;
    }
    let scale = window.scale_factor();
    let size = window.inner_size().to_logical::<f32>(scale);
    if size.width >= 176.0 && size.height >= 176.0 {
        let view = Viewport {
            width: size.width,
            height: size.height,
            safe: Insets::default(),
        };
        if shared.overlay.as_ref().is_none_or(|o| o.view != view) {
            shared.queue.push_back(Input::View(view));
        }
    }
    let time_ms = shared.clock_ms;
    match event {
        WindowEvent::Touch(touch) => {
            // Prevent synthesized mouse duplicates and ID collisions with winit.
            if touch.id == MOUSE {
                shared.fail("native touch ID collides with mouse ID");
                return;
            }
            let phase = match touch.phase {
                TouchPhase::Started => {
                    if shared.mouse_down {
                        shared.queue.push_back(Input::Touch(TouchEvent {
                            id: MOUSE,
                            phase: Phase::Cancel,
                            x: 0.0,
                            y: 0.0,
                            time_ms,
                        }));
                        shared.mouse_down = false;
                    }
                    shared.native_ids.insert(touch.id);
                    Phase::Down
                }
                TouchPhase::Moved => Phase::Move,
                TouchPhase::Ended => {
                    shared.native_ids.remove(&touch.id);
                    Phase::Up
                }
                TouchPhase::Cancelled => {
                    shared.native_ids.remove(&touch.id);
                    Phase::Cancel
                }
            };
            let p = touch.location.to_logical::<f32>(scale);
            shared.queue.push_back(Input::Touch(TouchEvent {
                id: touch.id,
                phase,
                x: p.x,
                y: p.y,
                time_ms,
            }));
        }
        WindowEvent::CursorMoved { position, .. } => {
            let p = position.to_logical::<f32>(scale);
            shared.mouse = Point::new(p.x, p.y);
            if shared.mouse_down && shared.native_ids.is_empty() {
                shared.queue.push_back(Input::Touch(TouchEvent {
                    id: MOUSE,
                    phase: Phase::Move,
                    x: p.x,
                    y: p.y,
                    time_ms,
                }));
            }
        }
        WindowEvent::MouseInput {
            button: MouseButton::Left,
            state,
            ..
        } if shared.native_ids.is_empty() => {
            let down = state.is_pressed();
            if shared.mouse_down != down {
                shared.mouse_down = down;
                let p = shared.mouse;
                shared.queue.push_back(Input::Touch(TouchEvent {
                    id: MOUSE,
                    phase: if down { Phase::Down } else { Phase::Up },
                    x: p.x,
                    y: p.y,
                    time_ms,
                }));
            }
        }
        WindowEvent::Focused(false) | WindowEvent::CloseRequested => {
            shared.queue.push_back(Input::Cancel);
            shared.native_ids.clear();
            shared.mouse_down = false;
        }
        _ => {}
    }
}

pub struct TouchPad {
    engine: Engine,
    shared: Arc<Mutex<Shared>>,
    events: Vec<Record>,
    cursor: usize,
    cached: Option<(u64, [u8; 8])>,
    commands: VecDeque<Buttons>,
    previous_command: Buttons,
    option_command: Option<(i32, u64)>,
    scene_key: (i32, i32, i32),
}
fn default_view() -> Viewport {
    Viewport {
        width: 640.0,
        height: 448.0,
        safe: Insets::default(),
    }
}
fn targets(snapshot: Snapshot, view: Viewport) -> Vec<(UiTarget, &'static str)> {
    let mut rows = Vec::new();
    let mut add = |bounds, action, label| {
        rows.push((
            UiTarget {
                bounds,
                action,
                enabled: true,
            },
            label,
        ))
    };
    if scene(snapshot).mode == Mode::Cutscene {
        if scene(snapshot).available.skip {
            // PORT: a tap anywhere skips eligible logos/FMVs using the original
            // skip binding. The movie has no visible overlay, as requested.
            add(
                view.content(),
                TargetAction::Ui {
                    request: UiAction::Skip,
                },
                "",
            );
        }
    } else if snapshot.state == 7 && matches!(snapshot.menu, 1 | 3) {
        // Title/difficulty cursors are private C statics. Labelled previous/next
        // fallback avoids pretending a guessed row is the live selection.
        for (index, button, label) in [(0, Buttons::UP, "PREV"), (1, Buttons::DOWN, "NEXT")] {
            add(
                Rect {
                    x: view.content().x + 16.0 + index as f32 * 64.0,
                    y: view.content().y + view.content().height - 64.0,
                    width: 56.0,
                    height: 48.0,
                },
                TargetAction::Pad {
                    state: PadState {
                        buttons: button,
                        ..PadState::default()
                    },
                    hold: false,
                },
                label,
            );
        }
    } else if snapshot.state == 18 && snapshot.step == 1 {
        // Original OPTION labels: x=64, y=56+16*i in the 320x224 display.
        // PORT: direct row taps navigate by ordinary pad pulses and feedback;
        // activation waits for C's existing highlight/transition guards.
        for index in 0..9 {
            add(
                Rect {
                    x: view.content().x + view.content().width * 0.18,
                    y: view.content().y
                        + view.content().height * (48.0 + 16.0 * index as f32) / 224.0,
                    width: view.content().width * 0.5,
                    height: view.content().height * 16.0 / 224.0,
                },
                TargetAction::Ui {
                    request: UiAction::Activate { id: index },
                },
                "",
            );
        }
    }
    rows
}
impl TouchPad {
    fn sample_snapshot(
        &mut self,
        tick: u64,
        snapshot: Snapshot,
        config: Config,
        context: GameContext,
    ) -> [u8; 8] {
        // A suspension release takes precedence over a repeated read of the
        // same tick. Ordinary repeated reads still retain their cached pulse.
        let cancelling = self
            .shared
            .lock()
            .expect("touch cancel state")
            .queue
            .iter()
            .any(|i| matches!(i, Input::Cancel));
        if let Some((previous, packet)) = self.cached
            && previous == tick
            && !cancelling
        {
            return packet;
        }
        let packet = match self.step(tick, snapshot, config, context) {
            Ok(packet) => packet,
            Err(error) => {
                self.engine.cancel_all();
                self.shared.lock().expect("touch queue").fail(error);
                PadState::default().ps1_packet()
            }
        };
        self.cached = Some((tick, packet));
        packet
    }
    fn step(
        &mut self,
        tick: u64,
        snapshot: Snapshot,
        config: Config,
        context: GameContext,
    ) -> Result<[u8; 8], String> {
        if self.cached.is_some_and(|(previous, _)| tick < previous) {
            return Err("touch clock rewound".into());
        }
        let time_ms = game_replay::tick_ms(tick);
        let mut shared = self.shared.lock().expect("touch queue");
        let key = (snapshot.state, snapshot.step, snapshot.menu);
        if self.scene_key != key {
            if shared.live {
                println!(
                    "TOUCH_SCENE tick={tick} state={} step={} menu={} option_entry={}",
                    snapshot.state, snapshot.step, snapshot.menu, snapshot.option_entry
                );
            }
            self.option_command = None;
            self.commands.clear();
            self.previous_command = Buttons::default();
            self.engine.cancel_all();
            shared
                .queue
                .retain(|i| matches!(i, Input::View(_) | Input::Cancel));
            self.scene_key = key;
        }
        if self.engine.config() != config || self.engine.context() != context {
            self.commands.clear();
            self.previous_command = Buttons::default();
            self.option_command = None;
            shared
                .queue
                .retain(|i| matches!(i, Input::View(_) | Input::Cancel));
        }
        self.engine.set_config(config).map_err(|e| e.to_string())?;
        self.engine.set_context(context);
        let queued: Vec<_> = shared.queue.drain(..).collect();
        for input in &queued {
            if let Input::View(view) = input {
                if self.engine.viewport() != *view {
                    self.commands.clear();
                    self.previous_command = Buttons::default();
                    self.option_command = None;
                }
                self.engine.set_viewport(*view).map_err(|e| e.to_string())?;
            }
        }
        let rows = targets(snapshot, self.engine.viewport());
        self.engine
            .set_targets(rows.iter().map(|(t, _)| t.clone()).collect())
            .map_err(|e| e.to_string())?;
        for input in queued {
            match input {
                Input::Touch(event) => {
                    if shared.live {
                        println!(
                            "TOUCH_EVENT tick={tick} phase={:?} x={} y={}",
                            event.phase, event.x, event.y
                        );
                    }
                    // A cancelled/resized scene may receive the tail of an old
                    // contact. Ignore only those tails, never new Down events.
                    match self.engine.touch(event) {
                        Ok(()) | Err(InputError::UnknownContact) if event.phase != Phase::Down => {}
                        result => result.map_err(|e| e.to_string())?,
                    }
                }
                Input::Cancel => {
                    self.engine.cancel_all();
                    self.option_command = None;
                    self.commands.clear();
                    self.previous_command = Buttons::default();
                }
                Input::View(_) => {}
            }
        }
        while let Some(record) = self.events.get(self.cursor) {
            if record.tick() > tick {
                break;
            }
            if let Some(event) = record.event() {
                self.engine
                    .touch(event)
                    .map_err(|e| format!("touch replay tick {}: {e}", record.tick()))?;
            }
            self.cursor += 1;
        }
        let mut output = self.engine.frame(time_ms).map_err(|e| e.to_string())?;
        for action in output.ui_actions {
            match action {
                UiAction::Confirm | UiAction::Continue => {
                    self.option_command = None;
                    self.commands.push_back(config.bindings.enter);
                }
                UiAction::Cancel => {
                    self.option_command = None;
                    self.commands.push_back(config.bindings.cancel);
                }
                UiAction::Skip => self.commands.push_back(config.bindings.skip),
                UiAction::Activate { id }
                    if snapshot.state == 18 && snapshot.step == 1 && id < 9 =>
                {
                    self.option_command = Some((id as i32, tick));
                }
                other => return Err(format!("core UI adapter required for {other:?}")),
            }
        }
        if let Some((desired, next)) = self.option_command
            && tick >= next
            && self.commands.is_empty()
        {
            if snapshot.option_entry == desired {
                self.commands.push_back(config.bindings.enter);
                self.option_command = None;
            } else {
                self.commands.push_back(if snapshot.option_entry > desired {
                    Buttons::UP
                } else {
                    Buttons::DOWN
                });
                self.option_command = Some((desired, tick + 40));
            }
        }
        // PORT: UI taps are serialized onto real logic reads, with a neutral
        // read between identical pulses so C's clicked flags see both taps.
        if let Some(button) = self.commands.front().copied()
            && button.0 & self.previous_command.0 == 0
        {
            self.commands.pop_front();
            output.pad.buttons |= button;
            self.previous_command = button;
        } else {
            self.previous_command = Buttons::default();
        }
        let mut regions: Vec<_> = self
            .engine
            .layout()
            .controls
            .iter()
            .map(|c| (c.bounds, c.control.label()))
            .collect();
        if !movie(snapshot) {
            regions.extend(rows.into_iter().map(|(t, label)| (t.bounds, label)));
        }
        shared.clock_ms = time_ms;
        shared.overlay = Some(Overlay {
            view: self.engine.viewport(),
            hidden: movie(snapshot),
            regions,
            stick: self.engine.stick_visual(),
        });
        Ok(output.pad.ps1_packet())
    }
}
impl PadSource for TouchPad {
    fn sample(&mut self, tick: u64) -> [u8; 8] {
        if let Some((previous, packet)) = self.cached
            && previous == tick
        {
            return packet;
        }
        // SAFETY: PadSource is sampled synchronously on the sole native C worker.
        let (snapshot, config, context) = unsafe { context_snapshot() };
        self.sample_snapshot(tick, snapshot, config, context)
    }
}

struct OverlayGpu {
    inner: Box<dyn GpuBackend>,
    shared: Arc<Mutex<Shared>>,
}
impl OverlayGpu {
    fn composite(&self, mut pixels: Vec<u32>, w: u32, h: u32) -> Vec<u32> {
        // SAFETY: GpuBackend frame extraction is on the same C worker as input.
        let (snapshot, _, _) = unsafe { context_snapshot() };
        let mut shared = self.shared.lock().expect("touch overlay");
        shared.base_lit = pixels.iter().filter(|p| **p != 0).count();
        shared.observe(snapshot);
        if let Some(overlay) = &shared.overlay
            && !movie(snapshot)
        {
            draw_overlay(&mut pixels, w, h, overlay);
        }
        pixels
    }
}
impl GpuBackend for OverlayGpu {
    fn begin_ordering_table(&mut self) {
        self.inner.begin_ordering_table();
    }
    fn packet(&mut self, words: &[u32]) {
        self.inner.packet(words);
    }
    fn end_ordering_table(&mut self) {
        self.inner.end_ordering_table();
    }
    fn env(&mut self, clip: [i32; 4], offset: [i32; 2]) {
        self.inner.env(clip, offset);
    }
    fn load(&mut self, rect: [i32; 4], pixels: &[u16]) {
        self.inner.load(rect, pixels);
    }
    fn read(&self, rect: [i32; 4]) -> Vec<u16> {
        self.inner.read(rect)
    }
    fn clear(&mut self, rect: [i32; 4], color: [u8; 3]) {
        self.inner.clear(rect, color);
    }
    fn frame(&self, x: i32, y: i32, w: u32, h: u32) -> Vec<u32> {
        self.composite(self.inner.frame(x, y, w, h), w, h)
    }
    fn frame_rgb24(&self, x: i32, y: i32, w: u32, h: u32) -> Vec<u32> {
        self.composite(self.inner.frame_rgb24(x, y, w, h), w, h)
    }
    fn primitives(&self) -> u64 {
        self.inner.primitives()
    }
}

fn draw_overlay(pixels: &mut [u32], w: u32, h: u32, overlay: &Overlay) {
    if overlay.hidden
        || pixels.len() != (w * h) as usize
        || (overlay.regions.is_empty() && overlay.stick.is_none())
    {
        return;
    }
    for y in 0..h {
        for x in 0..w {
            let p = Point::new(
                x as f32 * overlay.view.width / w as f32,
                y as f32 * overlay.view.height / h as f32,
            );
            let mut alpha = 0;
            for (r, label) in &overlay.regions {
                if r.contains(p) {
                    let edge = p.x - r.x < 2.0
                        || p.y - r.y < 2.0
                        || r.x + r.width - p.x < 2.0
                        || r.y + r.height - p.y < 2.0;
                    alpha = alpha.max(if edge { 64 } else { 20 });
                    if label_pixel(label, *r, p) {
                        alpha = 150;
                    }
                }
            }
            if let Some((origin, position)) = overlay.stick {
                let distance = (p.x - origin.x).hypot(p.y - origin.y);
                if (distance - 64.0).abs() < 2.0 || (p.x - position.x).hypot(p.y - position.y) < 7.0
                {
                    alpha = 80;
                }
            }
            if alpha != 0 {
                let pixel = &mut pixels[(y * w + x) as usize];
                let mut color = 0;
                for shift in [0, 8, 16] {
                    color |=
                        (((*pixel >> shift & 255) * (255 - alpha) + 220 * alpha) / 255) << shift;
                }
                *pixel = color;
            }
        }
    }
}
fn label_pixel(label: &str, r: Rect, p: Point) -> bool {
    // Original 3x5 glyphs for context labels; no font/game assets are loaded.
    let x = ((p.x - (r.x + (r.width - label.len() as f32 * 8.0) * 0.5)) / 2.0).floor() as i32;
    let y = ((p.y - (r.y + (r.height - 10.0) * 0.5)) / 2.0).floor() as i32;
    if x < 0 || !(0..5).contains(&y) {
        return false;
    }
    let Some(c) = label.as_bytes().get(x as usize / 4) else {
        return false;
    };
    let glyph = match c {
        b'A' => [2, 5, 7, 5, 5],
        b'B' => [6, 5, 6, 5, 6],
        b'C' => [3, 4, 4, 4, 3],
        b'D' => [6, 5, 5, 5, 6],
        b'E' => [7, 4, 6, 4, 7],
        b'F' => [7, 4, 6, 4, 4],
        b'G' => [3, 4, 5, 5, 3],
        b'H' => [5, 5, 7, 5, 5],
        b'I' => [7, 2, 2, 2, 7],
        b'K' => [5, 5, 6, 5, 5],
        b'L' => [4, 4, 4, 4, 7],
        b'M' => [5, 7, 7, 5, 5],
        b'N' => [5, 7, 7, 7, 5],
        b'O' => [2, 5, 5, 5, 2],
        b'P' => [6, 5, 6, 4, 4],
        b'R' => [6, 5, 6, 5, 5],
        b'S' => [3, 4, 2, 1, 6],
        b'T' => [7, 2, 2, 2, 2],
        b'U' => [5, 5, 5, 5, 7],
        b'V' => [5, 5, 5, 5, 2],
        b'W' => [5, 5, 7, 7, 5],
        b'X' => [5, 5, 2, 5, 5],
        b'<' => [1, 2, 4, 2, 1],
        b'>' => [4, 2, 1, 2, 4],
        _ => [0; 5],
    };
    let column = x as usize % 4;
    column < 3 && glyph[y as usize] & (1 << (2 - column)) != 0
}

pub fn accepts(input: Option<&Path>) -> bool {
    input.is_some_and(|p| p == Path::new("touch") || p.extension().is_some_and(|e| e == "jsonl"))
}

/// UIKit sends point coordinates into the same worker-owned engine/provider as
/// desktop. No UI thread reads C globals. A handle owns only the bounded mailbox.
#[derive(Clone)]
pub struct PlatformTouch {
    shared: Arc<Mutex<Shared>>,
}
impl PlatformTouch {
    pub fn new(view: Viewport) -> Result<(Self, TouchPad), String> {
        let engine = Engine::new(Config::default(), view, GameContext::default())
            .map_err(|e| e.to_string())?;
        let shared = Arc::new(Mutex::new(Shared {
            live: true,
            clock_ms: 0,
            queue: VecDeque::new(),
            native_ids: HashSet::new(),
            mouse: Point::default(),
            mouse_down: false,
            overlay: None,
            checks: Vec::new(),
            checked: 0,
            error: None,
            base_lit: 0,
        }));
        let pad = TouchPad {
            engine,
            shared: shared.clone(),
            events: Vec::new(),
            cursor: 0,
            cached: None,
            commands: VecDeque::new(),
            previous_command: Buttons::default(),
            option_command: None,
            scene_key: (-1, -1, -1),
        };
        Ok((Self { shared }, pad))
    }
    pub fn touch(&self, id: u64, phase: Phase, x: f32, y: f32) {
        let mut shared = self.shared.lock().expect("UIKit touch queue");
        // PORT: UIKit uptime is not the original game's sample clock. Events are
        // stamped at the last logic sample; holds advance only on game ticks,
        // including after a long suspension. Order within a batch is preserved.
        let time_ms = shared.clock_ms;
        if shared.queue.len() >= 512 {
            shared.queue.clear();
            shared.queue.push_back(Input::Cancel);
            eprintln!("IOS_TOUCH overflow; releasing all contacts");
        }
        shared.queue.push_back(Input::Touch(TouchEvent {
            id,
            phase,
            x,
            y,
            time_ms,
        }));
    }
    pub fn viewport(&self, view: Viewport) {
        let mut shared = self.shared.lock().expect("UIKit viewport");
        // Keep only the newest geometry; engine viewport changes cancel captures.
        shared.queue.retain(|i| !matches!(i, Input::View(_)));
        shared.queue.push_back(Input::View(view));
    }
    pub fn cancel(&self) {
        let mut shared = self.shared.lock().expect("UIKit cancel");
        shared.queue.retain(|i| matches!(i, Input::View(_)));
        shared.queue.push_back(Input::Cancel);
        shared.overlay = None;
    }
    pub fn draw(&self, pixels: &mut [u32], w: u32, h: u32) {
        let shared = self.shared.lock().expect("UIKit overlay");
        if let Some(overlay) = &shared.overlay {
            draw_overlay(pixels, w, h, overlay);
        }
    }
    pub fn error(&self) -> Option<String> {
        self.shared.lock().expect("UIKit error").error.clone()
    }

    /// Data-free CI exercises the production mailbox, engine and pad adapter.
    /// Synthetic snapshots are used ONLY here, never in a real game session.
    pub fn smoke_check(&self, pad: &mut TouchPad) -> Result<(), String> {
        self.smoke_check_with(pad, |id, phase, x, y| self.touch(id, phase, x, y))
    }
    pub fn smoke_check_with(
        &self,
        pad: &mut TouchPad,
        send: impl Fn(u64, Phase, f32, f32),
    ) -> Result<(), String> {
        let view = pad.engine.viewport();
        let snapshot = |tick| Snapshot {
            tick,
            state: 7,
            step: 1,
            menu: 1,
            ..Snapshot::default()
        };
        pad.sample_snapshot(0, snapshot(0), Config::default(), scene(snapshot(0)));
        let bounds = pad
            .engine
            .layout()
            .controls
            .iter()
            .find(|c| c.control == Control::Confirm)
            .ok_or("missing menu OK target")?
            .bounds;
        for id in [1, 2] {
            send(id, Phase::Down, bounds.center().x, bounds.center().y);
            send(id, Phase::Up, bounds.center().x, bounds.center().y);
        }
        let pressed = PadState {
            buttons: Buttons::CROSS,
            ..PadState::default()
        }
        .ps1_packet();
        for (tick, expected) in [
            (1, pressed),
            (2, PadState::default().ps1_packet()),
            (3, pressed),
        ] {
            if pad.sample_snapshot(
                tick,
                snapshot(tick),
                Config::default(),
                scene(snapshot(tick)),
            ) != expected
            {
                return Err("UIKit menu pulse/neutral-edge check failed".into());
            }
        }
        let exploring = |tick| Snapshot {
            tick,
            state: 11,
            sys: 0,
            ..Snapshot::default()
        };
        pad.sample_snapshot(4, exploring(4), Config::default(), scene(exploring(4)));
        let origin = Point::new(view.content().x + 80.0, view.content().y + 100.0);
        send(3, Phase::Down, origin.x, origin.y);
        send(3, Phase::Move, origin.x, origin.y - 55.0);
        let packet = pad.sample_snapshot(5, exploring(5), Config::default(), scene(exploring(5)));
        if packet[7] >= 64 {
            return Err("UIKit walking analog check failed".into());
        }
        self.cancel();
        if pad.sample_snapshot(6, exploring(6), Config::default(), scene(exploring(6)))
            != PadState::default().ps1_packet()
        {
            return Err("UIKit lifecycle release check failed".into());
        }
        self.error().map_or(Ok(()), Err)
    }
}
fn check_output(path: &Path) -> Result<(), String> {
    let private = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find_map(|parent| parent.join("private/work/touchwire").canonicalize().ok())
        .ok_or("private/work/touchwire directory is unavailable")?;
    let parent = path
        .parent()
        .unwrap_or(Path::new("."))
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !parent.starts_with(&private)
        || (path.exists()
            && !path
                .canonicalize()
                .map_err(|e| e.to_string())?
                .starts_with(&private))
    {
        return Err("touch screenshots must be inside private/work/touchwire/".into());
    }
    Ok(())
}

pub fn run(
    disc: GameDisc<DiscImage<File>>,
    limit: u64,
    screenshot: Option<PathBuf>,
    input: &Path,
    headless: bool,
    check: ReplayCheck,
) -> Result<(), String> {
    if headless {
        return Err(
            "touch headless needs core3's run_headless_with_backends hook (see REPORT.md)".into(),
        );
    }
    if check.min_movie_frames != 0 || check.movie_skips.is_some() {
        return Err(
            "movie counter expectations need the core3 headless hook; use JSONL game checkpoints"
                .into(),
        );
    }
    if let Some(path) = &screenshot {
        check_output(path)?;
    }
    let live = input == Path::new("touch");
    let records = if live {
        Vec::new()
    } else {
        game_replay::parse(BufReader::new(
            File::open(input).map_err(|e| e.to_string())?,
        ))?
    };
    if records.iter().any(|r| r.tick() > limit) {
        return Err("replay extends past frame limit".into());
    }
    let shared = Arc::new(Mutex::new(Shared {
        live,
        clock_ms: 0,
        queue: VecDeque::new(),
        native_ids: HashSet::new(),
        mouse: Point::default(),
        mouse_down: false,
        overlay: None,
        checks: records
            .iter()
            .filter(|r| matches!(r, Record::Check { .. }))
            .cloned()
            .collect(),
        checked: 0,
        error: None,
        base_lit: 0,
    }));
    let pad = TouchPad {
        engine: Engine::new(Config::default(), default_view(), GameContext::default())
            .map_err(|e| e.to_string())?,
        shared: shared.clone(),
        events: records
            .into_iter()
            .filter(|r| matches!(r, Record::Touch { .. }))
            .collect(),
        cursor: 0,
        cached: None,
        commands: VecDeque::new(),
        previous_command: Buttons::default(),
        option_command: None,
        scene_key: (-1, -1, -1),
    };
    {
        let mut active = ACTIVE.lock().expect("touch registration");
        if active.is_some() {
            return Err("touch session already running".into());
        }
        *active = Some(shared.clone());
    }
    let result = native::run_with_backends(
        disc,
        limit,
        screenshot,
        Arc::new(LiveInput::default()),
        Backends {
            pad: Box::new(pad),
            gpu: Box::new(OverlayGpu {
                inner: Box::<Raster>::default(),
                shared: shared.clone(),
            }),
            spu: Box::<SilentSpu>::default(),
        },
    );
    *ACTIVE.lock().expect("touch registration") = None;
    result?;
    // SAFETY: run_with_backends joined the sole C worker before returning.
    let (snapshot, _, _) = unsafe { context_snapshot() };
    let mut shared = shared.lock().expect("touch results");
    shared.observe(snapshot);
    if let Some(error) = shared.error.take() {
        return Err(error);
    }
    if shared.checked != shared.checks.len() {
        return Err("not all touch checkpoints were observed".into());
    }
    if snapshot.tick != limit
        || check.state.is_some_and(|s| s != snapshot.state)
        || check.step.is_some_and(|s| s != snapshot.step)
        || check.menu_state.is_some_and(|m| m != snapshot.menu)
        || check
            .option_entry
            .is_some_and(|e| e != snapshot.option_entry)
        || shared.base_lit < check.min_lit_pixels
    {
        return Err(format!(
            "touch final checkpoint failed: {snapshot:?}, base_lit={}",
            shared.base_lit
        ));
    }
    println!(
        "TOUCH_RESULT PASS checkpoints={} base_lit={} tick={}",
        shared.checked, shared.base_lit, snapshot.tick
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn platform_safe_area_menu_walking_and_lifecycle_probe() {
        for view in [default_view(), Viewport::default()] {
            let (handle, mut pad) = PlatformTouch::new(view).unwrap();
            handle.smoke_check(&mut pad).unwrap();
            let pixels = &mut vec![0; 320 * 224];
            handle.draw(pixels, 320, 224);
            assert!(pixels.iter().any(|p| *p != 0));
        }
    }
    #[test]
    fn cancel_releases_even_on_repeated_logic_read() {
        let mut pad = test_pad();
        assert_ne!(
            pad.sample_snapshot(0, title(0), Config::default(), scene(title(0))),
            PadState::default().ps1_packet()
        );
        pad.shared.lock().unwrap().queue.push_back(Input::Cancel);
        assert_eq!(
            pad.sample_snapshot(0, title(0), Config::default(), scene(title(0))),
            PadState::default().ps1_packet()
        );
    }
    fn test_pad() -> TouchPad {
        let shared = Arc::new(Mutex::new(Shared {
            live: false,
            clock_ms: 0,
            queue: VecDeque::new(),
            native_ids: HashSet::new(),
            mouse: Point::default(),
            mouse_down: false,
            overlay: None,
            checks: Vec::new(),
            checked: 0,
            error: None,
            base_lit: 0,
        }));
        let mut events = Vec::new();
        for id in [1, 2] {
            for phase in [Phase::Down, Phase::Up] {
                events.push(Record::Touch {
                    tick: 0,
                    id,
                    phase,
                    x: 552.0,
                    y: 416.0,
                });
            }
        }
        TouchPad {
            engine: Engine::new(Config::default(), default_view(), GameContext::default()).unwrap(),
            shared,
            events,
            cursor: 0,
            cached: None,
            commands: VecDeque::new(),
            previous_command: Buttons::default(),
            option_command: None,
            scene_key: (-1, -1, -1),
        }
    }
    fn title(tick: u64) -> Snapshot {
        Snapshot {
            tick,
            state: 7,
            step: 1,
            menu: 1,
            ..Snapshot::default()
        }
    }
    #[test]
    fn fast_ui_taps_keep_neutral_edges_and_repeated_reads_do_not_advance() {
        let mut pad = test_pad();
        let sample = |pad: &mut TouchPad, tick| {
            pad.sample_snapshot(tick, title(tick), Config::default(), scene(title(tick)))
        };
        let pressed = PadState {
            buttons: Buttons::CROSS,
            ..PadState::default()
        }
        .ps1_packet();
        assert_eq!(sample(&mut pad, 0), pressed);
        assert_eq!(sample(&mut pad, 0), pressed);
        assert_eq!(sample(&mut pad, 1), PadState::default().ps1_packet());
        assert_eq!(sample(&mut pad, 2), pressed);
        assert_eq!(sample(&mut pad, 3), PadState::default().ps1_packet());
    }
    #[test]
    fn interruption_flushes_pending_ui_and_tick_rewind_fails_closed() {
        let mut pad = test_pad();
        pad.sample_snapshot(0, title(0), Config::default(), scene(title(0)));
        pad.shared.lock().unwrap().queue.push_back(Input::Cancel);
        assert_eq!(
            pad.sample_snapshot(1, title(1), Config::default(), scene(title(1))),
            PadState::default().ps1_packet()
        );
        assert_eq!(
            pad.sample_snapshot(2, title(2), Config::default(), scene(title(2))),
            PadState::default().ps1_packet()
        );
        pad.sample_snapshot(0, title(0), Config::default(), scene(title(0)));
        assert!(
            pad.shared
                .lock()
                .unwrap()
                .error
                .as_ref()
                .unwrap()
                .contains("rewound")
        );
    }
    #[test]
    fn native_config_uses_lowest_accepted_bit_and_source_options() {
        let mut bits = [0; 14];
        bits[0] = Buttons::CROSS.0 | Buttons::START.0;
        bits[4] = Buttons::R1.0;
        let b = native_bindings(bits);
        assert_eq!(b.enter, Buttons::START);
        assert_eq!(b.aim, Buttons::R1);
        assert_eq!(b.cancel, Buttons::CIRCLE);
    }
    #[test]
    fn resize_or_live_binding_change_discards_old_queued_ui_commands() {
        for rebind in [false, true] {
            let mut pad = test_pad();
            pad.sample_snapshot(0, title(0), Config::default(), scene(title(0)));
            let mut config = Config::default();
            if rebind {
                config.bindings.enter = Buttons::START;
            } else {
                pad.shared
                    .lock()
                    .unwrap()
                    .queue
                    .push_back(Input::View(Viewport {
                        width: 800.0,
                        ..default_view()
                    }));
            }
            assert_eq!(
                pad.sample_snapshot(1, title(1), config, scene(title(1))),
                PadState::default().ps1_packet()
            );
            assert_eq!(
                pad.sample_snapshot(2, title(2), config, scene(title(2))),
                PadState::default().ps1_packet()
            );
        }
    }
    #[test]
    fn overlay_hides_movies_and_preserves_display_color_order() {
        let overlay = Overlay {
            view: default_view(),
            hidden: false,
            regions: vec![(
                Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 640.0,
                    height: 448.0,
                },
                "",
            )],
            stick: None,
        };
        let mut pixels = vec![0xff0000; 4];
        draw_overlay(&mut pixels, 2, 2, &overlay);
        assert_eq!(pixels[3], 0xfc1111);
        let before = pixels.clone();
        draw_overlay(
            &mut pixels,
            2,
            2,
            &Overlay {
                hidden: true,
                ..overlay
            },
        );
        assert_eq!(pixels, before);
        assert!(
            scene(Snapshot {
                state: 6,
                ..Snapshot::default()
            })
            .available
            .skip
        );
        assert_eq!(
            scene(Snapshot {
                state: 11,
                sys: 3,
                ..Snapshot::default()
            })
            .mode,
            Mode::Map
        );
    }
    #[test]
    fn checkpoint_mismatch_is_a_failure_even_when_endpoint_is_visible() {
        let mut shared = Shared {
            live: false,
            clock_ms: 0,
            queue: VecDeque::new(),
            native_ids: HashSet::new(),
            mouse: Point::default(),
            mouse_down: false,
            overlay: None,
            checks: vec![Record::Check {
                tick: 5,
                id: "difficulty".into(),
                state: 7,
                step: 1,
                menu: Some(3),
                option_entry: None,
            }],
            checked: 0,
            error: None,
            base_lit: 1000,
        };
        shared.observe(Snapshot {
            tick: 5,
            state: 7,
            step: 1,
            menu: 1,
            ..Snapshot::default()
        });
        assert!(shared.error.is_some());
        assert_eq!(shared.checked, 1);
    }
}
