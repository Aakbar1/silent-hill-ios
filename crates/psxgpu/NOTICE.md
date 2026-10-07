# Code provenance

The crate is distributed under GPL-3.0-only, as is the parent project.
No game executable bytes, extracted assets, disassembly or game screenshots are included.

`native/gte.c` is adapted from PCSX-ReARMed's `libpcsxcore/gte.c`, pinned to
[9d456e2c85df6a4d1f1cfd0ec8a5e81ed9bff42a](https://github.com/notaz/pcsx_rearmed/blob/9d456e2c85df6a4d1f1cfd0ec8a5e81ed9bff42a/libpcsxcore/gte.c).
The original PCSX-Revolution copyright and GPL-2.0-or-later notices remain in the file.
Changes on 2026-10-07: removed CPU/emulator/recompiler dependencies and stall scheduling;
added a per-context C ABI, portable overflow/leading-bit operations, explicit narrowing at
hardware register boundaries, and a standalone command dispatcher. No flagless or native
division approximation is enabled. This project uses the GPL version 3 option.

`native/gte_divider.c` and `.h` come from the same PCSX-ReARMed commit.
The divider was written by smf for MAME (Copyright 2003-2013 smf), imported into PCSX,
and is BSD-3-Clause. Only includes, leading-zero portability and explicit return narrowing
were adapted. The header retains its PCSX GPL-2.0-or-later notice.

The triangle interpolation anchor, fixed-point DDA rounding and 128-halfword transfer
burst model in the Rust/WGSL rasterizers follow the GPL-2.0-or-later
[Mednafen / Beetle PSX implementation](https://github.com/libretro/beetle-psx-libretro/tree/6f08abeaec4ec9f4611aa5b1bd0004fdddb4f9e4/mednafen/psx).
Its original licence notice applies to these derived portions; they are used under GPL version 3:

> Mednafen - Multi-system Emulator
>
> This program is free software; you can redistribute it and/or modify
> it under the terms of the GNU General Public License as published by
> the Free Software Foundation; either version 2 of the License, or
> (at your option) any later version.
>
> This program is distributed in the hope that it will be useful,
> but WITHOUT ANY WARRANTY; without even the implied warranty of
> MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
> GNU General Public License for more details.
>
> You should have received a copy of the GNU General Public License
> along with this program; if not, write to the Free Software
> Foundation, Inc., 59 Temple Place, Suite 330, Boston, MA 02111-1307 USA.

The GPL version 3 text is included in this crate's `LICENSE` and the repository root.

## BSD-3-Clause notice for the divider

Copyright 2003-2013 smf

Redistribution and use in source and binary forms, with or without modification, are
permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this list of
   conditions and the following disclaimer.
2. Redistributions in binary form must reproduce the above copyright notice, this list
   of conditions and the following disclaimer in the documentation and/or other materials
   provided with the distribution.
3. Neither the name of the copyright holder nor the names of its contributors may be used
   to endorse or promote products derived from this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY
EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF
MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL
THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT
OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR
TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE,
EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

## Specification references

Packet fields, masks, texture windows, display state and GTE test formulae were checked
against [PSX-SPX GPU](https://psx-spx.consoledev.net/ps1/gpu/) and
[GTE](https://psx-spx.consoledev.net/ps1/cpu/gte/geometrytransformationenginegte/).
No non-GPL renderer source was incorporated.
