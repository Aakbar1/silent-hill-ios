// SPDX-License-Identifier: GPL-3.0-only
//! These tests never extract/write game data; missing private images skip with a message.
use psxdisc::{Archive, DiscImage, GameDisc, IsoFileSystem, Release, SectorReader, verify};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
};

fn owned_disc() -> Option<PathBuf> {
    let path = std::env::var_os("PSXDISC_TEST_IMAGE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from("C:/Claude Projects/Silent Hill iOS/private/disc/Silent Hill (USA).bin")
        });
    if !path.is_file() {
        eprintln!("SKIP: player-owned disc not present: {}", path.display());
        return None;
    }
    Some(path)
}

#[test]
fn owned_disc_verifies_and_every_sector_matches_image() {
    let Some(path) = owned_disc() else {
        return;
    };
    let mut image = DiscImage::open(&path).unwrap();
    assert_eq!(verify(&mut image).unwrap(), Release::Us11);
    let mut independent = File::open(path).unwrap();
    for start in (0..image.sector_count()).step_by(512) {
        let count = (image.sector_count() - start).min(512);
        let bytes = image.read_image_range(start, count).unwrap();
        let mut expected = vec![0; count as usize * 2352];
        independent.read_exact(&mut expected).unwrap();
        assert_eq!(bytes, expected, "raw range starting at LBA {start}");
        for (i, raw) in expected.as_chunks::<2352>().0.iter().enumerate() {
            assert_eq!(
                image.read_sector(start + i as u32).unwrap().raw().unwrap(),
                raw
            );
        }
    }
    let fs = IsoFileSystem::open(&mut image).unwrap();
    assert_eq!(fs.read_directory(&mut image, &fs.root).unwrap().len(), 4);
}

#[test]
fn owned_disc_all_2074_entries_match_table_sizes_names_and_source_bytes() {
    let Some(path) = owned_disc() else {
        return;
    };
    let mut game = GameDisc::open(&path).unwrap();
    assert_eq!(game.entries().len(), 2074);
    let entries = game.entries().to_vec();
    assert_eq!(
        entries
            .iter()
            .filter(|e| e.archive == Archive::Hill)
            .count(),
        30
    );
    assert_eq!(
        entries.iter().map(|e| u64::from(e.table_size)).sum::<u64>(),
        82_799_872
    );
    let executable = game.read_disc_file("SLUS_007.07").unwrap();
    let mut independent = File::open(path).unwrap();
    let mut total = 0;
    for entry in &entries {
        let offset = 0xb91c + entry.id as usize * 12;
        let metadata = u32::from_le_bytes(executable[offset..offset + 4].try_into().unwrap());
        assert_eq!(entry.start_sector, metadata & 0x7ffff);
        assert_eq!(entry.table_size, ((metadata >> 19) & 0xfff) * 256);
        let bytes = game.read_entry(entry.id).unwrap();
        assert_eq!(bytes.len() as u64, entry.size, "{}", entry.path);
        independent
            .seek(SeekFrom::Start(u64::from(entry.start_sector) * 2352))
            .unwrap();
        let mut raw = vec![0; entry.sector_count as usize * 2352];
        independent.read_exact(&mut raw).unwrap();
        let mut expected = Vec::new();
        for sector in raw.as_chunks::<2352>().0 {
            if entry.archive == Archive::Hill {
                expected.extend_from_slice(&sector[16..]);
            } else {
                let offset = if sector[15] == 1 { 16 } else { 24 };
                expected.extend_from_slice(&sector[offset..offset + 2048]);
            }
        }
        expected.truncate(entry.size as usize);
        assert_eq!(bytes, expected, "{} (ID {})", entry.path, entry.id);
        if entry.size > 0 {
            let skip = entry.size.min(31);
            let count = (entry.size - skip).min(4097);
            assert_eq!(
                game.read_entry_range(entry.id, skip, count).unwrap(),
                bytes[skip as usize..(skip + count) as usize]
            );
        }
        assert_eq!(game.entry_by_name(&entry.path).unwrap().id, entry.id);
        if entry.archive == Archive::Silent {
            assert_eq!(entry.size, u64::from(entry.table_size));
        } else {
            assert_eq!(entry.size, u64::from(entry.sector_count) * 2336);
            let mut first = Vec::new();
            game.copy_entry_raw_sectors(entry.id, 0, 1, &mut first)
                .unwrap();
            assert_eq!(first, raw[..2352]);
        }
        total += entry.size;
    }
    eprintln!(
        "PASS: {} entries; {total} readable bytes; 82799872 table-size bytes",
        entries.len()
    );
    for entry in entries.iter().take(10) {
        eprintln!("{} {}", entry.path, entry.size);
    }
}

#[test]
fn owned_disc_index_matches_version_matched_decomp_table() {
    let Some(path) = owned_disc() else {
        return;
    };
    let reference = PathBuf::from(
        "C:/Claude Projects/Silent Hill iOS/reference/silent-hill-decomp/src/main/filetable.c.USA.inc",
    );
    if !reference.is_file() {
        eprintln!("SKIP: optional read-only decomp table is absent");
        return;
    }
    let game = GameDisc::open(path).unwrap();
    let source = std::fs::read_to_string(reference).unwrap();
    let mut compared = 0;
    for line in source.lines() {
        let Some((prefix, body)) = line.split_once("*/ { ") else {
            continue;
        };
        let id: u32 = prefix.trim_start_matches("/*").trim().parse().unwrap();
        let fields: Vec<_> = body
            .split(", FN(")
            .next()
            .unwrap()
            .split(',')
            .map(str::trim)
            .collect();
        let lba = u32::from_str_radix(fields[0].trim_start_matches("0x"), 16).unwrap();
        let blocks: u16 = fields[1].parse().unwrap();
        let path = body.split_once("// ").unwrap().1.trim();
        let entry = game.entry(id).unwrap();
        assert_eq!(entry.start_sector, lba, "ID {id}");
        assert_eq!(entry.block_count, blocks, "ID {id}");
        assert_eq!(entry.path, path, "ID {id}");
        compared += 1;
    }
    assert_eq!(compared, 2074);
}
