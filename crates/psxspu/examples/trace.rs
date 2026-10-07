//! Private sequencer trace for comparison with the original C. No game data
//! is embedded. Generated event data is written only into private/work/spu.
use psxspu::sequence::{Event, Sequence};
use std::{
    fs,
    io::{BufWriter, Write},
    path::Path,
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .ok_or("worktree layout")?;
    let private = root.join("private/work/spu");
    let mut args = std::env::args().skip(1);
    let name = args
        .next()
        .ok_or("usage: trace <private KDT filename> <ticks>")?;
    if name.contains(['/', '\\']) {
        return Err("use a filename under private/work/spu".into());
    }
    let ticks: usize = args.next().unwrap_or_else(|| "20000".into()).parse()?;
    let bytes = fs::read(private.join(&name))?;
    let mut sequence = Sequence::parse(&bytes)?;
    let mut output = BufWriter::new(fs::File::create(
        private.join(format!("{name}.native.csv")),
    )?);
    for tick in 1..=ticks {
        let mut error = None;
        sequence.tick(|event| {
            let values = match event {
                Event::NoteOn {
                    channel,
                    note,
                    velocity,
                } => ("N", channel, note, velocity),
                Event::NoteOff { channel, note } => ("O", channel, note, 0),
                Event::Program { channel, program } => ("P", channel, program, 0),
                Event::Control {
                    channel,
                    controller,
                    value,
                } => ("C", channel, controller, value),
                Event::Bend { channel, value } => ("B", channel, value, 0),
                _ => return,
            };
            if let Err(e) = writeln!(
                output,
                "{tick},{},{},{},{}",
                values.0, values.1, values.2, values.3
            ) {
                error = Some(e);
            }
        })?;
        if let Some(error) = error {
            return Err(error.into());
        }
    }
    output.flush()?;
    Ok(())
}
