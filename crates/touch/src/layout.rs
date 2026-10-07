use crate::{GameContext, InputError, Mode, Point};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn contains(self, p: Point) -> bool {
        p.x >= self.x && p.y >= self.y && p.x < self.x + self.width && p.y < self.y + self.height
    }

    pub fn center(self) -> Point {
        Point::new(self.x + self.width * 0.5, self.y + self.height * 0.5)
    }

    pub(crate) fn valid(self) -> bool {
        [self.x, self.y, self.width, self.height]
            .iter()
            .all(|v| v.is_finite())
            && self.width > 0.0
            && self.height > 0.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Insets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Viewport {
    pub width: f32,
    pub height: f32,
    pub safe: Insets,
}

impl Default for Viewport {
    fn default() -> Self {
        // PORT: provisional 6.9-inch landscape logical-point profile. Insets are
        // illustrative; UIKit/winit host must supply live bounds and safe area.
        Self {
            width: 956.0,
            height: 440.0,
            safe: Insets {
                top: 0.0,
                right: 62.0,
                bottom: 21.0,
                left: 62.0,
            },
        }
    }
}

impl Viewport {
    pub fn content(self) -> Rect {
        Rect {
            x: self.safe.left,
            y: self.safe.top,
            width: self.width - self.safe.left - self.safe.right,
            height: self.height - self.safe.top - self.safe.bottom,
        }
    }

    pub(crate) fn validate(self) -> Result<(), InputError> {
        if !self.content().valid()
            || !self.width.is_finite()
            || !self.height.is_finite()
            || [
                self.safe.top,
                self.safe.right,
                self.safe.bottom,
                self.safe.left,
            ]
            .iter()
            .any(|x| !x.is_finite() || *x < 0.0)
        {
            return Err(InputError::InvalidGeometry);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Hand {
    Left,
    #[default]
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Flashlight,
    Radio,
    Map,
    Inventory,
    Pause,
    StepLeft,
    StepRight,
    View,
    AimToggle,
    Back,
    Confirm,
    Continue,
    Skip,
    FloorUp,
    FloorDown,
}

impl Control {
    pub fn label(self) -> &'static str {
        match self {
            Self::Flashlight => "LIGHT",
            Self::Radio => "RADIO",
            Self::Map => "MAP",
            Self::Inventory => "ITEMS",
            Self::Pause => "PAUSE",
            Self::StepLeft => "STEP<",
            Self::StepRight => "STEP>",
            Self::View => "LOOK",
            Self::AimToggle => "AIM",
            Self::Back => "BACK",
            Self::Confirm => "OK",
            Self::Continue => "NEXT",
            Self::Skip => "SKIP",
            Self::FloorUp => "UP FL",
            Self::FloorDown => "DN FL",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlRegion {
    pub control: Control,
    pub bounds: Rect,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub content: Rect,
    pub left_zone: Rect,
    pub right_zone: Rect,
    pub controls: Vec<ControlRegion>,
}

impl Layout {
    pub(crate) fn new(
        view: Viewport,
        context: GameContext,
        hand: Hand,
        size: f32,
        gap: f32,
    ) -> Self {
        let content = view.content();
        let mut layout = Self {
            content,
            left_zone: Rect {
                width: content.width * 0.5,
                ..content
            },
            right_zone: Rect {
                x: content.x + content.width * 0.5,
                width: content.width * 0.5,
                ..content
            },
            controls: Vec::new(),
        };
        let a = context.available;
        let mut chips = Vec::new();
        if context.mode.gameplay() {
            for (visible, control) in [
                (a.flashlight, Control::Flashlight),
                (a.radio, Control::Radio),
                (a.map, Control::Map),
                (a.inventory, Control::Inventory),
                (a.pause, Control::Pause),
                (a.camera, Control::View),
                (a.movement, Control::StepLeft),
                (a.movement, Control::StepRight),
                (a.combat, Control::AimToggle),
            ] {
                if visible {
                    chips.push(control);
                }
            }
        } else {
            match context.mode {
                Mode::Map => chips.extend([
                    Control::Back,
                    Control::FloorUp,
                    Control::FloorDown,
                    Control::Confirm,
                ]),
                Mode::Cutscene => {
                    if a.skip {
                        chips.push(Control::Skip);
                    }
                }
                Mode::Dialogue => chips.extend([Control::Back, Control::Continue]),
                _ => chips.extend([Control::Back, Control::Confirm]),
            }
        }
        // PORT: labelled context chips, minimum 48pt targets, within 3 columns
        // and the bottom 176pt of the preferred thumb. No PS1 face-button overlay.
        for (i, control) in chips.into_iter().enumerate() {
            let col = (i % 3) as f32;
            let row = (i / 3) as f32;
            let x = match hand {
                Hand::Right => content.x + content.width - gap - size - col * (size + gap),
                Hand::Left => content.x + gap + col * (size + gap),
            };
            let y = content.y + content.height - gap - size - row * (size + gap);
            layout.controls.push(ControlRegion {
                control,
                bounds: Rect {
                    x,
                    y,
                    width: size,
                    height: size,
                },
            });
        }
        layout
    }

    pub fn region(&self, control: Control) -> Option<Rect> {
        self.controls
            .iter()
            .find(|r| r.control == control)
            .map(|r| r.bounds)
    }

    pub(crate) fn hit(&self, p: Point) -> Option<ControlRegion> {
        self.controls.iter().copied().find(|r| r.bounds.contains(p))
    }
}
