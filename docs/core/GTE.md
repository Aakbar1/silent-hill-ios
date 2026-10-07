# Native COP2 and PsyQ bridge

The host uses `psxgpu::gte::Gte`, backed by the crate's existing native C engine.
No second GTE or CPU execution loop is introduced. Register state is local to
the game worker. `InitGeom` and native PsyQ matrix/projection/lighting entry
points use the same register state as generated `gte_*` inline calls.

`tools/prepare_gte.py` lowers the pinned SDK's straight-line register transfers
at build time, with memcpy for unaligned words and explicit fixed-width shifts.
The pinned `inline_no_dmpsx.h` supplies real command words. Unknown helpers fail
generation. MVMVA's parameters use real COP2 bit positions (sf19, mx17, v15,
cv13, lm10), replacing the SDK's dmpsx pseudo-instruction bit encoding.

Default builds use exact integer coordinates. `--features precise-vertices`
also carries unrounded RTPS/RTPT coordinates beside original polygon packets.
The additive `GpuBackend::packet_precise` defaults to the existing packet hook,
so GPU lanes need no trait change. It must be implemented by a renderer before
precision metadata changes presentation. No float value feeds game logic.

Verification: C projection/transform probe, worker isolation, and register-by-
register comparison of all 65 pinned command variants covering 22 commands.
These three tests passed in exact and precise builds. This establishes the GTE
boundary; it does not establish working world rendering or collision.
