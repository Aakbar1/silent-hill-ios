//! Touch-only native replay. Scene/config/targets always come from the game.
use crate::{Phase, TouchEvent};
use serde::{Deserialize, Serialize};
use std::io::BufRead;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Record {
    Touch {
        tick: u64,
        id: u64,
        phase: Phase,
        x: f32,
        y: f32,
    },
    Check {
        tick: u64,
        id: String,
        state: i32,
        step: i32,
        menu: Option<i32>,
        option_entry: Option<i32>,
    },
}
impl Record {
    pub fn tick(&self) -> u64 {
        match self {
            Self::Touch { tick, .. } | Self::Check { tick, .. } => *tick,
        }
    }
    pub fn event(&self) -> Option<TouchEvent> {
        match *self {
            Self::Touch {
                tick,
                id,
                phase,
                x,
                y,
            } => Some(TouchEvent {
                id,
                phase,
                x,
                y,
                time_ms: tick_ms(tick),
            }),
            _ => None,
        }
    }
}

/// Exact virtual 60 Hz VBlank clock used by the native host, without float drift.
pub fn tick_ms(tick: u64) -> u64 {
    ((u128::from(tick) * 1000 / 60).min(u128::from(u64::MAX))) as u64
}

pub fn parse(reader: impl BufRead) -> Result<Vec<Record>, String> {
    let mut records = Vec::new();
    for (line, text) in reader.lines().enumerate() {
        let text = text.map_err(|e| format!("line {}: {e}", line + 1))?;
        if text.trim().is_empty() {
            continue;
        }
        let record: Record =
            serde_json::from_str(&text).map_err(|e| format!("line {}: {e}", line + 1))?;
        if records
            .last()
            .is_some_and(|prior: &Record| prior.tick() > record.tick())
        {
            return Err(format!("line {}: backwards tick", line + 1));
        }
        records.push(record);
    }
    if !records.iter().any(|r| matches!(r, Record::Touch { .. }))
        || !records.iter().any(|r| matches!(r, Record::Check { .. }))
    {
        return Err("native replay requires touch events and game checkpoints".into());
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_replay_cannot_inject_game_context_or_buttons() {
        for text in [
            r#"{"type":"context","context":{}}"#,
            r#"{"type":"touch","tick":0,"id":1,"phase":"down","x":0,"y":0,"buttons":8}"#,
            r#"{"type":"check","tick":0,"id":"fake","state":7,"step":1}"#,
        ] {
            assert!(parse(text.as_bytes()).is_err());
        }
        assert_eq!(tick_ms(60), 1000);
        assert_eq!(tick_ms(1400), 23333);
    }
    #[test]
    fn native_replay_checks_order_and_preserves_simultaneous_events() {
        let text = concat!(
            "{\"type\":\"touch\",\"tick\":10,\"id\":1,\"phase\":\"down\",\"x\":20,\"y\":30}\n",
            "{\"type\":\"touch\",\"tick\":10,\"id\":1,\"phase\":\"up\",\"x\":20,\"y\":30}\n",
            "{\"type\":\"check\",\"tick\":20,\"id\":\"title\",\"state\":7,\"step\":1}\n"
        );
        let records = parse(text.as_bytes()).unwrap();
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].event().unwrap().time_ms, 166);
        assert!(parse(text.replace("\"tick\":20", "\"tick\":9").as_bytes()).is_err());
    }
}
