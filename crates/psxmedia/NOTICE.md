This crate is GPL-3.0-only. See LICENSE.

`src/tables.rs` adapts the MPEG-1/2 VLC and quantization tables from
[FFmpeg n7.1, libavcodec/mpeg12data.c](https://github.com/FFmpeg/FFmpeg/blob/n7.1/libavcodec/mpeg12data.c).
Copyright (c) 2000,2001 Fabrice Bellard.
Copyright (c) 2002-2004 Michael Niedermayer <michaelni@gmx.at>.
These tables retain their LGPL-2.1-or-later licensing; see LICENSE-LGPL-2.1.
The changes are conversion of C arrays to Rust constants and removal of unused tables.
FFmpeg is free software; you can redistribute it and/or modify it under the GNU
Lesser General Public License as published by the Free Software Foundation,
version 2.1 or, at your option, any later version. It is distributed without any
warranty, including the implied warranties of merchantability or fitness for a
particular purpose. See the license for details.

The codec implementations were written for this crate using the format references
listed in README.md. No game bytes or extracted assets are distributed.
