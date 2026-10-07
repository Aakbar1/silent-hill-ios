# Disc lane checkpoint

Goal: standalone pure-Rust disc/ISO/archive library for the player's own Silent
Hill US v1.1 image, shared by Windows and iOS. Scope remains `crates/psxdisc`;
root REPORT.md is the required uncommitted handoff. No workspace edits, no
system installs, no main/merge/push, no game bytes or extracted tables in repo.

Implemented raw MODE1/MODE2 Form1/Form2 and 2048 ISO reads; single-track CUE;
ISO9660 directories; runtime executable table and original IDs/names/size
fields; whole-executable SHA-256 verification; bounded entry/XA streaming;
retained-image and packed app-owned imports. Source: decomp commit
`d9e28f8315c7938117224f21516786d9d149a145`, details in NOTICE.md.

Verified: all 262115 raw sectors/ranges and all 2074 entry contents match the
private BIN. All table IDs/names/size fields match the USA decomp table. Entries
total 598240960 readable bytes (82799872 bytes summed table_size); 30 XA, 13
empty data entries. `verify()` accepts the requested executable hash.

Decision: Packed default, based on smaller storage and faster import/data
traversal here; XA stream traversal is tied. Full measurements and limitations
are in BENCHMARK.md. XA table_size is not its physical span; preserve both.
Do not infer nonzero sizes for the game's empty data entries or decrypt here.
Do not convert XA to a 2048 ISO: audio/video/subheaders would be lost. Full game
imports therefore require raw BIN/CUE. Multi-track CUE/advanced ISO unsupported.

Verification commands and integration usage: README.md. Private benchmark
outputs remain under `private/work/disc`, outside git. No C/native build needed.
Final checks: format and Clippy clean; 17 tests passed (5 unit, 7 synthetic,
3 owned-disc, 2 doc). Missing-image integration path cleanly skips all 3 checks.

Handoff: director must add the workspace member/dependency; iOS lane must build
and measure on Apple targets, manage Files URL access, and call Packed import.
Media consumes raw HILL sectors/subheaders/payloads; game handles decryption.
Apple build/device timing and playable-game integration remain unverified.
