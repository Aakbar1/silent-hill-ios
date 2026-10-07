// SPDX-License-Identifier: GPL-3.0-only
use psxdisc::{DiscImage, Error, ImageFormat, IsoFileSystem, SectorKind, SectorReader, verify};
use std::{fs, io::Cursor, path::PathBuf};

fn raw(kind: SectorKind, byte: u8) -> Vec<u8> {
    let mut bytes = vec![0; 2352];
    bytes[1..11].fill(255);
    bytes[15] = if kind == SectorKind::Mode1 { 1 } else { 2 };
    match kind {
        SectorKind::Mode1 => bytes[16..2064].fill(byte),
        SectorKind::Mode2Form1 => bytes[24..2072].fill(byte),
        SectorKind::Mode2Form2 => {
            bytes[18] = 0x64;
            bytes[22] = 0x64;
            bytes[24..2348].fill(byte);
        }
        _ => unreachable!(),
    }
    bytes
}

fn both16(value: u16) -> [u8; 4] {
    let mut out = [0; 4];
    out[..2].copy_from_slice(&value.to_le_bytes());
    out[2..].copy_from_slice(&value.to_be_bytes());
    out
}
fn both32(value: u32) -> [u8; 8] {
    let mut out = [0; 8];
    out[..4].copy_from_slice(&value.to_le_bytes());
    out[4..].copy_from_slice(&value.to_be_bytes());
    out
}
fn record(name: &[u8], extent: u32, size: u32, directory: bool) -> Vec<u8> {
    let len = (33 + name.len()).next_multiple_of(2);
    let mut out = vec![0; len];
    out[0] = len as u8;
    out[2..10].copy_from_slice(&both32(extent));
    out[10..18].copy_from_slice(&both32(size));
    out[25] = if directory { 2 } else { 0 };
    out[28..32].copy_from_slice(&both16(1));
    out[32] = name.len() as u8;
    out[33..33 + name.len()].copy_from_slice(name);
    out
}

fn iso() -> Vec<u8> {
    let mut bytes = vec![0; 32 * 2048];
    let pvd = &mut bytes[16 * 2048..17 * 2048];
    pvd[0] = 1;
    pvd[1..6].copy_from_slice(b"CD001");
    pvd[6] = 1;
    pvd[80..88].copy_from_slice(&both32(32));
    pvd[120..124].copy_from_slice(&both16(1));
    pvd[124..128].copy_from_slice(&both16(1));
    pvd[128..132].copy_from_slice(&both16(2048));
    pvd[156..190].copy_from_slice(&record(&[0], 20, 4096, true));
    let terminator = &mut bytes[17 * 2048..18 * 2048];
    terminator[0] = 255;
    terminator[1..6].copy_from_slice(b"CD001");
    terminator[6] = 1;
    let root = [
        record(&[0], 20, 4096, true),
        record(&[1], 20, 4096, true),
        record(b"SYSTEM.CNF;1", 24, 68, false),
        record(b"SLUS_007.07;1", 25, 3000, false),
        record(b"SILENT.;1", 27, 2048, false),
        record(b"DIR", 29, 2048, true),
    ]
    .concat();
    bytes[20 * 2048..20 * 2048 + root.len()].copy_from_slice(&root);
    let hill = record(b"HILL.;1", 28, 2048, false);
    bytes[21 * 2048..21 * 2048 + hill.len()].copy_from_slice(&hill);
    bytes[24 * 2048..24 * 2048 + 68].fill(0xc1);
    bytes[25 * 2048..26 * 2048].fill(0x51);
    bytes[26 * 2048..27 * 2048].fill(0x52);
    let sub = record(b"NESTED.TXT;1", 30, 9, false);
    bytes[29 * 2048..29 * 2048 + sub.len()].copy_from_slice(&sub);
    bytes[30 * 2048..30 * 2048 + 9].copy_from_slice(b"synthetic");
    bytes
}

fn to_raw(iso: &[u8], kind: SectorKind) -> Vec<u8> {
    let mut result = Vec::new();
    for data in iso.as_chunks::<2048>().0 {
        let mut sector = raw(kind, 0);
        let offset = if kind == SectorKind::Mode1 { 16 } else { 24 };
        sector[offset..offset + 2048].copy_from_slice(data);
        result.extend(sector);
    }
    result
}

fn temp(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("psxdisc-{}-{name}", std::process::id()))
}

#[test]
fn mixed_raw_sectors_and_ranges_match_original() {
    let bytes = [
        raw(SectorKind::Mode1, 11),
        raw(SectorKind::Mode2Form1, 22),
        raw(SectorKind::Mode2Form2, 33),
    ]
    .concat();
    let mut image = DiscImage::detect(Cursor::new(bytes.clone())).unwrap();
    assert_eq!(image.format(), ImageFormat::Raw2352);
    let sectors = image.read_sectors(0, 3).unwrap();
    for (i, sector) in sectors.iter().enumerate() {
        assert_eq!(sector.raw().unwrap(), &bytes[i * 2352..(i + 1) * 2352]);
    }
    assert_eq!(sectors[0].logical_data().unwrap(), [11; 2048]);
    assert_eq!(sectors[1].logical_data().unwrap(), [22; 2048]);
    assert_eq!(sectors[2].kind, SectorKind::Mode2Form2);
    assert_eq!(sectors[2].xa_raw().unwrap().len(), 2336);
    assert_eq!(sectors[2].xa_data().unwrap(), [33; 2324]);
    assert_eq!(
        sectors[2].xa_subheader().unwrap(),
        [0, 0, 0x64, 0, 0, 0, 0x64, 0]
    );
    assert!(sectors[2].logical_data().is_err());
    assert_eq!(image.read_image_range(1, 2).unwrap(), bytes[2352..]);
    assert!(image.read_sector(3).is_err());
    assert!(image.read_sectors(u32::MAX, 1).is_err());
    assert!(image.read_sectors(2, u32::MAX).is_err());
    assert!(image.read_sectors(3, 0).unwrap().is_empty());
    let mut xa = Vec::new();
    image.copy_xa(1, 2320, 48, &mut xa).unwrap();
    let expected = [
        bytes[2352 + 16..2 * 2352].to_vec(),
        bytes[2 * 2352 + 16..3 * 2352].to_vec(),
    ]
    .concat();
    assert_eq!(xa, expected[2320..2368]);
    assert!(image.copy_xa(0, 0, 1, &mut Vec::new()).is_err());
    assert!(image.copy_xa(2, 0, 2337, &mut Vec::new()).is_err());
}

#[test]
fn unaligned_logical_range_crosses_sector_boundary() {
    let mut image = DiscImage::new(
        Cursor::new([raw(SectorKind::Mode1, 1), raw(SectorKind::Mode2Form1, 2)].concat()),
        ImageFormat::Raw2352,
    )
    .unwrap();
    let mut out = Vec::new();
    assert_eq!(image.copy_logical(0, 2040, 16, &mut out).unwrap(), 16);
    assert_eq!(out, [[1; 8], [2; 8]].concat());
    assert!(image.read_logical(1, 2049).is_err());
}

#[test]
fn iso_directories_padding_versions_and_nested_paths_in_all_formats() {
    for (bytes, format) in [
        (iso(), ImageFormat::Iso2048),
        (to_raw(&iso(), SectorKind::Mode1), ImageFormat::Raw2352),
        (to_raw(&iso(), SectorKind::Mode2Form1), ImageFormat::Raw2352),
    ] {
        let mut image = DiscImage::detect(Cursor::new(bytes)).unwrap();
        assert_eq!(image.format(), format);
        let fs = IsoFileSystem::open(&mut image).unwrap();
        let entries = fs.read_directory(&mut image, &fs.root).unwrap();
        assert_eq!(entries.len(), 5);
        for name in ["SYSTEM.CNF", "SLUS_007.07", "SILENT.", "HILL."] {
            assert!(fs.find(&mut image, name).is_ok());
        }
        assert_eq!(
            fs.read_file(&mut image, "/dir/nested.txt;1").unwrap(),
            b"synthetic"
        );
        assert_eq!(fs.read_file(&mut image, "SYSTEM.CNF").unwrap(), [0xc1; 68]);
        let exe = fs.read_file(&mut image, "SLUS_007.07").unwrap();
        assert_eq!(exe.len(), 3000);
        assert_eq!(exe[..2048], [0x51; 2048]);
        assert_eq!(exe[2048..], [0x52; 952]);
        assert!(fs.find(&mut image, "../SYSTEM.CNF").is_err());
        if format == ImageFormat::Iso2048 {
            assert!(image.read_sector(24).unwrap().raw().is_err());
        }
    }
}

#[test]
fn wrong_release_and_lost_xa_are_explicit_errors() {
    let mut image = DiscImage::detect(Cursor::new(iso())).unwrap();
    let error = verify(&mut image).unwrap_err();
    assert!(matches!(error, Error::WrongRelease { .. }));
    assert!(error.to_string().contains("US v1.1"));
    let mut bytes = to_raw(&iso(), SectorKind::Mode2Form1);
    bytes[28 * 2352..29 * 2352].copy_from_slice(&raw(SectorKind::Mode2Form2, 0x99));
    let mut image = DiscImage::detect(Cursor::new(bytes)).unwrap();
    let fs = IsoFileSystem::open(&mut image).unwrap();
    assert!(fs.read_file(&mut image, "HILL.").is_err());
}

#[test]
fn malformed_and_truncated_sectors_reject_cleanly() {
    assert!(DiscImage::new(Cursor::new(vec![0; 2353]), ImageFormat::Raw2352).is_err());
    assert!(DiscImage::new(Cursor::new(Vec::new()), ImageFormat::Iso2048).is_err());
    assert!(DiscImage::detect(Cursor::new(vec![0; 4096])).is_err());
    for (index, value) in [(0, 4), (15, 3), (20, 1)] {
        let mut bytes = raw(SectorKind::Mode2Form1, 0);
        bytes[index] = value;
        let mut image = DiscImage::new(Cursor::new(bytes), ImageFormat::Raw2352).unwrap();
        assert!(image.read_sector(0).is_err());
    }
}

#[test]
fn malformed_iso_endianness_extents_and_directory_lengths_reject() {
    let mut bytes = iso();
    bytes[16 * 2048 + 84] ^= 1;
    let mut image = DiscImage::new(Cursor::new(bytes), ImageFormat::Iso2048).unwrap();
    assert!(IsoFileSystem::open(&mut image).is_err());
    for (index, value) in [
        (20 * 2048, 2),
        (20 * 2048 + 32, 255),
        (20 * 2048 + 25, 0x80),
        (20 * 2048 + 26, 1),
    ] {
        let mut bytes = iso();
        bytes[index] = value;
        let mut image = DiscImage::new(Cursor::new(bytes), ImageFormat::Iso2048).unwrap();
        let fs = IsoFileSystem::open(&mut image).unwrap();
        assert!(fs.read_directory(&mut image, &fs.root).is_err());
    }
    let mut bytes = iso();
    bytes[16 * 2048 + 158..16 * 2048 + 166].copy_from_slice(&both32(31));
    let mut image = DiscImage::new(Cursor::new(bytes), ImageFormat::Iso2048).unwrap();
    assert!(IsoFileSystem::open(&mut image).is_err());
}

#[test]
fn cue_opens_relative_quoted_file_and_omits_stored_pregap() {
    let directory = temp("cue");
    fs::create_dir_all(&directory).unwrap();
    let bin = directory.join("own disc.bin");
    let cue = directory.join("disc.cue");
    let source = [vec![0; 150 * 2352], to_raw(&iso(), SectorKind::Mode2Form1)].concat();
    fs::write(&bin, &source).unwrap();
    fs::write(
        &cue,
        "FILE \"own disc.bin\" BINARY\nTRACK 01 MODE2/2352\nINDEX 00 00:00:00\nINDEX 01 00:02:00",
    )
    .unwrap();
    let mut image = DiscImage::open(&cue).unwrap();
    assert_eq!(image.sector_count(), 32);
    assert_eq!(
        image.read_sector(16).unwrap().raw().unwrap(),
        &source[(150 + 16) * 2352..(150 + 17) * 2352]
    );
    assert!(IsoFileSystem::open(&mut image).is_ok());
    fs::remove_file(bin).unwrap();
    fs::remove_file(cue).unwrap();
    fs::remove_dir(directory).unwrap();
}
