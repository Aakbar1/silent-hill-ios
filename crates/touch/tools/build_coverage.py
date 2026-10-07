"""Regenerate human-maintained coverage cases, without importing the Rust engine.

Expected pad bits come from USA settings_reset.c and joy.h, not engine output.
All positions are original test geometry. No game data or puzzle solutions.
"""
from pathlib import Path
import json

ROOT = Path(__file__).resolve().parents[1]
ROWS = []


def row(key, label, source, route, value=None, status="Source input confirmed; host/core integration required"):
    ROWS.append((key, label, source, route, value, status))


for key, label, route in [
    ("walk_forward", "Walk forward", "forward"), ("walk_backward", "Back away", "backward"),
    ("turn_left", "Turn left", "left"), ("turn_right", "Turn right", "right"),
    ("run", "Run by stick push distance", "run"), ("backward_hop", "Backward hop / retreat with run modifier (core-dependent)", "hop"), ("quick_turn", "180-degree turn", "quick"),
]:
    row(key, label, "PC: Player_Controller; Player_CharaTurn_0/_1", route, status="Pad path confirmed; on-device tuning provisional")
for key, label in [("interact", "Interact / examine / unlock doors"), ("pickup", "Pick up any item, including weapons, ammo and maps"),
                   ("kick_stomp", "Kick / stomp downed enemies"), ("escape_grab", "Action input during struggle / get-up")]:
    row(key, label, "PC: Player_Controller; EM: Event_Update", "tap", status="Pad path confirmed; core decides state-dependent result")
row("aim_auto", "Aim / original auto-target (respect auto-aim option)", "PC: Player_Controller; Player_CombatUpdate", "aim", status="Aim bit confirmed; target selection remains in C, not tested here")
row("attack_tap", "Fire / swing with a tap while aiming", "PC:8556-8575", "attack")
row("attack_hold", "Held melee attacks / sustained firearm / chainsaw / drill", "PC:8556-8575; WEAPON_ATTACK input types", "attack_hold")
row("attack_repeat", "Repeated melee attacks / repeated trigger taps", "PC:8556-8575", "attack_repeat")
for key, label, bit in [("step_left", "Sidestep left", 1024), ("step_right", "Sidestep right", 2048),
                       ("look", "Hold camera look", 256), ("flashlight", "Toggle flashlight", 8192),
                       ("open_map", "Open map", 4096), ("open_inventory", "Open inventory/status", 1),
                       ("pause", "Pause", 8)]:
    row(key, label, "PC:8526-8550; GS:236-263; VC: vcMoveAndSetCamera", "chip", bit)
row("radio", "Toggle radio when owned", "IS:928-942 (no dedicated radio pad bit)", "chip_ui", {"action": "toggle_radio"})
row("unpause", "Resume paused gameplay", "GS: SysState_GamePaused_Update", "pad_target", 8)
for boss in ["split_head", "twinfeeler", "floatstinger", "monster_cybil", "incubator", "incubus"]:
    row("boss_" + boss, "Boss combat: " + boss.replace("_", " "), "src/maps/characters/" + boss + ".c; PC", "attack", status="Uses original combat inputs; encounter outcome not verified")

for key, label, request in [
    ("title_new", "Title: new game", {"action": "activate", "id": 0}),
    ("title_continue", "Title: continue / auto-load", {"action": "activate", "id": 1}),
    ("title_load", "Title: load", {"action": "activate", "id": 2}),
    ("title_movie", "Title: opening movie / demo", {"action": "activate", "id": 3}),
    ("difficulty", "New game difficulty selection", {"action": "select", "id": 2}),
    ("options_open", "Open main options", {"action": "open_options"}),
    ("options_extra", "Enter/leave extra options", {"action": "activate", "id": 4}),
    ("menu_back", "Cancel / back from any menu", {"action": "cancel"}),
    ("menu_confirm", "Confirm menu choice", {"action": "confirm"}),
    ("return_title", "Return to title / quit through native confirmation", {"action": "return_to_title"}),
]:
    row(key, label, "TM: GameState_MainMenu_Update; OPT; WB", "menu", request)
for i, (key, label) in enumerate([
    ("option_screen_x", "Screen position horizontal"), ("option_screen_y", "Screen position vertical"),
    ("option_brightness", "Brightness"), ("option_controller", "Controller preset / per-action binding"),
    ("option_vibration", "Vibration"), ("option_autoload", "Auto-load"),
    ("option_sound", "Stereo / mono"), ("option_bgm", "Music volume"), ("option_sfx", "Effects volume"),
    ("option_weapon", "Extra weapon hold/toggle"), ("option_blood", "Blood colour"),
    ("option_view", "Extra view control"), ("option_view_mode", "Extra view mode"), ("option_retreat", "Retreat turn"),
    ("option_run", "Walk/run inversion"), ("option_autoaim", "Auto aiming"),
    ("option_bullet", "Bullet adjustment"), ("option_defaults", "Restore defaults"),
]):
    row(key, label, "OPT: OptionsMenu_UpdateMainMenu / UpdateExtraMenu / controller / brightness / position", "menu", {"action": "adjust", "id": i, "delta": 1})
for key, label, request in [
    ("save_slot", "Save at original save points: choose slot", {"action": "select", "id": 2}),
    ("load_slot", "Load: choose slot/file/card/page", {"action": "select", "id": 3}),
    ("save_confirm", "Save / overwrite / format confirmation", {"action": "confirm"}),
    ("save_cancel", "Cancel save / load / overwrite / format", {"action": "cancel"}),
    ("save_error", "Acknowledge card / save / load error", {"action": "continue"}),
    ("game_over", "Game-over retry / return", {"action": "activate", "id": 1}),
    ("ranking", "Results / ranking: acknowledge", {"action": "continue"}),
    ("credits", "Credits / completion: continue", {"action": "continue"}),
]:
    row(key, label, "SL: GameState_SaveScreen_Update / LoadSavegameScreen; GS: GameOver; ranking.c; credits.c", "menu", request)
for op in ["select", "use", "equip", "unequip", "examine", "reload", "toggle"]:
    row("inventory_" + op, "Inventory: " + op + " item (all eligible IDs)", "IS: GameState_StatusScreen_Update and item menu states", "inventory", {"action": "inventory", "id": 42, "op": op})
row("inventory_scroll", "Inventory carousel/category/page selection", "IS: Inventory_DirectionalInputSet", "inventory", {"action": "select", "id": 7})
row("inventory_rotate", "Item examination rotate / inspect", "IS; item_screens_3.c", "inventory", {"action": "adjust", "id": 42, "delta": -1})
row("inventory_close", "Close inventory / item description", "IS: controllerConfig.cancel / item", "inventory", {"action": "cancel"})
row("map_pan", "Pan paper map", "MS:362-364", "pan")
row("map_floor_up", "Paper map: previous/next floor (up)", "MS:323-359", "map_chip", {"action": "map_floor", "delta": 1})
row("map_floor_down", "Paper map: previous/next floor (down)", "MS:323-359", "map_chip", {"action": "map_floor", "delta": -1})
row("map_toggle", "Paper map alternate view / enter operation", "MS:301", "map_chip", {"action": "confirm"}, "Enter input confirmed; view semantics host-specific")
row("map_close", "Close map / return from map pickup", "MS:274-277; GS:490-500", "map_chip", {"action": "cancel"})
row("dialogue_next", "Advance dialogue / read clue / pickup confirmation", "MSG: Gfx_MapMsg_Draw; EU", "dialogue", {"action": "continue"})
row("dialogue_choice", "Direct yes/no / multi-choice / elevator floor selection", "MSG: g_MapMsg_Select; map3_s01.c; map3_s06.c", "dialogue", {"action": "select", "id": 1})
row("dialogue_cancel", "Cancel prompt / decline interaction", "MSG: controllerConfig.cancel", "dialogue", {"action": "cancel"})
row("cutscene_skip", "Skip eligible cutscenes / FMV / intro", "src/maps/* (skip sites); src/screens/stream/stream.c", "skip", {"action": "skip"})
row("boot_continue", "Dismiss boot logos / demo title wait", "src/screens/b_konami/b_konami.c; stream.c", "pad_target", 16384)

for key, label, kind, element, source, status in [
    ("piano_white", "School piano: white keys", "piano", 3, "M1S01: PianoPuzzle_Control:560-640", "Direct cursor/enter handler confirmed"),
    ("piano_black", "School piano: black keys", "piano", 8, "M1S01: PianoPuzzle_Control:560-640", "Direct cursor/enter handler confirmed"),
    ("plates_insert", "Hospital coloured plates: choose empty slot / insert", "hospital_plates", 3, "M3S03: func_800D1A58:330-399", "Direct cursor/enter handler confirmed"),
    ("plates_remove", "Hospital coloured plates: choose filled slot / remove", "hospital_plates", 5, "M3S03: func_800D1A58:439-516", "Direct cursor/enter handler confirmed"),
    ("motel_digits", "Resort door keypad: digits", "motel_keypad", 3, "M5S01: func_800EBA40:197-262", "Numeric keypad confirmed; location identity inferred"),
    ("motel_clear", "Resort door keypad: clear/delete entry", "motel_keypad", 10, "M5S01: func_800EBA40", "Element 10 meaning uncertain; host must label from panel"),
    ("motel_enter", "Resort door keypad: submit", "motel_keypad", 11, "M5S01: func_800EBA40:230-262", "Submit element confirmed"),
    ("nowhere_letters_a", "Nowhere letter keypad (M7S01)", "nowhere_letters", 4, "M7S01: func_800D94DC:998-1059", "26-element letter cursor confirmed; narrative name uncertain"),
    ("nowhere_letters_b", "Nowhere letter keypad (M7S02)", "nowhere_letters", 19, "M7S02: func_800E32E0:3184-3245", "Shared keypad confirmed; scene identity uncertain"),
    ("nowhere_lights_a", "Nowhere 3x3 door light panels (M7S01)", "nowhere_light_panels", 2, "M7S01: func_800D9C9C:1310-1385", "Three panel phases confirmed; host supplies active panel"),
    ("nowhere_lights_b", "Nowhere 3x3 door light panels (M7S02)", "nowhere_light_panels", 6, "M7S02: func_800DFDDC:2381-2459", "Duplicate/shared panel confirmed"),
    ("nowhere_zodiac", "Nowhere numeric / zodiac selection", "nowhere_zodiac", 6, "M7S01: func_800D8DB4; MSG", "Selection sequence confirmed; zodiac name inferred"),
]:
    row(key, label, source, "puzzle", {"action": "puzzle", "kind": kind, "element": element}, status + "; host adapter required")
for key, label, request in [
    ("safe_ring", "Resort safe: select one of four rings", {"action": "select", "id": 2}),
    ("safe_left", "Resort safe: rotate selected ring left", {"action": "adjust", "id": 2, "delta": 1}),
    ("safe_right", "Resort safe: rotate selected ring right", {"action": "adjust", "id": 2, "delta": -1}),
]:
    row(key, label, "M5S01: unk_draw_800CD20C.c:16-49", "safe", request, "Four eight-position rings confirmed; guard/animation kept by core")
for i, (key, label, source) in enumerate([
    ("school_clock", "School clock tower / gold & silver medallion sockets", "M1S00: MapEvent_ClockTowerInspect; medallion events"),
    ("school_chemistry", "School chemistry / gold medallion interaction", "M1S01: gold-medallion event:283-303"),
    ("school_picture", "School reception picture-card slot", "M1S02: MapEvent_DoorWithHorizontalSlotInteract"),
    ("school_roof", "School roof drain: rubber ball / key / drainage valve", "M1S03: MapEvent_RoofDrainPuzzleInteract0/1; RubberBallUse; DrainageValveInteract"),
    ("school_boiler", "School boiler controls: left / right valve / stop", "M1S00: MapEvent_Boiler0/1/2; M1S02; M1S06"),
    ("hospital_generator", "Hospital generator / elevator choices", "M3S01: MapEvent_Generator0; elevator events"),
    ("hospital_bottle", "Hospital liquid: use bottle to collect", "M3S01: MapEvent_UseBottleOnLiquid"),
    ("hospital_blood", "Hospital bloodsucker: use blood pack", "M3S03/M3S04; bloodsucker.c"),
    ("hospital_fire", "Hospital basement obstacle: alcohol and lighter", "M3S03; M3S05: item-use events"),
    ("motel_magnet", "Resort drain: magnet / motorcycle key", "M5S03: item-use event:181-182"),
    ("motel_motorcycle", "Resort motorcycle: key use / inspection", "M5S03: motorcycle event"),
    ("nowhere_clock", "Nowhere clock: Stone of Time", "M7S01: func_800D8FF8; func_800DAB64"),
    ("nowhere_bird", "Nowhere bird cage key", "M7S01: MapEvent_BirdCageKeyUse"),
    ("nowhere_tools", "Nowhere pliers / screwdriver obstacles", "M7S01/M7S02: item-use events"),
    ("nowhere_camera", "Nowhere camera: photograph hints", "M7S01: func_800DB3D0; M7S02: func_800E1398"),
    ("nowhere_fridge", "Nowhere refrigerator chain: Ring of Contract / dagger", "M7S02: func_800DCD00"),
    ("nowhere_power", "Nowhere electrical wires / generator / Aratron key", "M7S02: func_800E0FF0"),
    ("nowhere_final", "Nowhere final door: five ritual items", "M7S02: func_800E2DEC"),
    ("cybil_liquid", "Cybil encounter: use unknown liquid", "M6S04: encounter / item events"),
    ("channeling_stone", "Channeling Stone: eligible use locations", "M1S03/M5S01 and item events"),
    ("all_keys", "All remaining keys / doors, including eclipse, Gordon, sewer, Ophiel/Hagith/Phaleg/Bethor/Aratron", "src/maps/*_header.c callback inventory; EU; EM"),
]):
    row(key, label, source, "item_puzzle", {"action": "inventory", "id": 100 + i, "op": "use"},
        "Item/prompt route confirmed; scene label inferred where unnamed; host adapter required")
row("puzzle_cancel", "Cancel / leave every puzzle", "All direct puzzle controllerConfig.cancel sites", "puzzle", {"action": "cancel"})
row("unidentified_events", "All unnamed/map callback interactions and switches", "INPUT_AUDIT.md: complete g_MapEventFuncs inventory", "dialogue", {"action": "select", "id": 0},
    "Generic action + item-use + message selection path; semantic identity uncertain until playthrough")
for key, label, bits in [
    ("raw_select", "Legacy SELECT", 1), ("raw_l3", "Legacy L3", 2), ("raw_r3", "Legacy R3", 4),
    ("raw_start", "Legacy START", 8), ("raw_up", "Legacy D-pad up", 16), ("raw_right", "Legacy D-pad right", 32),
    ("raw_down", "Legacy D-pad down", 64), ("raw_left", "Legacy D-pad left", 128),
    ("raw_l2", "Legacy L2", 256), ("raw_r2", "Legacy R2", 512), ("raw_l1", "Legacy L1", 1024),
    ("raw_r1", "Legacy R1", 2048), ("raw_triangle", "Legacy triangle", 4096), ("raw_circle", "Legacy circle", 8192),
    ("raw_cross", "Legacy cross", 16384), ("raw_square", "Legacy square", 32768),
    ("warm_reset", "Legacy reset combo via labelled Return to title confirmation", 3848),
    ("sound_test", "Unused/debug sound-test play/stop/reset and selection", 8192),
]:
    row(key, label, "joy.h; WB; M1S04: func_800CCA2C; gfx/bodyprog_800652F4.c; VC", "pad_target", bits,
        "Explicit labelled host fallback; debug paths hidden by default; no new gameplay unlock")
row("debug_save", "Debug paused save combo (not an ordinary save-anywhere feature)", "GS:307-314", "debug_save", None,
    "Explicit development-only target; exact processed combo depends on joy.c; core integration unverified")
row("map_projection", "Legacy L1/R1 map-coordinate projection bypass", "MS:891-893", "pad_target", 1024,
    "Consumer confirmed; gameplay relevance uncertain; explicit labelled fallback")

AVAIL = {x: True for x in ["flashlight", "radio", "map", "inventory", "pause", "movement", "combat", "camera", "skip"]}
RECORDS = []


def emit(**record):
    RECORDS.append(record)


def context(mode):
    emit(type="context", context={"mode": mode, "available": AVAIL})


def touch(phase, x, y, time, id=1):
    emit(type="touch", id=id, phase=phase, x=x, y=y, time_ms=time)


def frame(time, buttons=0, left=None, right=None, actions=None):
    emit(type="frame", time_ms=time, expect={"buttons": buttons, "left": left or {"x": 0., "y": 0.},
        "right": right or {"x": 0., "y": 0.}, "ui_actions": actions or []})


def target(action):
    emit(type="targets", targets=[{"bounds": {"x": 200., "y": 130., "width": 80., "height": 48.},
        "action": action, "enabled": True}])


def tap_target(base, action, expected_buttons=0, ui=None, left=None, right=None):
    target(action)
    touch("down", 220, 150, base)
    touch("up", 220, 150, base + 10)
    frame(base + 20, expected_buttons, left, right, [ui] if ui else [])
    frame(base + 30)


CHIPS = {8192: (862, 387), 4096: (750, 387), 1: (862, 331), 8: (806, 331),
         256: (750, 331), 1024: (862, 275), 2048: (806, 275)}
for index, (key, label, source, route, value, status) in enumerate(ROWS):
    b = index * 2000
    emit(type="marker", id=key)
    emit(type="cancel_all")
    context("exploring")
    if route in ["forward", "backward", "left", "right", "run", "hop", "quick"]:
        delta = {"forward": (0, -38), "backward": (0, 38), "left": (-38, 0), "right": (38, 0), "run": (0, -64), "hop": (0, 64), "quick": (0, 80)}[route]
        touch("down", 240, 200, b)
        touch("move", 240 + delta[0], 200 + delta[1], b + 100)
        magnitude = 1. if route in ("run", "hop") else .6
        axis = {"x": (1 if delta[0] > 0 else -1) * magnitude if delta[0] else 0.,
                "y": (1 if delta[1] > 0 else -1) * magnitude if delta[1] else 0.}
        frame(b + 100, 3072 if route == "quick" else 32768 if route in ("run", "hop") else 0,
              None if route == "quick" else axis)
        touch("up", 240 + delta[0], 200 + delta[1], b + 110)
        frame(b + 120)
    elif route == "tap":
        touch("down", 650, 200, b)
        touch("up", 650, 200, b + 10)
        frame(b + 20, 16384)
        frame(b + 30)
    elif route in ["aim", "attack", "attack_hold", "attack_repeat"]:
        touch("down", 650, 200, b)
        frame(b + 280, 512)
        touch("up", 650, 200, b + 290)
        frame(b + 300, 512)
        if route != "aim":
            touch("down", 650, 200, b + 320)
            if route == "attack_hold":
                frame(b + 330, 16896)
                frame(b + 800, 16896)
                touch("up", 650, 200, b + 810)
                frame(b + 820, 512)
            else:
                touch("up", 650, 200, b + 330)
                frame(b + 340, 16896)
                frame(b + 350, 512)
                if route == "attack_repeat":
                    touch("down", 650, 200, b + 360)
                    touch("up", 650, 200, b + 370)
                    frame(b + 380, 16896)
                    frame(b + 390, 512)
    elif route == "chip":
        x, y = CHIPS[value]
        touch("down", x, y, b)
        if value in (256, 1024, 2048):
            frame(b + 1, value)
            frame(b + 200, value)
            touch("up", x, y, b + 210)
            frame(b + 220)
        else:
            touch("up", x, y, b + 10)
            frame(b + 20, value)
            frame(b + 30)
    elif route == "chip_ui":
        touch("down", 806, 387, b)
        touch("up", 806, 387, b + 10)
        frame(b + 20, actions=[value])
        frame(b + 30)
    elif route in ["menu", "inventory", "dialogue", "puzzle", "item_puzzle", "safe"]:
        mode = {"menu": "menu", "inventory": "inventory", "dialogue": "dialogue",
                "item_puzzle": {"puzzle": "item_use"}, "safe": {"puzzle": "motel_safe"},
                "puzzle": {"puzzle": value.get("kind", "other")}}[route]
        context(mode)
        tap_target(b, {"kind": "ui", "request": value}, ui=value)
    elif route == "pan":
        context("map")
        touch("down", 300, 150, b)
        touch("move", 340, 175, b + 20)
        frame(b + 20, actions=[{"action": "map_pan", "dx": 40., "dy": 25.}])
        touch("up", 340, 175, b + 30)
        frame(b + 40)
    elif route == "map_chip":
        context("map")
        x, y = {"map_floor_up": (806, 387), "map_floor_down": (750, 387),
                "map_toggle": (862, 331), "map_close": (862, 387)}[key]
        touch("down", x, y, b)
        touch("up", x, y, b + 10)
        frame(b + 20, actions=[value])
    elif route == "skip":
        context("cutscene")
        touch("down", 862, 387, b)
        touch("up", 862, 387, b + 10)
        frame(b + 20, actions=[value])
    elif route in ["pad_target", "debug_save"]:
        context("menu")
        left = {"x": -1., "y": 0.} if route == "debug_save" else {"x": 0., "y": 0.}
        right = left if route == "debug_save" else {"x": 0., "y": 0.}
        bits = 1410 if route == "debug_save" else value
        tap_target(b, {"kind": "pad", "state": {"buttons": bits, "left": left, "right": right}, "hold": False}, bits, left=left, right=right)
    else:
        raise ValueError(route)

(ROOT / "tests/replays").mkdir(parents=True, exist_ok=True)
(ROOT / "tests/replays/coverage.jsonl").write_text("\n".join(json.dumps(r, separators=(",", ":")) for r in RECORDS) + "\n", encoding="utf8")
macros = "\n".join(f"coverage_case!({r[0]});" for r in ROWS)
(ROOT / "tests/coverage.rs").write_text('''use sh_touch::{replay::{self, Record}, *};
use std::{collections::BTreeSet, io::Cursor};

fn records() -> Vec<Record> {
    replay::parse(Cursor::new(include_str!("replays/coverage.jsonl"))).expect("valid JSONL")
}
fn case(id: &str) {
    let all = records();
    let start = all.iter().position(|r| matches!(r, Record::Marker { id: key } if key == id)).expect("case exists");
    let end = all[start+1..].iter().position(|r| matches!(r, Record::Marker { .. })).map_or(all.len(), |n| start + 1 + n);
    let mut engine = Engine::new(Config::default(), Viewport::default(), GameContext { mode: Mode::Exploring, available: Availability::all() }).unwrap();
    let frames = replay::run(&mut engine, &all[start..end]).unwrap_or_else(|e| panic!("{id}: {e}"));
    assert!(!frames.is_empty(), "{id} must assert actual frames");
}
macro_rules! coverage_case { ($id:ident) => { #[test] fn $id() { case(stringify!($id)); } }; }

#[test]
fn coverage_table_and_asserted_replay_cases_are_one_to_one() {
    let ids: Vec<String> = records().iter().filter_map(|r| if let Record::Marker { id } = r { Some(id.clone()) } else { None }).collect();
    let replay_ids: BTreeSet<String> = ids.iter().cloned().collect();
    assert_eq!(ids.len(), replay_ids.len(), "no duplicate replay IDs");
    let table: BTreeSet<String> = include_str!("../COVERAGE.md").lines().filter(|line| line.starts_with("| `")).map(|line| line.split('`').nth(1).unwrap().to_string()).collect();
    assert_eq!(table, replay_ids, "every coverage row needs an asserted gesture replay");
}
''' + macros + "\n", encoding="utf8")

intro = '''# Touch coverage (US v1.1)

Derived independently from read-only decomp commit `d9e28f8315c7938117224f21516786d9d149a145`.
This checks touch -> pad/UI requests. It does **not** prove the game is playable end to end:
the host/core adapters, actual UI hit regions and every encounter still need integration.
Every row has its own named Rust test replaying `tests/replays/coverage.jsonl` with literal
expected pad bits/axes/actions. A test enforces an exact row/replay ID match.

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
'''
route_text = {"forward": "Floating stick up, 38pt", "backward": "Floating stick down slowly, 38pt", "left": "Floating stick left, 38pt", "right": "Floating stick right, 38pt", "run": "Floating stick up, 64pt", "hop": "Floating stick down, 64pt (below quick-turn distance)", "quick": "Swipe down 80pt in 100ms", "tap": "Right area tap", "aim": "Right hold 280ms", "attack": "Hold to aim then tap right", "attack_hold": "Hold to aim then hold attack", "attack_repeat": "Hold to aim then repeated right taps", "chip": "Labelled chip tap/hold", "chip_ui": "RADIO chip -> inventory toggle request", "menu": "Tap visible menu target", "inventory": "Tap item/operation target", "dialogue": "Tap message/choice target", "puzzle": "Tap current puzzle element", "safe": "Tap ring/rotation target", "item_puzzle": "Tap inventory item -> Use, then visible prompt/element", "pan": "Drag map", "map_chip": "Tap map operation chip", "skip": "Tap SKIP chip when allowed", "pad_target": "Tap labelled legacy target (not a gamepad overlay)", "debug_save": "Development-only labelled exact combo target"}
table = "\n".join(f"| `{key}` | {label} | {source} | {route_text[route]} | {status} |" for key, label, source, route, value, status in ROWS)
(ROOT / "COVERAGE.md").write_text(intro + table + "\n", encoding="utf8")
print(f"Generated {len(ROWS)} coverage rows/tests; {len(RECORDS)} JSONL records")
