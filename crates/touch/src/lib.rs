//! Deterministic touch input in logical points; no window system or game data.
//! Call [`Engine::frame`] once for each **game logic** tick, including neutral ticks.
//! Deliver UI requests to the host before the corresponding core tick; never also
//! translate a direct request to a pad click (that would activate twice).
#![doc = include_str!("../README.md")]

mod engine;
pub mod game_replay;
mod layout;
mod pad;
pub mod replay;

pub use engine::*;
pub use layout::*;
pub use pad::*;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub(crate) fn distance(self, other: Self) -> f32 {
        (self.x - other.x).hypot(self.y - other.y)
    }

    pub(crate) fn finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Down,
    Move,
    Up,
    Cancel,
}

/// Time is monotonic integer milliseconds, in the same clock domain as frame().
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TouchEvent {
    pub id: u64,
    pub phase: Phase,
    pub x: f32,
    pub y: f32,
    pub time_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PuzzleKind {
    Piano,
    HospitalPlates,
    MotelKeypad,
    MotelSafe,
    NowhereLetters,
    NowhereZodiac,
    NowhereLightPanels,
    ItemUse,
    Switch,
    Other,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Exploring,
    Aiming,
    Menu,
    Inventory,
    Map,
    Puzzle(PuzzleKind),
    Cutscene,
    Dialogue,
}

impl Mode {
    pub fn gameplay(self) -> bool {
        matches!(self, Self::Exploring | Self::Aiming)
    }
}

/// Host-owned availability: hide unavailable chips, including during loading.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Availability {
    pub flashlight: bool,
    pub radio: bool,
    pub map: bool,
    pub inventory: bool,
    pub pause: bool,
    pub movement: bool,
    pub combat: bool,
    pub camera: bool,
    pub skip: bool,
}

impl Availability {
    /// Harness convenience only. The real host must read actual game state.
    pub fn all() -> Self {
        Self {
            flashlight: true,
            radio: true,
            map: true,
            inventory: true,
            pause: true,
            movement: true,
            combat: true,
            camera: true,
            skip: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameContext {
    pub mode: Mode,
    pub available: Availability,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InventoryOp {
    Select,
    Use,
    Equip,
    Unequip,
    Examine,
    Reload,
    Toggle,
}

/// Semantic UI input, NOT permission to change saves or solve a puzzle.
/// IDs and hit regions come from the host's current visible, enabled UI.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum UiAction {
    Select { id: u32 },
    Activate { id: u32 },
    Adjust { id: u32, delta: i32 },
    Inventory { id: u32, op: InventoryOp },
    Puzzle { kind: PuzzleKind, element: u32 },
    MapPan { dx: f32, dy: f32 },
    MapFloor { delta: i32 },
    ToggleRadio,
    OpenOptions,
    Confirm,
    Cancel,
    Continue,
    Skip,
    ReturnToTitle,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UiTarget {
    pub bounds: Rect,
    pub action: TargetAction,
    pub enabled: bool,
}

/// A labelled direct UI target or a legacy pad fallback. Hosts normally use Ui;
/// Pad preserves obscure/debug inputs without putting a gamepad on the screen.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TargetAction {
    Ui { request: UiAction },
    Pad { state: PadState, hold: bool },
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FrameOutput {
    pub pad: PadState,
    pub ui_actions: Vec<UiAction>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputError {
    InvalidGeometry,
    InvalidTuning,
    NonFinitePosition,
    TimeWentBackwards,
    DuplicateContact,
    UnknownContact,
    TooManyContacts,
    InvalidTarget,
}

impl std::fmt::Display for InputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for InputError {}
