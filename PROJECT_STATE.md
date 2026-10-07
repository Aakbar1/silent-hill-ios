# core2 checkpoint

Goal: title, New Game and the first controllable map, then transitions and all map overlays. Own only the core2 brief paths. No GPU/SPU edits, game bytes/captures in Git, installs, pushes or merges.

Working step 1: STREAM state handlers compiled from pinned upstream into sh_stream_* namespace; debug static resets on activation. Native bounded STR/XA reader uses psxmedia, 15 fps on virtual 60 Hz VBlank, RGB24 packed VRAM through GpuBackend, CD PCM/stop through SpuBackend (silent fallback logs absent sink). Original open_main end-frame limit preserved; Start uses configured clicked skip flag. Headless runner checks ticks/state/step/video frames/skips. Capture path now private/work/core2.

Verified: native C /W4 /WX build; host 18 unit tests pass; release neutral 1900 ticks reaches state 6, decodes 147 frames and 370944 stereo XA sample frames. intro-1900.png visually inspected (actual movie frame). No audible SPU sink present. Title still has an explicit guard; end/skip/return and first map not yet proven. Build/test details and old core history: docs/core/BUILD.md, ABI.md, OVERLAYS.md, SEAMS.md.

Next: link title/text with native records, then original remaining screens and save service; resolve gameplay ABI/GTE/assets before linking first map. Retain all wire checks; do not fake a loaded map or successful native save UI.
