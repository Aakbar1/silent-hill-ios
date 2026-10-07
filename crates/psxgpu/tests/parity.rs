// SPDX-License-Identifier: GPL-3.0-only
#[path = "support/parity_cases.rs"]
mod fixtures;
use psxgpu::{Processor, SoftwareRenderer};

macro_rules! rule_test {
    ($name:ident) => {
        #[test]
        fn $name() {
            let case = fixtures::cases()
                .into_iter()
                .find(|c| c.name == stringify!($name))
                .unwrap();
            let mut cpu = Processor::new(SoftwareRenderer::new());
            fixtures::apply(&case, &mut cpu);
            fixtures::check(&case, cpu.renderer().vram());
        }
    };
}
rule_test!(title_odd_channel_average);
rule_test!(options_gouraud_integer_sample);
rule_test!(brightness_triangle_integer_coverage);
rule_test!(brightness_line_rounding_both_slopes);
rule_test!(brightness_clut_zero_black_stp);
rule_test!(brightness_modulation_255_clamp);
rule_test!(flat_polygon_and_rect_strip_low_three_bits);
rule_test!(options_reentry_gouraud_dither_matrix);
rule_test!(line_dither_but_modulated_rectangle_no_dither);

#[cfg(feature = "wgpu")]
#[test]
fn milestone_rules_match_all_native_and_scaled_words_at_1x() {
    use psxgpu::{Rect, Renderer, WgpuRenderer};
    let mut gpu = Processor::new(psxgpu::block_on(WgpuRenderer::new(1)).unwrap());
    for case in fixtures::cases() {
        gpu.renderer_mut()
            .fill(Rect::new(0, 0, 1024, 512), 0)
            .unwrap();
        gpu.gp1(0).unwrap();
        let mut cpu = Processor::new(SoftwareRenderer::new());
        fixtures::apply(&case, &mut cpu);
        fixtures::apply(&case, &mut gpu);
        let native = gpu.renderer_mut().read_vram().unwrap();
        fixtures::check(&case, &native);
        assert_eq!(native, cpu.renderer().vram(), "{} native", case.name);
        assert_eq!(
            gpu.renderer_mut().read_scaled().unwrap(),
            native,
            "{} scaled",
            case.name
        );
    }
}

#[cfg(feature = "wgpu")]
#[test]
fn scale4_and_scale6_preserve_exact_native_words_and_integer_anchors() {
    use psxgpu::{Rect, Renderer, WgpuRenderer};
    for scale in [4usize, 6] {
        let mut gpu = Processor::new(psxgpu::block_on(WgpuRenderer::new(scale as u32)).unwrap());
        for case in fixtures::cases() {
            gpu.renderer_mut()
                .fill(Rect::new(0, 0, 1024, 512), 0)
                .unwrap();
            gpu.gp1(0).unwrap();
            let mut cpu = Processor::new(SoftwareRenderer::new());
            fixtures::apply(&case, &mut cpu);
            fixtures::apply(&case, &mut gpu);
            let native = gpu.renderer_mut().read_vram().unwrap();
            assert_eq!(
                native,
                cpu.renderer().vram(),
                "{} {scale}x native",
                case.name
            );
            let scaled = gpu.renderer_mut().read_scaled().unwrap();
            for (i, &word) in native.iter().enumerate() {
                let anchor = (i / 1024 * scale) * (1024 * scale) + i % 1024 * scale;
                assert_eq!(scaled[anchor], word, "{} {scale}x anchor {i}", case.name);
            }
        }
    }
}
