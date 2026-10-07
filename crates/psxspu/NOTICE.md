# Licence and provenance

The crate is distributed under GPL-3.0-only, as is this repository.

`pitch_table.rs`, `sequence.rs`, and the volume/pitch formulas in `preview.rs`
are adapted from the read-only silent-hill-decomp, commit
d9e28f8315c7938117224f21516786d9d149a145, src/bodyprog/libsd/smf_tables.h,
smf_mid.c, smf_main.c and smf_io.c. Its licence notice is retained here:

> Copyright (C) 2026 shdecompilations
> This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 3.
> This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
> You should have received a copy of the GNU General Public License along with this program. If not, see https://www.gnu.org/licenses/.

`gaussian.rs` adapts the factual 512-entry hardware table from
[PCSX-Redux src/spu/gauss.h](https://github.com/grumpycoders/pcsx-redux/blob/e7cff5efe8c487131d97833c56f79814d5fa35f7/src/spu/gauss.h).
Copyright (C) 2026 PCSX-Redux authors; GPL version 2 or, at your option, any later
version. Its full file notice remains in gaussian.rs. Redistribution here
uses the allowed version-3 terms; see the repository's GPL text.

The independently written envelope/SPU/reverb/XA implementations use hardware
facts, formulae and constants from psx-spx, snapshot
5127db8ff11ced2d5fb3887c3b1287fd35b33371:

- [SPU specification](https://psx-spx.consoledev.net/ps1/spu/soundprocessingunitspu/)
- [CD format / XA resampling](https://psx-spx.consoledev.net/ps1/cdr/cdromformat/)
- [Timer registers](https://psx-spx.consoledev.net/ps1/system/timers/)

ADPCM codecs come from sibling psxmedia, including its own NOTICE.md. No game
bytes, extracted samples, event traces, generated disassembly or screenshots
are included. All imported/generated content stays in private/work/spu.
