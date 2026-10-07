//! Original shapes only: no game bytes, art, or screenshots.
use font8x8::UnicodeFonts;
use sh_touch::{replay, *};
use softbuffer::{Context, Surface};
use std::{
    collections::{BTreeSet, VecDeque},
    fs::File,
    io::{BufReader, Write},
    num::NonZeroU32,
    sync::Arc,
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton, TouchPhase, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

struct Harness {
    window: Option<Arc<Window>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    engine: Engine,
    start: Instant,
    mouse: Point,
    mouse_down: bool,
    last_native_touch: Option<Instant>,
    native_contacts: BTreeSet<u64>,
    toolbar_touches: BTreeSet<u64>,
    last_draw: Instant,
    mode_index: usize,
    output: FrameOutput,
    last_ui: String,
    frames: u32,
    smoke: bool,
    replay: Option<VecDeque<replay::Record>>,
    record: Option<File>,
    snapshot: Option<String>,
    failed: Option<String>,
}

const MODES: [Mode; 10] = [
    Mode::Exploring,
    Mode::Aiming,
    Mode::Menu,
    Mode::Inventory,
    Mode::Map,
    Mode::Puzzle(PuzzleKind::Piano),
    Mode::Puzzle(PuzzleKind::MotelKeypad),
    Mode::Puzzle(PuzzleKind::NowhereLightPanels),
    Mode::Cutscene,
    Mode::Dialogue,
];

impl Harness {
    fn new() -> Self {
        Self {
            window: None,
            surface: None,
            engine: Engine::new(
                Config::default(),
                Viewport::default(),
                GameContext {
                    mode: Mode::Exploring,
                    available: Availability::all(),
                },
            )
            .expect("default engine"),
            start: Instant::now(),
            mouse: Point::default(),
            mouse_down: false,
            last_native_touch: None,
            mode_index: 0,
            output: FrameOutput::default(),
            native_contacts: BTreeSet::new(),
            toolbar_touches: BTreeSet::new(),
            last_draw: Instant::now(),
            last_ui: String::new(),
            frames: 0,
            smoke: false,
            replay: None,
            record: None,
            snapshot: None,
            failed: None,
        }
    }

    fn time(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    fn record(&mut self, record: &replay::Record) {
        if let Some(file) = &mut self.record {
            let result = serde_json::to_writer(&mut *file, record)
                .and_then(|()| file.write_all(b"\n").map_err(serde_json::Error::io));
            if let Err(e) = result {
                self.failed = Some(format!("record: {e}"));
            }
        }
    }

    fn event(&mut self, id: u64, phase: Phase, p: Point) {
        if self.replay.is_some() {
            return;
        }
        let event = TouchEvent {
            id,
            phase,
            x: p.x,
            y: p.y,
            time_ms: self.time(),
        };
        self.record(&replay::Record::Touch(event));
        if let Err(e) = self.engine.touch(event) {
            eprintln!("input: {e}");
        }
    }

    fn change_mode(&mut self) {
        if self.replay.is_some() {
            return;
        }
        self.mode_index = (self.mode_index + 1) % MODES.len();
        let context = GameContext {
            mode: MODES[self.mode_index],
            available: Availability::all(),
        };
        self.engine.set_context(context);
        self.mouse_down = false;
        self.record(&replay::Record::Context { context });
        self.install_demo_targets();
    }

    fn install_demo_targets(&mut self) {
        let mode = self.engine.context().mode;
        let content = self.engine.layout().content;
        let requests: Vec<UiAction> = match mode {
            Mode::Menu => vec![
                UiAction::Activate { id: 0 },
                UiAction::Select { id: 1 },
                UiAction::OpenOptions,
                UiAction::Adjust { id: 3, delta: 1 },
                UiAction::ReturnToTitle,
            ],
            Mode::Inventory => [
                InventoryOp::Select,
                InventoryOp::Use,
                InventoryOp::Equip,
                InventoryOp::Unequip,
                InventoryOp::Examine,
                InventoryOp::Reload,
                InventoryOp::Toggle,
            ]
            .into_iter()
            .map(|op| UiAction::Inventory { id: 42, op })
            .collect(),
            Mode::Puzzle(kind) => (0..12)
                .map(|element| UiAction::Puzzle { kind, element })
                .collect(),
            Mode::Dialogue => vec![UiAction::Select { id: 0 }, UiAction::Select { id: 1 }],
            _ => Vec::new(),
        };
        let targets: Vec<UiTarget> = requests
            .into_iter()
            .enumerate()
            .map(|(i, request)| UiTarget {
                bounds: Rect {
                    x: content.x + 22.0 + (i % 4) as f32 * 78.0,
                    y: content.y + 140.0 + (i / 4) as f32 * 60.0,
                    width: 64.0,
                    height: 48.0,
                },
                action: TargetAction::Ui { request },
                enabled: true,
            })
            .collect();
        self.engine
            .set_targets(targets.clone())
            .expect("demo targets");
        self.record(&replay::Record::Targets { targets });
    }

    fn cancel(&mut self) {
        self.engine.cancel_all();
        self.mouse_down = false;
        self.native_contacts.clear();
        self.toolbar_touches.clear();
        self.record(&replay::Record::CancelAll);
    }

    fn next_frame(&mut self) -> Result<(), String> {
        if let Some(records) = &mut self.replay {
            let mut batch = Vec::new();
            while let Some(record) = records.pop_front() {
                let frame = matches!(record, replay::Record::Frame { .. });
                batch.push(record);
                if frame {
                    break;
                }
            }
            if let Some(frame) = replay::run(&mut self.engine, &batch)?.pop() {
                self.output = frame;
            }
        } else {
            let time_ms = self.time();
            self.output = self.engine.frame(time_ms).map_err(|e| e.to_string())?;
            let expect = replay::ExpectedFrame {
                buttons: self.output.pad.buttons,
                left: self.output.pad.left,
                right: self.output.pad.right,
                ui_actions: self.output.ui_actions.clone(),
            };
            self.record(&replay::Record::Frame {
                time_ms,
                expect: Some(expect),
            });
        }
        if !self.output.ui_actions.is_empty() {
            self.last_ui = format!("{:?}", self.output.ui_actions);
        }
        Ok(())
    }

    fn draw(&mut self) -> Result<(), String> {
        let window = self.window.as_ref().ok_or("no window")?;
        let physical = window.inner_size();
        let (Some(w), Some(h)) = (
            NonZeroU32::new(physical.width),
            NonZeroU32::new(physical.height),
        ) else {
            return Ok(());
        };
        let surface = self.surface.as_mut().ok_or("no surface")?;
        surface.resize(w, h).map_err(|e| e.to_string())?;
        let mut buffer = surface.buffer_mut().map_err(|e| e.to_string())?;
        buffer.fill(0x0c1420);
        let scale = window.scale_factor() as f32;
        let mut canvas = Canvas {
            pixels: &mut buffer,
            width: w.get(),
            height: h.get(),
            scale,
        };
        let layout = self.engine.layout();
        canvas.outline(layout.content, 0x4c6476);
        canvas.text(
            layout.content.x + 8.0,
            layout.content.y + 10.0,
            "MODE NEXT  [TAB]   HOLD RIGHT=AIM  TAP=ACTION/ATTACK",
            0xaadbd0,
        );
        canvas.text(
            layout.content.x + 8.0,
            layout.content.y + 32.0,
            &format!(
                "CONTEXT {:?} | PAD {:04X}",
                self.engine.context().mode,
                self.output.pad.buttons.0
            ),
            0xffffff,
        );
        canvas.text(
            layout.content.x + 8.0,
            layout.content.y + 50.0,
            &format!(
                "LEFT {:.3} {:.3} | RIGHT {:.3} {:.3}",
                self.output.pad.left.x,
                self.output.pad.left.y,
                self.output.pad.right.x,
                self.output.pad.right.y
            ),
            0xf6c978,
        );
        canvas.text(
            layout.content.x + 8.0,
            layout.content.y + 68.0,
            &format!("WIRE {:?}", self.output.pad.ps1_packet()),
            0xffffff,
        );
        let ui_line: String = format!("UI {}", self.last_ui)
            .chars()
            .take(((layout.content.width - 16.0) / 8.0) as usize)
            .collect();
        canvas.text(
            layout.content.x + 8.0,
            layout.content.y + 86.0,
            &ui_line,
            0xaadbd0,
        );
        if self.engine.context().mode.gameplay() {
            canvas.text(
                layout.content.x + 24.0,
                layout.content.y + 130.0,
                "FLOATING STICK",
                0x7eacbd,
            );
            canvas.text(
                layout.right_zone.x + 16.0,
                layout.content.y + 130.0,
                "TAP / HOLD AREA",
                0x7eacbd,
            );
        }
        for (i, target) in self.engine.targets().iter().enumerate() {
            canvas.fill_rect(target.bounds, 0x244052);
            canvas.outline(target.bounds, 0x649caa);
            canvas.text(
                target.bounds.x + 5.0,
                target.bounds.y + 19.0,
                &format!("ITEM {i}"),
                0xffffff,
            );
        }
        for region in &layout.controls {
            canvas.fill_rect(region.bounds, 0x274657);
            canvas.outline(region.bounds, 0x84d1bf);
            canvas.text(
                region.bounds.x + 2.0,
                region.bounds.y + 20.0,
                region.control.label(),
                0xffffff,
            );
        }
        if let Some((origin, point)) = self.engine.stick_visual() {
            canvas.circle(
                origin,
                self.engine.config().tuning.stick_radius,
                0x64b4bd,
                false,
            );
            canvas.circle(
                origin,
                self.engine.config().tuning.dead_zone,
                0x466477,
                false,
            );
            let d = (origin.x - point.x).hypot(origin.y - point.y);
            let factor = (self.engine.config().tuning.stick_radius / d.max(1.0)).min(1.0);
            canvas.circle(
                Point::new(
                    origin.x + (point.x - origin.x) * factor,
                    origin.y + (point.y - origin.y) * factor,
                ),
                14.0,
                0xa4ecd0,
                true,
            );
        }
        if self
            .output
            .pad
            .buttons
            .contains(self.engine.config().bindings.aim)
        {
            canvas.circle(
                Point::new(layout.right_zone.x + 170.0, layout.content.y + 195.0),
                30.0,
                0xf6c978,
                false,
            );
        }
        if self.frames == 9
            && let Some(path) = &self.snapshot
        {
            let mut file = File::create(path).map_err(|e| e.to_string())?;
            write!(file, "P6\n{} {}\n255\n", w, h).map_err(|e| e.to_string())?;
            for pixel in buffer.iter() {
                file.write_all(&[(pixel >> 16) as u8, (pixel >> 8) as u8, *pixel as u8])
                    .map_err(|e| e.to_string())?;
            }
        }
        buffer.present().map_err(|e| e.to_string())?;
        window.set_title(&format!(
            "Touch harness | {:?} | pad {:04X} | left {:.2},{:.2} | {}",
            self.engine.context().mode,
            self.output.pad.buttons.0,
            self.output.pad.left.x,
            self.output.pad.left.y,
            self.last_ui
        ));
        self.frames += 1;
        self.last_draw = Instant::now();
        Ok(())
    }
}

impl ApplicationHandler for Harness {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let result = (|| -> Result<(), String> {
            let window = Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_title("Touch harness")
                            .with_inner_size(LogicalSize::new(956.0, 440.0))
                            .with_min_inner_size(LogicalSize::new(640.0, 400.0)),
                    )
                    .map_err(|e| e.to_string())?,
            );
            let context = Context::new(window.clone()).map_err(|e| e.to_string())?;
            self.surface = Some(Surface::new(&context, window.clone()).map_err(|e| e.to_string())?);
            self.window = Some(window);
            Ok(())
        })();
        if let Err(e) = result {
            self.failed = Some(e);
            event_loop.exit();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Focused(false) if self.replay.is_none() => self.cancel(),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. }
                if self.replay.is_none() =>
            {
                if let Some(window) = &self.window {
                    let size = window.inner_size().to_logical::<f32>(window.scale_factor());
                    let mut view = self.engine.viewport();
                    view.width = size.width;
                    view.height = size.height;
                    if self.engine.set_viewport(view).is_ok() {
                        self.mouse_down = false;
                        self.record(&replay::Record::Viewport { viewport: view });
                        self.install_demo_targets();
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(window) = &self.window {
                    let p = position.to_logical::<f32>(window.scale_factor());
                    self.mouse = Point::new(p.x, p.y);
                    if self.mouse_down {
                        self.event(0, Phase::Move, self.mouse);
                    }
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                // Windows may synthesize mouse from touch: do not create a second finger.
                if !self.native_contacts.is_empty()
                    || self
                        .last_native_touch
                        .is_some_and(|t| t.elapsed() < Duration::from_millis(500))
                {
                    return;
                }
                if state == ElementState::Pressed {
                    let c = self.engine.layout().content;
                    if (Rect {
                        x: c.x + 8.0,
                        y: c.y + 8.0,
                        width: 120.0,
                        height: 18.0,
                    })
                    .contains(self.mouse)
                    {
                        self.change_mode();
                        return;
                    }
                    self.mouse_down = true;
                    self.event(0, Phase::Down, self.mouse);
                } else if self.mouse_down {
                    self.mouse_down = false;
                    self.event(0, Phase::Up, self.mouse);
                }
            }
            WindowEvent::Touch(touch) => {
                self.last_native_touch = Some(Instant::now());
                if self.mouse_down {
                    self.event(0, Phase::Cancel, self.mouse);
                    self.mouse_down = false;
                }
                let Some(id) = touch.id.checked_add(1) else {
                    return;
                };
                if touch.phase == TouchPhase::Started {
                    self.native_contacts.insert(id);
                }
                if matches!(touch.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                    self.native_contacts.remove(&id);
                }
                if let Some(window) = &self.window {
                    let p = touch.location.to_logical::<f32>(window.scale_factor());
                    let point = Point::new(p.x, p.y);
                    let c = self.engine.layout().content;
                    if touch.phase == TouchPhase::Started
                        && (Rect {
                            x: c.x + 8.0,
                            y: c.y + 8.0,
                            width: 120.0,
                            height: 18.0,
                        })
                        .contains(point)
                    {
                        self.toolbar_touches.insert(id);
                        self.change_mode();
                        return;
                    }
                    if self.toolbar_touches.contains(&id) {
                        if matches!(touch.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                            self.toolbar_touches.remove(&id);
                        }
                        return;
                    }
                    let phase = match touch.phase {
                        TouchPhase::Started => Phase::Down,
                        TouchPhase::Moved => Phase::Move,
                        TouchPhase::Ended => Phase::Up,
                        TouchPhase::Cancelled => Phase::Cancel,
                    };
                    self.event(id, phase, point);
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed && !event.repeat =>
            {
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::Tab) => self.change_mode(),
                    PhysicalKey::Code(KeyCode::Escape) => self.cancel(),
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.next_frame().and_then(|()| self.draw()) {
                    self.failed = Some(e);
                    event_loop.exit();
                }
                if self.smoke
                    && self.frames >= 10
                    && self.replay.as_ref().is_none_or(VecDeque::is_empty)
                {
                    println!(
                        "HARNESS_PRESENTED {} FRAMES; REPLAY_OK; PAD={:04X}",
                        self.frames, self.output.pad.buttons.0
                    );
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let deadline = self.last_draw + Duration::from_millis(33);
        if Instant::now() >= deadline
            && let Some(window) = &self.window
        {
            window.request_redraw();
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
    }

    fn suspended(&mut self, _: &ActiveEventLoop) {
        self.cancel();
        self.surface = None;
        self.window = None;
    }
}

struct Canvas<'a> {
    pixels: &'a mut [u32],
    width: u32,
    height: u32,
    scale: f32,
}
impl Canvas<'_> {
    fn pixel(&mut self, x: i32, y: i32, color: u32) {
        if x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height {
            self.pixels[y as usize * self.width as usize + x as usize] = color;
        }
    }
    fn fill_rect(&mut self, r: Rect, color: u32) {
        for y in (r.y * self.scale).max(0.0) as i32
            ..((r.y + r.height) * self.scale).min(self.height as f32) as i32
        {
            for x in (r.x * self.scale).max(0.0) as i32
                ..((r.x + r.width) * self.scale).min(self.width as f32) as i32
            {
                self.pixel(x, y, color);
            }
        }
    }
    fn outline(&mut self, r: Rect, color: u32) {
        self.fill_rect(Rect { height: 1.0, ..r }, color);
        self.fill_rect(
            Rect {
                y: r.y + r.height - 1.0,
                height: 1.0,
                ..r
            },
            color,
        );
        self.fill_rect(Rect { width: 1.0, ..r }, color);
        self.fill_rect(
            Rect {
                x: r.x + r.width - 1.0,
                width: 1.0,
                ..r
            },
            color,
        );
    }
    fn circle(&mut self, p: Point, radius: f32, color: u32, filled: bool) {
        let r = radius * self.scale;
        let cx = p.x * self.scale;
        let cy = p.y * self.scale;
        for y in ((cy - r).max(0.0) as i32)..=((cy + r).min(self.height as f32 - 1.0) as i32) {
            for x in ((cx - r).max(0.0) as i32)..=((cx + r).min(self.width as f32 - 1.0) as i32) {
                let d = (x as f32 - cx).hypot(y as f32 - cy);
                if d <= r && (filled || d >= r - 2.0 * self.scale) {
                    self.pixel(x, y, color);
                }
            }
        }
    }
    fn text(&mut self, x: f32, y: f32, text: &str, color: u32) {
        for (i, c) in text.to_uppercase().chars().enumerate() {
            if let Some(glyph) = font8x8::BASIC_FONTS.get(c) {
                for (row, bits) in glyph.iter().enumerate() {
                    for col in 0..8 {
                        if bits & (1 << col) != 0 {
                            self.fill_rect(
                                Rect {
                                    x: x + (i * 8 + col) as f32,
                                    y: y + row as f32,
                                    width: 1.0,
                                    height: 1.0,
                                },
                                color,
                            );
                        }
                    }
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut harness = Harness::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--smoke" => harness.smoke = true,
            "--replay" => {
                let path = args.next().ok_or("--replay needs a path")?;
                harness.replay = Some(replay::parse(BufReader::new(File::open(path)?))?.into());
            }
            "--record" => {
                harness.record = Some(File::create(args.next().ok_or("--record needs a path")?)?)
            }
            "--snapshot" => {
                harness.snapshot = Some(args.next().ok_or("--snapshot needs a .ppm path")?)
            }
            _ => return Err(format!("unknown option {arg}").into()),
        }
    }
    if harness.replay.is_some() && harness.record.is_some() {
        return Err("record and replay are mutually exclusive".into());
    }
    harness.record(&replay::Record::Context {
        context: harness.engine.context(),
    });
    harness.record(&replay::Record::Viewport {
        viewport: harness.engine.viewport(),
    });
    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut harness)?;
    if let Some(error) = harness.failed {
        return Err(error.into());
    }
    if let Some(file) = &mut harness.record {
        file.flush()?;
    }
    Ok(())
}
