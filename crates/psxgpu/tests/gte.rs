// SPDX-License-Identifier: GPL-3.0-only
use psxgpu::gte::Gte;
fn xy(x: i16, y: i16) -> u32 {
    u32::from(x as u16) | (u32::from(y as u16) << 16)
}
fn identity() -> Gte {
    let mut g = Gte::new();
    for base in [0, 8, 16] {
        g.write_control(base, 4096);
        g.write_control(base + 2, 4096);
        g.write_control(base + 4, 4096);
    }
    g.write_control(26, 256);
    g
}
#[test]
fn rtps_known_identity_projection() {
    let mut g = identity();
    g.write_data(0, xy(100, -50));
    g.write_data(1, 512);
    g.execute(0x180001).unwrap();
    assert_eq!(g.read_data(14), xy(50, -25));
    assert_eq!(g.read_data(19), 512);
    assert_eq!(g.read_data(25), 100);
    assert_eq!(g.read_data(26), (-50i32) as u32);
    assert_eq!(g.read_control(31), 0);
}
#[test]
fn rtpt_fifo_and_translation() {
    let mut g = identity();
    g.write_control(5, 10);
    g.write_control(7, 256);
    for i in 0..3u8 {
        g.write_data(i * 2, xy(i16::from(i) * 20, 10));
        g.write_data(i * 2 + 1, 256);
    }
    g.execute(0x280030).unwrap();
    assert_eq!(g.read_data(12), xy(5, 5));
    assert_eq!(g.read_data(13), xy(15, 5));
    assert_eq!(g.read_data(14), xy(25, 5));
    for i in 17..=19 {
        assert_eq!(g.read_data(i), 512);
    }
}
#[test]
fn nclip_signed_triangle_area() {
    let mut g = Gte::new();
    for (r, v) in [(12, xy(10, 10)), (13, xy(20, 10)), (14, xy(10, 30))] {
        g.write_data(r, v);
    }
    g.execute(6).unwrap();
    assert_eq!(g.read_data(24), 200);
    assert_eq!(g.read_control(31), 0);
    g.write_data(13, xy(10, 30));
    g.write_data(14, xy(20, 10));
    g.execute(6).unwrap();
    assert_eq!(g.read_data(24), (-200i32) as u32);
}
#[test]
fn avsz3_avsz4_and_saturation() {
    let mut g = Gte::new();
    for i in 16..=19 {
        g.write_data(i, 1000);
    }
    g.write_control(29, 1365);
    g.write_control(30, 1024);
    g.execute(45).unwrap();
    assert_eq!(g.read_data(7), 999);
    assert_eq!(g.read_data(24), 4095000);
    g.execute(46).unwrap();
    assert_eq!(g.read_data(7), 1000);
    g.write_control(30, 0xffff);
    g.execute(46).unwrap();
    assert_eq!(g.read_data(7), 0);
    assert_eq!(g.read_control(31) & (1 << 18), 1 << 18);
}
#[test]
fn mvmva_matrix_vector_translation() {
    let mut g = identity();
    g.write_data(0, xy(-8, 9));
    g.write_data(1, 10);
    g.write_control(5, 11);
    g.write_control(6, 12);
    g.write_control(7, 13);
    g.execute(0x80012).unwrap();
    for (r, v) in [(9, 3), (10, 21), (11, 23)] {
        assert_eq!(g.read_data(r), v);
    }
    g.execute(0x86012).unwrap();
    assert_eq!(g.read_data(9), (-8i32) as u32);
}
#[test]
fn mvmva_far_color_hardware_bug() {
    let mut g = identity();
    g.write_data(0, xy(11, 12));
    g.write_data(1, 13);
    g.write_control(21, 100);
    g.write_control(22, 200);
    g.write_control(23, 300);
    g.execute(0x84012).unwrap();
    // FC + first column is evaluated for flags, then discarded by the hardware.
    assert_eq!(g.read_data(25), 0);
    assert_eq!(g.read_data(26), 12);
    assert_eq!(g.read_data(27), 13);
}
#[test]
fn rtps_divide_and_screen_saturation_flags() {
    let mut g = identity();
    g.write_data(0, xy(32767, -32768));
    g.write_data(1, 1);
    g.execute(0x80001).unwrap();
    assert_eq!(g.read_data(14), xy(1023, -1024));
    assert_eq!(g.read_control(31) & 0x80026000, 0x80026000);
}
#[test]
fn register_extension_color_conversion_and_lzc() {
    let mut g = Gte::new();
    g.write_data(9, 0xffff);
    assert_eq!(g.read_data(9), u32::MAX);
    g.write_data(19, 0xffff_ffff);
    assert_eq!(g.read_data(19), 65535);
    g.write_control(26, 0xffff);
    assert_eq!(g.read_control(26), u32::MAX);
    g.write_data(28, 0x7fff);
    for r in 9..=11 {
        assert_eq!(g.read_data(r), 3968);
    }
    assert_eq!(g.read_data(29), 0x7fff);
    for (input, count) in [
        (0, 32),
        (u32::MAX, 32),
        (0x00008000, 16),
        (0xffff0000, 16),
        (0x40000000, 1),
    ] {
        g.write_data(30, input);
        assert_eq!(g.read_data(31), count);
    }
    g.write_data(31, 123);
    assert_eq!(g.read_data(31), 1);
}
#[test]
fn sxy_push_alias_and_flag_writes() {
    let mut g = Gte::new();
    for i in 1..=3 {
        g.write_data(15, i);
    }
    assert_eq!(g.read_data(12), 1);
    assert_eq!(g.read_data(13), 2);
    assert_eq!(g.read_data(14), 3);
    assert_eq!(g.read_data(15), 3);
    g.write_control(31, u32::MAX);
    assert_eq!(g.read_control(31), 0xfffff000);
    g.write_control(31, 0x1000);
    assert_eq!(g.read_control(31), 0x1000);
}
#[test]
fn color_depth_cue_endpoints_and_fifo() {
    let mut g = identity();
    g.write_data(6, 0x34c08040);
    g.write_data(8, 0);
    g.execute(0x80010).unwrap();
    assert_eq!(g.read_data(22), 0x34c08040);
    g.write_control(21, 16 * 20);
    g.write_control(22, 16 * 30);
    g.write_control(23, 16 * 40);
    g.write_data(8, 4096);
    g.execute(0x80010).unwrap();
    assert_eq!(g.read_data(22), 0x34281e14);
    assert_eq!(g.read_data(21), 0x34c08040);
}
#[test]
fn normal_color_and_general_interpolation() {
    let mut g = identity();
    g.write_data(0, xy(1024, 2048));
    g.write_data(1, 3072);
    g.write_data(6, 0x22000000);
    g.execute(0x8041e).unwrap();
    assert_eq!(g.read_data(22), 0x22c08040);
    g.write_data(8, 2048);
    g.write_data(9, 64);
    g.write_data(10, 128);
    g.write_data(11, 256);
    g.execute(0x8003d).unwrap();
    assert_eq!(g.read_data(25), 32);
    assert_eq!(g.read_data(26), 64);
    assert_eq!(g.read_data(27), 128);
    g.execute(0x8003e).unwrap();
    assert_eq!(g.read_data(25), 48);
}
#[test]
fn squared_vector_and_outer_product() {
    let mut g = identity();
    g.write_data(9, 64);
    g.write_data(10, 128);
    g.write_data(11, 256);
    g.execute(0x80028).unwrap();
    assert_eq!(g.read_data(25), 1);
    assert_eq!(g.read_data(26), 4);
    assert_eq!(g.read_data(27), 16);
    g.write_data(9, 1);
    g.write_data(10, 2);
    g.write_data(11, 3);
    g.execute(0x8000c).unwrap();
    assert_eq!(g.read_data(25), 1);
    assert_eq!(g.read_data(26), (-2i32) as u32);
    assert_eq!(g.read_data(27), 1);
}
#[test]
fn precise_projection_is_presentation_only() {
    let mut g = identity();
    g.write_data(0, xy(1, 1));
    g.write_data(1, 768);
    let before = g.clone();
    let p = g.project_precise(0).unwrap();
    assert!((p[0] - 1.0 / 3.0).abs() < 0.000001);
    assert_eq!(g, before);
    g.execute(0x80001).unwrap();
    assert_eq!(g.read_data(14), 0);
}
#[test]
fn undefined_command_preserves_state() {
    let mut g = identity();
    let before = g.clone();
    assert!(g.execute(0x2b).is_err());
    assert_eq!(g, before);
}

#[test]
fn divider_matches_independent_unr_formula_for_all_16_bit_denominators() {
    let mut g = identity();
    g.write_control(27, 1);
    for denominator in 1..=65535u32 {
        let h = (denominator.wrapping_mul(31337).wrapping_add(97)) & 65535;
        g.write_control(26, h);
        g.write_control(7, denominator);
        g.execute(0x80001).unwrap();
        let expected = if h >= denominator * 2 {
            131071
        } else {
            let shift = denominator.leading_zeros() - 16;
            let d = denominator << shift;
            let index = (d - 0x7fc0) >> 7;
            let seed = (0x40000 / (index + 0x100))
                .div_ceil(2)
                .saturating_sub(0x101)
                + 0x101;
            let r1 = (0x2000080 - d * seed) >> 8;
            let r2 = (0x80 + r1 * seed) >> 8;
            (((u64::from(h << shift) * u64::from(r2) + 0x8000) >> 16) as u32).min(131071)
        };
        assert_eq!(g.read_data(24), expected, "H={h} SZ3={denominator}");
        assert_eq!(
            g.read_control(31) & (1 << 17),
            if h >= denominator * 2 { 1 << 17 } else { 0 }
        );
    }
}

#[test]
fn all_normal_color_commands_preserve_code_and_expected_fifos() {
    for (op, expected) in [
        (0x1e, 0x22c08040),
        (0x20, 0x22c08040),
        (0x1b, 0x22604020),
        (0x3f, 0x22604020),
        (0x13, 0x22604020),
        (0x16, 0x22604020),
    ] {
        let mut g = identity();
        for i in 0..3 {
            g.write_data(i * 2, xy(1024, 2048));
            g.write_data(i * 2 + 1, 3072);
        }
        g.write_data(6, 0x22808080);
        g.write_data(8, 0);
        g.execute(0x80400 | op).unwrap();
        assert_eq!(g.read_data(22), expected, "opcode={op:x}");
        if [0x20, 0x3f, 0x16].contains(&op) {
            for r in 20..=22 {
                assert_eq!(g.read_data(r), expected);
            }
        }
    }
}

#[test]
fn mac44_overflow_is_wrapped_and_flagged_at_each_addition() {
    for translation in [i32::MAX, i32::MIN] {
        for vector in [32767i16, -32768] {
            let mut g = identity();
            g.write_control(0, 0x7fff7fff);
            g.write_control(1, 0x00007fff);
            g.write_control(5, translation as u32);
            g.write_data(0, xy(vector, vector));
            g.write_data(1, vector as u16 as u32);
            let mut mac = i64::from(translation) * 4096;
            let mut flags = 0u32;
            for _ in 0..3 {
                mac += 32767 * i64::from(vector);
                if mac > (1i64 << 43) - 1 {
                    flags |= 1 << 30;
                }
                if mac < -(1i64 << 43) {
                    flags |= 1 << 27;
                }
                mac = ((mac + (1i64 << 43)) & ((1i64 << 44) - 1)) - (1i64 << 43);
            }
            g.execute(0x80012).unwrap();
            assert_eq!(g.read_data(25), (mac >> 12) as u32);
            assert_eq!(g.read_control(31) & ((1 << 30) | (1 << 27)), flags);
        }
    }
}
