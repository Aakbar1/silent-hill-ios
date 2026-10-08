# Transit replay handoff - 9 October 2026

`python tools/milestones.py --opening-noskip` passes at 11000 frames. It requires
the full 338-frame opening movie, all 15 opening callback steps, gradual rollout
and completion of messages 15-19, the full 119-frame Cheryl movie, the exact
first-gate event at 9552 and original destination spawn. Control is enabled in
room 22 at 9615, with 1386 subsequent gameplay samples. It injects no state.

The same file continues beyond that passing checkpoint. A 15000-frame headless
run selects the second door at 12901 (source trigger point 20, destination point
22). It stops at `MAP0_S00/wheel positional SFX`, state 10/step 11/VBlank 12961.
No movie is skipped after New Game. This deeper boundary is **not** a completed
dream/combat/cafe milestone. Evidence: private/work/transit/opening-deeper-4.log.

`police_return.txt` is a **failed candidate**. The opt-in native walking test
uses original boot/main-loop services and only one initial MAP2_S04:4 warp;
all subsequent movement would use recorded pads and original triggers. It fails
at frame 1651 with `map animation fragment identity is not linked`, before play.
The exact native whitelist still accepts only HB_M0S00.ANM. The director should
replace its condition in host/src/native.rs with
`!crate::maps::is_player_map_fragment(&entry.name)`; its existing owned-frame
decoder already checks stride, arena capacity and tail clearing. The new
predicate accepts only the exact overlay catalog family and rejects bad names.
Both police transitions and their timings remain unverified.

```text
SH_MAP_WARP=MAP2_S04:4
SH_MAP_DISC=<owned disc path>
SH_TRANSIT_REPLAY=<absolute docs/core/replays/police_return.txt>
SH_TRANSIT_FRAMES=3500
SH_PLAYER_TRACE=1
SH_EVENTS_TRACE=1
tools/dev-cargo.cmd test --release -p silent-hill-boot transit_walking_replay -- --ignored --nocapture --test-threads=1
```

The 43-map normal smoke remains 15 passed/28 blocked/zero crashes. Initializer
completion rises from 41 to 42 maps: MAP1_S00 now reaches a real 30-mesh frame
but fails shading (zero varied pixels). Meaningful-pixel checks remain intact.
Shared loading/control/BGM bindings add 833 descriptor providers across the
other 42 overlays; remaining story/enemy leaves are not claimed complete.

Logs/captures stay in private/work/transit. The final REPORT.md records Cargo,
layout, render and arm64 gates. The renderer's generated test artifact output
paths were redirected to this lane without changing any assertions or tracked
render source; earlier first-map-view runs used the existing move scratch path.
Root PROJECT_STATE.md and the host whitelist belong to the director/core lane.
