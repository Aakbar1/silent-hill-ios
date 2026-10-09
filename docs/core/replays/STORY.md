# Story checkpoint - 9 October 2026

**Goals 1–2 pass; the required win in Goal 3 is not achieved.** The no-skip
replay reaches the original alley encounter and finishes 19650 VBlanks without
a guard. Both opening/Cheryl movies finish (338/119 frames), both doors execute,
and the darker-alley/match/dead-body events execute through original callbacks.
No host warp, equipment/HP grant, pose or completion flag is injected.

Original `Sfx_WithFalloffAndPitchPlay` now updates libsd/SPU voices. Private
`enemy-load-audio.json` checks 1736 wheel updates, three attenuation values,
224 nonzero voice-volume samples and mixed PCM with no output rails. Full WAV
scratch was deleted; `enemy-load-wheel.wav` retains the checked window.

`encounter-final.log/.json/.png` show three live Grey Children, 16 production
hits and Harry HP falling from 100 to 43.4. All enemies remain at 350 HP; no
Harry-to-enemy hit or kill is claimed. The dim private screenshot was visually
reviewed; rendering parity is unverified. `check_fight.py --run` defaults to
this original story/19650-frame checkpoint and correctly returns failure for
the absent kill. Static warps and fixtures still cannot establish combat.

The script deliberately leaves Harry unequipped. Ordinary weapon attacks are
unavailable; the unarmed finish check requires a weakened enemy (see original
`player_control.c::func_8007F95C` and `stalker.c::Stalker_Control_9`). Recorded
movement/action attempts produce no enemy damage. The longer candidate reaches
Harry's scripted defeat. Director: clarify the kill requirement against the
first armed/winnable encounter after the café, rather than altering this opening.

Native fixes include dialogue's original timer, flag/animation helpers, normal
grab handling, two owned audio-command words for the two-page body dialogue,
and 32-bit head-camera coordinates. New work is namespaced/reset-probed. Camera
and dialogue faults found during integration were repaired, with evidence in
`transform*.log` and `real-encounter.log`. `--warp` gameplay remains unchanged.

Validation: release MSVC `/W4 /WX`; fmt; workspace/all-target Clippy with denied
warnings; 312 workspace tests (9 ignored); available 10/10; first-map 422 samples;
opening-no-skip; separate default-4x first-map/first-street views; 90/90 arm64 C
frontend units; fight evaluator 2 positive/18 rejection controls. This is not
an Apple SDK/device or PS1 visual/audio parity certification.

All evidence is under `C:/Claude Projects/Silent Hill iOS/private/work/story/`.
Existing exact gate/control ticks (9552/9615) and their assertions are preserved.
