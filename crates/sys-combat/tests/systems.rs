// SPDX-License-Identifier: GPL-3.0-only
fn case(index: i32) {
    assert_eq!(
        sys_combat::run_fixture(index),
        0,
        "C case {index} failed (value is source line)"
    );
}
#[test]
fn six_npc_hit_masks() {
    case(0)
}
#[test]
fn original_enemy_damage_changes_health() {
    case(1)
}
#[test]
fn enemy_attacker_identity_and_death_counter() {
    case(2)
}
#[test]
fn original_reload_cases() {
    case(3)
}
#[test]
fn decoded_collision_interpolation() {
    case(4)
}
#[test]
fn firearm_ray_damage_and_history() {
    case(5)
}
#[test]
fn scripted_enemy_state_steps() {
    case(6)
}
#[test]
fn complete_small_ai_dispatch() {
    case(7)
}
#[test]
fn pause_blocks_attack_entry() {
    case(8)
}
#[test]
fn original_enemy_persistence() {
    case(9)
}
#[test]
fn ps1_division_edge_cases() {
    case(10)
}
#[test]
fn checked_attack_wire_boundary() {
    case(11)
}
