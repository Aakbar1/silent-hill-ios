//! Q15 envelopes following psx-spx's fractional counter model, including
//! frozen all-ones rates, exponential knee and signed volume sweeps.

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AdsrPhase {
    #[default]
    Off,
    Attack,
    Decay,
    Sustain,
    Release,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Adsr {
    pub phase: AdsrPhase,
    pub level: u16,
    counter: u32,
}

#[derive(Clone, Copy)]
struct Rate {
    shift: u32,
    step: u32,
    decrease: bool,
    exponential: bool,
    negative: bool,
    frozen: bool,
}

fn step(level: i32, counter: &mut u32, rate: Rate) -> i32 {
    let Rate {
        shift,
        step,
        decrease,
        exponential,
        negative,
        frozen,
    } = rate;
    if frozen {
        return level;
    }
    let mut change = 7 - step as i32;
    if decrease ^ negative {
        change = !change;
    }
    change <<= 11u32.saturating_sub(shift);
    let mut increment = 0x8000u32 >> shift.saturating_sub(11);
    if exponential && !decrease && level > 0x6000 {
        if shift < 10 {
            change >>= 2;
        } else if shift >= 11 {
            increment >>= 2;
        } else {
            change >>= 1;
            increment >>= 1;
        }
    } else if exponential && decrease {
        change = ((i64::from(change) * i64::from(level)) >> 15) as i32;
    }
    *counter += increment.max(1);
    if *counter & 0x8000 == 0 {
        return level;
    }
    *counter &= 0x7fff;
    let value = level + change;
    if !decrease {
        value.clamp(-0x8000, 0x7fff)
    } else if negative {
        value.clamp(-0x8000, 0)
    } else {
        value.max(0)
    }
}

impl Adsr {
    pub fn key_on(&mut self) {
        *self = Self {
            phase: AdsrPhase::Attack,
            ..Self::default()
        };
    }
    pub fn key_off(&mut self) {
        if self.phase != AdsrPhase::Off {
            self.phase = AdsrPhase::Release;
            self.counter = 0;
        }
    }
    pub fn stop(&mut self) {
        *self = Self::default();
    }
    /// Advance exactly one 44.1 kHz envelope clock and return ENVX.
    pub fn tick(&mut self, adsr1: u16, adsr2: u16) -> u16 {
        let (shift, step_value, decrease, exponential, frozen) = match self.phase {
            AdsrPhase::Off => return 0,
            AdsrPhase::Attack => {
                let rate = (adsr1 >> 8) & 127;
                (
                    u32::from(rate >> 2),
                    u32::from(rate & 3),
                    false,
                    adsr1 & 0x8000 != 0,
                    rate == 127,
                )
            }
            AdsrPhase::Decay => (u32::from((adsr1 >> 4) & 15), 0, true, true, false),
            AdsrPhase::Sustain => {
                let rate = (adsr2 >> 6) & 127;
                (
                    u32::from(rate >> 2),
                    u32::from(rate & 3),
                    adsr2 & 0x4000 != 0,
                    adsr2 & 0x8000 != 0,
                    rate == 127,
                )
            }
            AdsrPhase::Release => (
                u32::from(adsr2 & 31),
                0,
                true,
                adsr2 & 32 != 0,
                adsr2 & 31 == 31,
            ),
        };
        self.level = step(
            i32::from(self.level),
            &mut self.counter,
            Rate {
                shift,
                step: step_value,
                decrease,
                exponential,
                negative: false,
                frozen,
            },
        )
        .clamp(0, 0x7fff) as u16;
        let next = match self.phase {
            AdsrPhase::Attack if self.level == 0x7fff => AdsrPhase::Decay,
            AdsrPhase::Decay if u32::from(self.level) <= (u32::from(adsr1 & 15) + 1) * 0x800 => {
                AdsrPhase::Sustain
            }
            AdsrPhase::Release if self.level == 0 => AdsrPhase::Off,
            _ => self.phase,
        };
        if next != self.phase {
            self.counter = 0;
            self.phase = next;
        }
        self.level
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Volume {
    pub register: u16,
    pub current: i16,
    counter: u32,
}
impl Volume {
    pub fn write(&mut self, register: u16) {
        self.register = register;
        self.counter = 0;
        if register & 0x8000 == 0 {
            self.current = (register << 1) as i16;
        }
    }
    pub fn tick(&mut self) -> i16 {
        if self.register & 0x8000 != 0 {
            let rate = self.register;
            self.current = step(
                i32::from(self.current),
                &mut self.counter,
                Rate {
                    shift: u32::from((rate >> 2) & 31),
                    step: u32::from(rate & 3),
                    decrease: rate & 0x2000 != 0,
                    exponential: rate & 0x4000 != 0,
                    negative: rate & 0x1000 != 0 && rate & 0x6000 != 0x6000,
                    frozen: rate & 127 == 127,
                },
            )
            .clamp(-32768, 32767) as i16;
        }
        self.current
    }
}
