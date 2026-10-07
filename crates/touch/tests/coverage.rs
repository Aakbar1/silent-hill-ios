use sh_touch::{
    replay::{self, Record},
    *,
};
use std::{collections::BTreeSet, io::Cursor};

fn records() -> Vec<Record> {
    replay::parse(Cursor::new(include_str!("replays/coverage.jsonl"))).expect("valid JSONL")
}
fn case(id: &str) {
    let all = records();
    let start = all
        .iter()
        .position(|r| matches!(r, Record::Marker { id: key } if key == id))
        .expect("case exists");
    let end = all[start + 1..]
        .iter()
        .position(|r| matches!(r, Record::Marker { .. }))
        .map_or(all.len(), |n| start + 1 + n);
    let mut engine = Engine::new(
        Config::default(),
        Viewport::default(),
        GameContext {
            mode: Mode::Exploring,
            available: Availability::all(),
        },
    )
    .unwrap();
    let frames = replay::run(&mut engine, &all[start..end]).unwrap_or_else(|e| panic!("{id}: {e}"));
    assert!(!frames.is_empty(), "{id} must assert actual frames");
}
macro_rules! coverage_case {
    ($id:ident) => {
        #[test]
        fn $id() {
            case(stringify!($id));
        }
    };
}

#[test]
fn coverage_table_and_asserted_replay_cases_are_one_to_one() {
    let ids: Vec<String> = records()
        .iter()
        .filter_map(|r| {
            if let Record::Marker { id } = r {
                Some(id.clone())
            } else {
                None
            }
        })
        .collect();
    let replay_ids: BTreeSet<String> = ids.iter().cloned().collect();
    assert_eq!(ids.len(), replay_ids.len(), "no duplicate replay IDs");
    let table: BTreeSet<String> = include_str!("../COVERAGE.md")
        .lines()
        .filter(|line| line.starts_with("| `"))
        .map(|line| line.split('`').nth(1).unwrap().to_string())
        .collect();
    assert_eq!(
        table, replay_ids,
        "every coverage row needs an asserted gesture replay"
    );
}
coverage_case!(walk_forward);
coverage_case!(walk_backward);
coverage_case!(turn_left);
coverage_case!(turn_right);
coverage_case!(run);
coverage_case!(backward_hop);
coverage_case!(quick_turn);
coverage_case!(interact);
coverage_case!(pickup);
coverage_case!(kick_stomp);
coverage_case!(escape_grab);
coverage_case!(aim_auto);
coverage_case!(attack_tap);
coverage_case!(attack_hold);
coverage_case!(attack_repeat);
coverage_case!(step_left);
coverage_case!(step_right);
coverage_case!(look);
coverage_case!(flashlight);
coverage_case!(open_map);
coverage_case!(open_inventory);
coverage_case!(pause);
coverage_case!(radio);
coverage_case!(unpause);
coverage_case!(boss_split_head);
coverage_case!(boss_twinfeeler);
coverage_case!(boss_floatstinger);
coverage_case!(boss_monster_cybil);
coverage_case!(boss_incubator);
coverage_case!(boss_incubus);
coverage_case!(title_new);
coverage_case!(title_continue);
coverage_case!(title_load);
coverage_case!(title_movie);
coverage_case!(difficulty);
coverage_case!(options_open);
coverage_case!(options_extra);
coverage_case!(menu_back);
coverage_case!(menu_confirm);
coverage_case!(return_title);
coverage_case!(option_screen_x);
coverage_case!(option_screen_y);
coverage_case!(option_brightness);
coverage_case!(option_controller);
coverage_case!(option_vibration);
coverage_case!(option_autoload);
coverage_case!(option_sound);
coverage_case!(option_bgm);
coverage_case!(option_sfx);
coverage_case!(option_weapon);
coverage_case!(option_blood);
coverage_case!(option_view);
coverage_case!(option_view_mode);
coverage_case!(option_retreat);
coverage_case!(option_run);
coverage_case!(option_autoaim);
coverage_case!(option_bullet);
coverage_case!(option_defaults);
coverage_case!(save_slot);
coverage_case!(load_slot);
coverage_case!(save_confirm);
coverage_case!(save_cancel);
coverage_case!(save_error);
coverage_case!(game_over);
coverage_case!(ranking);
coverage_case!(credits);
coverage_case!(inventory_select);
coverage_case!(inventory_use);
coverage_case!(inventory_equip);
coverage_case!(inventory_unequip);
coverage_case!(inventory_examine);
coverage_case!(inventory_reload);
coverage_case!(inventory_toggle);
coverage_case!(inventory_scroll);
coverage_case!(inventory_rotate);
coverage_case!(inventory_close);
coverage_case!(map_pan);
coverage_case!(map_floor_up);
coverage_case!(map_floor_down);
coverage_case!(map_toggle);
coverage_case!(map_close);
coverage_case!(dialogue_next);
coverage_case!(dialogue_choice);
coverage_case!(dialogue_cancel);
coverage_case!(cutscene_skip);
coverage_case!(boot_continue);
coverage_case!(piano_white);
coverage_case!(piano_black);
coverage_case!(plates_insert);
coverage_case!(plates_remove);
coverage_case!(motel_digits);
coverage_case!(motel_clear);
coverage_case!(motel_enter);
coverage_case!(nowhere_letters_a);
coverage_case!(nowhere_letters_b);
coverage_case!(nowhere_lights_a);
coverage_case!(nowhere_lights_b);
coverage_case!(nowhere_zodiac);
coverage_case!(safe_ring);
coverage_case!(safe_left);
coverage_case!(safe_right);
coverage_case!(school_clock);
coverage_case!(school_chemistry);
coverage_case!(school_picture);
coverage_case!(school_roof);
coverage_case!(school_boiler);
coverage_case!(hospital_generator);
coverage_case!(hospital_bottle);
coverage_case!(hospital_blood);
coverage_case!(hospital_fire);
coverage_case!(motel_magnet);
coverage_case!(motel_motorcycle);
coverage_case!(nowhere_clock);
coverage_case!(nowhere_bird);
coverage_case!(nowhere_tools);
coverage_case!(nowhere_camera);
coverage_case!(nowhere_fridge);
coverage_case!(nowhere_power);
coverage_case!(nowhere_final);
coverage_case!(cybil_liquid);
coverage_case!(channeling_stone);
coverage_case!(all_keys);
coverage_case!(puzzle_cancel);
coverage_case!(unidentified_events);
coverage_case!(raw_select);
coverage_case!(raw_l3);
coverage_case!(raw_r3);
coverage_case!(raw_start);
coverage_case!(raw_up);
coverage_case!(raw_right);
coverage_case!(raw_down);
coverage_case!(raw_left);
coverage_case!(raw_l2);
coverage_case!(raw_r2);
coverage_case!(raw_l1);
coverage_case!(raw_r1);
coverage_case!(raw_triangle);
coverage_case!(raw_circle);
coverage_case!(raw_cross);
coverage_case!(raw_square);
coverage_case!(warm_reset);
coverage_case!(sound_test);
coverage_case!(debug_save);
coverage_case!(map_projection);
