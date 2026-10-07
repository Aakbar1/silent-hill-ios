use crate::Point;
use serde::{Deserialize, Serialize};

/// Active-high bits matching include/bodyprog/sys/joy.h. Wire data is active-low.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Buttons(pub u16);

impl Buttons {
    pub const SELECT: Self = Self(1 << 0);
    pub const L3: Self = Self(1 << 1);
    pub const R3: Self = Self(1 << 2);
    pub const START: Self = Self(1 << 3);
    pub const UP: Self = Self(1 << 4);
    pub const RIGHT: Self = Self(1 << 5);
    pub const DOWN: Self = Self(1 << 6);
    pub const LEFT: Self = Self(1 << 7);
    pub const L2: Self = Self(1 << 8);
    pub const R2: Self = Self(1 << 9);
    pub const L1: Self = Self(1 << 10);
    pub const R1: Self = Self(1 << 11);
    pub const TRIANGLE: Self = Self(1 << 12);
    pub const CIRCLE: Self = Self(1 << 13);
    pub const CROSS: Self = Self(1 << 14);
    pub const SQUARE: Self = Self(1 << 15);

    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl std::ops::BitOr for Buttons {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for Buttons {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Tank controls: negative left.y = forward; left.x = turn, NOT camera-relative strafe.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PadState {
    pub buttons: Buttons,
    pub left: Point,
    pub right: Point,
}

impl PadState {
    /// Byte layout read by Joy_ReadP1 / Joy_ControllerDataUpdate.
    /// Connected DualShock (0x73), four words; neutral axes are 128.
    pub fn ps1_packet(self) -> [u8; 8] {
        let active_low = (!self.buttons.0).to_le_bytes();
        fn axis(v: f32) -> u8 {
            if !v.is_finite() {
                return 128;
            }
            (128.0 + v.clamp(-1.0, 1.0) * if v < 0.0 { 128.0 } else { 127.0 }).round() as u8
        }
        [
            0,
            0x73,
            active_low[0],
            active_low[1],
            axis(self.right.x),
            axis(self.right.y),
            axis(self.left.x),
            axis(self.left.y),
        ]
    }
}

/// Use the actual game's bindings; selecting a nonzero bit avoids simultaneous
/// enter/cancel alternatives. USA presets 0/1/2 are from settings_reset.c.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bindings {
    pub action: Buttons,
    pub aim: Buttons,
    pub light: Buttons,
    pub run: Buttons,
    pub view: Buttons,
    pub step_left: Buttons,
    pub step_right: Buttons,
    pub pause: Buttons,
    pub item: Buttons,
    pub map: Buttons,
    pub enter: Buttons,
    pub cancel: Buttons,
    pub skip: Buttons,
}

impl Default for Bindings {
    fn default() -> Self {
        Self::usa_preset(0).expect("preset zero exists")
    }
}

impl Bindings {
    pub fn usa_preset(preset: u8) -> Option<Self> {
        if preset > 2 {
            return None;
        }
        Some(Self {
            action: Buttons::CROSS,
            aim: if preset == 1 {
                Buttons::R1
            } else {
                Buttons::R2
            },
            light: Buttons::CIRCLE,
            run: Buttons::SQUARE,
            view: if preset == 1 {
                Buttons::L1
            } else {
                Buttons::L2
            },
            step_left: if preset == 1 {
                Buttons::L2
            } else {
                Buttons::L1
            },
            step_right: if preset == 1 {
                Buttons::R2
            } else {
                Buttons::R1
            },
            pause: Buttons::START,
            item: if preset == 2 {
                Buttons::TRIANGLE
            } else {
                Buttons::SELECT
            },
            map: if preset == 2 {
                Buttons::SELECT
            } else {
                Buttons::TRIANGLE
            },
            enter: Buttons::CROSS,
            cancel: Buttons::CIRCLE,
            skip: Buttons::START,
        })
    }
}
