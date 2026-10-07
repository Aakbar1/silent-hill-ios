# Items lane: preparation and integration

Pinned reference: `d9e28f8315c7938117224f21516786d9d149a145`, USA v1.1, GPL-3.0-only.
All source paths below are relative to `game/decomp`. Generated code retains the reference notices and `LICENSE.decomp`.

## Checkpoint and verification boundary

Goal: prepare inventory, item inspection/use, maps/markers, notes, radio/flashlight, SAVELOAD, STF_ROLL, ending bookkeeping and all puzzle interaction families. Owned changes are confined to `port/sys/items/`, `tools/prepare_items.py`, `crates/sys-items/` and this document. `REPORT.md` is the requested uncommitted handoff.

The complete preparation manifest is `manifest.json` in the generated directory. It distinguishes complete translation units from selected map event functions and shared-header instantiations. Native compilation does **not** mean that these screens are currently linked into core. The harness links exact prepared original function bodies for its tests; it does not substitute new inventory or puzzle algorithms. The complete C archive is a separate strict compilation gate. Fixture identities/services in `crates/sys-items/probes.c` are linked only by test code.

Verified data/logic: original inventory add/stack/use/remove; original piano acceptance/rejection with synthetic and real disc tables; radio command restrictions in both sewer maps; flashlight/note command availability; original ranking/Next Fear preparation; original SAVELOAD save/load paths through the **read-only included existing** `host/src/saves.rs`; menu-row metadata pointer identity; overlay compiled-image reset; BSS aliases; bounded data decoding. The disc test verifies the release hash before reading anything and skips explicitly when `SH_DISC`/the default owned disc is absent.

Unverified: rendered inventory/map/puzzle/save/credits screens, touch interaction, credits timing/audio, gameplay event/camera/animation sequences, actual Apple SDK/runtime, and integration into the parallel core branch. A compile/test pass is not a playthrough.

## Original source inventory

| Prepared complete unit | Original source | Role |
|---|---|---|
| `item_screens_2` | `src/bodyprog/items/item_screens_2.c` | Inventory state machine, command dispatch, use/remove, sorting/stacking, input, result/save transitions |
| `item_screens_3` | `src/bodyprog/items/item_screens_3.c` | Inspection, inventory geometry, item loading/display, pickups, special-item bookkeeping |
| `item_screens_cam` | `src/bodyprog/items/item_screens_cam.c` | TMD binding, inventory camera/transforms, SDK callback registration |
| `item_unk_data` | `src/bodyprog/items/item_unk_data.c` | Historical duplicate item/weapon data split; compile gate only, **do not link alongside the corresponding definitions in `_3`** |
| `item_utils` | `src/bodyprog/items/item_utils.c` | Inventory exit state, time, accessory/unlock tests; gas/weapon helpers remain combat imports when integrating |
| `mapscreen` | `src/bodyprog/bodyprog_mapscreen_80066D90.c` | Paper map acquisition checks, floors, pan/zoom, player arrow, persistent markings |
| `radio` | `src/bodyprog/events/radio.c` | Radio state/noise lifecycle; native audio/NPC identities are engine imports |
| `text_messages` | `src/bodyprog/text/map_msg_display.c` | Notes/dialogue rollout, yes/no and multi-choice selection |
| `save_menu` | `src/bodyprog/sys/memcard_2.c` | Original logical-card menu population, sorting/selection, metadata references |
| `saveload` | `src/screens/saveload/saveload.c` | SAVELOAD overlay, save/load/autoload/continue/overwrite paths |
| `stf_roll` | `src/screens/credits/credits.c` | STF_ROLL overlay, normal/UFO text and presentation sequencing; includes GPL string/width tables |
| `ranking` | `src/bodyprog/ranking.c` | Result collection, ending history, Next Fear and unlock bookkeeping |

`item_rotations.h`, credits `stringtable.h`/`widthtable.h`, and native SDK/header dependencies are prepared with those units. Original glyph rendering (`text_draw.c`), generic item/event dispatch (`events_util.c`, `events_main.c`), flashlight effects (`world_effects.c`) and map/character/animation services are **core dependencies**, not additional linked lane implementations. Original PS1 card filesystem code (`memcard.c`) is replaced at the `MemCard_*` service boundary by `save_backend.c`; it is not falsely advertised as a native filesystem implementation.

`port/sys/items/puzzle_sources.json` is the exact map-function compilation list, including ordinary key/item pickups and uses. No enemy AI/player attack update source is imported. The generated namespace headers identify map-owned imports; core must bind them to its corresponding live map objects.

## Every puzzle / item-use progression family

Names describe the original interactions, not newly introduced UI. Ordinary door keys and narrative choices are included so touch coverage does not omit them.

| Interaction | Original location and principal handler(s) | Input/state and hit path |
|---|---|---|
| Doghouse house key and Keys of Lion/Woodman/Scarecrow (Door of Eclipse) | M2S00 `MapEvent_DoghouseKeyTake`, `MapEvent_HouseKeyUse`, `func_800E9DD8` | Inventory Use + common message choices; three original locks/flags, world event eligibility |
| Chemical on hand statue / gold medallion | M1S01 `MapEvent_HandStatueInspect`, `func_800D7308`, `MapEvent_GoldMedallionItemPickup` | Inventory Chemical Use, examination/confirmation; original timed event and flag 74 |
| Piano / silver medallion | M1S01 `MapEvent_PianoPuzzleInteract`, `PianoPuzzle_Control`, `func_800D857C` | Piano cursor/key regions below; five-entry sequence, waits and solved flag retained |
| Clock tower Gold/Silver medallions, school boiler activation | M1S00 `MapEvent_GoldenSunItemUse`, `MapEvent_SilverMoonItemUse`, `MapEvent_ClockTowerInspect`, `func_800D8948`, `MapEvent_Boiler0/1/2` | Inventory Use and common choices; flags 71/72/73/79/80/83/84 and original animation gates |
| Picture card / horizontal-slot door | M1S02 `MapEvent_DoorWithHorizontalSlotInteract`, `func_800DB310` | Item Use / inspection; no independent spatial cursor |
| Rubber ball / roof drain / Library Reserve key | M1S03 `MapEvent_RoofDrainPuzzleInteract0/1`, `MapEvent_LibraryReserveKeyItemTake` | Item-trigger event for ball, ordinary interact for water/drain and pickup; both drain ends remain distinct |
| School basement boiler valves | M1S02 `func_800DC1E0`, `func_800DCF00` | Common message selection: left/right/stop. Valve state `D_800E1FDC`, `D_800E1FE0/2/3`; not a free-drag valve UI |
| Plastic bottle / spilled unknown liquid | M3S01 `MapEvent_UnknownLiquidInteract`, `MapEvent_UseBottleOnLiquid`, `MapEvent_PlasticBottleTake` | Bottle Use + original event; replacement inventory item and flag preserved |
| Hospital generator | M3S01 `MapEvent_Generator0` | Common prompt/choice and `EventFlag_M3S01_GeneratorOn`; world interaction region |
| Hospital colored plates | M3S03 `func_800D1A58` and plate pickups in M3S03/M3S04 | Cursor/held plate/refund/placement geometry below; save flags and inventory item flags |
| Blood pack / bloodsucker access | M3S03 `func_800D2C2C`, `func_800D2CDC` | Inventory Use + original context event; NPC/animation execution remains core/combat |
| Alcohol + lighter on vines / basement storeroom access | M3S05 `func_800D5FC4`, `func_800D64E0` | Sequential world item uses; alcohol flag before burn event; common prompts |
| Pushing hospital cabinet / hidden-room access | M3S05 `func_800D72AC` | Common world prompt/choice; cabinet flag and original movement/animation |
| Video tape and hospital/Nowhere viewing | M3S03 `func_800D27F4`, M3S05 `func_800D6BB4`, M7S02 `func_800DB738` | Inventory Use/examine; original playback/skip/return gating |
| Drawbridge key / bridge controls | M2S01 `func_800CF7C4`; associated original map events/data | Inventory key + world controls/common choices; core binds adjoining transition callbacks |
| Sewer entrance/exit keys | M5S00 `func_800D69DC`, `MapEvent_SewerKeyTake`, `func_800D6B00` | Inventory/world use and pickup; ordinary door affordances |
| Resort numeric keypad (DORPANEL / receipt progression) | M5S01 `func_800EBA40` | Twelve original cells, four-entry history; regions below |
| Kaufmann key, receipt, safe key / motel room safe | M5S01/M5S02 `func_800EC2D8`, `func_800D54D0`, `func_800D4DF8`, `func_800D4E64`, `func_800D519C` | Inventory Use + original inspect/choice paths; no invented safe dial screen |
| Magnet / dropped motorcycle key / motorcycle compartment | M5S03 `func_800D1628`, `func_800D1ACC`, `func_800D1A84` | Item Use and pickup; motorcycle/Kaufmann outcome flags remain original map event data |
| Unknown liquid on Cybil / ending branch | `Inventory_ItemUse` (M6S04) and original Cybil event/combat consumers | Inventory Use, distance/state gates. USA's missing NPC identity restriction is retained; no JAP1 bugfix imported |
| Nowhere bird cage / Key of Phaleg | M7S01 `MapEvent_BirdCageKeyUse`, `func_800D89D8`, `func_800DC080` | Inventory key Use and common pickup; no independent cursor |
| Nowhere pliers / key in faucet | M7S01 `func_800D8FF8`, `func_800D9414` | Item Use and timed faucet event; common prompts |
| Nowhere clock / Stone of Time | M7S01 `func_800D8A5C`, `func_800DAB64` | Inventory Use and original clock event; common choices |
| Nowhere screwdriver / electrical plate / Key of Aratron | M7S01/M7S02 `func_800D9440`, `func_800DCE20`, `func_800E0FF0` | Item Use, generator/electricity eligibility, original event waits |
| Nowhere zodiac / limb-count (HS_PANEL) | M7S01 `func_800D9C9C`, M7S02 `func_800DFDDC` | Nine hit cells; `D_800E1690.field_0`/corresponding event variants, selection `D_800E156E`; regions below |
| Nowhere ALERT / Grim Reaper letter door | M7S01 `func_800D94DC`, M7S02 `func_800E32E0` | Twenty-six letter cells, five-entry history, original completion/control handling; regions below |
| Nowhere camera reveal / two 3×3 light doors | M7S01 `func_800DB3D0`, M7S02 `func_800E1398`, shared `sharedFunc_800DB60C_7_s01` | Camera inventory Use reveals clues; **27 individual light hit regions**, masks/flags 493/495 and original reset/back behavior |
| Nowhere refrigerator chain / Ring of Contract / Dagger of Melchior | M7S01 `func_800DADA8`; M7S02 `func_800DC954`, `func_800DD2D4`, `func_800DCD00` | Inventory ring Use then dagger event; original trap/eligibility branches |
| Nowhere generator, disks and electric-key eligibility | M7S02 `func_800DDEC8`, `func_800DE1FC`; related original events/flags | Common world choices and generator flag; does not grant an item by direct flag writing |
| Final altar / five sacred objects | M7S02 `func_800DF21C`, `func_800E0CB4`; Ankh, Amulet of Solomon, Crest of Mercury, Dagger of Melchior, Disk of Ouroboros pickups | Inventory/context uses and common confirmations; all five item/event requirements retained |
| Optional UFO / Channeling Stone chain | M0S02 pickup; M1S03, M4S05, M5S01, M6S01, M6S02 use handlers in compilation manifest | Context item Use at every original location; availability/order flags, original cutscene skip and UFO ending |
| Ordinary keys / locked doors | House, K. Gordon, Lobby, Library Reserve, Classroom, Drawbridge, Basement/Storeroom, Examination, Antique Shop, Sewer/Exit, Kaufmann/Safe/Motorcycle, Bird Cage, Ophiel/Hagith/Phaleg/Bethor/Aratron | Same inventory Use + event eligibility + common text choices; each pickup/use is in the source census or original generic event dispatcher |

There is no separate inventory Combine command in `e_InvCmdId`: combinations are contextual world events. Notes to School/Doghouse and Receipt use inventory Look; other memos/clues use map-message examination. No separate memo-browser overlay was found in this pinned source inventory.

## Touch lane: variables, exact geometry and gates

Puzzle screen coordinates below are the original cursor coordinates after `FP_FROM(...,12)`, relative to the 320×240 image center `(160,120)`. Convert the displayed image point back through its actual transform before writing Q12 cursor values. Bounds/inclusivity are significant. Emit one semantic Enter click on the next logic tick, with stick displacement zero. Preserve busy/loading/wait steps and Cancel behavior. A direct tap must not set solved flags or bypass the handler.

| Screen | Original writable cursor/selection | Original hit region / action |
|---|---|---|
| Piano | `g_PianoCursorX/Y`, `g_PianoKeyCounter`; `g_SysWork.sysStateSteps[1]` | X clamp [-89,85], Y [-71,84]. For Y<34, key = first `g_PianoKeys[i]` strictly greater than X (else 11). For Y≥34, boundaries -64,-39,-14,11,36,61 select white-key indices 0,2,4,5,7,9,11. Include all black/silent keys. Odd step[1] values take their original waits, step 4 takes its sound transition; other even values normalize to zero before ordinary input |
| Colored plates | `D_800D6BD0/4` Q12 cursor; plate/slot state `D_800D8140[4]`, held/animation selection `D_800D8144`, availability `D_800D8145`, animation `D_800D6BD8`; save plate flags | Cursor clamp [-100,100] both axes. For each `D_800D6B40[i]`: X ∈ [px−160,px−132), Y ∈ [py−120,py−92). Held-plate and placement/removal phases use the same original table. Cancel returns held inventory before exit |
| DORPANEL | `D_800F0354/8` Q12 cursor, `D_800F0350[4]` history, `D_800F0170[4]` expected indices | Clamp X [-160,160], Y [-120,120]. Each of 12 `D_800F0158[i]` cells: X ∈ [px−160,px−114], Y ∈ [py−120,py−74]. Cells 0..10 shift history; 11 checks it. Preserve index 10's original filler/control semantics |
| ALERTDOR M7S01 | `sharedData_800E2CA8/2CAC_7_s01` cursor, `D_800E1688[5]` history; `D_800E1544[5]` expected sequence | Clamp both axes [-120,120]. 26 `D_800E1510[i]` centers: bounding X [px−174,px−146], Y [py−134,py−106], plus squared distance from (px−160,py−120) **?196**. Retain letter-history reset/completion and Back |
| ALERTDOR M7S02 | Same shared cursor; `D_800EA4AC[5]` history, `D_800E9E1C[5]` expected sequence; centers `D_800E9DE8[26][2]` | Corresponding distance? ?196 and radius-14 bounding checks in `func_800E32E0`; same center-relative transform and phase-4 interaction gate |
| HS_PANEL, both variants | Same shared cursor; `D_800E156E` / counterpart selected number; active symbol selector `D_800E1690.field_0` / counterpart | Clamp [-80,80]. Row i/column j: X ∈ [35j−46,35j−24], Y ∈ [35i−37,35i−15], and squared distance from (35j−35,35i−26) **<122**. Value = j+3i+1. Keep preparation prompt (step 4), input step 9 and original Back |
| 3X3DOR, both variants | Same shared cursor; `sharedData_800E1694_7_s01` bit mask; `sharedData_800E1570/1574_7_s01` targets | Clamp X [-115,115], Y [-105,105]. All 27 `sharedData_800E1578_7_s01[i][j][k]` centers: inclusive bounding X [px−169,px−151], Y [py−129,py−111], distance² **≤81**. Bit = `(i*9)+(k*3)+j`, distinct from array storage order. Cancel clears nonzero mask, otherwise leaves; preserve flags 493/495 |
| Elevators (six map variants) | `sharedData_800D4D10/4D14_3_s01` cursor, `sharedData_800D4D0C/4D18_3_s01` selection/state; table `sharedData_800D4CD4_3_s01` | Clamp X [-70,68], Y [-110,110]. X lower bound px−160; upper bound px−140 inclusive for cell 0, px−141 inclusive for other cells. Y [py−120,py−100]. Use the compiled variant's `MAX_IDX`, floor availability and original open/travel states; Skip only when the travel branch allows it |
| World item-use puzzles / doors | `g_Inventory_SelectedItemIdx`, `g_Inventory_CmdSelectedIdx`, `g_SysWork.invItemSelectedIdx`, `playerWork.extra.lastUsedItem`, `g_MapEventData` activation/point | Tap eligible interact object or inventory Use. Hit geometry is the original event/collision trigger region from the live map, not a new puzzle rectangle. Preserve facing/distance and requested-item filtering |
| Common choices / boiler / generators / narrative prompts | `g_MapMsg_Select.selectedEntryIdx`, `.maxIdx`, `g_MapMsg_SelectCancelIdx`; rollout/menuSelection statics in `Gfx_MapMsg_Draw` | Choice text starts at (32,96+16i) in original text coordinates. Use the emitted text glyph bounds for row hit regions. Keep text rollout/Enter/Cancel/held Skip distinction. Left/right/stop valve choices use this same selection |
| Inventory panes | `g_Inventory_SelectionId`, `g_Inventory_PrevSelectionId`, `g_Inventory_SelectedItemIdx`, `g_Inventory_CmdSelectedIdx`, scrolling timers | Original USA inner outlines (x,y,w,h): item (-32,-52,64,128); equipped (-48,-200,96,144); Exit (-48,185,96,24); Settings (-144,185,96,24); Map (48,185,96,24); item/equipped command (48,-200,96,128); health (-144,-200,96,156); examine (-48,-200,192,256). These are centered inventory-render coordinates, not 320×240 puzzle pixels |
| Inventory thumbnails / commands | Same variables; `D_800C3E18[0..9]` maps displayed models to inventory slots; `g_Items_Coords`, `g_Items_Transforms` | Original controller UI has no thumbnail mouse hit test. Use projected live model/selection bounds and the emitted command text rectangles; resolve display slot through `D_800C3E18`, retain scroll/reload/equip availability. Index 7 aliases equipped index; index 9 belongs to pickup display |
| Item details / note Look | `InvSelectionId_Examine`, item/detail timers and gameStateSteps in `ItemScreen_*` | Native inspection image/model/text bounds; original Enter/Cancel readiness gate in `item_screens_3.c` inspection input path. No memo page-list cursor to invent |
| Paper map / floor selection / zoom | function statics `paperMapIdx`, `markingIdx`, `activeMarkingFileIdx`, `screenPosX/Y`; globals `D_800C4454` zoom, `D_800AE770` mode; `savegame.paperMapIdx`, `.paperMapFlags` | Map image rectangle is drag/pan region. Enter toggles original zoom; floor controls follow `D_800AE740` adjacent-map table and acquired-map checks. Preserve darkness/flashlight restrictions and marker flags |
| SAVELOAD slots / rows | `g_SelectedSaveSlotIdx`, `g_SlotElementSelectedIdx[2]`, `g_Savegame_SelectedElementIdx`, `g_SelectedDeviceId`, `g_SelectedFileIdx`; visual offset arrays in overlay | Two columns offset by 150, row pitch 20. Original slot/frame X range `150*slot−142 .. 150*slot−6`; derive each visible row Y using `SaveScreen_SaveBorderDraw`/`SaveScreen_NavigationDraw` and current scroll offsets. Select the logical row, not its painted ordinal; resolve native 24-byte row's metadata/file/element identity |
| SAVELOAD overwrite / format choices | hoisted `sh_saveload_SaveScreen_LogicUpdate_isSaveWriteOptionSelected`; `gameStateSteps[1]`, formatting/new-save flags | Use original Yes/No text/outline bounds from `SaveScreen_WriteOptionsStepDraw`; Enter/Cancel retains meaning. Native cards are always formatted: no filesystem format action is offered by the service |
| Autoload / Continue | `g_GameState_AutoLoadSavegame_Funcs`, selection fields above; gameStateSteps | Original newest-save selection and continue transition; tap only where original screen permits manual confirmation/Back |
| Credits / results | `sh_stf_roll_D_801E5E88`, ending index `D_801E5E8C`, text states/scroll values; item-screen result gameStateSteps | No positional credits cursor. Provide original Skip/confirm affordances at the branches reading semantic pad input (`credits.c`); retain normal versus UFO credit loops and result/save transitions |

Map and overlay namespace headers translate these original names to lane names. For direct taps, setters belong in the corresponding C unit/namespace so touch never guesses work-record offsets. Zero normalized stick input for the tap tick; do not also issue directional pulses that move the cursor again.

## Wire / native records and data import

| Record | Wire bytes | Native bytes / decoding |
|---|---:|---|
| Inventory entry | 4 | 4, explicit byte fields |
| Save payload | **636** | Original C `s_Savegame` remains 636; Rust lossless owner preserves every reserved byte, signed difficulty nibble and bit pattern |
| Save metadata | 12 | 12; total count/timer/count/location/packed flags decoded separately |
| Save menu row | 16 | C native 24; metadata pointer at 16, resolved explicitly, never a serialized pointer |
| PS1 save/footer/options/header containers | 640 / 4 / 128 / 256 | Size asserted for schema awareness; **never** written into native `.shs` files |
| Credits state / 3D state | 28 / 88 | 40 / 104; owned width/color tables and explicit pointer offsets |
| Ending parameters | 6 each | 6; six original entries, including unused/fifth-ending cases |
| Map marker descriptor | 12 | 24; two owned u32 tables, first/end event flag; NULL rows remain unavailable |
| Panel coordinate pair | 2 | 2, unsigned pixel coordinates; reused for plates, keypad, letters, lights and elevators |
| Generic signed point | 4 | 4, explicit LE signed halves |
| Piano data block | 17 (includes one pad byte) | 16, 11 thresholds + 5 indices; failed decoding does not publish partial C state |
| TMD header / object | 12 / 28 | 24 / 48, stable owned model/vertex/normal/primitive tables; flags mark native pointer readiness |

`abi.h`/`abi.c` retain independent wire/native assertions. `native_sizes.json` freezes only explicitly migrated work-record sizes; unchanged original assertions stay active. Mixed-width controller/collision/spawn/camera bitfields are made explicit, with one layout on MSVC and arm64. Generated SDK `long` words are fixed 32-bit scalars; host pointers and CRT sizes stay native.

For TMD, relative offsets resolve from byte 12 (object-table base), not a host address. The original mapper consumes TMD data after its file ID and marks mapped data; see the original [Sony library reference](https://psx.arthus.net/sdk/NetYaroze/Net%20Yaroze%20-%20Library%20Reference.pdf). `GameFs_TmdDataAlloc` now validates the already-decoded native header; core must publish a native TMD graph before calling it. Do not relocate immutable disc bytes or run `GsMapModelingData` on the native graph.

`data_tables.json` and `DISC_TABLES` provide 21 numeric puzzle-table descriptors (including both letter/light variants and all six elevators). Address provenance distinguishes explicit pinned symbols from source identifier addresses; the latter are additionally checked for live-disc bounds and sequence/mask shape. These checks establish data decoding, not rendered alignment.

Actual-disc data checks: 87 `ITEM/*.TMD` files through Rust decode + C graph probes; MAP1_S01 piano thresholds/sequence; MAP5_S01 12 panel coordinates; STF_ROLL six ending records and width/color tables; all 24 BODYPROG map-marker descriptors and their bounded leaves. BODYPROG alone in this set is encrypted: `encrypted_data_leaf` derives the exact wrapping XOR seed from `src/main/fileinfo.c::Fs_DecryptOverlay` and decrypts only requested data spans. Plain map/credits overlays are read directly. No overlay machine code is copied into the repo, relocated or executed.

## Core plug-in recipe

1. Initialize the pinned submodule. Generate once with `python -X utf8 tools/prepare_items.py --out <core OUT_DIR/items>`. Compile the units in `units.txt` with the generated include directory first, `VER_USA`, `SKIP_ASM`, C11 and strict warnings. Exclude `item_unk_data` from production linkage and avoid duplicate bodyprog functions already linked by core/combat. Compile `abi.c`/`save_backend.c`. Never link `probes.c` or `headless_original.c` into the game.
2. Establish **one shared native ABI** before removing guards. This baseline core's `port/boot.h` has partial `PortGameWork`/`PortSysWork`, while prepared original consumers use full `s_GameWork` (1496) and `s_SysWork` (11760). The prepared original map header (4760) is also different from core's explicit native map descriptor. **No cast between those layouts is valid.** Core must extend/migrate the authoritative records or translate these consumers to its checked accessors, and re-run both-side offset gates. `native_sizes.json` and generated headers supply exact definitions; preserve the existing independent broad original-header gate.
3. Bind live `g_GameWork`, `g_SavegamePtr`, `g_SysWork`, controller, item/event/animation and GPU/audio identities. `port_scratch` is the shared aligned native scratch arena. `sh_items_address(u32)` must resolve original arena identities to bounded native allocations (including sub-buffer aliases), never cast a PS1 address to a host pointer. Keep decoded TMD owners alive; GsLinkObject4 consumes a native object record, not wire offsets. Implement the listed libgs TMD render/camera services in the appropriate core/GPU lane.
4. At native screen queue completion select SAVELOAD/STF_ROLL namespace and call `sh_saveload_reset` / `sh_stf_roll_reset`. Replace only their guards with `sh_saveload_GameState_LoadSavegameScreen_Update`, `sh_saveload_GameState_AutoLoadSavegame_Update`, `sh_stf_roll_GameState_Credits_Update` and the matching source-driven state dispatch. No BIN code bytes are loaded as executable native code. Reset restores 27 SAVELOAD and 24 STF_ROLL writable objects, including hoisted locals and credits width/cache/text state; immutable dispatch tables stay constant. `overlay_inventory.json` rejects inventory drift.
5. Keep the existing `port_save_read/write` callbacks unchanged (0 success, 1 missing, 2 invalid/I/O), exact 636-byte payloads. Add the **reviewable unapplied** `private/work/items/core-integration.patch`: explicit workspace registration for this checkout, removal of the harness's nested workspace, and two sidecar callbacks using the tested `host_sidecars.rs`. It passes `git apply --check` on this baseline. It deliberately does not remove runtime guards before step 2. Rebase its context on core5's current files rather than applying blindly.
6. Service `sh_items_save_service_tick()` once per logic tick, before the next SAVELOAD update. `sh_items_slot_id` maps only original devices 0/4 and file 0..14/save 0..10 to `card*165+file*11+save` (0..329). Metadata `.shmi` files are 12 bytes; options `.shoc` files are 56 bytes, id `card*15+file`. Both sidecars use atomic replacement independently of the save payload; a sidecar failure returns original UI I/O failure (the payload may already have succeeded). Older payload-only native slots recover display metadata and retain active options until a new save; their payloads remain untouched. Init/query/menu services preserve original `memcard_2` row logic. Reset the service on session/warm reset, not on every screen update.
7. Bind the map event callbacks in `private/work/items/event-bindings.json` to the director's map descriptors; bind the associated namespaced live map objects and shared puzzle tables from the generated namespace headers. Use original C initializers where available, otherwise bounded numeric table reads from the owned disc using pinned symbol offsets. Keep the original event/camera/animation and item-trigger order. Combat retains NPC/weapon behavior and those imports. Map partial linkage must still guard missing engine consumers.
8. Touch supplies validated C setters for the variables/regions above and semantic actions through active controller bindings. Only after ABI/services/data bindings are complete should core replay inventory/map/pickup, SAVELOAD overwrite/load/continue/autoload, both credits types and every puzzle/event variant. Record rendered/touch passes separately from the existing compile/headless/data passes.

## Commands and handoff evidence

```text
cargo fmt --manifest-path crates/sys-items/Cargo.toml -- --check
tools\dev-cargo.cmd clippy --manifest-path crates/sys-items/Cargo.toml --all-targets -- -D warnings
tools\dev-cargo.cmd test --manifest-path crates/sys-items/Cargo.toml
python -X utf8 tools/prepare_items.py --out <private/work/items/clang> --clang-compiler C:/BuildTools2022/VC/Tools/Llvm/x64/bin/clang-tidy.exe
git apply --check <private/work/items/core-integration.patch>
```

The clang gate targets `aarch64-apple-ios15.0` with `-Wall -Wextra -Werror`, includes compiler diagnostics (not silently filtered warnings), and uses core's compile-only CRT declarations because this installation lacks Apple SDK/resource headers. It is not an Apple SDK build. `manifest.json`, `units.txt`, per-unit logs, `clang-results.json`, the integration patch, and event bindings live under `C:/Claude Projects/Silent Hill iOS/private/work/items/`. Test-generated saves are synthetic and kept in per-process temporary directories.

Director requests: matching full work/map ABI and live map-data bindings; TMD/GPU/audio/event import implementations; sidecar callback registration and logic-tick service; source-driven screen activation/dispatch after those prerequisites. No system installation, main merge, push, spend or asset distribution is requested.
