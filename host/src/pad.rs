// SPDX-License-Identifier: GPL-3.0-only
use sh_touch::{Buttons, PadState};
use std::{
    collections::HashSet,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU16, Ordering},
    },
};
use winit::keyboard::KeyCode;

/// Sample once per VBlank clock value; repeated reads at that tick must agree.
/// touch::Engine output can be supplied through a PadSource without C changes.
pub trait PadSource: Send {
    fn sample(&mut self, tick: u64) -> [u8; 8];
}

pub struct LiveInput {
    held: AtomicU16,
    focused: AtomicBool,
    keys: Mutex<HashSet<KeyCode>>,
}
impl Default for LiveInput {
    fn default() -> Self {
        Self {
            held: AtomicU16::new(0),
            focused: AtomicBool::new(true),
            keys: Mutex::new(HashSet::new()),
        }
    }
}
impl LiveInput {
    pub fn focus(&self, focused: bool) {
        self.focused.store(focused, Ordering::Relaxed);
        if !focused {
            self.keys.lock().expect("input keys lock").clear();
            self.held.store(0, Ordering::Relaxed);
        }
    }
    pub fn key(&self, code: KeyCode, pressed: bool) {
        let mut keys = self.keys.lock().expect("input keys lock");
        if pressed {
            keys.insert(code);
        } else {
            keys.remove(&code);
        }
        let held = keys.iter().fold(0, |held, key| held | Self::button(*key).0);
        self.held.store(held, Ordering::Relaxed);
    }
    fn button(code: KeyCode) -> Buttons {
        match code {
            KeyCode::ArrowUp | KeyCode::KeyW => Buttons::UP,
            KeyCode::ArrowDown | KeyCode::KeyS => Buttons::DOWN,
            KeyCode::ArrowLeft | KeyCode::KeyA => Buttons::LEFT,
            KeyCode::ArrowRight | KeyCode::KeyD => Buttons::RIGHT,
            KeyCode::Enter => Buttons::START,
            KeyCode::Space => Buttons::CROSS,
            KeyCode::Escape => Buttons::CIRCLE,
            KeyCode::ShiftLeft => Buttons::SQUARE,
            KeyCode::Tab => Buttons::SELECT,
            KeyCode::KeyM => Buttons::TRIANGLE,
            KeyCode::KeyQ => Buttons::L1,
            KeyCode::KeyE => Buttons::R1,
            KeyCode::KeyZ => Buttons::L2,
            KeyCode::KeyC => Buttons::R2,
            _ => Buttons(0),
        }
    }
}

pub struct KeyboardController {
    pub input: Arc<LiveInput>,
    cached: Option<(u64, [u8; 8])>,
}
impl KeyboardController {
    pub fn new(input: Arc<LiveInput>) -> Self {
        Self {
            input,
            cached: None,
        }
    }
}
impl PadSource for KeyboardController {
    fn sample(&mut self, tick: u64) -> [u8; 8] {
        if let Some((previous, packet)) = self.cached
            && previous == tick
        {
            return packet;
        }
        let mut state = PadState::default();
        if self.input.focused.load(Ordering::Relaxed) {
            state = controller();
            state.buttons |= Buttons(self.input.held.load(Ordering::Relaxed));
        }
        let packet = state.ps1_packet();
        self.cached = Some((tick, packet));
        packet
    }
}

#[cfg(windows)]
fn controller() -> PadState {
    use windows_sys::Win32::UI::Input::XboxController::*;
    for index in 0..4 {
        let mut state = XINPUT_STATE::default();
        // SAFETY: XInput writes one correctly sized state; no pointer survives the call.
        if unsafe { XInputGetState(index, &mut state) } != 0 {
            continue;
        }
        let g = state.Gamepad;
        let mut pad = PadState::default();
        for (xbox, ps1) in [
            (XINPUT_GAMEPAD_DPAD_UP, Buttons::UP),
            (XINPUT_GAMEPAD_DPAD_DOWN, Buttons::DOWN),
            (XINPUT_GAMEPAD_DPAD_LEFT, Buttons::LEFT),
            (XINPUT_GAMEPAD_DPAD_RIGHT, Buttons::RIGHT),
            (XINPUT_GAMEPAD_START, Buttons::START),
            (XINPUT_GAMEPAD_BACK, Buttons::SELECT),
            (XINPUT_GAMEPAD_A, Buttons::CROSS),
            (XINPUT_GAMEPAD_B, Buttons::CIRCLE),
            (XINPUT_GAMEPAD_X, Buttons::SQUARE),
            (XINPUT_GAMEPAD_Y, Buttons::TRIANGLE),
            (XINPUT_GAMEPAD_LEFT_SHOULDER, Buttons::L1),
            (XINPUT_GAMEPAD_RIGHT_SHOULDER, Buttons::R1),
            (XINPUT_GAMEPAD_LEFT_THUMB, Buttons::L3),
            (XINPUT_GAMEPAD_RIGHT_THUMB, Buttons::R3),
        ] {
            if g.wButtons & xbox != 0 {
                pad.buttons |= ps1;
            }
        }
        if u16::from(g.bLeftTrigger) > XINPUT_GAMEPAD_TRIGGER_THRESHOLD {
            pad.buttons |= Buttons::L2;
        }
        if u16::from(g.bRightTrigger) > XINPUT_GAMEPAD_TRIGGER_THRESHOLD {
            pad.buttons |= Buttons::R2;
        }
        fn axis(value: i16) -> f32 {
            let denominator = if value < 0 { 32768.0 } else { 32767.0 };
            f32::from(value) / denominator
        }
        pad.left = sh_touch::Point::new(axis(g.sThumbLX), -axis(g.sThumbLY));
        pad.right = sh_touch::Point::new(axis(g.sThumbRX), -axis(g.sThumbRY));
        return pad;
    }
    PadState::default()
}
#[cfg(not(windows))]
fn controller() -> PadState {
    PadState::default()
}

/// Text replay: `tick active_high_buttons_hex [rx ry lx ly]`; axes default to 128.
/// A row replaces the held packet until the next row. Tick zero must be present.
pub struct ReplayPad {
    rows: Vec<(u64, [u8; 8])>,
    cursor: usize,
}
impl ReplayPad {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut rows = Vec::new();
        for (line, text) in text.lines().enumerate() {
            let text = text.split('#').next().unwrap_or("").trim();
            if text.is_empty() {
                continue;
            }
            let fields: Vec<_> = text.split_whitespace().collect();
            let invalid = || format!("invalid pad replay line {}", line + 1);
            if !matches!(fields.len(), 2 | 6) {
                return Err(invalid());
            }
            let tick = fields[0].parse::<u64>().map_err(|_| invalid())?;
            if rows.last().is_some_and(|(prior, _)| tick <= *prior) {
                return Err(invalid());
            }
            let held = u16::from_str_radix(fields[1].trim_start_matches("0x"), 16)
                .map_err(|_| invalid())?;
            let mut packet = PadState {
                buttons: Buttons(held),
                ..Default::default()
            }
            .ps1_packet();
            if fields.len() == 6 {
                for (out, value) in packet[4..].iter_mut().zip(&fields[2..]) {
                    *out = value.parse::<u8>().map_err(|_| invalid())?;
                }
            }
            rows.push((tick, packet));
        }
        if rows.first().is_none_or(|(tick, _)| *tick != 0) {
            return Err("pad replay must start at tick 0".into());
        }
        Ok(Self { rows, cursor: 0 })
    }
}
impl PadSource for ReplayPad {
    fn sample(&mut self, tick: u64) -> [u8; 8] {
        // Rewinds are supported for deterministic verification/re-entry.
        if self.rows[self.cursor].0 > tick {
            self.cursor = 0;
        }
        while self.cursor + 1 < self.rows.len() && self.rows[self.cursor + 1].0 <= tick {
            self.cursor += 1;
        }
        self.rows[self.cursor].1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_active_low_axes_repeated_reads_release_and_rewind() {
        let mut replay = ReplayPad::parse("0 0000\n10 4008 1 2 3 4\n11 0").unwrap();
        assert_eq!(replay.sample(9), [0, 0x73, 255, 255, 128, 128, 128, 128]);
        assert_eq!(replay.sample(10), [0, 0x73, 0xf7, 0xbf, 1, 2, 3, 4]);
        assert_eq!(replay.sample(10), replay.sample(10));
        assert_eq!(replay.sample(11)[2..4], [255, 255]);
        assert_eq!(replay.sample(0)[2..4], [255, 255]);
        for bad in ["", "1 0", "0 0\n0 1", "0 10000", "0 0 256 0 0 0"] {
            assert!(ReplayPad::parse(bad).is_err());
        }
    }
    #[test]
    fn keyboard_focus_loss_releases_buttons() {
        let input = LiveInput::default();
        input.key(KeyCode::Space, true);
        input.key(KeyCode::Enter, true);
        assert_eq!(
            input.held.load(Ordering::Relaxed),
            Buttons::CROSS.0 | Buttons::START.0
        );
        input.focus(false);
        assert_eq!(input.held.load(Ordering::Relaxed), 0);
        input.focus(true);
        input.key(KeyCode::KeyW, true);
        input.key(KeyCode::ArrowUp, true);
        input.key(KeyCode::KeyW, false);
        assert_eq!(input.held.load(Ordering::Relaxed), Buttons::UP.0);
    }
}
