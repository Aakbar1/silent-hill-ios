use sh_touch::{
    replay::{self, Record},
    *,
};
use std::io::Cursor;

fn engine(config: Config) -> Engine {
    Engine::new(
        config,
        Viewport::default(),
        GameContext {
            mode: Mode::Exploring,
            available: Availability::all(),
        },
    )
    .unwrap()
}
fn event(e: &mut Engine, id: u64, phase: Phase, x: f32, y: f32, time_ms: u64) {
    e.touch(TouchEvent {
        id,
        phase,
        x,
        y,
        time_ms,
    })
    .unwrap();
}
fn tap(e: &mut Engine, p: Point, time: u64) {
    event(e, 1, Phase::Down, p.x, p.y, time);
    event(e, 1, Phase::Up, p.x, p.y, time + 1);
}
fn target(request: UiAction, enabled: bool) -> UiTarget {
    UiTarget {
        bounds: Rect {
            x: 200.,
            y: 130.,
            width: 80.,
            height: 48.,
        },
        action: TargetAction::Ui { request },
        enabled,
    }
}

#[test]
fn ps1_packet_has_exact_active_low_bits_and_axis_order() {
    assert_eq!(
        PadState::default().ps1_packet(),
        [0, 0x73, 255, 255, 128, 128, 128, 128]
    );
    let pad = PadState {
        buttons: Buttons::CROSS | Buttons::R2,
        right: Point::new(1., -1.),
        left: Point::new(-1., 1.),
    };
    assert_eq!(pad.ps1_packet(), [0, 0x73, 255, 189, 255, 0, 0, 255]);
}

#[test]
fn ps1_packet_sanitizes_external_axes_without_wraparound() {
    let pad = PadState {
        left: Point::new(f32::NAN, 7.),
        right: Point::new(-8., f32::INFINITY),
        ..PadState::default()
    };
    assert_eq!(pad.ps1_packet(), [0, 115, 255, 255, 0, 128, 128, 255]);
}

#[test]
fn stick_dead_zone_and_game_dead_zone_agree_even_diagonally() {
    let mut e = engine(Config::default());
    event(&mut e, 1, Phase::Down, 240., 200., 0);
    event(&mut e, 1, Phase::Move, 252., 200., 1);
    assert_eq!(e.frame(1).unwrap().pad, PadState::default());
    event(&mut e, 1, Phase::Move, 249., 191., 2);
    let packet = e.frame(2).unwrap().pad.ps1_packet();
    // joy.c accepts positive >=16 and negative <-16.
    assert!(packet[6] >= 144 && packet[7] < 112);
}

#[test]
fn run_hysteresis_prevents_threshold_chatter() {
    let mut e = engine(Config::default());
    event(&mut e, 1, Phase::Down, 240., 200., 0);
    for (time, distance, run) in [
        (1, 47., true),
        (2, 43., true),
        (3, 41., false),
        (4, 45., false),
        (5, 46., true),
    ] {
        event(&mut e, 1, Phase::Move, 240., 200. - distance, time);
        assert_eq!(
            e.frame(time).unwrap().pad.buttons.contains(Buttons::SQUARE),
            run
        );
    }
}

#[test]
fn run_inversion_matches_original_extra_option() {
    let mut e = engine(Config {
        run_inverted: true,
        ..Config::default()
    });
    event(&mut e, 1, Phase::Down, 240., 200., 0);
    event(&mut e, 1, Phase::Move, 240., 162., 1);
    assert_eq!(e.frame(1).unwrap().pad.buttons, Buttons::SQUARE);
    event(&mut e, 1, Phase::Move, 240., 136., 2);
    assert_eq!(e.frame(2).unwrap().pad.buttons, Buttons::default());
}

#[test]
fn slow_downward_push_is_movement_and_fast_down_is_only_one_turn() {
    let mut e = engine(Config::default());
    event(&mut e, 1, Phase::Down, 240., 200., 0);
    event(&mut e, 1, Phase::Move, 240., 280., 300);
    let pad = e.frame(300).unwrap().pad;
    assert_eq!(pad.left.y, 1.);
    assert!(!pad.buttons.contains(Buttons::L1));
    e.cancel_all();
    event(&mut e, 1, Phase::Down, 240., 200., 310);
    event(&mut e, 1, Phase::Move, 240., 280., 400);
    assert_eq!(e.frame(400).unwrap().pad.buttons, Buttons::L1 | Buttons::R1);
    event(&mut e, 1, Phase::Move, 240., 290., 410);
    assert_eq!(e.frame(410).unwrap().pad, PadState::default());
}

#[test]
fn diagonal_fast_swipe_does_not_quick_turn() {
    let mut e = engine(Config::default());
    event(&mut e, 1, Phase::Down, 240., 200., 0);
    event(&mut e, 1, Phase::Move, 280., 280., 100);
    assert!(!e.frame(100).unwrap().pad.buttons.contains(Buttons::L1));
}

#[test]
fn two_fast_taps_between_ticks_produce_two_distinct_click_edges() {
    let mut e = engine(Config::default());
    tap(&mut e, Point::new(650., 200.), 0);
    tap(&mut e, Point::new(650., 200.), 2);
    let bits: Vec<Buttons> = (4..8).map(|t| e.frame(t).unwrap().pad.buttons).collect();
    assert_eq!(
        bits,
        [
            Buttons::CROSS,
            Buttons::default(),
            Buttons::CROSS,
            Buttons::default()
        ]
    );
}

#[test]
fn held_aim_is_sampled_without_move_events_and_latches_for_one_finger() {
    let mut e = engine(Config::default());
    event(&mut e, 1, Phase::Down, 650., 200., 0);
    assert_eq!(e.frame(279).unwrap().pad.buttons, Buttons::default());
    assert_eq!(e.frame(280).unwrap().pad.buttons, Buttons::R2);
    event(&mut e, 1, Phase::Up, 650., 200., 281);
    assert_eq!(e.frame(282).unwrap().pad.buttons, Buttons::R2);
    let aim = e.layout().region(Control::AimToggle).unwrap().center();
    tap(&mut e, aim, 283);
    assert_eq!(e.frame(285).unwrap().pad.buttons, Buttons::default());
}

#[test]
fn while_held_aim_releases_without_latching_or_action() {
    let mut e = engine(Config {
        aim_release: AimRelease::WhileHeld,
        ..Config::default()
    });
    event(&mut e, 1, Phase::Down, 650., 200., 0);
    assert_eq!(e.frame(280).unwrap().pad.buttons, Buttons::R2);
    event(&mut e, 1, Phase::Up, 650., 200., 281);
    assert_eq!(e.frame(282).unwrap().pad, PadState::default());
}

#[test]
fn release_after_long_hold_between_ticks_still_aims() {
    let mut e = engine(Config::default());
    event(&mut e, 1, Phase::Down, 650., 200., 0);
    event(&mut e, 1, Phase::Up, 650., 200., 300);
    assert_eq!(e.frame(300).unwrap().pad.buttons, Buttons::R2);
}

#[test]
fn drift_before_hold_deadline_does_not_arm_or_tap() {
    let mut e = engine(Config::default());
    event(&mut e, 1, Phase::Down, 650., 200., 0);
    event(&mut e, 1, Phase::Move, 700., 200., 300);
    assert_eq!(e.frame(300).unwrap().pad, PadState::default());
    event(&mut e, 1, Phase::Up, 650., 200., 301);
    assert_eq!(e.frame(301).unwrap().pad, PadState::default());
}

#[test]
fn combat_unavailable_never_arms() {
    let mut e = engine(Config::default());
    let mut context = e.context();
    context.available.combat = false;
    e.set_context(context);
    event(&mut e, 1, Phase::Down, 650., 200., 0);
    assert_eq!(e.frame(1000).unwrap().pad, PadState::default());
}

#[test]
fn multiple_fingers_can_move_aim_and_attack_independently() {
    let mut e = engine(Config::default());
    event(&mut e, 1, Phase::Down, 240., 200., 0);
    event(&mut e, 1, Phase::Move, 240., 136., 20);
    event(&mut e, 2, Phase::Down, 650., 200., 20);
    e.frame(300).unwrap();
    event(&mut e, 3, Phase::Down, 700., 200., 301);
    let pad = e.frame(302).unwrap().pad;
    assert_eq!(pad.buttons, Buttons::SQUARE | Buttons::R2 | Buttons::CROSS);
    assert_eq!(pad.left.y, -1.);
    event(&mut e, 3, Phase::Up, 700., 200., 303);
    assert_eq!(
        e.frame(304).unwrap().pad.buttons,
        Buttons::SQUARE | Buttons::R2
    );
    event(&mut e, 1, Phase::Cancel, 240., 136., 305);
    assert_eq!(e.frame(306).unwrap().pad.left, Point::default());
    assert_eq!(e.frame(307).unwrap().pad.buttons, Buttons::R2);
}

#[test]
fn a_second_left_finger_cannot_steal_the_stick() {
    let mut e = engine(Config::default());
    event(&mut e, 1, Phase::Down, 240., 200., 0);
    event(&mut e, 1, Phase::Move, 240., 162., 1);
    event(&mut e, 2, Phase::Down, 350., 200., 2);
    event(&mut e, 2, Phase::Move, 300., 200., 3);
    assert!((e.frame(3).unwrap().pad.left.y + 0.6).abs() < 0.0001);
}

#[test]
fn stick_capture_cannot_activate_a_chip_it_crosses() {
    let mut e = engine(Config::default());
    event(&mut e, 1, Phase::Down, 240., 200., 0);
    event(&mut e, 1, Phase::Move, 862., 387., 300);
    event(&mut e, 1, Phase::Up, 862., 387., 301);
    assert_eq!(e.frame(302).unwrap(), FrameOutput::default());
}

#[test]
fn cancelled_contacts_do_not_click_or_latch() {
    let mut e = engine(Config::default());
    event(&mut e, 1, Phase::Down, 650., 200., 0);
    e.frame(300).unwrap();
    event(&mut e, 1, Phase::Cancel, 650., 200., 301);
    assert_eq!(e.frame(302).unwrap(), FrameOutput::default());
}

#[test]
fn focus_cancel_drops_queued_taps_stick_and_latched_aim() {
    let mut e = engine(Config::default());
    event(&mut e, 1, Phase::Down, 650., 200., 0);
    event(&mut e, 1, Phase::Up, 650., 200., 300);
    tap(&mut e, Point::new(650., 200.), 301);
    e.cancel_all();
    assert_eq!(e.frame(303).unwrap(), FrameOutput::default());
}

#[test]
fn entering_menu_cancels_old_input_and_late_up_is_harmless() {
    let mut e = engine(Config::default());
    event(&mut e, 1, Phase::Down, 650., 200., 0);
    e.frame(300).unwrap();
    let mut context = e.context();
    context.mode = Mode::Inventory;
    e.set_context(context);
    event(&mut e, 1, Phase::Up, 650., 200., 301);
    assert_eq!(e.frame(302).unwrap(), FrameOutput::default());
    assert!(e.targets().is_empty());
}

#[test]
fn host_exploring_to_aiming_transition_preserves_the_aim_finger() {
    let mut e = engine(Config::default());
    event(&mut e, 1, Phase::Down, 650., 200., 0);
    e.frame(280).unwrap();
    let mut context = e.context();
    context.mode = Mode::Aiming;
    e.set_context(context);
    assert_eq!(e.frame(281).unwrap().pad.buttons, Buttons::R2);
}

#[test]
fn resized_or_rotated_viewport_invalidates_captures_and_hotspots() {
    let mut e = engine(Config::default());
    event(&mut e, 1, Phase::Down, 240., 200., 0);
    let mut view = e.viewport();
    view.width = 440.;
    view.height = 956.;
    e.set_viewport(view).unwrap();
    assert_eq!(e.frame(1).unwrap(), FrameOutput::default());
    assert!(e.stick_visual().is_none());
}

#[test]
fn stable_context_viewport_and_targets_do_not_drop_a_capture() {
    let mut e = engine(Config::default());
    e.set_targets(vec![target(UiAction::Select { id: 3 }, true)])
        .unwrap();
    event(&mut e, 1, Phase::Down, 220., 150., 0);
    e.set_context(e.context());
    e.set_viewport(e.viewport()).unwrap();
    e.set_targets(e.targets().to_vec()).unwrap();
    event(&mut e, 1, Phase::Up, 220., 150., 1);
    assert_eq!(e.frame(2).unwrap().ui_actions, [UiAction::Select { id: 3 }]);
}

#[test]
fn changing_targets_discards_old_pending_actions_and_captures() {
    let mut e = engine(Config::default());
    e.set_targets(vec![target(UiAction::Select { id: 3 }, true)])
        .unwrap();
    tap(&mut e, Point::new(220., 150.), 0);
    e.set_targets(vec![target(UiAction::Select { id: 4 }, true)])
        .unwrap();
    assert_eq!(e.frame(2).unwrap(), FrameOutput::default());
}

#[test]
fn direct_ui_target_emits_once_without_pad_and_disabled_targets_ignore_taps() {
    let mut e = engine(Config::default());
    let mut c = e.context();
    c.mode = Mode::Menu;
    e.set_context(c);
    e.set_targets(vec![target(UiAction::Activate { id: 3 }, false)])
        .unwrap();
    tap(&mut e, Point::new(220., 150.), 0);
    assert_eq!(e.frame(2).unwrap(), FrameOutput::default());
    e.set_targets(vec![target(UiAction::Activate { id: 3 }, true)])
        .unwrap();
    tap(&mut e, Point::new(220., 150.), 3);
    let f = e.frame(5).unwrap();
    assert_eq!(f.pad, PadState::default());
    assert_eq!(f.ui_actions, [UiAction::Activate { id: 3 }]);
    assert_eq!(e.frame(6).unwrap(), FrameOutput::default());
}

#[test]
fn target_drag_out_and_back_does_not_activate() {
    let mut e = engine(Config::default());
    e.set_targets(vec![target(UiAction::Confirm, true)])
        .unwrap();
    event(&mut e, 1, Phase::Down, 220., 150., 0);
    event(&mut e, 1, Phase::Move, 300., 150., 1);
    event(&mut e, 1, Phase::Up, 220., 150., 2);
    assert_eq!(e.frame(3).unwrap(), FrameOutput::default());
}

#[test]
fn safe_area_touch_is_ignored_and_regions_remain_reachable_for_both_hands() {
    for hand in [Hand::Left, Hand::Right] {
        let mut e = engine(Config {
            hand,
            ..Config::default()
        });
        let c = e.layout().content;
        for r in &e.layout().controls {
            assert!(c.contains(r.bounds.center()));
            assert!(r.bounds.y >= c.height - 176.);
            assert!(r.bounds.width >= 44. && r.bounds.height >= 44.);
        }
        tap(&mut e, Point::new(10., 10.), 0);
        assert_eq!(e.frame(2).unwrap(), FrameOutput::default());
    }
}

#[test]
fn live_availability_hides_chips_and_releases_previous_input() {
    let mut e = engine(Config::default());
    event(&mut e, 1, Phase::Down, 650., 200., 0);
    e.frame(280).unwrap();
    let mut c = e.context();
    c.available = Availability::default();
    e.set_context(c);
    assert!(e.layout().controls.is_empty());
    assert_eq!(e.frame(281).unwrap(), FrameOutput::default());
}

#[test]
fn unskippable_cutscene_has_no_skip_path_until_host_enables_it() {
    let mut e = engine(Config::default());
    let mut c = e.context();
    c.mode = Mode::Cutscene;
    c.available.skip = false;
    e.set_context(c);
    assert!(e.layout().region(Control::Skip).is_none());
    tap(&mut e, Point::new(862., 387.), 0);
    assert_eq!(e.frame(2).unwrap(), FrameOutput::default());
}

#[test]
fn all_three_usa_presets_change_the_gesture_output_bindings() {
    for (preset, aim, item, map, step) in [
        (
            0,
            Buttons::R2,
            Buttons::SELECT,
            Buttons::TRIANGLE,
            Buttons::L1,
        ),
        (
            1,
            Buttons::R1,
            Buttons::SELECT,
            Buttons::TRIANGLE,
            Buttons::L2,
        ),
        (
            2,
            Buttons::R2,
            Buttons::TRIANGLE,
            Buttons::SELECT,
            Buttons::L1,
        ),
    ] {
        let mut e = engine(Config {
            bindings: Bindings::usa_preset(preset).unwrap(),
            ..Config::default()
        });
        for (control, expected, time) in [
            (Control::Inventory, item, 0),
            (Control::Map, map, 4),
            (Control::StepLeft, step, 8),
            (Control::AimToggle, aim, 12),
        ] {
            let p = e.layout().region(control).unwrap().center();
            tap(&mut e, p, time);
            assert_eq!(e.frame(time + 2).unwrap().pad.buttons, expected);
            e.frame(time + 3).unwrap();
        }
    }
    assert!(Bindings::usa_preset(3).is_none());
}

#[test]
fn original_toggle_weapon_option_gets_edges_only_on_intent_changes() {
    let mut e = engine(Config {
        weapon_toggle: true,
        ..Config::default()
    });
    event(&mut e, 1, Phase::Down, 650., 200., 0);
    assert_eq!(e.frame(280).unwrap().pad.buttons, Buttons::R2);
    assert_eq!(e.frame(281).unwrap().pad.buttons, Buttons::default());
    event(&mut e, 1, Phase::Up, 650., 200., 282);
    assert_eq!(e.frame(283).unwrap().pad.buttons, Buttons::default());
    tap(&mut e, Point::new(650., 200.), 284);
    assert_eq!(e.frame(286).unwrap().pad.buttons, Buttons::CROSS);
    e.cancel_all();
    assert_eq!(e.frame(287).unwrap().pad.buttons, Buttons::R2);
    assert_eq!(e.frame(288).unwrap().pad.buttons, Buttons::default());
}

#[test]
fn malformed_geometry_tuning_targets_and_events_are_rejected_atomically() {
    let mut config = Config::default();
    config.tuning.dead_zone = 100.;
    assert!(matches!(
        Engine::new(config, Viewport::default(), GameContext::default()),
        Err(InputError::InvalidTuning)
    ));
    let mut e = engine(Config::default());
    let old = e.viewport();
    let mut invalid = old;
    invalid.safe.left = f32::NAN;
    assert_eq!(e.set_viewport(invalid), Err(InputError::InvalidGeometry));
    assert_eq!(e.viewport(), old);
    let mut t = target(UiAction::Confirm, true);
    t.bounds.width = 1000.;
    assert_eq!(e.set_targets(vec![t]), Err(InputError::InvalidTarget));
    assert!(e.targets().is_empty());
    assert_eq!(
        e.touch(TouchEvent {
            id: 1,
            phase: Phase::Down,
            x: f32::NAN,
            y: 0.,
            time_ms: 0
        }),
        Err(InputError::NonFinitePosition)
    );
    e.frame(10).unwrap();
    assert_eq!(e.frame(9), Err(InputError::TimeWentBackwards));
    event(&mut e, 1, Phase::Down, 240., 200., 11);
    assert_eq!(
        e.touch(TouchEvent {
            id: 1,
            phase: Phase::Down,
            x: 240.,
            y: 200.,
            time_ms: 12
        }),
        Err(InputError::DuplicateContact)
    );
    assert_eq!(
        e.touch(TouchEvent {
            id: 2,
            phase: Phase::Move,
            x: 240.,
            y: 200.,
            time_ms: 12
        }),
        Err(InputError::UnknownContact)
    );
}

#[test]
fn ten_contact_limit_fails_without_stealing_an_existing_finger() {
    let mut e = engine(Config::default());
    for id in 0..10 {
        event(&mut e, id, Phase::Down, 240., 200., 0);
    }
    assert_eq!(
        e.touch(TouchEvent {
            id: 10,
            phase: Phase::Down,
            x: 240.,
            y: 200.,
            time_ms: 1
        }),
        Err(InputError::TooManyContacts)
    );
    event(&mut e, 0, Phase::Move, 240., 136., 2);
    assert_eq!(e.frame(2).unwrap().pad.left.y, -1.);
}

#[test]
fn legacy_target_can_hold_both_axes_and_buttons_without_extra_release_tick() {
    let mut e = engine(Config::default());
    let state = PadState {
        buttons: Buttons::L3 | Buttons::R3,
        left: Point::new(-1., 0.5),
        right: Point::new(0.25, -1.),
    };
    e.set_targets(vec![UiTarget {
        bounds: target(UiAction::Confirm, true).bounds,
        action: TargetAction::Pad { state, hold: true },
        enabled: true,
    }])
    .unwrap();
    event(&mut e, 1, Phase::Down, 220., 150., 0);
    assert_eq!(e.frame(1).unwrap().pad, state);
    event(&mut e, 1, Phase::Up, 220., 150., 2);
    assert_eq!(e.frame(3).unwrap().pad, PadState::default());
}

#[test]
fn replay_codec_rejects_unknown_fields_and_reports_mismatch_line() {
    assert!(replay::parse(Cursor::new("{\"type\":\"touch\",\"id\":1,\"phase\":\"down\",\"x\":0,\"y\":0,\"time_ms\":0,\"typo\":1}\n")).is_err());
    let records = replay::parse(Cursor::new(
        "{\"type\":\"frame\",\"time_ms\":0,\"expect\":{\"buttons\":16384}}\n",
    ))
    .unwrap();
    assert!(
        replay::run(&mut engine(Config::default()), &records)
            .unwrap_err()
            .contains("line 1: expected")
    );
}

#[test]
fn complete_jsonl_suite_replays_deterministically_and_roundtrips() {
    let records = replay::parse(Cursor::new(include_str!("replays/coverage.jsonl"))).unwrap();
    let a = replay::run(&mut engine(Config::default()), &records).unwrap();
    let b = replay::run(&mut engine(Config::default()), &records).unwrap();
    assert_eq!(a, b);
    for record in records {
        let encoded = serde_json::to_string(&record).unwrap();
        let decoded: Record = serde_json::from_str(&encoded).unwrap();
        assert_eq!(record, decoded);
    }
}

#[test]
fn options_can_update_live_bindings_without_retaining_old_captures() {
    let mut e = engine(Config::default());
    event(&mut e, 1, Phase::Down, 650., 200., 0);
    e.frame(280).unwrap();
    let mut config = e.config();
    config.bindings = Bindings::usa_preset(1).unwrap();
    e.set_config(config).unwrap();
    assert_eq!(e.frame(281).unwrap(), FrameOutput::default());
    event(&mut e, 1, Phase::Up, 650., 200., 282);
    let aim = e.layout().region(Control::AimToggle).unwrap().center();
    tap(&mut e, aim, 283);
    assert_eq!(e.frame(285).unwrap().pad.buttons, Buttons::R1);
    config.tuning.dead_zone = 100.;
    assert_eq!(e.set_config(config), Err(InputError::InvalidTuning));
    assert_eq!(e.config().bindings.aim, Buttons::R1);
}

#[test]
fn original_overlay_demo_runs_simultaneous_stick_and_aim_then_releases() {
    let records = replay::parse(Cursor::new(include_str!("replays/demo.jsonl"))).unwrap();
    let frames = replay::run(&mut engine(Config::default()), &records).unwrap();
    assert_eq!(frames.len(), 12);
    assert_eq!(frames.last().unwrap(), &FrameOutput::default());
}
