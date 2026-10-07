// SPDX-License-Identifier: GPL-3.0-only
use super::*;
use std::{cell::RefCell, path::PathBuf, sync::Mutex};
static C_LOCK: Mutex<()> = Mutex::new(());
thread_local! { static STORE: RefCell<Option<crate::host_saves::SaveStore>> = const { RefCell::new(None) }; }
#[link(name = "sh_sys_items", kind = "static")]
unsafe extern "C" {
    fn sh_items_probe_inventory() -> i32;
    fn sh_items_probe_piano(tables: *const PianoTables, wrong: i32) -> i32;
    fn sh_items_probe_save(
        bytes: *mut u8,
        length: usize,
        device: i32,
        file: i32,
        save: i32,
        fail: i32,
    ) -> i32;
    fn sh_items_probe_overlays() -> i32;
    fn sh_items_probe_aliases() -> i32;
    fn sh_items_probe_radio_notes() -> i32;
    fn sh_items_probe_endings() -> i32;
    fn sh_items_probe_ending_table(bytes: *const u8, length: usize) -> i32;
    fn sh_items_probe_tmd(header: *mut NativeTmdHeader) -> i32;
    fn sh_items_save_layout_probe(bytes: *const u8, length: usize) -> i32;
    fn sh_items_piano_decode(bytes: *const u8, length: usize, output: *mut PianoTables) -> i32;
    fn sh_items_slot_id(device: i32, file: i32, save: i32, out: *mut u32) -> i32;
}
#[unsafe(no_mangle)]
unsafe extern "C" fn port_save_read(slot: u32, output: *mut u8, count: u32) -> i32 {
    if output.is_null() || count != 636 {
        return 2;
    }
    STORE.with(|store| {
        match store
            .borrow()
            .as_ref()
            .and_then(|store| store.read(slot).ok())
        {
            Some(Some(bytes)) => {
                unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), output, 636) };
                0
            }
            Some(None) => 1,
            None => 2,
        }
    })
}
#[unsafe(no_mangle)]
unsafe extern "C" fn port_save_write(slot: u32, source: *const u8, count: u32) -> i32 {
    if source.is_null() || count != 636 {
        return 2;
    }
    STORE.with(|store| {
        let store = store.borrow();
        match store.as_ref() {
            Some(store) => {
                i32::from(
                    store
                        .write(slot, unsafe { std::slice::from_raw_parts(source, 636) })
                        .is_err(),
                ) * 2
            }
            None => 2,
        }
    })
}
fn sidecar_store(kind: u32, id: u32, count: u32) -> Option<SidecarStore> {
    if !SidecarStore::valid(kind, id, count) {
        return None;
    }
    STORE.with(|store| {
        store
            .borrow()
            .as_ref()
            .map(|s| SidecarStore::new(s.root().to_path_buf()))
    })
}
#[unsafe(no_mangle)]
unsafe extern "C" fn port_items_sidecar_read(
    kind: u32,
    id: u32,
    output: *mut u8,
    count: u32,
) -> i32 {
    if output.is_null() {
        return 2;
    }
    let Some(store) = sidecar_store(kind, id, count) else {
        return 2;
    };
    unsafe { crate::read_sidecar_record(store.root(), kind, id, output, count) }
}
#[unsafe(no_mangle)]
unsafe extern "C" fn port_items_sidecar_write(
    kind: u32,
    id: u32,
    bytes: *const u8,
    count: u32,
) -> i32 {
    if bytes.is_null() {
        return 2;
    }
    let Some(store) = sidecar_store(kind, id, count) else {
        return 2;
    };
    unsafe { crate::write_sidecar_record(store.root(), kind, id, bytes, count) }
}
#[test]
fn original_inventory_add_stack_use_remove() {
    let _lock = C_LOCK.lock().unwrap();
    assert_eq!(unsafe { sh_items_probe_inventory() }, 0);
}
#[test]
fn original_bss_aliases_remain_identical_in_native_allocations() {
    let _lock = C_LOCK.lock().unwrap();
    assert_eq!(unsafe { sh_items_probe_aliases() }, 0);
}
#[test]
fn radio_sewer_restrictions_flashlight_and_note_commands_are_original() {
    let _lock = C_LOCK.lock().unwrap();
    assert_eq!(unsafe { sh_items_probe_radio_notes() }, 0);
}
#[test]
fn original_ranking_preserves_ending_history_and_prepares_next_fear_save() {
    let _lock = C_LOCK.lock().unwrap();
    assert_eq!(unsafe { sh_items_probe_endings() }, 0);
}
#[test]
fn native_app_data_location_is_constructed_without_writing() {
    if let Ok(store) = crate::host_saves::SaveStore::app_data() {
        assert!(store.root().ends_with("SilentHillIOS/saves"));
    }
}
#[test]
fn original_piano_accepts_sequence_rejects_wrong_keys() {
    let _lock = C_LOCK.lock().unwrap();
    // Synthetic ordered hit table and silent-key sequence; no extracted bytes.
    let bytes: Vec<u8> = [
        -80i8, -65, -50, -35, -20, -5, 10, 25, 40, 55, 70, 0, 2, 7, 9, 10, 1,
    ]
    .iter()
    .map(|v| *v as u8)
    .collect();
    let tables = PianoTables::decode(&bytes).unwrap();
    let mut c_tables = PianoTables {
        thresholds: [0; 11],
        sequence: [0; 5],
    };
    assert_eq!(
        unsafe { sh_items_piano_decode(bytes.as_ptr(), bytes.len(), &mut c_tables) },
        0
    );
    assert_eq!(tables, c_tables);
    assert_eq!(unsafe { sh_items_probe_piano(&tables, 0) }, 0);
    assert_eq!(unsafe { sh_items_probe_piano(&tables, 1) }, 0);
    assert!(PianoTables::decode(&bytes[..16]).is_err());
    let mut bad = bytes;
    bad[13] = 0;
    assert!(PianoTables::decode(&bad).is_err());
    assert_eq!(
        unsafe { sh_items_piano_decode(bad.as_ptr(), bad.len(), &mut c_tables) },
        2
    );
    assert_eq!(tables, c_tables); // failed decoding never publishes partial state
}
#[test]
fn original_saveload_roundtrip_uses_existing_native_store() {
    let _lock = C_LOCK.lock().unwrap();
    let root = std::env::temp_dir().join(format!("sh-items-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    STORE.with(|store| *store.borrow_mut() = Some(crate::host_saves::SaveStore::new(root.clone())));
    for (device, file, save, expected_slot) in [(0, 0, 0, 0), (4, 14, 10, 329)] {
        let mut bytes = std::array::from_fn::<_, 636, _>(|i| (i * 37) as u8);
        bytes[0xa8] = 3;
        bytes[0xa6..0xa8].copy_from_slice(&7i16.to_le_bytes());
        let mut expected = bytes;
        expected[0xa6..0xa8].copy_from_slice(&8i16.to_le_bytes());
        assert_eq!(
            unsafe { sh_items_probe_save(bytes.as_mut_ptr(), 636, device, file, save, 0) },
            0
        );
        assert_eq!(bytes, expected);
        assert_eq!(
            std::fs::read(root.join(format!("slot-{expected_slot:03}.shs"))).unwrap(),
            expected
        );
    }
    STORE.with(|store| *store.borrow_mut() = None);
    let mut bytes = [0u8; 636];
    assert_eq!(
        unsafe { sh_items_probe_save(bytes.as_mut_ptr(), 636, 0, 0, 0, 1) },
        0
    );
    // Only remove the two explicitly created files in this fixture's directory.
    for slot in [0, 329] {
        std::fs::remove_file(root.join(format!("slot-{slot:03}.shs"))).unwrap();
    }
    for name in [
        "slot-000.shmi",
        "slot-329.shmi",
        "options-00.shoc",
        "options-29.shoc",
    ] {
        std::fs::remove_file(root.join(name)).unwrap();
    }
    std::fs::remove_dir(root).unwrap();
}
#[test]
fn original_overlay_images_restore_writable_state() {
    let _lock = C_LOCK.lock().unwrap();
    assert_eq!(unsafe { sh_items_probe_overlays() }, 0);
}
#[test]
fn all_save_bits_and_signed_difficulty_match_original_c() {
    for nibble in 0..16 {
        let mut bytes = std::array::from_fn::<_, 636, _>(|i| (i * 97) as u8);
        bytes[0x263] = (nibble << 4) | 0xf;
        let decoded = SavePayload::decode(&bytes).unwrap();
        assert_eq!(decoded.encode(), bytes);
        assert_eq!(decoded.difficulty(), (nibble as i8) << 4 >> 4);
        assert_eq!(
            unsafe { sh_items_save_layout_probe(bytes.as_ptr(), bytes.len()) },
            0
        );
        assert_eq!(decoded.items()[39].order, bytes[159]);
        assert_eq!(
            decoded.health(),
            i32::from_le_bytes(bytes[0x240..0x244].try_into().unwrap())
        );
        let _ = (decoded.toggles(), decoded.event_flags(), decoded.endings());
    }
    assert!(SavePayload::decode(&[0; 635]).is_err());
    assert!(SavePayload::decode(&[0; 637]).is_err());
}
#[test]
fn slot_mapping_matches_original_two_cards_and_rejects_aliases() {
    for (device, file, save, expected) in [
        (0, 0, 0, 0),
        (0, 14, 10, 164),
        (4, 0, 0, 165),
        (4, 14, 10, 329),
    ] {
        let mut actual = 999;
        assert_eq!(
            unsafe { sh_items_slot_id(device, file, save, &mut actual) },
            0
        );
        assert_eq!(actual, expected);
    }
    for (device, file, save) in [(1, 0, 0), (-1, 0, 0), (4, 15, 0), (0, 0, 11), (0, -1, 0)] {
        let mut actual = 999;
        assert_eq!(
            unsafe { sh_items_slot_id(device, file, save, &mut actual) },
            2
        );
        assert_eq!(actual, 999);
    }
}
#[test]
fn credits_decode_owned_tables_rejects_bad_addresses() {
    let mut image = vec![0u8; 540];
    for i in 0..256 {
        image[i * 2..i * 2 + 2].copy_from_slice(&(i as i16 - 128).to_le_bytes());
    }
    image[512..516].copy_from_slice(&0xfedcba98u32.to_le_bytes());
    let mut wire = [0u8; 28];
    wire[12..16].copy_from_slice(&0x8000u32.to_le_bytes());
    wire[16..20].copy_from_slice(&0x8200u32.to_le_bytes());
    let native = CreditState::decode(&wire, &image, 0x8000).unwrap();
    image.fill(0);
    assert_eq!(native.widths[0], -128);
    assert_eq!(native.colors[0], 0xfedcba98);
    assert!(CreditState::decode(&wire, &image[..539], 0x8000).is_err());
    wire[12..16].copy_from_slice(&0x7fffu32.to_le_bytes());
    assert!(CreditState::decode(&wire, &image, 0x8000).is_err());
    assert!(decode_points(&[0, 128, 255, 127], usize::MAX).is_err());
    assert_eq!(
        decode_points(&[0, 128, 255, 127], 1).unwrap(),
        [(-32768, 32767)]
    );
    assert!(decode_panel_cells(&[0; 54], 28).is_err());
    assert_eq!(SaveMetadata::decode(&[0; 12]).unwrap().total_count, 0);
}
#[test]
fn tmd_native_ownership_and_negative_wire_controls() {
    let _lock = C_LOCK.lock().unwrap();
    // One synthetic model, one vertex/normal and one bounded three-word primitive.
    let mut wire = vec![0u8; 68];
    for (offset, value) in [
        (0usize, 0x41u32),
        (8, 1),
        (12, 28),
        (16, 1),
        (20, 36),
        (24, 1),
        (28, 44),
        (32, 1),
        (56, 0x20030204),
    ] {
        wire[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    let mut graph = Tmd::decode(&wire).unwrap();
    wire.fill(0);
    assert_eq!(unsafe { sh_items_probe_tmd(&mut graph.header) }, 0);
    assert_eq!(graph.model_count(), 1);
    assert_eq!(graph.decoded_word_count(), 7);
    let mut bad = vec![0u8; 12];
    bad[..4].copy_from_slice(&0x41u32.to_le_bytes());
    bad[4..8].copy_from_slice(&1u32.to_le_bytes());
    assert!(Tmd::decode(&bad).is_err());
    bad[4..8].fill(0);
    bad[8..12].copy_from_slice(&1u32.to_le_bytes());
    assert!(Tmd::decode(&bad).is_err());
    assert!(encrypted_data_leaf(&[0; 8], usize::MAX, 1).is_err());
    assert!(encrypted_data_leaf(&[0; 7], 0, 4).is_err());
    assert_eq!(encrypted_data_leaf(&[0; 8], 8, 0).unwrap(), []);
}
#[test]
fn native_credit_3d_and_save_row_resolve_checked_owned_data() {
    let image = vec![0u8; 540];
    let mut wire = [0u8; 88];
    wire[12..16].copy_from_slice(&0x8000u32.to_le_bytes());
    wire[16..20].copy_from_slice(&0x8200u32.to_le_bytes());
    let mut owner = Credit3dState::decode(&wire, &image, 0x8000).unwrap();
    let native = owner.native();
    assert!(!native.text.widths.is_null());
    assert!(Credit3dState::decode(&wire[..87], &image, 0x8000).is_err());
    let mut row = [0u8; 16];
    row[12..16].copy_from_slice(&0x8000u32.to_le_bytes());
    let row = SaveRow::decode(&row, &image, 0x8000).unwrap();
    let native = row.native();
    assert!(!native.record.metadata.is_null());
    assert!(native.metadata().is_some());
    let mut bad = [0u8; 16];
    bad[12..16].copy_from_slice(&0x7fffu32.to_le_bytes());
    assert!(SaveRow::decode(&bad, &image, 0x8000).is_err());
}
#[test]
fn owned_disc_piano_tables_and_original_solution_path() {
    let _lock = C_LOCK.lock().unwrap();
    let path = std::env::var_os("SH_DISC")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../../../private/disc/Silent Hill (USA).bin")
        });
    if !path.exists() {
        eprintln!("SKIP: owned disc not available at {}", path.display());
        return;
    }
    let mut disc = psxdisc::GameDisc::open(&path).expect("pinned USA v1.1 release verification");
    for &(file, base, address, count, kind) in DISC_TABLES {
        let id = disc.entry_by_name(file).unwrap().id;
        let stride = if kind == "u8_xy" {
            2
        } else if kind == "mask27" {
            4
        } else {
            1
        };
        let bytes = disc
            .read_entry_range(id, u64::from(address - base), (count * stride) as u64)
            .unwrap();
        match kind {
            "u8_xy" => {
                assert_eq!(decode_panel_cells(&bytes, count).unwrap().len(), count);
            }
            "digit_sequence" => assert!(
                bytes.iter().all(|b| *b <= 10),
                "invalid digit sequence in {file}"
            ),
            "letter_sequence" => assert!(
                bytes.iter().all(|b| *b < 26),
                "invalid letter sequence in {file}"
            ),
            "mask27" => {
                let mask = u32::from_le_bytes(bytes.try_into().unwrap());
                assert!(
                    mask != 0 && mask < (1 << 27),
                    "invalid light mask in {file}"
                );
            }
            "i8" => assert_eq!(bytes.len(), count),
            _ => panic!("unknown puzzle table schema"),
        }
    }
    let tmds: Vec<_> = disc
        .entries()
        .iter()
        .filter(|e| e.path.starts_with("ITEM/") && e.path.ends_with(".TMD"))
        .map(|e| (e.id, e.name.clone()))
        .collect();
    assert!(!tmds.is_empty());
    for (id, name) in &tmds {
        let bytes = disc.read_entry(*id).unwrap();
        let original = bytes.clone();
        let mut tmd = Tmd::decode(&bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(unsafe { sh_items_probe_tmd(&mut tmd.header) }, 0);
        assert_eq!(bytes, original);
        assert_eq!(tmd.header.model_count as usize, tmd.model_count());
        let _ = tmd.decoded_word_count();
    }
    eprintln!(
        "Owned disc: {} inventory TMD files passed Rust decode and native C graph probe",
        tmds.len()
    );
    let id = disc.entry_by_name("MAP1_S01.BIN").unwrap().id;
    // Numeric source-declared tables only. No executable bytes are emitted.
    let bytes = disc
        .read_entry_range(id, 0x800dd500 - 0x800c9578, 17)
        .unwrap();
    let tables = PianoTables::decode(&bytes).unwrap();
    assert_eq!(unsafe { sh_items_probe_piano(&tables, 0) }, 0);
    assert_eq!(unsafe { sh_items_probe_piano(&tables, 1) }, 0);
    let panel_id = disc.entry_by_name("MAP5_S01.BIN").unwrap().id;
    let panel_bytes = disc
        .read_entry_range(panel_id, 0x800f0158 - 0x800c9578, 24)
        .unwrap();
    assert_eq!(decode_panel_cells(&panel_bytes, 12).unwrap().len(), 12);
    let credits_id = disc.entry_by_name("STF_ROLL.BIN").unwrap().id;
    let endings = disc
        .read_entry_range(credits_id, 0x801e5558 - 0x801e2600, 36)
        .unwrap();
    assert_eq!(
        unsafe { sh_items_probe_ending_table(endings.as_ptr(), endings.len()) },
        0
    );
    for bytes in endings.as_chunks::<6>().0.iter() {
        assert!(EndingParams::decode(bytes).unwrap().duration > 0);
    }
    let image = disc.read_entry(credits_id).unwrap();
    let mut wire = [0u8; 28];
    wire[12..16].copy_from_slice(&0x801e5c24u32.to_le_bytes());
    wire[16..20].copy_from_slice(&0x801e5e24u32.to_le_bytes());
    let mut credits = CreditState::decode(&wire, &image, 0x801e2600).unwrap();
    assert_eq!(credits.widths.len(), 256);
    assert_eq!(credits.colors.len(), 7);
    let native = credits.native();
    assert!(!native.widths.is_null());
    let body_id = disc.entry_by_name("BODYPROG.BIN").unwrap().id;
    let image = disc.read_entry(body_id).unwrap();
    let table = encrypted_data_leaf(&image, 0x800aedbc - 0x80024b60, 24 * 12).unwrap();
    for (index, wire) in table.as_chunks::<12>().0.iter().enumerate() {
        let decoded = MapMarkings::decode_bodyprog(wire, &image, 0x80024b60)
            .unwrap_or_else(|error| panic!("marker row {index}: {error}"));
        let native = decoded.native();
        assert_eq!(
            native.table_lengths(),
            (decoded.atlas.len(), decoded.entries.len())
        );
    }
}
