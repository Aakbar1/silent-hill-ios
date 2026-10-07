# Touch coverage (US v1.1)

Derived independently from read-only decomp commit `d9e28f8315c7938117224f21516786d9d149a145`.
This checks touch -> pad/UI requests. It does **not** prove the game is playable end to end:
the host/core adapters, actual UI hit regions and every encounter still need integration.
Every row has its own named Rust test replaying `tests/replays/coverage.jsonl` with literal
expected pad bits/axes/actions. A test enforces an exact row/replay ID match.

Native integration evidence is separate from the standalone rows: touchwire's
`milestones.json` and `tests/replays/native-*.jsonl` drive the real C game using
live native context. Intro skip, title, New Game difficulty entry/back, options,
and a direct brightness-row tap passed twice with identical overlay PNG hashes.
These replays contain no pad/context injection. Title/difficulty row targeting,
aiming/ownership/puzzle context, headless backend injection and root milestone
registration still require core hooks. See README.md and PROJECT_STATE.md.

Source aliases (relative to the reference decomp): PC=`src/bodyprog/player_control.c`;
EM=`src/bodyprog/events/events_main.c`; EU=`src/bodyprog/events/events_util.c`;
GS=`src/bodyprog/events/game_sys_states.c`; IS=`src/bodyprog/items/item_screens_2.c`;
MS=`src/bodyprog/bodyprog_mapscreen_80066D90.c`; MSG=`src/bodyprog/text/map_msg_display.c`;
TM=`src/bodyprog/events/title.c`; OPT=`src/screens/options/options.c`;
SL=`src/screens/saveload/saveload.c`; WB=`src/bodyprog/sys/warm_boot.c`;
VC=`src/bodyprog/view/vc_main.c`; MxSyy=`src/maps/mapx_syy/mapx_syy[ _2].c`.
See `INPUT_AUDIT.md` for every source pad-consumer function and map-event callback table.

Gameplay uses original tank controls. Left stick moves/turns, outer push runs, quick down
swipe sends both step inputs. Right tap acts, hold aims, subsequent tap/hold/repeated taps
attack. Aim can latch on release for one-finger use; AIM chip releases it; while-held mode
is configurable. Camera and step chips hold their original pad input. Ordinary context
chips only appear when host availability permits. Direct UI taps never also issue a pad
click. Every direct target is supplied by the host from the current visible UI, not a
hardcoded solution or an unconditional save/item mutation. Unknown event identities are
explicitly retained in the callback inventory and use action/item/message paths.

The puzzle element and item numbers in tests are **opaque demo IDs**, not game solutions.
Panel-specific C adapters must interpret those IDs against actual current panel/slot state,
including wrong choices, cancellation, animation guards and item prerequisites. Pad fallback
targets accept all 16 buttons and both axes when a legacy/debug panel needs them. Debug
paths are not ordinary gameplay chips and do not authorise new save points or unlocks.

| Test ID | Action / menu / puzzle | Decompiled evidence | Touch path / replay | Status / uncertainty |
| --- | --- | --- | --- | --- |
| `walk_forward` | Walk forward | PC: Player_Controller; Player_CharaTurn_0/_1 | Floating stick up, 38pt | Pad path confirmed; on-device tuning provisional |
| `walk_backward` | Back away | PC: Player_Controller; Player_CharaTurn_0/_1 | Floating stick down slowly, 38pt | Pad path confirmed; on-device tuning provisional |
| `turn_left` | Turn left | PC: Player_Controller; Player_CharaTurn_0/_1 | Floating stick left, 38pt | Pad path confirmed; on-device tuning provisional |
| `turn_right` | Turn right | PC: Player_Controller; Player_CharaTurn_0/_1 | Floating stick right, 38pt | Pad path confirmed; on-device tuning provisional |
| `run` | Run by stick push distance | PC: Player_Controller; Player_CharaTurn_0/_1 | Floating stick up, 64pt | Pad path confirmed; on-device tuning provisional |
| `backward_hop` | Backward hop / retreat with run modifier (core-dependent) | PC: Player_Controller; Player_CharaTurn_0/_1 | Floating stick down, 64pt (below quick-turn distance) | Pad path confirmed; on-device tuning provisional |
| `quick_turn` | 180-degree turn | PC: Player_Controller; Player_CharaTurn_0/_1 | Swipe down 80pt in 100ms | Pad path confirmed; on-device tuning provisional |
| `interact` | Interact / examine / unlock doors | PC: Player_Controller; EM: Event_Update | Right area tap | Pad path confirmed; core decides state-dependent result |
| `pickup` | Pick up any item, including weapons, ammo and maps | PC: Player_Controller; EM: Event_Update | Right area tap | Pad path confirmed; core decides state-dependent result |
| `kick_stomp` | Kick / stomp downed enemies | PC: Player_Controller; EM: Event_Update | Right area tap | Pad path confirmed; core decides state-dependent result |
| `escape_grab` | Action input during struggle / get-up | PC: Player_Controller; EM: Event_Update | Right area tap | Pad path confirmed; core decides state-dependent result |
| `aim_auto` | Aim / original auto-target (respect auto-aim option) | PC: Player_Controller; Player_CombatUpdate | Right hold 280ms | Aim bit confirmed; target selection remains in C, not tested here |
| `attack_tap` | Fire / swing with a tap while aiming | PC:8556-8575 | Hold to aim then tap right | Source input confirmed; host/core integration required |
| `attack_hold` | Held melee attacks / sustained firearm / chainsaw / drill | PC:8556-8575; WEAPON_ATTACK input types | Hold to aim then hold attack | Source input confirmed; host/core integration required |
| `attack_repeat` | Repeated melee attacks / repeated trigger taps | PC:8556-8575 | Hold to aim then repeated right taps | Source input confirmed; host/core integration required |
| `step_left` | Sidestep left | PC:8526-8550; GS:236-263; VC: vcMoveAndSetCamera | Labelled chip tap/hold | Source input confirmed; host/core integration required |
| `step_right` | Sidestep right | PC:8526-8550; GS:236-263; VC: vcMoveAndSetCamera | Labelled chip tap/hold | Source input confirmed; host/core integration required |
| `look` | Hold camera look | PC:8526-8550; GS:236-263; VC: vcMoveAndSetCamera | Labelled chip tap/hold | Source input confirmed; host/core integration required |
| `flashlight` | Toggle flashlight | PC:8526-8550; GS:236-263; VC: vcMoveAndSetCamera | Labelled chip tap/hold | Source input confirmed; host/core integration required |
| `open_map` | Open map | PC:8526-8550; GS:236-263; VC: vcMoveAndSetCamera | Labelled chip tap/hold | Source input confirmed; host/core integration required |
| `open_inventory` | Open inventory/status | PC:8526-8550; GS:236-263; VC: vcMoveAndSetCamera | Labelled chip tap/hold | Source input confirmed; host/core integration required |
| `pause` | Pause | PC:8526-8550; GS:236-263; VC: vcMoveAndSetCamera | Labelled chip tap/hold | Source input confirmed; host/core integration required |
| `radio` | Toggle radio when owned | IS:928-942 (no dedicated radio pad bit) | RADIO chip -> inventory toggle request | Source input confirmed; host/core integration required |
| `unpause` | Resume paused gameplay | GS: SysState_GamePaused_Update | Tap labelled legacy target (not a gamepad overlay) | Source input confirmed; host/core integration required |
| `boss_split_head` | Boss combat: split head | src/maps/characters/split_head.c; PC | Hold to aim then tap right | Uses original combat inputs; encounter outcome not verified |
| `boss_twinfeeler` | Boss combat: twinfeeler | src/maps/characters/twinfeeler.c; PC | Hold to aim then tap right | Uses original combat inputs; encounter outcome not verified |
| `boss_floatstinger` | Boss combat: floatstinger | src/maps/characters/floatstinger.c; PC | Hold to aim then tap right | Uses original combat inputs; encounter outcome not verified |
| `boss_monster_cybil` | Boss combat: monster cybil | src/maps/characters/monster_cybil.c; PC | Hold to aim then tap right | Uses original combat inputs; encounter outcome not verified |
| `boss_incubator` | Boss combat: incubator | src/maps/characters/incubator.c; PC | Hold to aim then tap right | Uses original combat inputs; encounter outcome not verified |
| `boss_incubus` | Boss combat: incubus | src/maps/characters/incubus.c; PC | Hold to aim then tap right | Uses original combat inputs; encounter outcome not verified |
| `title_new` | Title: new game | TM: GameState_MainMenu_Update; OPT; WB | Tap visible menu target | Source input confirmed; host/core integration required |
| `title_continue` | Title: continue / auto-load | TM: GameState_MainMenu_Update; OPT; WB | Tap visible menu target | Source input confirmed; host/core integration required |
| `title_load` | Title: load | TM: GameState_MainMenu_Update; OPT; WB | Tap visible menu target | Source input confirmed; host/core integration required |
| `title_movie` | Title: opening movie / demo | TM: GameState_MainMenu_Update; OPT; WB | Tap visible menu target | Source input confirmed; host/core integration required |
| `difficulty` | New game difficulty selection | TM: GameState_MainMenu_Update; OPT; WB | Tap visible menu target | Source input confirmed; host/core integration required |
| `options_open` | Open main options | TM: GameState_MainMenu_Update; OPT; WB | Tap visible menu target | Source input confirmed; host/core integration required |
| `options_extra` | Enter/leave extra options | TM: GameState_MainMenu_Update; OPT; WB | Tap visible menu target | Source input confirmed; host/core integration required |
| `menu_back` | Cancel / back from any menu | TM: GameState_MainMenu_Update; OPT; WB | Tap visible menu target | Source input confirmed; host/core integration required |
| `menu_confirm` | Confirm menu choice | TM: GameState_MainMenu_Update; OPT; WB | Tap visible menu target | Source input confirmed; host/core integration required |
| `return_title` | Return to title / quit through native confirmation | TM: GameState_MainMenu_Update; OPT; WB | Tap visible menu target | Source input confirmed; host/core integration required |
| `option_screen_x` | Screen position horizontal | OPT: OptionsMenu_UpdateMainMenu / UpdateExtraMenu / controller / brightness / position | Tap visible menu target | Source input confirmed; host/core integration required |
| `option_screen_y` | Screen position vertical | OPT: OptionsMenu_UpdateMainMenu / UpdateExtraMenu / controller / brightness / position | Tap visible menu target | Source input confirmed; host/core integration required |
| `option_brightness` | Brightness | OPT: OptionsMenu_UpdateMainMenu / UpdateExtraMenu / controller / brightness / position | Tap visible menu target | Source input confirmed; host/core integration required |
| `option_controller` | Controller preset / per-action binding | OPT: OptionsMenu_UpdateMainMenu / UpdateExtraMenu / controller / brightness / position | Tap visible menu target | Source input confirmed; host/core integration required |
| `option_vibration` | Vibration | OPT: OptionsMenu_UpdateMainMenu / UpdateExtraMenu / controller / brightness / position | Tap visible menu target | Source input confirmed; host/core integration required |
| `option_autoload` | Auto-load | OPT: OptionsMenu_UpdateMainMenu / UpdateExtraMenu / controller / brightness / position | Tap visible menu target | Source input confirmed; host/core integration required |
| `option_sound` | Stereo / mono | OPT: OptionsMenu_UpdateMainMenu / UpdateExtraMenu / controller / brightness / position | Tap visible menu target | Source input confirmed; host/core integration required |
| `option_bgm` | Music volume | OPT: OptionsMenu_UpdateMainMenu / UpdateExtraMenu / controller / brightness / position | Tap visible menu target | Source input confirmed; host/core integration required |
| `option_sfx` | Effects volume | OPT: OptionsMenu_UpdateMainMenu / UpdateExtraMenu / controller / brightness / position | Tap visible menu target | Source input confirmed; host/core integration required |
| `option_weapon` | Extra weapon hold/toggle | OPT: OptionsMenu_UpdateMainMenu / UpdateExtraMenu / controller / brightness / position | Tap visible menu target | Source input confirmed; host/core integration required |
| `option_blood` | Blood colour | OPT: OptionsMenu_UpdateMainMenu / UpdateExtraMenu / controller / brightness / position | Tap visible menu target | Source input confirmed; host/core integration required |
| `option_view` | Extra view control | OPT: OptionsMenu_UpdateMainMenu / UpdateExtraMenu / controller / brightness / position | Tap visible menu target | Source input confirmed; host/core integration required |
| `option_view_mode` | Extra view mode | OPT: OptionsMenu_UpdateMainMenu / UpdateExtraMenu / controller / brightness / position | Tap visible menu target | Source input confirmed; host/core integration required |
| `option_retreat` | Retreat turn | OPT: OptionsMenu_UpdateMainMenu / UpdateExtraMenu / controller / brightness / position | Tap visible menu target | Source input confirmed; host/core integration required |
| `option_run` | Walk/run inversion | OPT: OptionsMenu_UpdateMainMenu / UpdateExtraMenu / controller / brightness / position | Tap visible menu target | Source input confirmed; host/core integration required |
| `option_autoaim` | Auto aiming | OPT: OptionsMenu_UpdateMainMenu / UpdateExtraMenu / controller / brightness / position | Tap visible menu target | Source input confirmed; host/core integration required |
| `option_bullet` | Bullet adjustment | OPT: OptionsMenu_UpdateMainMenu / UpdateExtraMenu / controller / brightness / position | Tap visible menu target | Source input confirmed; host/core integration required |
| `option_defaults` | Restore defaults | OPT: OptionsMenu_UpdateMainMenu / UpdateExtraMenu / controller / brightness / position | Tap visible menu target | Source input confirmed; host/core integration required |
| `save_slot` | Save at original save points: choose slot | SL: GameState_SaveScreen_Update / LoadSavegameScreen; GS: GameOver; ranking.c; credits.c | Tap visible menu target | Source input confirmed; host/core integration required |
| `load_slot` | Load: choose slot/file/card/page | SL: GameState_SaveScreen_Update / LoadSavegameScreen; GS: GameOver; ranking.c; credits.c | Tap visible menu target | Source input confirmed; host/core integration required |
| `save_confirm` | Save / overwrite / format confirmation | SL: GameState_SaveScreen_Update / LoadSavegameScreen; GS: GameOver; ranking.c; credits.c | Tap visible menu target | Source input confirmed; host/core integration required |
| `save_cancel` | Cancel save / load / overwrite / format | SL: GameState_SaveScreen_Update / LoadSavegameScreen; GS: GameOver; ranking.c; credits.c | Tap visible menu target | Source input confirmed; host/core integration required |
| `save_error` | Acknowledge card / save / load error | SL: GameState_SaveScreen_Update / LoadSavegameScreen; GS: GameOver; ranking.c; credits.c | Tap visible menu target | Source input confirmed; host/core integration required |
| `game_over` | Game-over retry / return | SL: GameState_SaveScreen_Update / LoadSavegameScreen; GS: GameOver; ranking.c; credits.c | Tap visible menu target | Source input confirmed; host/core integration required |
| `ranking` | Results / ranking: acknowledge | SL: GameState_SaveScreen_Update / LoadSavegameScreen; GS: GameOver; ranking.c; credits.c | Tap visible menu target | Source input confirmed; host/core integration required |
| `credits` | Credits / completion: continue | SL: GameState_SaveScreen_Update / LoadSavegameScreen; GS: GameOver; ranking.c; credits.c | Tap visible menu target | Source input confirmed; host/core integration required |
| `inventory_select` | Inventory: select item (all eligible IDs) | IS: GameState_StatusScreen_Update and item menu states | Tap item/operation target | Source input confirmed; host/core integration required |
| `inventory_use` | Inventory: use item (all eligible IDs) | IS: GameState_StatusScreen_Update and item menu states | Tap item/operation target | Source input confirmed; host/core integration required |
| `inventory_equip` | Inventory: equip item (all eligible IDs) | IS: GameState_StatusScreen_Update and item menu states | Tap item/operation target | Source input confirmed; host/core integration required |
| `inventory_unequip` | Inventory: unequip item (all eligible IDs) | IS: GameState_StatusScreen_Update and item menu states | Tap item/operation target | Source input confirmed; host/core integration required |
| `inventory_examine` | Inventory: examine item (all eligible IDs) | IS: GameState_StatusScreen_Update and item menu states | Tap item/operation target | Source input confirmed; host/core integration required |
| `inventory_reload` | Inventory: reload item (all eligible IDs) | IS: GameState_StatusScreen_Update and item menu states | Tap item/operation target | Source input confirmed; host/core integration required |
| `inventory_toggle` | Inventory: toggle item (all eligible IDs) | IS: GameState_StatusScreen_Update and item menu states | Tap item/operation target | Source input confirmed; host/core integration required |
| `inventory_scroll` | Inventory carousel/category/page selection | IS: Inventory_DirectionalInputSet | Tap item/operation target | Source input confirmed; host/core integration required |
| `inventory_rotate` | Item examination rotate / inspect | IS; item_screens_3.c | Tap item/operation target | Source input confirmed; host/core integration required |
| `inventory_close` | Close inventory / item description | IS: controllerConfig.cancel / item | Tap item/operation target | Source input confirmed; host/core integration required |
| `map_pan` | Pan paper map | MS:362-364 | Drag map | Source input confirmed; host/core integration required |
| `map_floor_up` | Paper map: previous/next floor (up) | MS:323-359 | Tap map operation chip | Source input confirmed; host/core integration required |
| `map_floor_down` | Paper map: previous/next floor (down) | MS:323-359 | Tap map operation chip | Source input confirmed; host/core integration required |
| `map_toggle` | Paper map alternate view / enter operation | MS:301 | Tap map operation chip | Enter input confirmed; view semantics host-specific |
| `map_close` | Close map / return from map pickup | MS:274-277; GS:490-500 | Tap map operation chip | Source input confirmed; host/core integration required |
| `dialogue_next` | Advance dialogue / read clue / pickup confirmation | MSG: Gfx_MapMsg_Draw; EU | Tap message/choice target | Source input confirmed; host/core integration required |
| `dialogue_choice` | Direct yes/no / multi-choice / elevator floor selection | MSG: g_MapMsg_Select; map3_s01.c; map3_s06.c | Tap message/choice target | Source input confirmed; host/core integration required |
| `dialogue_cancel` | Cancel prompt / decline interaction | MSG: controllerConfig.cancel | Tap message/choice target | Source input confirmed; host/core integration required |
| `cutscene_skip` | Skip eligible cutscenes / FMV / intro | src/maps/* (skip sites); src/screens/stream/stream.c | Tap SKIP chip when allowed | Source input confirmed; host/core integration required |
| `boot_continue` | Dismiss boot logos / demo title wait | src/screens/b_konami/b_konami.c; stream.c | Tap labelled legacy target (not a gamepad overlay) | Source input confirmed; host/core integration required |
| `piano_white` | School piano: white keys | M1S01: PianoPuzzle_Control:560-640 | Tap current puzzle element | Direct cursor/enter handler confirmed; host adapter required |
| `piano_black` | School piano: black keys | M1S01: PianoPuzzle_Control:560-640 | Tap current puzzle element | Direct cursor/enter handler confirmed; host adapter required |
| `plates_insert` | Hospital coloured plates: choose empty slot / insert | M3S03: func_800D1A58:330-399 | Tap current puzzle element | Direct cursor/enter handler confirmed; host adapter required |
| `plates_remove` | Hospital coloured plates: choose filled slot / remove | M3S03: func_800D1A58:439-516 | Tap current puzzle element | Direct cursor/enter handler confirmed; host adapter required |
| `motel_digits` | Resort door keypad: digits | M5S01: func_800EBA40:197-262 | Tap current puzzle element | Numeric keypad confirmed; location identity inferred; host adapter required |
| `motel_clear` | Resort door keypad: clear/delete entry | M5S01: func_800EBA40 | Tap current puzzle element | Element 10 meaning uncertain; host must label from panel; host adapter required |
| `motel_enter` | Resort door keypad: submit | M5S01: func_800EBA40:230-262 | Tap current puzzle element | Submit element confirmed; host adapter required |
| `nowhere_letters_a` | Nowhere letter keypad (M7S01) | M7S01: func_800D94DC:998-1059 | Tap current puzzle element | 26-element letter cursor confirmed; narrative name uncertain; host adapter required |
| `nowhere_letters_b` | Nowhere letter keypad (M7S02) | M7S02: func_800E32E0:3184-3245 | Tap current puzzle element | Shared keypad confirmed; scene identity uncertain; host adapter required |
| `nowhere_lights_a` | Nowhere 3x3 door light panels (M7S01) | M7S01: func_800D9C9C:1310-1385 | Tap current puzzle element | Three panel phases confirmed; host supplies active panel; host adapter required |
| `nowhere_lights_b` | Nowhere 3x3 door light panels (M7S02) | M7S02: func_800DFDDC:2381-2459 | Tap current puzzle element | Duplicate/shared panel confirmed; host adapter required |
| `nowhere_zodiac` | Nowhere numeric / zodiac selection | M7S01: func_800D8DB4; MSG | Tap current puzzle element | Selection sequence confirmed; zodiac name inferred; host adapter required |
| `safe_ring` | Resort safe: select one of four rings | M5S01: unk_draw_800CD20C.c:16-49 | Tap ring/rotation target | Four eight-position rings confirmed; guard/animation kept by core |
| `safe_left` | Resort safe: rotate selected ring left | M5S01: unk_draw_800CD20C.c:16-49 | Tap ring/rotation target | Four eight-position rings confirmed; guard/animation kept by core |
| `safe_right` | Resort safe: rotate selected ring right | M5S01: unk_draw_800CD20C.c:16-49 | Tap ring/rotation target | Four eight-position rings confirmed; guard/animation kept by core |
| `school_clock` | School clock tower / gold & silver medallion sockets | M1S00: MapEvent_ClockTowerInspect; medallion events | Tap inventory item -> Use, then visible prompt/element | Item/prompt route confirmed; scene label inferred where unnamed; host adapter required |
| `school_chemistry` | School chemistry / gold medallion interaction | M1S01: gold-medallion event:283-303 | Tap inventory item -> Use, then visible prompt/element | Item/prompt route confirmed; scene label inferred where unnamed; host adapter required |
| `school_picture` | School reception picture-card slot | M1S02: MapEvent_DoorWithHorizontalSlotInteract | Tap inventory item -> Use, then visible prompt/element | Item/prompt route confirmed; scene label inferred where unnamed; host adapter required |
| `school_roof` | School roof drain: rubber ball / key / drainage valve | M1S03: MapEvent_RoofDrainPuzzleInteract0/1; RubberBallUse; DrainageValveInteract | Tap inventory item -> Use, then visible prompt/element | Item/prompt route confirmed; scene label inferred where unnamed; host adapter required |
| `school_boiler` | School boiler controls: left / right valve / stop | M1S00: MapEvent_Boiler0/1/2; M1S02; M1S06 | Tap inventory item -> Use, then visible prompt/element | Item/prompt route confirmed; scene label inferred where unnamed; host adapter required |
| `hospital_generator` | Hospital generator / elevator choices | M3S01: MapEvent_Generator0; elevator events | Tap inventory item -> Use, then visible prompt/element | Item/prompt route confirmed; scene label inferred where unnamed; host adapter required |
| `hospital_bottle` | Hospital liquid: use bottle to collect | M3S01: MapEvent_UseBottleOnLiquid | Tap inventory item -> Use, then visible prompt/element | Item/prompt route confirmed; scene label inferred where unnamed; host adapter required |
| `hospital_blood` | Hospital bloodsucker: use blood pack | M3S03/M3S04; bloodsucker.c | Tap inventory item -> Use, then visible prompt/element | Item/prompt route confirmed; scene label inferred where unnamed; host adapter required |
| `hospital_fire` | Hospital basement obstacle: alcohol and lighter | M3S03; M3S05: item-use events | Tap inventory item -> Use, then visible prompt/element | Item/prompt route confirmed; scene label inferred where unnamed; host adapter required |
| `motel_magnet` | Resort drain: magnet / motorcycle key | M5S03: item-use event:181-182 | Tap inventory item -> Use, then visible prompt/element | Item/prompt route confirmed; scene label inferred where unnamed; host adapter required |
| `motel_motorcycle` | Resort motorcycle: key use / inspection | M5S03: motorcycle event | Tap inventory item -> Use, then visible prompt/element | Item/prompt route confirmed; scene label inferred where unnamed; host adapter required |
| `nowhere_clock` | Nowhere clock: Stone of Time | M7S01: func_800D8FF8; func_800DAB64 | Tap inventory item -> Use, then visible prompt/element | Item/prompt route confirmed; scene label inferred where unnamed; host adapter required |
| `nowhere_bird` | Nowhere bird cage key | M7S01: MapEvent_BirdCageKeyUse | Tap inventory item -> Use, then visible prompt/element | Item/prompt route confirmed; scene label inferred where unnamed; host adapter required |
| `nowhere_tools` | Nowhere pliers / screwdriver obstacles | M7S01/M7S02: item-use events | Tap inventory item -> Use, then visible prompt/element | Item/prompt route confirmed; scene label inferred where unnamed; host adapter required |
| `nowhere_camera` | Nowhere camera: photograph hints | M7S01: func_800DB3D0; M7S02: func_800E1398 | Tap inventory item -> Use, then visible prompt/element | Item/prompt route confirmed; scene label inferred where unnamed; host adapter required |
| `nowhere_fridge` | Nowhere refrigerator chain: Ring of Contract / dagger | M7S02: func_800DCD00 | Tap inventory item -> Use, then visible prompt/element | Item/prompt route confirmed; scene label inferred where unnamed; host adapter required |
| `nowhere_power` | Nowhere electrical wires / generator / Aratron key | M7S02: func_800E0FF0 | Tap inventory item -> Use, then visible prompt/element | Item/prompt route confirmed; scene label inferred where unnamed; host adapter required |
| `nowhere_final` | Nowhere final door: five ritual items | M7S02: func_800E2DEC | Tap inventory item -> Use, then visible prompt/element | Item/prompt route confirmed; scene label inferred where unnamed; host adapter required |
| `cybil_liquid` | Cybil encounter: use unknown liquid | M6S04: encounter / item events | Tap inventory item -> Use, then visible prompt/element | Item/prompt route confirmed; scene label inferred where unnamed; host adapter required |
| `channeling_stone` | Channeling Stone: eligible use locations | M1S03/M5S01 and item events | Tap inventory item -> Use, then visible prompt/element | Item/prompt route confirmed; scene label inferred where unnamed; host adapter required |
| `all_keys` | All remaining keys / doors, including eclipse, Gordon, sewer, Ophiel/Hagith/Phaleg/Bethor/Aratron | src/maps/*_header.c callback inventory; EU; EM | Tap inventory item -> Use, then visible prompt/element | Item/prompt route confirmed; scene label inferred where unnamed; host adapter required |
| `puzzle_cancel` | Cancel / leave every puzzle | All direct puzzle controllerConfig.cancel sites | Tap current puzzle element | Source input confirmed; host/core integration required |
| `unidentified_events` | All unnamed/map callback interactions and switches | INPUT_AUDIT.md: complete g_MapEventFuncs inventory | Tap message/choice target | Generic action + item-use + message selection path; semantic identity uncertain until playthrough |
| `raw_select` | Legacy SELECT | joy.h; WB; M1S04: func_800CCA2C; gfx/bodyprog_800652F4.c; VC | Tap labelled legacy target (not a gamepad overlay) | Explicit labelled host fallback; debug paths hidden by default; no new gameplay unlock |
| `raw_l3` | Legacy L3 | joy.h; WB; M1S04: func_800CCA2C; gfx/bodyprog_800652F4.c; VC | Tap labelled legacy target (not a gamepad overlay) | Explicit labelled host fallback; debug paths hidden by default; no new gameplay unlock |
| `raw_r3` | Legacy R3 | joy.h; WB; M1S04: func_800CCA2C; gfx/bodyprog_800652F4.c; VC | Tap labelled legacy target (not a gamepad overlay) | Explicit labelled host fallback; debug paths hidden by default; no new gameplay unlock |
| `raw_start` | Legacy START | joy.h; WB; M1S04: func_800CCA2C; gfx/bodyprog_800652F4.c; VC | Tap labelled legacy target (not a gamepad overlay) | Explicit labelled host fallback; debug paths hidden by default; no new gameplay unlock |
| `raw_up` | Legacy D-pad up | joy.h; WB; M1S04: func_800CCA2C; gfx/bodyprog_800652F4.c; VC | Tap labelled legacy target (not a gamepad overlay) | Explicit labelled host fallback; debug paths hidden by default; no new gameplay unlock |
| `raw_right` | Legacy D-pad right | joy.h; WB; M1S04: func_800CCA2C; gfx/bodyprog_800652F4.c; VC | Tap labelled legacy target (not a gamepad overlay) | Explicit labelled host fallback; debug paths hidden by default; no new gameplay unlock |
| `raw_down` | Legacy D-pad down | joy.h; WB; M1S04: func_800CCA2C; gfx/bodyprog_800652F4.c; VC | Tap labelled legacy target (not a gamepad overlay) | Explicit labelled host fallback; debug paths hidden by default; no new gameplay unlock |
| `raw_left` | Legacy D-pad left | joy.h; WB; M1S04: func_800CCA2C; gfx/bodyprog_800652F4.c; VC | Tap labelled legacy target (not a gamepad overlay) | Explicit labelled host fallback; debug paths hidden by default; no new gameplay unlock |
| `raw_l2` | Legacy L2 | joy.h; WB; M1S04: func_800CCA2C; gfx/bodyprog_800652F4.c; VC | Tap labelled legacy target (not a gamepad overlay) | Explicit labelled host fallback; debug paths hidden by default; no new gameplay unlock |
| `raw_r2` | Legacy R2 | joy.h; WB; M1S04: func_800CCA2C; gfx/bodyprog_800652F4.c; VC | Tap labelled legacy target (not a gamepad overlay) | Explicit labelled host fallback; debug paths hidden by default; no new gameplay unlock |
| `raw_l1` | Legacy L1 | joy.h; WB; M1S04: func_800CCA2C; gfx/bodyprog_800652F4.c; VC | Tap labelled legacy target (not a gamepad overlay) | Explicit labelled host fallback; debug paths hidden by default; no new gameplay unlock |
| `raw_r1` | Legacy R1 | joy.h; WB; M1S04: func_800CCA2C; gfx/bodyprog_800652F4.c; VC | Tap labelled legacy target (not a gamepad overlay) | Explicit labelled host fallback; debug paths hidden by default; no new gameplay unlock |
| `raw_triangle` | Legacy triangle | joy.h; WB; M1S04: func_800CCA2C; gfx/bodyprog_800652F4.c; VC | Tap labelled legacy target (not a gamepad overlay) | Explicit labelled host fallback; debug paths hidden by default; no new gameplay unlock |
| `raw_circle` | Legacy circle | joy.h; WB; M1S04: func_800CCA2C; gfx/bodyprog_800652F4.c; VC | Tap labelled legacy target (not a gamepad overlay) | Explicit labelled host fallback; debug paths hidden by default; no new gameplay unlock |
| `raw_cross` | Legacy cross | joy.h; WB; M1S04: func_800CCA2C; gfx/bodyprog_800652F4.c; VC | Tap labelled legacy target (not a gamepad overlay) | Explicit labelled host fallback; debug paths hidden by default; no new gameplay unlock |
| `raw_square` | Legacy square | joy.h; WB; M1S04: func_800CCA2C; gfx/bodyprog_800652F4.c; VC | Tap labelled legacy target (not a gamepad overlay) | Explicit labelled host fallback; debug paths hidden by default; no new gameplay unlock |
| `warm_reset` | Legacy reset combo via labelled Return to title confirmation | joy.h; WB; M1S04: func_800CCA2C; gfx/bodyprog_800652F4.c; VC | Tap labelled legacy target (not a gamepad overlay) | Explicit labelled host fallback; debug paths hidden by default; no new gameplay unlock |
| `sound_test` | Unused/debug sound-test play/stop/reset and selection | joy.h; WB; M1S04: func_800CCA2C; gfx/bodyprog_800652F4.c; VC | Tap labelled legacy target (not a gamepad overlay) | Explicit labelled host fallback; debug paths hidden by default; no new gameplay unlock |
| `debug_save` | Debug paused save combo (not an ordinary save-anywhere feature) | GS:307-314 | Development-only labelled exact combo target | Explicit development-only target; exact processed combo depends on joy.c; core integration unverified |
| `map_projection` | Legacy L1/R1 map-coordinate projection bypass | MS:891-893 | Tap labelled legacy target (not a gamepad overlay) | Consumer confirmed; gameplay relevance uncertain; explicit labelled fallback |
