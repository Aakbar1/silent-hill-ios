// SPDX-License-Identifier: GPL-3.0-only
//! PSXCAP v1: little-endian words, no addresses or native pointers.
use crate::{Error, Processor, Renderer, Result};
use std::io::{Read, Write};

const MAGIC: [u8; 8] = *b"PSXCAP1\0";
const MAX_RECORD: usize = 4 * 1024 * 1024;
pub struct CaptureWriter<W: Write> {
    output: W,
}
impl<W: Write> CaptureWriter<W> {
    pub fn new(mut output: W) -> Result<Self> {
        output.write_all(&MAGIC)?;
        Ok(Self { output })
    }
    fn words(&mut self, tag: u32, words: &[u32]) -> Result<()> {
        if words.len() > MAX_RECORD / 4 {
            return Err(Error("capture record too large".into()));
        }
        self.output.write_all(&tag.to_le_bytes())?;
        self.output
            .write_all(&((words.len() * 4) as u32).to_le_bytes())?;
        for word in words {
            self.output.write_all(&word.to_le_bytes())?;
        }
        Ok(())
    }
    pub fn gp0(&mut self, words: &[u32]) -> Result<()> {
        self.words(0, words)
    }
    pub fn gp1(&mut self, words: &[u32]) -> Result<()> {
        self.words(1, words)
    }
    pub fn frame(&mut self) -> Result<()> {
        self.words(2, &[])
    }
    pub fn into_inner(self) -> W {
        self.output
    }
}
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ReplayStats {
    pub gp0_words: u64,
    pub gp1_words: u64,
    pub frames: u64,
}

/// Replay with a cumulative byte budget. Truncation and unfinished packets fail.
pub fn replay<R: Renderer>(
    input: &mut impl Read,
    gpu: &mut Processor<R>,
    byte_budget: u64,
) -> Result<ReplayStats> {
    let stats = records(input, byte_budget, |tag, payload| {
        if tag == 2 {
            if !gpu.idle() {
                return Err(Error("frame boundary inside unfinished GP0 packet".into()));
            }
            gpu.renderer_mut().flush()?;
        } else {
            for word in payload.as_chunks::<4>().0 {
                let w = u32::from_le_bytes(*word);
                if tag == 0 {
                    gpu.gp0(w)?;
                } else {
                    gpu.gp1(w)?;
                }
            }
        }
        Ok(())
    })?;
    if !gpu.idle() {
        return Err(Error("capture ends inside GP0 packet".into()));
    }
    gpu.renderer_mut().flush()?;
    Ok(stats)
}

/// Compare full VRAM at every frame boundary and EOF, using bounded memory.
pub fn compare_replay<A: Renderer, B: Renderer>(
    input: &mut impl Read,
    left: &mut Processor<A>,
    right: &mut Processor<B>,
    byte_budget: u64,
) -> Result<ReplayStats> {
    let mut frame = 0;
    let stats = records(input, byte_budget, |tag, payload| {
        if tag == 2 {
            if !left.idle() || !right.idle() {
                return Err(Error("frame boundary inside unfinished GP0 packet".into()));
            }
            compare_vram(left, right, frame)?;
            frame += 1;
        } else {
            for word in payload.as_chunks::<4>().0 {
                let w = u32::from_le_bytes(*word);
                if tag == 0 {
                    left.gp0(w)?;
                    right.gp0(w)?;
                } else {
                    left.gp1(w)?;
                    right.gp1(w)?;
                }
            }
        }
        Ok(())
    })?;
    if !left.idle() || !right.idle() {
        return Err(Error("capture ends inside GP0 packet".into()));
    }
    compare_vram(left, right, frame)?;
    Ok(stats)
}
fn compare_vram<A: Renderer, B: Renderer>(
    left: &mut Processor<A>,
    right: &mut Processor<B>,
    frame: u64,
) -> Result<()> {
    let rect = crate::Rect::new(0, 0, 1024, 512);
    let a = left.renderer_mut().read(rect)?;
    let b = right.renderer_mut().read(rect)?;
    if let Some((index, (&a, &b))) = a.iter().zip(&b).enumerate().find(|(_, (a, b))| a != b) {
        return Err(Error(format!(
            "frame {frame}: VRAM ({},{}) differs: {a:#06x} vs {b:#06x}",
            index % 1024,
            index / 1024
        )));
    }
    if a.len() != crate::VRAM_WORDS || b.len() != crate::VRAM_WORDS {
        return Err(Error("renderer returned incomplete VRAM".into()));
    }
    Ok(())
}

fn records(
    input: &mut impl Read,
    byte_budget: u64,
    mut consume: impl FnMut(u32, &[u8]) -> Result<()>,
) -> Result<ReplayStats> {
    let mut magic = [0; 8];
    input.read_exact(&mut magic)?;
    if magic != MAGIC {
        return Err(Error("invalid PSXCAP version/magic".into()));
    }
    let mut stats = ReplayStats::default();
    let mut consumed = 8u64;
    loop {
        let mut header = [0; 8];
        let n = input.read(&mut header[..1])?;
        if n == 0 {
            break;
        }
        input.read_exact(&mut header[1..])?;
        let tag = u32::from_le_bytes(header[..4].try_into().expect("four bytes"));
        let len = u32::from_le_bytes(header[4..].try_into().expect("four bytes")) as usize;
        consumed = consumed
            .checked_add(8 + len as u64)
            .ok_or_else(|| Error("capture budget overflow".into()))?;
        if len > MAX_RECORD || !len.is_multiple_of(4) || consumed > byte_budget {
            return Err(Error("capture length/budget invalid".into()));
        }
        if tag > 2 || tag == 2 && len != 0 {
            return Err(Error("invalid capture record".into()));
        }
        let mut payload = vec![0; len];
        input.read_exact(&mut payload)?;
        consume(tag, &payload)?;
        if tag == 2 {
            stats.frames += 1;
        } else if tag == 0 {
            stats.gp0_words += (len / 4) as u64;
        } else {
            stats.gp1_words += (len / 4) as u64;
        }
    }
    if consumed > byte_budget {
        return Err(Error("capture exceeds budget".into()));
    }
    Ok(stats)
}

/// File creation guard. The supplied private root must end in private/work/gpu.
pub fn create_private(root: &std::path::Path, name: &str) -> Result<CaptureWriter<std::fs::File>> {
    if std::path::Path::new(name).components().count() != 1 || !name.ends_with(".psxcap") {
        return Err(Error(
            "capture name must be a single .psxcap filename".into(),
        ));
    }
    let root = root.canonicalize()?;
    let suffix = std::path::Path::new("private").join("work").join("gpu");
    if !root.ends_with(suffix) {
        return Err(Error("captures must be under private/work/gpu".into()));
    }
    CaptureWriter::new(
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join(name))?,
    )
}
