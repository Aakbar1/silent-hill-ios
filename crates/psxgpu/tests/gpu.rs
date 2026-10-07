// SPDX-License-Identifier: GPL-3.0-only
use psxgpu::{capture, rgb888, scanout, Display, Processor, Rect, Renderer, SoftwareRenderer};
fn xy(x: i32, y: i32) -> u32 {
    (x as u32 & 65535) | ((y as u32 & 65535) << 16)
}
fn gpu() -> Processor<SoftwareRenderer> {
    let mut g = Processor::new(SoftwareRenderer::new());
    g.gp0_words(&[0xe3000000, 0xe407ffff]).unwrap();
    g
}
fn pixel(g: &Processor<SoftwareRenderer>, x: usize, y: usize) -> u16 {
    g.renderer().vram()[y * 1024 + x]
}
#[test]
fn quad_top_left_rule_without_diagonal_double_blend() {
    let mut g = gpu();
    g.gp0_words(&[0x2a0000ff, xy(0, 0), xy(8, 0), xy(0, 8), xy(8, 8)])
        .unwrap();
    assert_eq!(g.renderer().vram().iter().filter(|&&v| v != 0).count(), 64);
    for y in 0..8 {
        for x in 0..8 {
            assert_eq!(pixel(&g, x, y), 15);
        }
    }
    assert_eq!(pixel(&g, 8, 8), 0);
}
#[test]
fn triangles_both_windings_and_rejected_oversize() {
    let mut a = gpu();
    let mut b = gpu();
    a.gp0_words(&[0x200000ff, xy(0, 0), xy(8, 0), xy(0, 8)])
        .unwrap();
    b.gp0_words(&[0x200000ff, xy(0, 0), xy(0, 8), xy(8, 0)])
        .unwrap();
    assert_eq!(a.renderer().vram(), b.renderer().vram());
    assert_eq!(a.renderer().vram().iter().filter(|&&v| v != 0).count(), 36);
    let before = a.renderer().vram().to_vec();
    a.gp0_words(&[0x2000ffff, xy(0, 0), xy(20, 512), xy(30, 0)])
        .unwrap();
    assert_eq!(a.renderer().vram(), before);
}
#[test]
fn all_four_blend_modes_saturate() {
    for (mode, expected) in [(0, 14), (1, 28), (2, 12), (3, 22)] {
        let mut g = gpu();
        g.renderer_mut()
            .upload(Rect::new(0, 0, 1, 1), &[20], 0)
            .unwrap();
        g.gp0_words(&[0xe1000000 | (mode << 5), 0x6a000040, 0])
            .unwrap();
        assert_eq!(pixel(&g, 0, 0), expected);
    }
    let mut g = gpu();
    g.gp0_words(&[0xe1000020, 0x6a0000ff, 0, 0x6a0000ff, 0])
        .unwrap();
    assert_eq!(pixel(&g, 0, 0), 31);
}
#[test]
fn indexed_textures_clut_transparency_and_bit15() {
    for (page, packed) in [(0x0a, 0x0321), (0x8a, 0x0201)] {
        let mut g = gpu();
        g.renderer_mut()
            .upload(Rect::new(640, 0, 1, 1), &[packed], 0)
            .unwrap();
        g.renderer_mut()
            .upload(Rect::new(0, 480, 4, 1), &[0, 31, 0x83e0, 0x7c00], 0)
            .unwrap();
        g.gp0_words(&[
            0xe1000000 | page,
            0x65000000,
            xy(10, 10),
            (480 << 22),
            xy(3, 1),
        ])
        .unwrap();
        assert_eq!(pixel(&g, 10, 10), 31);
        assert_eq!(pixel(&g, 11, 10), 0x83e0);
        if page == 0x0a {
            assert_eq!(pixel(&g, 12, 10), 0x7c00);
        }
    }
}
#[test]
fn texture_stp_gates_semitransparency() {
    let mut g = gpu();
    g.renderer_mut()
        .upload(Rect::new(640, 0, 3, 1), &[0, 31, 0x801f], 0)
        .unwrap();
    g.renderer_mut()
        .upload(Rect::new(10, 10, 3, 1), &[8; 3], 0)
        .unwrap();
    g.gp0_words(&[0xe100010a, 0x67000000, xy(10, 10), 0, xy(3, 1)])
        .unwrap();
    assert_eq!(pixel(&g, 10, 10), 8);
    assert_eq!(pixel(&g, 11, 10), 31);
    assert_eq!(pixel(&g, 12, 10), 0x8013);
}
#[test]
fn texture_window_flip_and_modulation() {
    let mut g = gpu();
    g.renderer_mut()
        .upload(Rect::new(640, 0, 32, 1), &(0..32).collect::<Vec<u16>>(), 0)
        .unwrap();
    g.gp0_words(&[0xe100110a, 0x65808080, xy(10, 10), 3, xy(3, 1)])
        .unwrap();
    assert_eq!(pixel(&g, 10, 10), 3);
    assert_eq!(pixel(&g, 11, 10), 2);
    assert_eq!(pixel(&g, 12, 10), 1);
    g.gp0_words(&[
        0xe100010a,
        0xe2000001 | (1 << 10),
        0x65808080,
        xy(20, 10),
        1,
        xy(1, 1),
    ])
    .unwrap();
    assert_eq!(pixel(&g, 20, 10), 9);
    g.gp0_words(&[0xe2000000, 0x64404040, xy(30, 10), 16, xy(1, 1)])
        .unwrap();
    assert_eq!(pixel(&g, 30, 10), 8);
}
#[test]
fn dithering_applies_to_lines_and_gouraud_but_not_tiles() {
    let mut g = gpu();
    g.gp0_words(&[0xe1000200, 0x60070707, 0, xy(4, 4)]).unwrap();
    assert_eq!(pixel(&g, 1, 1), 0);
    g.gp0_words(&[0x40070707, xy(0, 8), xy(3, 8)]).unwrap();
    assert_eq!(pixel(&g, 3, 8), 0x421);
    g.gp0_words(&[
        0x30070707,
        xy(0, 16),
        0x070707,
        xy(8, 16),
        0x070707,
        xy(0, 24),
    ])
    .unwrap();
    assert_eq!(pixel(&g, 3, 16), 0x421);
}
#[test]
fn draw_area_offset_signed11_and_mask() {
    let mut g = gpu();
    g.gp0_words(&[
        0xe3000000 | 4 | (4 << 10),
        0xe4000000 | 7 | (7 << 10),
        0xe5000000 | 4 | (4 << 11),
        0xe6000001,
        0x600000ff,
        0,
        xy(8, 8),
    ])
    .unwrap();
    assert_eq!(pixel(&g, 4, 4), 0x801f);
    assert_eq!(pixel(&g, 8, 8), 0);
    g.gp0_words(&[0xe6000002, 0x6000ff00, 0, xy(8, 8)]).unwrap();
    assert_eq!(pixel(&g, 4, 4), 0x801f);
    g.gp0_words(&[0xe6000000, 0xe50007ff, 0x680000ff, xy(5, 5)])
        .unwrap();
    assert_eq!(pixel(&g, 4, 5), 31);
}
#[test]
fn upload_download_odd_word_count_and_wrap() {
    let mut g = gpu();
    g.gp0_words(&[0xa0000000, xy(1023, 511), xy(3, 1), 0x22221111])
        .unwrap();
    assert!(!g.idle());
    g.gp0(0xdead3333).unwrap();
    assert!(g.idle());
    assert_eq!(pixel(&g, 1023, 511), 0x1111);
    assert_eq!(pixel(&g, 0, 511), 0x2222);
    g.gp0_words(&[0xc0000000, xy(1023, 511), xy(3, 1)]).unwrap();
    assert_ne!(g.status() & (1 << 27), 0);
    assert_eq!(g.read_gp0(), 0x22221111);
    assert_eq!(g.read_gp0(), 0x3333);
    assert_eq!(g.status() & (1 << 27), 0);
}
#[test]
fn vram_copy_burst_overlap_mask_and_fill_alignment() {
    let mut g = gpu();
    g.renderer_mut()
        .upload(Rect::new(0, 0, 4, 1), &[1, 2, 3, 4], 0)
        .unwrap();
    g.gp0_words(&[0x80000000, 0, xy(1, 0), xy(3, 1)]).unwrap();
    assert_eq!(&g.renderer().vram()[..4], &[1, 1, 2, 3]);
    g.gp0_words(&[0xe6000003, 0x020000ff, xy(19, 2), xy(1, 1)])
        .unwrap();
    for x in 16..32 {
        assert_eq!(pixel(&g, x, 2), 31);
    }
    assert_eq!(pixel(&g, 32, 2), 0);
}
#[test]
fn lines_endpoints_zero_length_and_polyline_fifo() {
    let mut g = gpu();
    g.gp0_words(&[
        0x400000ff,
        xy(1, 1),
        xy(4, 1),
        0x4000ff00,
        xy(6, 6),
        xy(6, 6),
    ])
    .unwrap();
    for x in 1..=4 {
        assert_eq!(pixel(&g, x, 1), 31);
    }
    assert_eq!(pixel(&g, 6, 6), 0x3e0);
    g.gp0_words(&[0x480000ff, xy(10, 10), xy(14, 10), xy(14, 14), 0x50005000])
        .unwrap();
    assert!(g.idle());
    g.gp0_words(&[
        0x580000ff,
        xy(20, 20),
        0x0000ff00,
        xy(24, 20),
        0x00ff0000,
        xy(24, 24),
        0x50005000,
    ])
    .unwrap();
    assert!(g.idle());
    assert_eq!(pixel(&g, 24, 24), 0x7c00);
}
#[test]
fn ot_order_empty_links_bounds_and_cycles() {
    let mut ram = vec![0u8; 64];
    for (i, w) in [
        0x03000010,
        0x680000ff,
        xy(0, 0),
        0,
        0x03ffffff,
        0x6800ff00,
        xy(0, 0),
        0,
    ]
    .into_iter()
    .enumerate()
    {
        ram[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
    }
    // Fixed 1x1 tile uses two words, so packet header count must be two.
    ram[..4].copy_from_slice(&0x02000010u32.to_le_bytes());
    ram[16..20].copy_from_slice(&0x02ffffffu32.to_le_bytes());
    let mut g = gpu();
    assert_eq!(g.walk_ot(&ram, 0, 10).unwrap(), 2);
    assert_eq!(pixel(&g, 0, 0), 0x3e0);
    assert!(g.walk_ot(&ram, 0, 1).is_err());
    assert!(g.walk_ot(&ram, 1, 10).is_err());
    assert!(g.walk_ot(&ram, 64, 10).is_err());
    ram[..4].copy_from_slice(&0u32.to_le_bytes());
    assert!(g.walk_ot(&ram, 0, 10).is_err());
}
#[test]
fn gp1_reset_irq_info_and_display() {
    let mut g = gpu();
    g.gp0(0x1f000000).unwrap();
    assert_ne!(g.status() & (1 << 24), 0);
    g.gp1(0x02000000).unwrap();
    assert_eq!(g.status() & (1 << 24), 0);
    g.gp0(0xe5000000 | 2047).unwrap();
    g.gp1(0x10000005).unwrap();
    assert_eq!(g.read_gp0(), 2047);
    g.gp0(0x200000ff).unwrap();
    g.gp1(0x01000000).unwrap();
    assert!(g.idle());
    g.renderer_mut()
        .upload(Rect::new(0, 0, 1, 1), &[31], 0)
        .unwrap();
    g.gp1(0).unwrap();
    assert_eq!(g.status(), 0x14802000);
    assert_eq!(pixel(&g, 0, 0), 31);
    assert_eq!(g.display.dimensions(), [256, 240]);
}
#[test]
fn scanout_15_and_24_bit_and_disabled() {
    let mut vram = vec![0; psxgpu::VRAM_WORDS];
    vram[0] = 31;
    let mut d = Display {
        disabled: false,
        ..Default::default()
    };
    assert_eq!(&scanout(&vram, d, false).unwrap()[..4], &[255, 0, 0, 255]);
    assert_eq!(rgb888(0x7c00), [0, 0, 255]);
    vram[0] = 0x2211;
    vram[1] = 0x4433;
    d.mode = 16;
    assert_eq!(
        &scanout(&vram, d, false).unwrap()[..4],
        &[0x11, 0x22, 0x33, 255]
    );
    d.disabled = true;
    assert_eq!(&scanout(&vram, d, false).unwrap()[..4], &[0, 0, 0, 255]);
}
#[test]
fn capture_roundtrip_and_malformed_records() {
    let mut w = capture::CaptureWriter::new(Vec::new()).unwrap();
    w.gp0(&[0xe3000000, 0xe407ffff, 0x680000ff, 0]).unwrap();
    w.gp1(&[0x03000000]).unwrap();
    w.frame().unwrap();
    let bytes = w.into_inner();
    let mut g = gpu();
    let stats = capture::replay(&mut bytes.as_slice(), &mut g, 1024).unwrap();
    assert_eq!(stats.frames, 1);
    assert_eq!(pixel(&g, 0, 0), 31);
    for n in [0, 7, 9, bytes.len() - 1] {
        assert!(capture::replay(&mut &bytes[..n], &mut gpu(), 1024).is_err());
    }
    assert!(capture::replay(&mut bytes.as_slice(), &mut gpu(), 10).is_err());
    let mut bad = bytes;
    bad[8] = 99;
    assert!(capture::replay(&mut bad.as_slice(), &mut gpu(), 1024).is_err());
}
#[test]
fn precise_metadata_and_widescreen_contract() {
    let mut g = gpu();
    assert!(g
        .polygon_with_positions(
            &[0x200000ff, 0, xy(8, 0), xy(0, 8)],
            &[[0.25, 0.25], [8.25, 0.25], [0.25, 8.25]]
        )
        .is_ok());
    assert!(g
        .polygon_with_positions(&[0x200000ff, 0, xy(8, 0), xy(0, 8)], &[[f32::NAN, 0.0]; 3])
        .is_err());
    g.gp0_words(&[0xe3000000 | 100, 0xe4000000 | 399 | (239 << 10)])
        .unwrap();
    g.set_widescreen(true);
    g.gp0_words(&[0x6800ff00, xy(50, 0)]).unwrap();
    assert_eq!(pixel(&g, 50, 0), 0x3e0);
}

#[test]
fn aborted_upload_retains_data_and_odd_read_includes_trailing_halfword() {
    let mut g = gpu();
    g.gp0_words(&[0xa0000000, 0, xy(5, 1), 0x22221111]).unwrap();
    g.gp1(0x01000000).unwrap();
    assert!(g.idle());
    assert_eq!(&g.renderer().vram()[..3], &[0x1111, 0x2222, 0]);
    g.renderer_mut()
        .upload(Rect::new(2, 0, 2, 1), &[0x3333, 0xabcd], 0)
        .unwrap();
    g.gp0_words(&[0xc0000000, 0, xy(3, 1)]).unwrap();
    assert_eq!(g.read_gp0(), 0x22221111);
    assert_eq!(g.read_gp0(), 0xabcd3333);
}

#[test]
fn line_halfway_rounding_and_polyline_joints_are_hardware_style() {
    let mut g = gpu();
    g.gp0_words(&[0x400000ff, xy(0, 0), xy(2, 4)]).unwrap();
    assert_eq!(pixel(&g, 0, 1), 31);
    assert_eq!(pixel(&g, 1, 1), 0);
    g.gp0_words(&[0x4a0000ff, xy(10, 10), xy(14, 10), xy(14, 14), 0x50005000])
        .unwrap();
    assert_eq!(pixel(&g, 14, 10), 23); // The shared endpoint is rendered by both segments.
}

#[test]
fn capture_comparison_checks_frames_before_later_clear_hides_a_mismatch() {
    let mut writer = capture::CaptureWriter::new(Vec::new()).unwrap();
    writer.frame().unwrap();
    writer.gp0(&[0x02000000, 0, xy(16, 16)]).unwrap();
    writer.frame().unwrap();
    let bytes = writer.into_inner();
    let mut a = gpu();
    let mut b = gpu();
    a.renderer_mut()
        .upload(Rect::new(1, 1, 1, 1), &[31], 0)
        .unwrap();
    let error = capture::compare_replay(&mut bytes.as_slice(), &mut a, &mut b, 4096).unwrap_err();
    assert!(error.0.contains("frame 0"));
    let stats =
        capture::compare_replay(&mut bytes.as_slice(), &mut gpu(), &mut gpu(), 4096).unwrap();
    assert_eq!(stats.frames, 2);
}
