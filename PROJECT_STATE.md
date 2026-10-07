# Boot lane state

## Outcome and constraints

Phase 1 Windows boot spike passes: real native C `main` → BODYPROG `MainLoop` → B_KONAMI; recognisable Konami screenshot after 600 VBlank ticks, clean exit. No emulator or system installations. No game bytes/screenshots in Git. Own branch remains `lane/boot`; `docs/survey/` is untouched.

## Verified

- MSVC 14.44 from `C:\BuildTools2022`, cc 1.6.0, Rust stable 1.99.0, x86-64 Windows.
- Raw MODE2/2352 reader opens the given disc directly and verifies its ISO root.
- Native boot compiles with `/W4 /WX`; Rust formatting and Clippy pass; eight tests pass.
- Final target run: 600 ticks, 9.997 seconds, B_KONAMI first tick 95, 598 window presents with final frame presented, 3,413 GPU sprite/tile primitives; command including build/startup 14.728 seconds.
- Private evidence: `../../private/work/boot/konami.png`, `boot-600.log`, check logs and generated C under `native-source/`.
- Longer run: requested 800 ticks; stopped cleanly after 731 at the KCET guard. Disc SHA-1 matches PLAN.md; native PE machine is AMD64; outside-private screenshot guard passed.

## Decisions and limits

- Compile selected real boot functions from external decomp commit `d9e28f8315c7938117224f21516786d9d149a145` with native boot runtime declarations; preserve packet/image wire assertions.
- Use software packet rasterization and winit/softbuffer for the spike. wgpu and 3D/GTE remain future work; this is not an iOS-ready host.
- Keep the original full-header compile probe as evidence: 43 size assertions still fail. Disabling them globally was rejected because it would hide real ABI/file-layout errors.
- Statically link native overlay functions; read/decrypt original binaries into data only. Full map/global data relocation remains unresolved.
- Stop explicitly at unimplemented states after Konami. Audio, input, memory cards and other irrelevant boot helpers are logged stubs.

## Next work and references

Full-game port is plausible but not proved. Director decides Phase 2 scope. First priorities: wire/native ABI separation and overlay globals, native KCET/title/stream flow, GTE/polygon rendering, media, then interactive/timing systems. Keep the external reference path for now; arrange a pinned submodule or controlled source import before portable CI if needed.

See [boot details](docs/boot/READINESS.md), [every PORT change and shim inventory](docs/boot/PORT_FIXES.md), and uncommitted `REPORT.md`. The read-only reference checkout remained unchanged.
