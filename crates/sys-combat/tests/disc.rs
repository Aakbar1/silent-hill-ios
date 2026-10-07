// SPDX-License-Identifier: GPL-3.0-only
use std::path::PathBuf;
use sys_combat::{
    animation::decode_enemy_animation,
    assets::{decode_ipd, decode_lm},
    gameplay::{NativeAnimation, NativeLm},
};
unsafe extern "C" {
    fn sh_combat_attack_compare(bytes: *const u8, size: usize) -> i32;
    fn sh_combat_model_probe(lm: *const std::ffi::c_void, anm: *const std::ffi::c_void) -> i32;
    fn sh_combat_sqrt_disc_probe(bytes: *const u8, size: usize) -> i32;
    fn sh_combat_collision_surface_height(
        collision: *const std::ffi::c_void,
        index: u32,
        height: *mut i16,
    ) -> bool;
}
fn disc() -> Option<psxdisc::GameDisc<psxdisc::DiscImage<std::fs::File>>> {
    let path = std::env::var_os("SH_DISC")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../../../private/disc/Silent Hill (USA).bin")
        });
    if !path.exists() {
        assert!(
            std::env::var_os("SH_REQUIRE_DISC").is_none(),
            "required disc missing: {}",
            path.display()
        );
        eprintln!("SKIP: owned disc unavailable; set SH_DISC (SH_REQUIRE_DISC=1 enforces it)");
        return None;
    }
    Some(psxdisc::GameDisc::open(path).expect("verified US 1.1 disc"))
}
#[test]
fn real_attack_table_matches_all_seventy_native_records() {
    let Some(mut disc) = disc() else { return };
    let mut body = disc.read_entry_by_name("1ST/BODYPROG.BIN").unwrap();
    let mut seed = 0u32;
    for bytes in body.as_chunks_mut::<4>().0 {
        seed = seed.wrapping_add(0x01309125).wrapping_mul(0x03a452f7);
        *bytes = (u32::from_le_bytes(*bytes) ^ seed).to_le_bytes();
    }
    let offset = 0x800ad4c8usize - 0x80024b60;
    let data = &body[offset..offset + 70 * 24];
    // SAFETY: C reads exactly size bytes, decodes scalars and compares source data.
    assert_eq!(
        unsafe { sh_combat_attack_compare(data.as_ptr(), data.len()) },
        0
    );
    let offset = 0x800affccusize - 0x80024b60;
    let table = &body[offset..offset + 384];
    // SAFETY: C validates size and decodes 192 halfwords into its fixture table.
    assert_eq!(
        unsafe { sh_combat_sqrt_disc_probe(table.as_ptr(), table.len()) },
        0
    );
}
#[test]
fn real_enemy_models_and_animations_reach_native_c() {
    let Some(mut disc) = disc() else { return };
    // File IDs come from the pinned GPL CHARA_FILE_INFOS source, never a copied
    // list of extracted disc contents. Generator supplies all type/variant pairs.
    let pairs: &[(u32, u32)] = include!(concat!(env!("OUT_DIR"), "/enemy_asset_pairs.rs"));
    let mut count = 0;
    for &(model_id, anim_id) in pairs {
        let model = disc.read_entry(model_id).unwrap();
        let animation = disc.read_entry(anim_id).unwrap();
        let original_model = model.clone();
        let original_animation = animation.clone();
        let lm = decode_lm(&model).unwrap_or_else(|e| panic!("model {model_id}: {e}"));
        let anm=decode_enemy_animation(&animation).unwrap_or_else(|e| panic!("animation {anim_id} model {model_id}: {e}; bones={} scale={} declared={} available={}",animation[6],animation[18],u32::from_le_bytes(animation[12..16].try_into().unwrap()),animation.len()));
        let native_lm = NativeLm::new(&lm);
        let native_anm = NativeAnimation::new(&anm);
        // SAFETY: aligned graphs own all their decoded leaves throughout the call.
        let vertices = unsafe {
            sh_combat_model_probe(
                std::ptr::from_ref(&*native_lm.header).cast(),
                std::ptr::from_ref(&*native_anm.header).cast(),
            )
        };
        assert!(vertices > 0, "model={model_id}, animation={anim_id}");
        assert_eq!(model, original_model);
        assert_eq!(animation, original_animation);
        count += 1;
    }
    assert!(count > 10);
    eprintln!("Real native enemy model/animation graph pairs checked: {count}");
}
#[test]
fn real_map_collision_records_reach_native_c() {
    let Some(mut disc) = disc() else { return };
    let ids: Vec<_> = disc
        .entries()
        .iter()
        .filter(|e| e.name.starts_with("THR") && e.name.ends_with(".IPD"))
        .map(|e| e.id)
        .collect();
    assert_eq!(ids.len(), 128);
    let mut surfaces = 0;
    for id in ids {
        let bytes = disc.read_entry(id).unwrap();
        let original = bytes.clone();
        let decoded = decode_ipd(&bytes).unwrap();
        let native = sys_combat::maps::CombatCollision::new(&decoded.collision);
        for (index, wire) in decoded
            .collision
            .surfaces
            .bytes()
            .as_chunks::<12>()
            .0
            .iter()
            .enumerate()
        {
            let mut height = 0i16;
            // SAFETY: the native graph owns aligned leaves; output is writable.
            assert!(unsafe {
                sh_combat_collision_surface_height(
                    std::ptr::from_ref(&*native.header).cast(),
                    index as u32,
                    &mut height,
                )
            });
            assert_eq!(height, i16::from_le_bytes([wire[2], wire[3]]));
            surfaces += 1;
        }
        let mut unchanged = 123i16;
        // SAFETY: live graph and writable output, deliberately invalid index.
        assert!(!unsafe {
            sh_combat_collision_surface_height(
                std::ptr::from_ref(&*native.header).cast(),
                256,
                &mut unchanged,
            )
        });
        assert_eq!(unchanged, 123);
        assert_eq!(bytes, original);
    }
    assert!(surfaces > 0);
    eprintln!("Real THR collision graphs: 128; native surface height reads: {surfaces}");
}
