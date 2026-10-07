# core4 replay evidence

The requested pass is incomplete: movie playback works, OPTION is linked, and title/difficulty selection works; SAVELOAD, STF_ROLL, opening gameplay and all map transitions remain open.

`tools/milestones.cmd` builds release C with /W4 /WX. Its default full-brief gate exits 2 while required milestones are pending. `tools/milestones.cmd --available --repeat 2` runs the implemented suite headlessly with audio explicitly off, no live input or wall-clock pacing, in fresh processes. It checks exit 0, exact tick count/state/step, applicable menu selections/movie counters, and at least 1000 nonblack pixels. Repeated runs must produce identical PNG SHA-256 values. These are native determinism checks; PS1 golden-image comparisons and audible playback remain unverified. Each replay has a 180-second process timeout.

| Replay | Ticks | Expected endpoint |
|---|---:|---|
| boot.txt | 850 | KCET, state 2 / step 6 |
| movie-intro.txt | 1900 | intro, state 6, 147 decoded frames |
| movie-intro.txt | 9800 | natural movie end, 2060 decoded frames, title state 7 / step 1 |
| title.txt | 1650 | Start skip after 22 decoded frames, visible title |
| new-game-menu.txt | 1650 | original difficulty selector, menu 3 |
| new-game-opening.txt | 2200 | original spawn -> opening movie, state 9 / step 0; 153 cumulative movie frames |
| options.txt | 1850 | OPTION, state 18 / step 1 / EXIT selected |
| options-reentry.txt | 2500 | OPTION -> title -> OPTION, state 18 / step 1 / EXIT selected |
| options-brightness.txt | 1750 | brightness screen, state 18 / step 3 |
| options-controller.txt | 1750 | controller binding screen, state 18 / step 4 |

`--only options-brightness options-controller` selects a debug subset and still requires `--available`; it cannot bypass the full-brief gate. `--disc PATH` overrides the player-owned disc path. Input rows are completed-VBlank tick, active-high pad hex, optional axes; the original game derives clicks and menu pulse timing.

Core4 captures, logs and results.json live under `C:/Claude Projects/Silent Hill iOS/private/work/core4/milestones/<UTC run>/`. Ten cases passed twice (20 runs, 10 matching PNG SHA-256 comparisons) at `20261007T142509849872Z`. No game assets or captures are in Git. Historical core3 evidence remains under private/work/core3, including its queued-TIM lifetime fix and earlier corruption captures.

`new-game-blocker.txt` retains the historical input path; the descriptor guard is now passed. `new-game-opening.txt` reaches the movie after native descriptor/frame loading and original spawn/reset. Spawn is (-25395,0,657408), heading/initial camera heading 2048; movement and camera follow are unverified. At tick 2200 the 153 cumulative frames comprise 22 intro and 131 opening frames.

`new-game-opening-skip.txt` is excluded from passing milestones. Start at 2201 reaches state 10/step 0, then the original loading dispatcher stops at map0_s00/GameBoot_LoadScreen_PlayerRun at 2204 (native code 3, process exit 1). Private evidence: new-game-opening.png/.log and new-game-world-guard.png/.log under private/work/core4. No first_map.txt pass was added. A deliberately wrong opening endpoint exits 1; the full-brief gate still exits 2.

Native saves: `LOCALAPPDATA/SilentHillIOS/saves/slot-NNN.shs`, 330 stable slot identities (two original cards × 15 files × 11 saves). Each file is the exact 636-byte original save payload, preserving unknown bytes and signed bit patterns. Writes sync a new temporary file then atomically replace the slot. Tests exercise actual Windows replacement and malformed-length rejection. C read/write hooks are bounded; SAVELOAD/memory-card UI and settings persistence remain unconnected, so cards still report absent.
