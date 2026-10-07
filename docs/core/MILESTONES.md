# core2 replay evidence

The requested pass is incomplete: movie playback works, OPTION is linked, and title/difficulty selection works; SAVELOAD, STF_ROLL, opening gameplay and all map transitions remain open.

`tools/milestones.cmd` builds release C with /W4 /WX. Its default full-brief gate exits 2 while required milestones are pending. `tools/milestones.cmd --available --repeat 2` runs the implemented suite headlessly, with no live input or wall-clock pacing, in fresh processes. It checks exit 0, exact tick count/state/step, applicable menu selections/movie counters, and at least 1000 nonblack pixels. Repeated runs must produce identical PNG SHA-256 values. These are native determinism checks, not original-PS1 golden-image comparisons. Each replay has a 180-second process timeout.

| Replay | Ticks | Expected endpoint |
|---|---:|---|
| boot.txt | 850 | KCET, state 2 / step 6 |
| movie-intro.txt | 1900 | intro, state 6, 147 decoded frames |
| movie-intro.txt | 9800 | natural movie end, 2060 decoded frames, title state 7 / step 1 |
| title.txt | 1650 | Start skip after 22 decoded frames, visible title |
| new-game-menu.txt | 1650 | original difficulty selector, menu 3 |
| options.txt | 1850 | OPTION, state 18 / step 1 / EXIT selected |
| options-reentry.txt | 2500 | OPTION -> title -> OPTION, state 18 / step 1 / EXIT selected |
| options-brightness.txt | 1750 | brightness screen, state 18 / step 3 |
| options-controller.txt | 1750 | controller binding screen, state 18 / step 4 |

`--only options-brightness options-controller` selects a debug subset and still requires `--available`; it cannot bypass the full-brief gate. `--disc PATH` overrides the player-owned disc path. Input rows are completed-VBlank tick, active-high pad hex, optional axes; the original game derives clicks and menu pulse timing.

All captures, logs and results.json live under `C:/Claude Projects/Silent Hill iOS/private/work/core2/milestones/<UTC run>/`. No game assets or captures are in Git. Separate visually reviewed captures include intro-1900.png, title-1650.png, difficulty-1650.png, options-1850.png, reentry-reviewed.png, brightness-reviewed.png and controller-reviewed.png under private/work/core2. Use a new filename for visual review after replacing a capture: the image-preview tool retained an earlier image at an overwritten path in this run.

`new-game-blocker.txt` is a diagnostic replay, excluded from passing milestones. At tick 1650, NORMAL confirmation initializes the original save record, then GameBoot_WorldInit's guard stops with native code 3/process exit 1. This is not a first-map result. Its private log and capture are new-game-blocker.log/png.

Native saves: `LOCALAPPDATA/SilentHillIOS/saves/slot-NNN.shs`, 330 stable slot identities (two original cards × 15 files × 11 saves). Each file is the exact 636-byte original save payload, preserving unknown bytes and signed bit patterns. Writes sync a new temporary file then atomically replace the slot. Tests exercise actual Windows replacement and malformed-length rejection. C read/write hooks are bounded; SAVELOAD/memory-card UI and settings persistence remain unconnected, so cards still report absent.
