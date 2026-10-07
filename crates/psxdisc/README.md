# psxdisc

Standalone, pure-Rust GPL-3.0 library; Rust stable 1.88 or newer. No game data,
native build tools, emulator or platform-specific dependencies. The only direct
dependency is RustCrypto `sha2` (no assembly feature). See [NOTICE.md](NOTICE.md).

```no_run
use psxdisc::{GameDisc, SectorReader};

fn main() -> psxdisc::Result<()> {
    let mut game = GameDisc::open("my Silent Hill.bin")?; // also accepts a CUE
    let overlay = game.read_entry_by_name("1ST/B_KONAMI.BIN")?;
    // These bytes are still encrypted exactly as on disc; the game decrypts them.
    let stream = game.entry_by_name("XA/M1_03500")?;
    let (id, lba) = (stream.id, stream.start_sector);
    let sector = game.source_mut().read_sector(lba)?;
    let _original_2352_bytes = sector.raw()?;
    let _xa_decoder_payload = sector.xa_data()?; // 2324 Form2 / 2048 Form1
    game.copy_entry_raw_sectors(id, 0, 8, &mut std::io::sink())?;
    assert_eq!(overlay.len(), 4096);
    Ok(())
}
```

- `DiscImage<R: Read + Seek>` supports files and in-memory synthetic images.
  Detection uses sync/PVD signatures, including lengths divisible by both sizes.
- `SectorReader`: single/range sectors, logical data ranges, bounded XA streaming.
  `DiscImage::read_image_range` reads contiguous original bytes. Raw MODE1 data
  begin at 16, MODE2 Form1 at 24. Form2 is never silently shortened to 2048.
- `Sector`: original 2352 bytes; 2336-byte `xa_raw()` includes duplicate
  subheaders, payload and EDC/ECC; `xa_data()` returns just the user payload.
- `IsoFileSystem`: PVD, directories (including sector padding), nested lookups,
  ISO version suffix removal, and file reads. `SILENT.` / `HILL.` keep their dots.
  Endian copies and extent bounds are checked. Multi-volume, multi-extent and
  interleaved ISO files are explicitly rejected, as are directories above 16 MiB.
- CUE: a single BINARY file, TRACK 01 MODE1/2352, MODE2/2352 or MODE1/2048,
  INDEX 00/01, quoted relative paths, stored or virtual pregaps. Multiple tracks
  and other modes get an explicit error. LBA 0 begins at the stored INDEX 01.
- `verify(&mut source)`: SHA-256 of the **entire** `SLUS_007.07` file, accepting
  only US v1.1 `e73859ccd2e8000d259c6fe640bb8a6d55fed6044f67fbf071e3d86c0f202398`.
  `GameDisc::open` and `open_packed` verify before parsing an archive table.
- `GameDisc`: entries in the original game ID order; read by ID, full path or
  unique bare name (case-insensitive, either slash). Ambiguous bare names fail.
  Full/range reads and `copy_entry[_range]` preserve original bytes, including
  encryption/compression. Streaming uses bounded buffers, independent of FMV size.
  `source_mut` gives the media lane sector access. Use separate handles for
  concurrent readers; a handle has a mutable seek position.

The runtime table comes from the player's verified executable at `0xB91C`,
with 2074 12-byte entries. Names stop at the first encoded space, matching
`Fs_GetFileInfoName`; size fields are 12 bits, matching `s_FileInfo`.
`Entry.start_sector` is the game's absolute LBA, with no 150-sector adjustment.
For data, `table_size == size == block_count * 256`, including zero-length files;
`sector_count` rounds up to 2048. No inferred padding is returned as file data.
For XA, `table_size` still preserves `Fs_GetFileSize`, but the actual stream
`size` is the distance to the next XA start (or HILL's end) times **2336**.
`copy_entry_raw_sectors` returns the complete **2352** bytes per sector instead.
`HILL.`'s ISO length counts nominal 2048-byte blocks, **not** XA payload bytes;
use entries/sector streaming to read it. `read_disc_file("HILL.")` rejects upfront.

A plain 2048-byte ISO can be indexed/read for data and verified if its executable
is intact, but it cannot retain XA subheaders/audio/video. XA reads fail clearly.
Both app importer modes require a raw BIN/CUE for a complete playable game.
The executable hash identifies the release; it does not authenticate every asset.
Sector sync/mode/subheader copies are checked; EDC/ECC correction is not implemented.

## Importer and phone choice

Choose `ImportMode::Packed` (also `Default`). It copies only the four required
ISO file extents into **one** app-owned file: SYSTEM.CNF, executable and SILENT
as contiguous 2048-byte data; HILL as original **2352-byte raw sectors**.
Original LBAs and the executable's table are retained, rather than writing 2074
small asset files. Metadata outside these extents and data-sector sync/ECC are
omitted. Raw sector reads in packed SILENT are therefore unavailable; raw HILL
reads remain exact, including mixed Form1/Form2 sectors. Media must use HILL.
No decryption, decompression, transcoding, compression or game-behaviour change.

```no_run
use psxdisc::{GameDisc, ImportMode, import};

fn main() -> psxdisc::Result<()> {
    // The host acquires Files/security-scoped access before calling import.
    let stats = import("my Silent Hill.cue", "app-data/new-import", ImportMode::Packed)?;
    // It may release the source URL after import returns.
    let game = GameDisc::open_packed(&stats.path)?;
    assert_eq!(game.entries().len(), 2074);
    Ok(())
}
```

`KeepImage` instead copies the normalized data track as `disc.bin`, then
`GameDisc::open` indexes it. CUE pregap bytes are omitted; the CUE file is not
needed afterwards. Neither mode relies on a user's original URL staying valid.
The importer verifies first, refuses overwrites, flushes/syncs before success,
and removes a newly created output on an ordinary write failure. A process/power
interruption may leave a partial file: reopen rejects it; retry in a new folder.
An app should retain its previous successful import until the new one succeeds.

Measured on this Windows machine: packed is 604,129,552 bytes against
616,494,480 for the BIN (12,364,928 bytes / 2.006% smaller). Three alternating
release-mode runs gave median import **1044 vs 1664 ms**, data-entry loads
**51 vs 69 ms**, and whole archive streams **647 vs 648 ms**. Packed saves
space and improves import/data loading here; XA-heavy scans were effectively
tied. Timing includes OS caches and is not an iPhone measurement. Retaining raw
XA avoids a playback conversion and keeps importer work bounded. Full results,
method and limitations are in [BENCHMARK.md](BENCHMARK.md); the iOS lane should
verify these timings on-device before shipping.

## Checks

Run from this crate directory; no parent workspace change is required:

```text
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test -- --nocapture
```

Synthetic tests cover sector modes, offset/range bounds, malformed metadata,
ISO padding/directories, CUE pregaps and packed round-trips. Owned-disc tests
use `PSXDISC_TEST_IMAGE`, otherwise the project private BIN path; they print
`SKIP` and return successfully if absent. On the project machine they compare
every raw sector/range and all 2074 entry bytes against independent BIN reads.
An optional test also checks all IDs/names/size fields against the read-only,
version-matched decomp table if it exists. No test writes game bytes.

Explicit importer measurement writes game data **only to the private output
directory** you supply (about 3.7 GB for six imports), never to the repository:

```text
cargo run --release --example import_bench -- <owned BIN/CUE> <private directory>
```

The example verifies all entry-stream hashes match across both modes and checks
overwrite refusal. Inputs/output paths are arguments; it does not embed assets.
