# Events replay handoff

- `opening_noskip.txt`: skip the title attract movie only, select New Game, then
  send no input. The 338-frame opening movie and messages 15-19 play normally;
  the original opening callback finishes at 4296 and its border/control state
  finishes settling at 4359. `python tools/milestones.py --opening-noskip` checks
  the movie, every callback step, gradual text/page completion and restored play.
- `first_area_combat.txt` is a **candidate**, not a passed combat replay. It runs
  into the footsteps trigger, waits through the 119-frame Cheryl movie/spotted
  event, turns west, follows into the alley, runs south and interacts with the
  first gate. Four-frame action pulses are required here; one-frame pulses missed
  the gameplay input sample. The exact guard is the generic room-transition
  state at VBlank 9552. No positions, completion flags, weapons or HP are injected.
- `python tools/milestones.py --first-area-combat` must fail at that boundary.
  Its evaluator requires native `COMBAT_HIT` records plus observed Harry and
  enemy HP loss within three VBlanks of the corresponding hit. Queued damage,
  static HP, scripted non-enemy actors and synthetic fixtures cannot pass it.

Native integration uses one live work ABI and stable owned model/ANM graphs.
No harness provider is linked into the playable host. The combat lane's pinned
production migration supplies original attacks; the existing actor scheduler
retains its ordering. Variable Stalker frame divisors preserve PS1 DIV results,
including zero and signed overflow, rather than invoking undefined native C.
Numeric map imports validate pinned symbol addresses, callback identities and
record/index bounds. No disc bytes or captures are stored in Git.

Run the events arm64 frontend over the actual prepared production units, not
only the older core checker: private/work/events/check-events-clang.py. This is
a freestanding LLVM frontend check with compile-only CRT declarations; it does
not certify an Apple SDK build or device runtime. Private results/logs live in
private/work/events; REPORT.md lists the final measured checks.
