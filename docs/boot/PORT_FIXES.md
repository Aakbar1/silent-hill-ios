# PORT changes and boot shim inventory

All code changes remain outside the read-only reference checkout. `// PORT:` comments mark the adaptations in the files named below. Boot functions come from external GPL-3.0 source; generated derivatives stay under `private/work/boot/native-source/`. Upstream license text is retained as repository `LICENSE`.

## Every adaptation

1. **Native interface selection — `host/build.rs`, `port/include/common.h`, `port/boot.h`:** replace broad PS1 headers with a boot-only interface, retain actual function bodies, select only Konami/background functions needed by this spike. Native runtime records contain the accessed boot fields and are never loaded from disc bytes. USA gates remain fixed to the USA target. This is not a migration of the full game records.
2. **Entry/sections — `host/build.rs`, `port/include/common.h`:** rename C `main` to `sh_main` for the Rust process entry; replace PS1/GCC section-placement annotations with native storage.
3. **CRT type — `port/include/common.h`:** use the native `size_t` rather than the SDK's 32-bit typedef; SDK scalar packet types remain 32-bit on Windows. Packet/image size assertions remain active.
4. **Fixed memory — `port/include/common.h`, `port/boot.h`, `host/build.rs`:** scratchpad, file buffers, packet arena and BODYPROG/dynamic overlay destinations become aligned native allocations. Buffer sizes are bounded for the boot loads. MIPS overlay code is not called.
5. **Overlay decryption/linkage — `port/runtime.c`:** decode original overlays into data storage using unsigned wrapping seed arithmetic (same bit results as PS1), while their C boot entry points are statically linked. Full overlay globals/data relocation are unresolved.
6. **OT encoding/layout — `port/boot.h`, `port/runtime.c`:** use explicit 32-bit OT wire tags and 24-bit native pointer tokens. MSVC otherwise gives the SDK's mixed-type `GsOT_TAG` bitfields an 8-byte stride. Keep the wire tag at 4 bytes and resolve native pointers through a bounded registry.
7. **OT tail/aliases — `host/build.rs`:** replace warning-background reliance on adjacent PS1 BSS with the explicit native table tail; cast 32-bit tag aliases in Konami/fade emitters.
8. **Coordinate arithmetic — `port/boot.h`, `host/build.rs`:** pack signed 16-bit XY with unsigned shifts; replace negative background Y left shifts with multiplication. Preserve original coordinate bits without C undefined behavior.
9. **Explicit narrowing — `port/boot.h`, `host/build.rs`:** add explicit 16-bit display/width/height and 8-bit color casts, including main framebuffer Y, screen positions and fade RGB. Values are unchanged at PS1 packet precision; no warning suppression is used.
10. **Unused local — `host/build.rs`:** remove unused `gameState` in `GameState_Init_Update`; it has no reads or effects.
11. **Native FS queue — `port/runtime.c`:** replace hardware CD command/retry scheduling with a bounded job queue completed by direct raw-sector reads; retain file IDs, lengths, TIM placement and boot call order. Asset readiness is faster; hardware error timing is not reproduced.
12. **Clear/interlace — `port/runtime.c`:** inject a clear TILE at the ordering-table tail and render both fields for the interlaced Konami screen. Drawing/display buffer positions and offsets remain separate native state. Exact PsyQ interlace behavior beyond the inspected boot output is unverified.
13. **Clock/termination — `port/runtime.c`, `host/src/native.rs`:** use host-paced NTSC VBlank ticks and 263 HBlank counts per tick. Stop at N ticks or cancellation using a C-only `setjmp` boundary after Rust callbacks return. No jump crosses an active Rust callback frame.
14. **Out-of-scope systems — `port/runtime.c`, `host/src/raster.rs`:** stub audio/GTE initialization, input, memory-card, demo/loader/camera helpers and later states. Function stubs log their first call once. Later state guards log then exit; unknown GPU commands log once per opcode.

The separate **baseline compile probe** in `game/compiler-probe/build.rs` and `port/msvc_prelude.h` also reserves the C entry as `sh_main`, preserves native CRT `size_t`, removes only GCC section annotations, and selects the decomp's existing `SKIP_ASM` mode. It does not invent missing ASM/RODATA definitions, relax size assertions, or link a playable executable. The probe's failure is intentional evidence of the unresolved full ABI.

## PsyQ functions: 22 shims, 4 stubs

Implemented for the boot subset, not complete SDK implementations:

`ResetCallback`, `ResetGraph`, `DrawSync`, `ClearImage2`, `PutDispEnv`, `PutDrawEnv`, `SetDispMask`, `DrawPrim`, `AddPrim`, `SetDrawMode`, `GsClearOt`, `GsDrawOt`, `GsSortClear`, `GsGetActiveBuff`, `GsInitVcount`, `GsGetVcount`, `GsClearVcount`, `GsInitGraph2`, `GsDefDispBuff2`, `GsSwapDispBuff`, `VSyncCallback`, `VSync`.

PsyQ stubs: `CdInit`, `SpuInit`, `InitGeom`, `GsInit3D`. `CdInit` reports success because the host already opened the disc; subsequent boot FS calls use the native reader directly, not libcd emulation. GPU macros are compile-time interfaces and are not counted as shimmed functions.

## Game helpers: 28 stubs, 19 later-state guards

Helper stubs:

`Demo_ControllerDataUpdate`, `Demo_GameRandSeedSet`, `Demo_PresentIntervalUpdate`, `Demo_Update`, `GameFs_BgItemLoad`, `Game_WarmBoot`, `ItemScreen_TmdGsFCallInit`, `Joy_ControllerDataUpdate`, `Joy_Init`, `Joy_ReadP1`, `Joy_Update`, `MainLoop_ShouldWarmReset`, `Map_EffectTexturesLoad`, `MemCard_ElementsUpdate`, `MemCard_InitStatus`, `MemCard_SysEnable`, `MemCard_SysInit`, `MemCard_Update`, `SD_Call`, `SD_Init`, `Sd_AudioStreamingCheck`, `Sd_TaskPoolExecute`, `WorldGfx_HarryCharaLoad`, `func_80089090`, `func_800890B8`, `func_80089128`, `func_8008D78C`, `nullsub_800334C8`.

The 19 guards are enumerated by `PORT_OTHER_STATES` in `port/boot.h`, starting with `GameState_KcetLogo_Update`. They are separate from the PsyQ count. No unsupported GPU opcode was observed in the successful 600-tick run.
