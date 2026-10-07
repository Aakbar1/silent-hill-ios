# Disk32 and native load boundary

Schemas are derived from the pinned upstream headers: formats/model.h and lm.h (models/materials), formats/ipd.h (maps/collision), dms.h, and savegame.h. Copyright (C) 2026 shdecompilations; GPL-3.0-only. Upstream notices remain in game/decomp/LICENSE.

host/src/assets.rs explicitly decodes little-endian values into native graphs and checked leaf views. Serialized 32-bit offsets never become host integers pretending to be pointers, and loaded bytes are immutable. Collision offsets are relative to the collision header, LM offsets to the embedded LM base, and IPD offsets to the map base. Model instance slots remain indices; global model/texture identities remain unresolved identities until engine linkage. Triangle index 0xff and model vertex offsets follow the renderer source, rather than assuming all four indices address a mesh-local vertex list.

AssetStore owns the backing bytes, validates before publishing a handle, preserves active file identity, and never reuses released handles. C can query a native 32-byte summary or bounded pointer-free leaf spans and decode numeric fields with checked helpers. Spans remain valid only until close. Native graph callers retain their decoded mutable working state. Legacy raw reads refuse PLM/ILM/IPD/DMS, preventing unported C from doing in-place relocation. Full C gameplay consumers still need adapters when those translation units are linked.

Savegame has no serialized pointers: it is 636 bytes. Decode/encode preserves unknown/reserved bytes and original signed/bit patterns. This implements serialization, not a memory-card filesystem or a working save UI.

port/disk32.h checks wire sizes/offsets independently of pointer width. Native span/summary ABI is checked separately. PsyQ long and u_long become fixed 32-bit scalars in generated headers; native CRT sizes stay native. Original broad gameplay assertions have not been removed or disabled.

Verification:
- tools/check-layouts.cmd: MSVC x86 and x64, /std:c11 /W4 /WX, both pass.
- tools/check-ios-layouts.py --compiler "C:/BuildTools2022/VC/Tools/Llvm/x64/bin/clang-tidy.exe": installed LLVM 19.1.5 frontend targets aarch64-apple-ios15.0; disk/native/SDK assertions pass, LP64 and 64-bit pointers asserted, deliberately wrong pointer assertion rejected. VS lacks clang resource headers, so this local compile-only mode derives standard types from compiler target builtins. Runtime builds do not use these minimal declarations.
- Windows workflow includes the two MSVC modes and a macOS clang arm64 job using real standard headers. Remote CI has not been run/pushed.
- Synthetic tests cover bad bounds/counts, unaligned reads through C, nested bases, indices/sentinels, immutable file slots, native working state, save roundtrip and stale handles.
- `tools/dev-cargo.cmd run -- --inspect-assets`: 622 real files decode (45 DMS, 28 PLM, 498 IPD, 51 ILM). It exits nonzero for TEST/TEST2.DMS, which does not match the runtime DMS schema. No explicit C source caller for FILE_TEST_TEST2_DMS was found; that is not proof of unreachability. The bounds rejection is retained. Private log: ../../private/work/core/asset-audit.log.

Core2 additionally compiles the upstream pointer-free save/input records with original 636/44/8/28-byte assertions. Save health/tail offsets (0x240/0x27B) are checked on native builds and arm64. Native initialization probes validate signed difficulty nibble bits, all 45 enemy-state defaults and all 40 inventory IDs. Native file storage preserves exact 636-byte save payloads; memory-card UI integration is still open.

The current replay reaches title, difficulty selection and OPTION screens; New Game confirmation stops before GameBoot_WorldInit's native model/animation/collision consumers. The old KCET guard evidence is superseded by [MILESTONES.md](MILESTONES.md). Broad original-header compilation still fails with 43 ABI assertions. Gameplay ABI portability, first-map integration and an iOS runtime are not established.
