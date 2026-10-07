//! JSONL: one tagged record per line. Events are in points and monotonic ms.
use crate::*;
use serde::{Deserialize, Serialize};
use std::io::BufRead;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Record {
    Marker {
        id: String,
    },
    Touch(TouchEvent),
    Context {
        context: GameContext,
    },
    Config {
        config: Config,
    },
    Viewport {
        viewport: Viewport,
    },
    Targets {
        targets: Vec<UiTarget>,
    },
    CancelAll,
    Frame {
        time_ms: u64,
        expect: Option<ExpectedFrame>,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedFrame {
    pub buttons: Buttons,
    #[serde(default)]
    pub left: Point,
    #[serde(default)]
    pub right: Point,
    #[serde(default)]
    pub ui_actions: Vec<UiAction>,
}

impl ExpectedFrame {
    pub fn matches(&self, frame: &FrameOutput) -> bool {
        self.buttons == frame.pad.buttons
            && self.ui_actions == frame.ui_actions
            && (self.left.x - frame.pad.left.x).abs() <= 0.0001
            && (self.left.y - frame.pad.left.y).abs() <= 0.0001
            && (self.right.x - frame.pad.right.x).abs() <= 0.0001
            && (self.right.y - frame.pad.right.y).abs() <= 0.0001
    }
}

pub fn parse(reader: impl BufRead) -> Result<Vec<Record>, String> {
    reader
        .lines()
        .enumerate()
        .map(|(index, line)| {
            let line = line.map_err(|e| format!("line {}: {e}", index + 1))?;
            serde_json::from_str(&line).map_err(|e| format!("line {}: {e}", index + 1))
        })
        .collect()
}

/// Uses the same engine as live input; mismatches include the JSONL line number.
pub fn run(engine: &mut Engine, records: &[Record]) -> Result<Vec<FrameOutput>, String> {
    let mut frames = Vec::new();
    for (index, record) in records.iter().enumerate() {
        let result = match record {
            Record::Marker { .. } => Ok(()),
            Record::Touch(event) => engine.touch(*event),
            Record::Context { context } => {
                engine.set_context(*context);
                Ok(())
            }
            Record::Config { config } => engine.set_config(*config),
            Record::Viewport { viewport } => engine.set_viewport(*viewport),
            Record::Targets { targets } => engine.set_targets(targets.clone()),
            Record::CancelAll => {
                engine.cancel_all();
                Ok(())
            }
            Record::Frame { time_ms, expect } => {
                let frame = engine
                    .frame(*time_ms)
                    .map_err(|e| format!("line {}: {e}", index + 1))?;
                if let Some(expected) = expect
                    && !expected.matches(&frame)
                {
                    return Err(format!(
                        "line {}: expected {expected:?}, got {frame:?}",
                        index + 1
                    ));
                }
                frames.push(frame);
                Ok(())
            }
        };
        result.map_err(|e| format!("line {}: {e}", index + 1))?;
    }
    Ok(frames)
}
