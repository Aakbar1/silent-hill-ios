use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tuning {
    pub stick_radius: f32,
    pub dead_zone: f32,
    pub run_distance: f32,
    pub run_hysteresis: f32,
    pub tap_slop: f32,
    pub aim_hold_ms: u64,
    pub quick_turn_distance: f32,
    pub quick_turn_max_ms: u64,
    pub quick_turn_max_sideways: f32,
    pub chip_size: f32,
    pub chip_gap: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        // PORT: input affordances in points, provisional until device play-test.
        Self {
            stick_radius: 64.0,
            dead_zone: 12.0,
            run_distance: 46.0,
            run_hysteresis: 4.0,
            tap_slop: 12.0,
            aim_hold_ms: 280,
            quick_turn_distance: 72.0,
            quick_turn_max_ms: 220,
            quick_turn_max_sideways: 28.0,
            chip_size: 48.0,
            chip_gap: 8.0,
        }
    }
}

impl Tuning {
    fn valid(self) -> bool {
        let values = [
            self.stick_radius,
            self.dead_zone,
            self.run_distance,
            self.run_hysteresis,
            self.tap_slop,
            self.quick_turn_distance,
            self.quick_turn_max_sideways,
            self.chip_size,
            self.chip_gap,
        ];
        values.iter().all(|v| v.is_finite() && *v > 0.0)
            && self.dead_zone < self.run_distance - self.run_hysteresis
            && self.run_distance < self.stick_radius
            && self.quick_turn_distance > self.stick_radius
            && self.chip_size >= 44.0
            && self.aim_hold_ms > 0
            && self.quick_turn_max_ms > 0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AimRelease {
    WhileHeld,
    #[default]
    Latch,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub tuning: Tuning,
    pub bindings: Bindings,
    pub hand: Hand,
    pub aim_release: AimRelease,
    /// Match extraWalkRunCtrl; this is input translation, not a core option change.
    pub run_inverted: bool,
    /// Match extraWeaponCtrl == 0: emit edges on aim intent changes.
    pub weapon_toggle: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tuning: Tuning::default(),
            bindings: Bindings::default(),
            hand: Hand::Right,
            aim_release: AimRelease::Latch,
            run_inverted: false,
            weapon_toggle: false,
        }
    }
}

#[derive(Clone, Debug)]
enum Role {
    Stick { running: bool, turned: bool },
    Right { aiming: bool },
    Attack,
    Control(ControlRegion),
    Target(UiTarget),
    MapPan,
    Ignore,
}

#[derive(Clone, Debug)]
struct Contact {
    origin: Point,
    point: Point,
    start_ms: u64,
    max_distance: f32,
    observed: bool,
    role: Role,
}

/// Entirely host-independent. No static mutable state, platform clocks, or assets.
pub struct Engine {
    config: Config,
    view: Viewport,
    context: GameContext,
    layout: Layout,
    targets: Vec<UiTarget>,
    contacts: BTreeMap<u64, Contact>,
    pulses: VecDeque<PadState>,
    previous_pulse: Buttons,
    ui: Vec<UiAction>,
    latched_aim: bool,
    previous_aim: bool,
    time_ms: u64,
}

impl Engine {
    pub fn new(config: Config, view: Viewport, context: GameContext) -> Result<Self, InputError> {
        if !config.tuning.valid() {
            return Err(InputError::InvalidTuning);
        }
        Self::validate_view(config, view)?;
        let layout = Layout::new(
            view,
            context,
            config.hand,
            config.tuning.chip_size,
            config.tuning.chip_gap,
        );
        Ok(Self {
            config,
            view,
            context,
            layout,
            targets: Vec::new(),
            contacts: BTreeMap::new(),
            pulses: VecDeque::new(),
            previous_pulse: Buttons::default(),
            ui: Vec::new(),
            latched_aim: context.mode == Mode::Aiming,
            previous_aim: config.weapon_toggle && context.mode == Mode::Aiming,
            time_ms: 0,
        })
    }

    fn validate_view(config: Config, view: Viewport) -> Result<(), InputError> {
        view.validate()?;
        let minimum =
            3.0 * (config.tuning.chip_size + config.tuning.chip_gap) + config.tuning.chip_gap;
        if view.content().width < minimum || view.content().height < minimum {
            return Err(InputError::InvalidGeometry);
        }
        Ok(())
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }
    pub fn context(&self) -> GameContext {
        self.context
    }
    pub fn config(&self) -> Config {
        self.config
    }
    pub fn viewport(&self) -> Viewport {
        self.view
    }
    /// Install bindings/options after the core changes them. Captures and queued
    /// commands are invalidated, while the monotonic replay clock is retained.
    pub fn set_config(&mut self, config: Config) -> Result<(), InputError> {
        if !config.tuning.valid() {
            return Err(InputError::InvalidTuning);
        }
        Self::validate_view(config, self.view)?;
        if self.config != config {
            self.cancel_all();
            self.targets.clear();
            self.config = config;
            self.previous_aim = config.weapon_toggle && self.context.mode == Mode::Aiming;
            self.rebuild_layout();
        }
        Ok(())
    }
    pub fn targets(&self) -> &[UiTarget] {
        &self.targets
    }

    pub fn stick_visual(&self) -> Option<(Point, Point)> {
        self.contacts.values().find_map(|c| {
            matches!(c.role, Role::Stick { turned: false, .. }).then_some((c.origin, c.point))
        })
    }

    fn rebuild_layout(&mut self) {
        self.layout = Layout::new(
            self.view,
            self.context,
            self.config.hand,
            self.config.tuning.chip_size,
            self.config.tuning.chip_gap,
        );
    }

    /// Invalidates captures on rotation/resize. Host must re-register UI regions.
    pub fn set_viewport(&mut self, view: Viewport) -> Result<(), InputError> {
        Self::validate_view(self.config, view)?;
        if self.view != view {
            self.cancel_all();
            self.targets.clear();
            self.view = view;
            self.rebuild_layout();
        }
        Ok(())
    }

    /// Context changes suppress old fingers and queued actions. Core mode and
    /// availability must be installed before accepting new events for that scene.
    pub fn set_context(&mut self, context: GameContext) {
        if self.context != context {
            if !(self.context.mode.gameplay()
                && context.mode.gameplay()
                && self.context.available == context.available)
            {
                self.cancel_all();
                self.targets.clear();
                // The game drops combat on entry into a non-gameplay system state.
                if !context.mode.gameplay() {
                    self.previous_aim = false;
                }
            }
            self.context = context;
            self.rebuild_layout();
        }
    }

    /// Atomic replacement; the first enabled target wins if regions overlap.
    /// Reserved chips always win over host targets. Stable lists are a no-op.
    pub fn set_targets(&mut self, targets: Vec<UiTarget>) -> Result<(), InputError> {
        for t in &targets {
            let content = self.layout.content;
            if !t.bounds.valid()
                || t.bounds.x < content.x
                || t.bounds.y < content.y
                || t.bounds.x + t.bounds.width > content.x + content.width
                || t.bounds.y + t.bounds.height > content.y + content.height
            {
                return Err(InputError::InvalidTarget);
            }
            match &t.action {
                TargetAction::Ui {
                    request: UiAction::MapPan { dx, dy },
                } if !dx.is_finite() || !dy.is_finite() => return Err(InputError::InvalidTarget),
                TargetAction::Pad { state, .. }
                    if !state.left.finite()
                        || !state.right.finite()
                        || [state.left.x, state.left.y, state.right.x, state.right.y]
                            .iter()
                            .any(|v| v.abs() > 1.0) =>
                {
                    return Err(InputError::InvalidTarget);
                }
                _ => {}
            }
        }
        if self.targets != targets {
            self.cancel_all();
            self.targets = targets;
        }
        Ok(())
    }

    /// Lifecycle/focus loss: release everything, discard taps and latched aim.
    pub fn cancel_all(&mut self) {
        self.contacts.clear();
        self.pulses.clear();
        self.ui.clear();
        self.latched_aim = false;
        self.previous_pulse = Buttons::default();
    }

    fn advance(&mut self, time_ms: u64) -> Result<(), InputError> {
        if time_ms < self.time_ms {
            return Err(InputError::TimeWentBackwards);
        }
        self.time_ms = time_ms;
        for c in self.contacts.values_mut() {
            if let Role::Right { aiming } = &mut c.role
                && self.context.available.combat
                && c.max_distance <= self.config.tuning.tap_slop
                && time_ms - c.start_ms >= self.config.tuning.aim_hold_ms
            {
                *aiming = true;
            }
        }
        Ok(())
    }

    pub fn touch(&mut self, event: TouchEvent) -> Result<(), InputError> {
        let p = Point::new(event.x, event.y);
        if !p.finite() {
            return Err(InputError::NonFinitePosition);
        }
        if event.phase == Phase::Down && self.contacts.contains_key(&event.id) {
            return Err(InputError::DuplicateContact);
        }
        if event.phase == Phase::Down && self.contacts.len() >= 10 {
            return Err(InputError::TooManyContacts);
        }
        // Late up/cancel from a scene/focus invalidation is harmless.
        if event.phase != Phase::Down && !self.contacts.contains_key(&event.id) {
            return if matches!(event.phase, Phase::Up | Phase::Cancel) {
                Ok(())
            } else {
                Err(InputError::UnknownContact)
            };
        }
        if event.time_ms < self.time_ms {
            return Err(InputError::TimeWentBackwards);
        }
        if event.phase == Phase::Down {
            self.advance(event.time_ms)?;
            let role = self.capture(p);
            self.contacts.insert(
                event.id,
                Contact {
                    origin: p,
                    point: p,
                    start_ms: event.time_ms,
                    max_distance: 0.0,
                    observed: false,
                    role,
                },
            );
            return Ok(());
        }
        let mut c = self.contacts.remove(&event.id).expect("contact validated");
        let delta = Point::new(p.x - c.point.x, p.y - c.point.y);
        c.point = p;
        c.max_distance = c.max_distance.max(p.distance(c.origin));
        self.advance(event.time_ms)?;
        if event.phase == Phase::Cancel {
            return Ok(());
        }
        if let Role::Right { aiming } = &mut c.role
            && self.context.available.combat
            && c.max_distance <= self.config.tuning.tap_slop
            && self.time_ms - c.start_ms >= self.config.tuning.aim_hold_ms
        {
            *aiming = true;
        }
        if let Role::Stick { running, turned } = &mut c.role {
            let dx = p.x - c.origin.x;
            let dy = p.y - c.origin.y;
            if !*turned
                && self.time_ms - c.start_ms <= self.config.tuning.quick_turn_max_ms
                && dy >= self.config.tuning.quick_turn_distance
                && dx.abs() <= self.config.tuning.quick_turn_max_sideways
            {
                // Both step taps, not down+run: Player_CharaTurn_0/_1.
                self.pulse(self.config.bindings.step_left | self.config.bindings.step_right);
                *turned = true;
            }
            let threshold = self.config.tuning.run_distance
                - if *running {
                    self.config.tuning.run_hysteresis
                } else {
                    0.0
                };
            *running = p.distance(c.origin) >= threshold;
        }
        if matches!(c.role, Role::MapPan) && delta != Point::default() {
            self.ui.push(UiAction::MapPan {
                dx: delta.x,
                dy: delta.y,
            });
        }
        if event.phase == Phase::Up {
            self.release(c);
        } else {
            self.contacts.insert(event.id, c);
        }
        Ok(())
    }

    fn capture(&self, p: Point) -> Role {
        if !self.layout.content.contains(p) {
            return Role::Ignore;
        }
        if let Some(region) = self.layout.hit(p) {
            return Role::Control(region);
        }
        if let Some(target) = self
            .targets
            .iter()
            .find(|t| t.enabled && t.bounds.contains(p))
        {
            return Role::Target(target.clone());
        }
        if self.context.mode == Mode::Map {
            return Role::MapPan;
        }
        if !self.context.mode.gameplay() {
            return Role::Ignore;
        }
        if self.layout.left_zone.contains(p) && self.context.available.movement {
            if self
                .contacts
                .values()
                .any(|c| matches!(c.role, Role::Stick { .. }))
            {
                return Role::Ignore;
            }
            return Role::Stick {
                running: false,
                turned: false,
            };
        }
        if self.layout.right_zone.contains(p) {
            if self.aim_intent() || self.context.mode == Mode::Aiming {
                return if self.context.available.combat {
                    Role::Attack
                } else {
                    Role::Ignore
                };
            }
            // Only one aim owner. Additional unrecognised fingers cannot fire accidentally.
            if self
                .contacts
                .values()
                .any(|c| matches!(c.role, Role::Right { .. }))
            {
                return Role::Ignore;
            }
            return Role::Right { aiming: false };
        }
        Role::Ignore
    }

    fn pulse(&mut self, buttons: Buttons) {
        self.pulses.push_back(PadState {
            buttons,
            ..PadState::default()
        });
    }

    fn aim_intent(&self) -> bool {
        self.latched_aim || self.contacts.values().any(|c| matches!(c.role, Role::Right { aiming: true } | Role::Attack)
            || matches!(c.role, Role::Control(r) if r.control == Control::AimToggle && self.config.aim_release == AimRelease::WhileHeld))
    }

    fn release(&mut self, c: Contact) {
        let tap = c.max_distance <= self.config.tuning.tap_slop;
        match c.role {
            Role::Right { aiming: true } if self.config.aim_release == AimRelease::Latch => {
                // PORT: retaining aim intent permits one-finger attack after a hold.
                // It still sends the game's ordinary aim bit and auto-target stays in C.
                self.latched_aim = true;
            }
            Role::Right { aiming: false } if tap => self.pulse(self.config.bindings.action),
            Role::Attack if !c.observed => self.pulse(
                self.config.bindings.action
                    | if self.config.weapon_toggle {
                        Buttons::default()
                    } else {
                        self.config.bindings.aim
                    },
            ),
            Role::Control(r)
                if tap
                    && r.bounds.contains(c.point)
                    && (!c.observed
                        || !matches!(
                            r.control,
                            Control::StepLeft | Control::StepRight | Control::View
                        )) =>
            {
                self.activate_control(r.control)
            }
            Role::Target(t) if tap && t.bounds.contains(c.point) => match t.action {
                TargetAction::Ui { request } => self.ui.push(request),
                TargetAction::Pad { state, hold } if !hold || !c.observed => {
                    self.pulses.push_back(state)
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn activate_control(&mut self, control: Control) {
        let b = self.config.bindings;
        match control {
            Control::Flashlight => self.pulse(b.light),
            Control::Radio => self.ui.push(UiAction::ToggleRadio),
            Control::Map => self.pulse(b.map),
            Control::Inventory => self.pulse(b.item),
            Control::Pause => self.pulse(b.pause),
            Control::StepLeft => self.pulse(b.step_left),
            Control::StepRight => self.pulse(b.step_right),
            Control::View => self.pulse(b.view),
            Control::AimToggle if self.config.aim_release == AimRelease::Latch => {
                self.latched_aim = !self.latched_aim
            }
            Control::AimToggle => {}
            Control::Back => self.ui.push(UiAction::Cancel),
            Control::Confirm => self.ui.push(UiAction::Confirm),
            Control::Continue => self.ui.push(UiAction::Continue),
            Control::Skip => self.ui.push(UiAction::Skip),
            Control::FloorUp => self.ui.push(UiAction::MapFloor { delta: 1 }),
            Control::FloorDown => self.ui.push(UiAction::MapFloor { delta: -1 }),
        }
    }

    pub fn frame(&mut self, time_ms: u64) -> Result<FrameOutput, InputError> {
        self.advance(time_ms)?;
        let mut pad = PadState::default();
        let aim = self.context.available.combat && self.aim_intent();
        if self.config.weapon_toggle {
            if aim != self.previous_aim {
                self.pulse(self.config.bindings.aim);
            }
        } else if aim {
            pad.buttons |= self.config.bindings.aim;
        }
        self.previous_aim = aim;
        for c in self.contacts.values_mut() {
            match &c.role {
                Role::Stick {
                    running,
                    turned: false,
                } => {
                    let dx = c.point.x - c.origin.x;
                    let dy = c.point.y - c.origin.y;
                    let d = dx.hypot(dy);
                    if d > self.config.tuning.dead_zone {
                        // Preserve the game's own 16/128 analog dead zone; don't
                        // produce D-pad+analog simultaneously (joy.c double paths).
                        let mag = (0.20
                            + 0.80
                                * ((d - self.config.tuning.dead_zone)
                                    / (self.config.tuning.stick_radius
                                        - self.config.tuning.dead_zone))
                                    .clamp(0.0, 1.0))
                        .min(1.0);
                        pad.left = Point::new(dx / d * mag, dy / d * mag);
                        if *running != self.config.run_inverted {
                            pad.buttons |= self.config.bindings.run;
                        }
                    }
                }
                Role::Attack => {
                    pad.buttons |= self.config.bindings.action;
                    c.observed = true;
                }
                Role::Control(r)
                    if r.bounds.contains(c.point)
                        && c.max_distance <= self.config.tuning.tap_slop =>
                {
                    match r.control {
                        Control::View => {
                            pad.buttons |= self.config.bindings.view;
                            c.observed = true;
                        }
                        Control::StepLeft => {
                            pad.buttons |= self.config.bindings.step_left;
                            c.observed = true;
                        }
                        Control::StepRight => {
                            pad.buttons |= self.config.bindings.step_right;
                            c.observed = true;
                        }
                        _ => {}
                    }
                }
                Role::Target(UiTarget {
                    action: TargetAction::Pad { state, hold: true },
                    ..
                }) if c.max_distance <= self.config.tuning.tap_slop => {
                    pad.buttons |= state.buttons;
                    if state.left != Point::default() {
                        pad.left = state.left;
                    }
                    if state.right != Point::default() {
                        pad.right = state.right;
                    }
                    c.observed = true;
                }
                _ => {}
            }
        }
        // Each discrete command gets one logic tick. Repeated identical commands
        // require a neutral tick so the original clicked flags see distinct edges.
        if self
            .pulses
            .front()
            .is_some_and(|p| p.buttons.0 & self.previous_pulse.0 == 0)
        {
            let pulse = self.pulses.pop_front().expect("front exists");
            pad.buttons |= pulse.buttons;
            if pulse.left != Point::default() {
                pad.left = pulse.left;
            }
            if pulse.right != Point::default() {
                pad.right = pulse.right;
            }
            self.previous_pulse = pulse.buttons;
        } else {
            self.previous_pulse = Buttons::default();
        }
        Ok(FrameOutput {
            pad,
            ui_actions: std::mem::take(&mut self.ui),
        })
    }
}
