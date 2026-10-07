// SPDX-License-Identifier: GPL-3.0-only
use crate::{DiscImage, Error, ImageFormat, Result, error::invalid};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

pub(crate) fn open(path: &Path) -> Result<DiscImage<File>> {
    let text = fs::read_to_string(path)?;
    let parsed = parse(&text)?;
    let input = path.parent().unwrap_or(Path::new(".")).join(parsed.file);
    let file = File::open(input)?;
    let length = file.metadata()?.len();
    let offset = u64::from(parsed.index) * parsed.format.sector_size() as u64;
    let remaining = length
        .checked_sub(offset)
        .ok_or_else(|| invalid("CUE INDEX 01 exceeds the image"))?;
    DiscImage::with_region(file, parsed.format, offset, remaining)
}

struct Cue {
    file: PathBuf,
    format: ImageFormat,
    index: u32,
}

fn parse(text: &str) -> Result<Cue> {
    let mut file = None;
    let mut format = None;
    let mut index = None;
    let mut index0 = None;
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let (command, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        match command.to_ascii_uppercase().as_str() {
            "FILE" => {
                if file.is_some() {
                    return Err(Error::Unsupported(
                        "multi-file CUE; provide the single data-track BIN".into(),
                    ));
                }
                let (name, kind) = if let Some(quoted) = rest.trim().strip_prefix('"') {
                    let (name, tail) = quoted
                        .split_once('"')
                        .ok_or_else(|| invalid("unterminated CUE filename"))?;
                    (name, tail.trim())
                } else {
                    rest.trim()
                        .rsplit_once(char::is_whitespace)
                        .ok_or_else(|| invalid("missing CUE FILE type"))?
                };
                if name.is_empty() || !kind.eq_ignore_ascii_case("BINARY") {
                    return Err(Error::Unsupported(
                        "CUE FILE must name a BINARY image".into(),
                    ));
                }
                file = Some(PathBuf::from(name));
            }
            "TRACK" => {
                if file.is_none() || format.is_some() {
                    return Err(Error::Unsupported(
                        "only a single data-track CUE is supported".into(),
                    ));
                }
                let parts: Vec<_> = rest.split_whitespace().collect();
                if parts.len() != 2 || parts[0] != "01" {
                    return Err(Error::Unsupported("CUE must contain TRACK 01".into()));
                }
                format = Some(match parts[1].to_ascii_uppercase().as_str() {
                    "MODE1/2352" | "MODE2/2352" => ImageFormat::Raw2352,
                    "MODE1/2048" => ImageFormat::Iso2048,
                    other => return Err(Error::Unsupported(format!("CUE track type {other}"))),
                });
            }
            "INDEX" => {
                let parts: Vec<_> = rest.split_whitespace().collect();
                if format.is_none() || parts.len() != 2 {
                    return Err(invalid("CUE INDEX must follow TRACK"));
                }
                let time: Vec<_> = parts[1].split(':').map(str::parse::<u32>).collect();
                if time.len() != 3 || time.iter().any(|value| value.is_err()) {
                    return Err(invalid("invalid CUE MSF time"));
                }
                let values: Vec<_> = time
                    .into_iter()
                    .collect::<std::result::Result<_, _>>()
                    .map_err(|_| invalid("CUE time"))?;
                if values[1] >= 60 || values[2] >= 75 {
                    return Err(invalid("CUE seconds/frames out of range"));
                }
                let frames = values[0]
                    .checked_mul(4500)
                    .and_then(|v| v.checked_add(values[1] * 75 + values[2]))
                    .ok_or_else(|| invalid("CUE time overflow"))?;
                match parts[0] {
                    "00" if index0.replace(frames).is_none() => {}
                    "01" if index.replace(frames).is_none() => {}
                    _ => {
                        return Err(Error::Unsupported(
                            "duplicate or non-00/01 CUE INDEX".into(),
                        ));
                    }
                }
            }
            "REM" | "TITLE" | "PERFORMER" | "CATALOG" | "ISRC" | "FLAGS" | "PREGAP" | "POSTGAP" => {
            }
            other => return Err(Error::Unsupported(format!("CUE directive {other}"))),
        }
    }
    let index = index.ok_or_else(|| invalid("missing CUE INDEX 01"))?;
    if index0.is_some_and(|v| v > index) {
        return Err(invalid("CUE INDEX 00 is after INDEX 01"));
    }
    Ok(Cue {
        file: file.ok_or_else(|| invalid("missing CUE FILE"))?,
        format: format.ok_or_else(|| invalid("missing CUE TRACK"))?,
        index,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cue_offsets_and_rejections() {
        let cue = parse("FILE \"own disc.bin\" BINARY\n TRACK 01 MODE2/2352\n INDEX 00 00:00:00\n INDEX 01 00:02:00").unwrap();
        assert_eq!(cue.index, 150);
        assert_eq!(cue.file, PathBuf::from("own disc.bin"));
        for text in [
            "FILE a.bin BINARY\nTRACK 01 AUDIO\nINDEX 01 00:00:00",
            "FILE a.bin BINARY\nTRACK 01 MODE2/2352\nINDEX 01 00:60:00",
            "FILE a.bin BINARY\nTRACK 01 MODE2/2352\nINDEX 01 00:00:00\nTRACK 02 AUDIO",
        ] {
            assert!(parse(text).is_err());
        }
    }
}
