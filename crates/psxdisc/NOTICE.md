# Licence and source attribution

This crate is GPL-3.0-only. The full licence and the upstream copyright notice
are retained in LICENSE. RustCrypto sha2 dependencies retain their own licences.

Silent Hill file-table interpretation is derived from the read-only
[silent-hill-decomp](https://github.com/Vatuu/silent-hill-decomp)
at commit `d9e28f8315c7938117224f21516786d9d149a145`:

- `docs/File Formats.md`: the two containers have no internal file table.
- `include/main/fileinfo.h`: 12-byte bitfield layout, 19-bit sector, 12-bit block
  count, 256-byte blocks and packed six-bit names.
- `src/main/fileinfo.c`: directory/extension mappings, name termination at the
  first encoded space, and `Fs_GetFileSize` / `Fs_GetFileSectorAlignedSize`.
- `src/main/fsqueue.c`: `Fs_QueueTickSetLoc` passes the absolute `startSector`
  unchanged; `Fs_QueueTickRead` rounds data size up to 2048-byte sectors.
- `configs/USA/main.yaml`: executable file table at byte offset `0xB91C`.
- `tools/silentassets/extract.py`: US 1.1 entry count and XA extraction uses
  spans between start sectors (the last ends at HILL's end), at 2336 bytes/sector.

Copyright (C) 2026 shdecompilations. The format interpretation is translated
into Rust under GPL-3.0-only. No generated file table or game bytes are embedded.
The runtime parses the verified player's executable.

ISO directory structure is based on
[ECMA-119](https://ecma-international.org/wp-content/uploads/ECMA-119_6th_edition_december_2025.pdf).
