// SPDX-License-Identifier: GPL-3.0-only
// Hand-authored synthetic inputs only. Expected words are independent rule calculations.
use psxgpu::{Processor, Rect, Renderer};

pub struct Case {
    pub name: &'static str,
    pub uploads: Vec<(Rect, Vec<u16>)>,
    pub packets: Vec<Vec<u32>>,
    pub expected: Vec<(usize, usize, u16)>,
}
pub fn xy(x: u32, y: u32) -> u32 {
    x | (y << 16)
}
pub fn cases() -> Vec<Case> {
    let mut result = vec![
        Case {
            name: "title_odd_channel_average",
            uploads: vec![(Rect::new(0, 0, 2, 1), vec![0x0421, 0x0421])],
            packets: vec![vec![0xe1000000], vec![0x62080808, 0, xy(2, 1)]],
            // floor((1 + 1) / 2) = 1; floor(1/2)+floor(1/2) would erase it.
            expected: vec![(0, 0, 0x0421), (1, 0, 0x0421)],
        },
        Case {
            name: "options_gouraud_integer_sample",
            uploads: vec![],
            packets: vec![vec![
                0x38000000,
                xy(0, 0),
                0x000000c0,
                xy(4, 0),
                0,
                xy(0, 4),
                0x000000c0,
                xy(4, 4),
            ]],
            // Red plane = 48*x. Sampling x+0.5 adds three 5-bit levels (25 RGB8).
            expected: vec![
                (0, 0, 0),
                (1, 0, 6),
                (2, 1, 12),
                (3, 0, 18),
                (3, 1, 18),
                (4, 0, 0),
            ],
        },
        Case {
            name: "brightness_triangle_integer_coverage",
            uploads: vec![],
            packets: vec![vec![0x20ffffff, xy(0, 0), xy(8, 0), xy(0, 8)]],
            // Pixel (7,0) is inside; x+y=8 is the excluded lower/right edge.
            expected: vec![(7, 0, 0x7fff), (0, 7, 0x7fff), (7, 1, 0), (0, 8, 0)],
        },
        Case {
            name: "brightness_line_rounding_both_slopes",
            uploads: vec![],
            packets: vec![
                vec![0x40ffffff, xy(0, 0), xy(4, 2)],
                vec![0x400000ff, xy(8, 4), xy(12, 2)],
            ],
            // DDA nearest rounding: positive Y half ties rise, negative ties fall.
            expected: vec![
                (1, 1, 0x7fff),
                (1, 0, 0),
                (3, 2, 0x7fff),
                (4, 2, 0x7fff),
                (9, 3, 31),
                (9, 4, 0),
                (11, 2, 31),
                (12, 2, 31),
            ],
        },
        Case {
            name: "brightness_clut_zero_black_stp",
            uploads: vec![
                (Rect::new(640, 0, 1, 1), vec![0x3210]),
                (Rect::new(0, 480, 4, 1), vec![0, 0x8000, 0x001f, 0x801f]),
                (Rect::new(0, 0, 4, 1), vec![10; 4]),
            ],
            packets: vec![vec![0xe100000a], vec![0x66808080, 0, 480 << 22, xy(4, 1)]],
            // Palette value 0000 skips; 8000 is black and blends; only STP blends.
            expected: vec![(0, 0, 10), (1, 0, 0x8005), (2, 0, 31), (3, 0, 0x8014)],
        },
        Case {
            name: "brightness_modulation_255_clamp",
            uploads: vec![(Rect::new(640, 0, 4, 1), vec![0, 0x8000, 0x4210, 0x7fff])],
            packets: vec![vec![0xe100030a], vec![0x64ffffff, 0, 0, xy(4, 1)]],
            // floor(16*255/128)=31; 31*255/128 saturates, never wraps.
            expected: vec![(0, 0, 0), (1, 0, 0x8000), (2, 0, 0x7fff), (3, 0, 0x7fff)],
        },
        Case {
            name: "flat_polygon_and_rect_strip_low_three_bits",
            uploads: vec![],
            packets: vec![
                vec![0xe1000200],
                vec![0x200f0f0f, 0, xy(4, 0), xy(0, 4)],
                vec![0x600f0f0f, xy(8, 0), xy(4, 4)],
            ],
            // Dither enable does not affect flat unmodulated polygons or rectangles.
            expected: vec![(0, 0, 0x421), (1, 1, 0x421), (8, 0, 0x421), (9, 1, 0x421)],
        },
    ];
    let matrix = [-4i32, 0, -3, 1, 2, -2, 3, -1, -3, 1, -4, 0, 3, -1, 2, -2];
    result.push(Case {
        name: "options_reentry_gouraud_dither_matrix",
        uploads: vec![],
        packets: vec![
            vec![0xe1000200],
            vec![
                0x38080808,
                0,
                0x080808,
                xy(4, 0),
                0x080808,
                xy(0, 4),
                0x080808,
                xy(4, 4),
            ],
        ],
        expected: matrix
            .iter()
            .enumerate()
            .map(|(i, d)| (i % 4, i / 4, if 8 + d >= 8 { 0x421 } else { 0 }))
            .collect(),
    });
    result.push(Case {
        name: "line_dither_but_modulated_rectangle_no_dither",
        uploads: vec![(Rect::new(640, 0, 4, 4), vec![0x421; 16])],
        packets: vec![
            vec![0xe100030a],
            vec![0x40080808, 0, xy(3, 0)],
            vec![0x64808080, xy(0, 4), 0, xy(4, 4)],
        ],
        expected: vec![
            (0, 0, 0),
            (1, 0, 0x421),
            (2, 0, 0),
            (3, 0, 0x421),
            (0, 4, 0x421),
            (2, 4, 0x421),
        ],
    });
    result
}
pub fn apply<R: Renderer>(case: &Case, gpu: &mut Processor<R>) {
    gpu.gp0_words(&[0xe3000000, 0xe407ffff]).unwrap();
    for (rect, pixels) in &case.uploads {
        gpu.renderer_mut().upload(*rect, pixels, 0).unwrap();
    }
    for packet in &case.packets {
        gpu.gp0_words(packet).unwrap();
    }
    gpu.renderer_mut().flush().unwrap();
}
pub fn check(case: &Case, vram: &[u16]) {
    for &(x, y, expected) in &case.expected {
        assert_eq!(vram[y * 1024 + x], expected, "{} ({x},{y})", case.name);
    }
}
