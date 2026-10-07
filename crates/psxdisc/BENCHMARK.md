# Import measurement, 2026-10-07

Windows x86_64/MSVC, Rust 1.99.0 stable, release profile. Player-owned US 1.1
BIN; executable SHA-256 matches the requested release. Outputs were written
outside the repository to `C:/Claude Projects/Silent Hill iOS/private/work/disc/imports-20261007-streaming/`.
No other crate tests were running during this final measurement.

Command from `crates/psxdisc`:

```text
cargo run --release --example import_bench -- "C:/Claude Projects/Silent Hill iOS/private/disc/Silent Hill (USA).bin" "C:/Claude Projects/Silent Hill iOS/private/work/disc/imports-20261007-streaming"
```

| Mode | Run | Bytes | Import ms | Open ms | All entries ms | Data entries ms |
|---|---:|---:|---:|---:|---:|---:|
| KeepImage | 0 | 616494480 | 937.177 | 9.346 | 585.164 | 42.231 |
| Packed | 0 | 604129552 | 724.274 | 7.617 | 552.455 | 30.319 |
| Packed | 1 | 604129552 | 1044.314 | 9.982 | 747.229 | 65.571 |
| KeepImage | 1 | 616494480 | 1663.813 | 10.571 | 648.175 | 68.913 |
| KeepImage | 2 | 616494480 | 1698.513 | 8.396 | 675.795 | 93.496 |
| Packed | 2 | 604129552 | 1325.139 | 11.165 | 646.843 | 50.935 |
| KeepImage median | | 616494480 | 1663.813 | 9.346 | 648.175 | 68.913 |
| Packed median | | 604129552 | 1044.314 | 9.982 | 646.843 | 50.935 |

`import_ms` includes executable verification, copy/conversion, flush and
`sync_all`; opening rehashes the executable and rebuilds the index. `all_entries`
streams all 2074 entries (598,240,960 bytes) into SHA-256 with bounded buffers.
`data_entries` then loads every nonempty SILENT entry into a Vec, with its result
passed to `black_box`. It measures that traversal, not a single load's latency.
All six entry-stream hashes were identical; all six overwrite attempts failed
without changing the imported file's length. The owned-disc integration tests
independently compare entry bytes with original sectors, preventing a shared
import/read mistake from merely satisfying the cross-mode hash comparison.

Pick **Packed** for the phone: one file, 2.006% less storage, faster measured
import and data traversal; whole streaming traversal essentially tied. It
preserves complete raw XA sectors and keeps the original file IDs. No decoder
work moves into the importer. Packed's 112-byte header contains extent, nominal
ISO length, stored sector width and byte offset for each of the four files.
Header sizes/offsets/order/nonoverlap/truncation are validated on reopening.

This is three alternating runs with filesystem caches uncontrolled, not a
cold-cache experiment or a statistical performance guarantee. File sync/import
latency varies; opening has sub-millisecond differences comparable to noise.
No iPhone hardware, iOS compilation, playback quality or phone energy was
measured. The iOS lane should repeat the benchmark on-device. Earlier runs
informed bulk sector streaming and are excluded from the final table above.
