// SPDX-License-Identifier: GPL-3.0-only
#![cfg(feature = "wgpu")]
use psxgpu::{capture, Processor, Rect, Renderer, SoftwareRenderer, WgpuRenderer};
fn xy(x: u32, y: u32) -> u32 {
    x | (y << 16)
}
fn packet(op: u32, x: u32, y: u32, page: u32) -> Vec<u32> {
    let mut words = vec![(op << 24) | 0x808080];
    if op < 0x40 {
        let vertices = if op & 8 != 0 { 4 } else { 3 };
        for i in 0..vertices {
            if i != 0 && op & 16 != 0 {
                words.push([0x304080, 0x80f040, 0xf02080, 0x8080f0][i as usize]);
            }
            words.push(xy(
                x + if i & 1 != 0 { 28 } else { 0 },
                y + if i & 2 != 0 { 24 } else { 0 },
            ));
            if op & 4 != 0 {
                words.push(
                    (if i & 1 != 0 { 28 } else { 0 })
                        | ((if i & 2 != 0 { 24 } else { 0 }) << 8)
                        | if i == 0 {
                            480 << 22
                        } else if i == 1 {
                            page << 16
                        } else {
                            0
                        },
                );
            }
        }
    } else if op < 0x60 {
        words.push(xy(x, y));
        if op & 16 != 0 {
            words.push(0x2040f0);
        }
        words.push(xy(x + 28, y + 24));
        if op & 8 != 0 {
            if op & 16 != 0 {
                words.push(0x80f010);
            }
            words.push(xy(x + 4, y + 24));
            words.push(0x50005000);
        }
    } else {
        words.push(xy(x, y));
        if op & 4 != 0 {
            words.push(480 << 22);
        }
        if op & 24 == 0 {
            words.push(xy(28, 24));
        }
    }
    words
}
fn setup<R: Renderer>(g: &mut Processor<R>) {
    g.gp0_words(&[0xe3000000, 0xe4000000 | 639 | (383 << 10)])
        .unwrap();
    let data: Vec<u16> = (0..256 * 256)
        .map(|n| {
            if n % 17 == 0 {
                0
            } else {
                (n as u16 & 0x7fff) | if n % 2 == 0 { 0x8000 } else { 0 }
            }
        })
        .collect();
    g.renderer_mut()
        .upload(Rect::new(768, 0, 256, 256), &data, 0)
        .unwrap();
    let clut: Vec<u16> = (0..256)
        .map(|i| {
            if i == 0 {
                0
            } else {
                (i * 127) & 0x7fff | if i % 2 == 0 { 0x8000 } else { 0 }
            }
        })
        .collect();
    g.renderer_mut()
        .upload(Rect::new(0, 480, 256, 1), &clut, 0)
        .unwrap();
}
fn compare(cpu: &Processor<SoftwareRenderer>, gpu: &mut Processor<WgpuRenderer>, label: &str) {
    let result = gpu.renderer_mut().read_vram().unwrap();
    let differences: Vec<_> = result
        .iter()
        .zip(cpu.renderer().vram())
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .take(8)
        .collect();
    assert!(
        differences.is_empty(),
        "{label}: first differences {differences:?}"
    );
}
#[test]
fn every_primitive_opcode_depth_blend_dither_mask_matches_exactly() {
    let mut cpu = Processor::new(SoftwareRenderer::new());
    let mut gpu = Processor::new(pollster::block_on(WgpuRenderer::new(1)).unwrap());
    println!(
        "actual comparison adapter {:?}",
        gpu.renderer().adapter_info()
    );
    setup(&mut cpu);
    setup(&mut gpu);
    for depth in 0..3 {
        for blend in 0..4 {
            for dither in 0..2 {
                let page = 12 | (depth << 7) | (blend << 5) | (dither << 9);
                for op in 0x20..0x80 {
                    let index = op - 0x20;
                    let x = (index % 16) * 36;
                    let y = (index / 16) * 36;
                    let words = packet(op, x, y, page);
                    for state in [0, 1, 2, 3] {
                        let env = [0xe1000000 | page, 0xe6000000 | state];
                        cpu.gp0_words(&env).unwrap();
                        gpu.gp0_words(&env).unwrap();
                        // Split words individually to exercise the real FIFO for each primitive.
                        for &w in &words {
                            cpu.gp0(w).unwrap();
                            gpu.gp0(w).unwrap();
                        }
                    }
                }
                compare(
                    &cpu,
                    &mut gpu,
                    &format!("depth={depth} blend={blend} dither={dither}"),
                );
            }
        }
    }
    let output = gpu.renderer_mut().read_scaled().unwrap();
    assert_eq!(output, cpu.renderer().vram());
}
#[test]
fn feedback_transfer_and_capture_replay_match_exactly() {
    let mut cpu = Processor::new(SoftwareRenderer::new());
    let mut gpu = Processor::new(pollster::block_on(WgpuRenderer::new(1)).unwrap());
    let mut capture = capture::CaptureWriter::new(Vec::new()).unwrap();
    let words = [
        0xe3000000,
        0xe407ffff,
        0xe1000100,
        0x600000f8,
        xy(0, 0),
        xy(64, 64),
        0x65000000,
        xy(64, 0),
        0,
        xy(64, 64),
        0x65000000,
        xy(128, 0),
        64,
        xy(64, 64),
        0x80000000,
        xy(0, 0),
        xy(512, 256),
        xy(192, 64),
        0xe6000001,
        0xa0000000,
        xy(1023, 511),
        xy(3, 1),
        0x01020304,
        0xdead0809,
        0xe6000002,
        0x80000000,
        xy(1023, 511),
        xy(1022, 510),
        xy(3, 1),
        0x80000000,
        xy(0, 0),
        xy(1, 0),
        xy(7, 1),
        0x0200f800,
        xy(0, 128),
        xy(256, 16),
    ];
    capture.gp0(&words).unwrap();
    capture.frame().unwrap();
    let bytes = capture.into_inner();
    capture::replay(&mut bytes.as_slice(), &mut cpu, 1024 * 1024).unwrap();
    capture::replay(&mut bytes.as_slice(), &mut gpu, 1024 * 1024).unwrap();
    compare(&cpu, &mut gpu, "feedback+transfers");
}
#[test]
fn scale4_is_sharp_keeps_native_pixels_and_subpixel_is_opt_in() {
    let mut cpu = Processor::new(SoftwareRenderer::new());
    let mut gpu = Processor::new(pollster::block_on(WgpuRenderer::new(4)).unwrap());
    for words in [
        &[0xe3000000, 0xe407ffff][..],
        &[0x200000ff, xy(2, 2), xy(10, 2), xy(2, 10)][..],
    ] {
        cpu.gp0_words(words).unwrap();
        gpu.gp0_words(words).unwrap();
    }
    compare(&cpu, &mut gpu, "scale4 native");
    let before = gpu.renderer_mut().read_scaled().unwrap();
    assert_eq!(before.len(), psxgpu::VRAM_WORDS * 16);
    // The diagonal resolves at quarter-pixel increments, not repeated 1x squares.
    assert_eq!(before[8 * 4096 + 39], 31);
    assert_eq!(before[9 * 4096 + 39], 0);
    let words = [0x2000ff00, xy(20, 20), xy(28, 20), xy(20, 28)];
    let positions = [[20.75, 20.75], [28.75, 20.75], [20.75, 28.75]];
    cpu.gp0_words(&words).unwrap();
    gpu.polygon_with_positions(&words, &positions).unwrap();
    compare(&cpu, &mut gpu, "precise native");
    let after = gpu.renderer_mut().read_scaled().unwrap();
    assert_eq!(after[80 * 4096 + 80], 0);
    assert_eq!(after[83 * 4096 + 83], 0x3e0);
    gpu.renderer_mut()
        .copy(Rect::new(20, 20, 16, 16), 100, 100, 0)
        .unwrap();
    let copied = gpu.renderer_mut().read_scaled().unwrap();
    assert_eq!(copied[400 * 4096 + 400], 0);
    assert_eq!(copied[403 * 4096 + 403], 0x3e0);
    // Overlap fallback must retain the same sharp edge, not replicate native pixels.
    gpu.renderer_mut()
        .copy(Rect::new(20, 20, 16, 16), 21, 20, 0)
        .unwrap();
    let overlap = gpu.renderer_mut().read_scaled().unwrap();
    assert_eq!(overlap[80 * 4096 + 84], 0);
    assert_eq!(overlap[83 * 4096 + 87], 0x3e0);
}
#[test]
fn random_triangles_clipping_offsets_and_windows_match() {
    let mut cpu = Processor::new(SoftwareRenderer::new());
    let mut gpu = Processor::new(pollster::block_on(WgpuRenderer::new(1)).unwrap());
    setup(&mut cpu);
    setup(&mut gpu);
    let mut seed = 0x12345678u32;
    for i in 0..300 {
        let mut rand = || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            seed
        };
        let op = 0x20 | (rand() & 31);
        let page = 12 | ((rand() % 3) << 7) | ((rand() % 4) << 5) | 512;
        let env = [
            0xe1000000 | page,
            0xe2000000 | (rand() & 0xfffff),
            0xe5000000 | (rand() & 2047) | ((rand() & 2047) << 11),
            0xe6000000 | (i % 4),
        ];
        cpu.gp0_words(&env).unwrap();
        gpu.gp0_words(&env).unwrap();
        let mut p = packet(op, rand() % 600, rand() % 350, page);
        // Perturb all polygon positions, retaining valid packet field boundaries.
        let mut j = 1;
        for k in 0..if op & 8 != 0 { 4 } else { 3 } {
            if k > 0 && op & 16 != 0 {
                j += 1;
            }
            p[j] = xy(rand() % 640, rand() % 384);
            j += 1 + usize::from(op & 4 != 0);
        }
        cpu.gp0_words(&p).unwrap();
        gpu.gp0_words(&p).unwrap();
    }
    compare(&cpu, &mut gpu, "random clipped triangles");
}

fn read_texture(gpu: &WgpuRenderer, texture: &wgpu::Texture) -> Vec<u8> {
    let stride = (texture.width() * 4).div_ceil(256) * 256;
    let staging = gpu.device().create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(stride) * u64::from(texture.height()),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device().create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: None,
            },
        },
        texture.size(),
    );
    gpu.queue().submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    staging
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    gpu.wait_idle();
    rx.recv().unwrap().unwrap();
    let mapped = staging.slice(..).get_mapped_range();
    let out = mapped
        .chunks(stride as usize)
        .flat_map(|r| r[..(texture.width() * 4) as usize].iter().copied())
        .collect();
    drop(mapped);
    staging.unmap();
    out
}

#[test]
fn gpu_scanout_matches_cpu_for_15_24_bit_disabled_reverse_and_fields() {
    let mut gpu = pollster::block_on(WgpuRenderer::new(1)).unwrap();
    let data: Vec<u16> = (0..psxgpu::VRAM_WORDS)
        .map(|i| i.wrapping_mul(31337) as u16)
        .collect();
    gpu.upload(Rect::new(0, 0, 1024, 512), &data, 0).unwrap();
    for (mode, disabled, field) in [
        (0, false, false),
        (16, false, false),
        (128, false, false),
        (0, true, false),
        (36, false, false),
        (36, false, true),
    ] {
        let display = psxgpu::Display {
            disabled,
            mode,
            x: 1000,
            y: 500,
            ..Default::default()
        };
        let texture = gpu.create_scanout_texture(display).unwrap();
        gpu.render_scanout(display, field, &texture).unwrap();
        assert_eq!(
            read_texture(&gpu, &texture),
            psxgpu::scanout(&data, display, field).unwrap(),
            "mode={mode} field={field}"
        );
    }
}

#[test]
fn scale8_and_configuration_bounds() {
    assert!(pollster::block_on(WgpuRenderer::new(0)).is_err());
    assert!(pollster::block_on(WgpuRenderer::new(9)).is_err());
    let mut g = Processor::new(pollster::block_on(WgpuRenderer::new(8)).unwrap());
    g.gp0_words(&[0xe3000000, 0xe407ffff, 0x680000ff, xy(1023, 511)])
        .unwrap();
    assert_eq!(
        g.renderer_mut().read(Rect::new(1023, 511, 1, 1)).unwrap(),
        vec![31]
    );
    let d = psxgpu::Display {
        disabled: false,
        x: 1023,
        y: 511,
        horizontal: [0, 10],
        vertical: [0, 1],
        ..Default::default()
    };
    let target = g.renderer().create_scanout_texture(d).unwrap();
    g.renderer_mut().render_scanout(d, false, &target).unwrap();
    let out = read_texture(g.renderer(), &target);
    assert_eq!(out.len(), 8 * 8 * 4);
    assert!(out
        .as_chunks::<4>()
        .0
        .iter()
        .all(|c| c == &[255, 0, 0, 255]));
}

#[test]
fn interlaced_draw_and_fill_obey_dfe_and_field_parity() {
    let mut cpu = Processor::new(SoftwareRenderer::new());
    let mut gpu = Processor::new(pollster::block_on(WgpuRenderer::new(1)).unwrap());
    for g in [
        &mut cpu as &mut dyn InterlaceFixture,
        &mut gpu as &mut dyn InterlaceFixture,
    ] {
        g.commands(&[0xe3000000, 0xe407ffff]);
        g.control(0x08000024);
        g.field(false);
        g.commands(&[0x0200ff00, 0, xy(16, 4)]);
    }
    compare(&cpu, &mut gpu, "480i fill skips even lines");
    assert_eq!(cpu.renderer().vram()[0], 0);
    assert_eq!(cpu.renderer().vram()[1024], 0x3e0);
    for g in [
        &mut cpu as &mut dyn InterlaceFixture,
        &mut gpu as &mut dyn InterlaceFixture,
    ] {
        g.field(true);
        g.commands(&[0x60ff0000, 0, xy(16, 4)]);
    }
    compare(&cpu, &mut gpu, "480i draw skips odd lines");
    assert_eq!(cpu.renderer().vram()[0], 0x7c00);
    assert_eq!(cpu.renderer().vram()[1024], 0x3e0);
    for g in [
        &mut cpu as &mut dyn InterlaceFixture,
        &mut gpu as &mut dyn InterlaceFixture,
    ] {
        g.commands(&[0xe1000400, 0x600000ff, 0, xy(16, 4)]);
    }
    compare(&cpu, &mut gpu, "DFE enables both parities");
    assert_eq!(cpu.renderer().vram()[1024], 31);
}
trait InterlaceFixture {
    fn commands(&mut self, words: &[u32]);
    fn control(&mut self, word: u32);
    fn field(&mut self, field: bool);
}
impl<R: Renderer> InterlaceFixture for Processor<R> {
    fn commands(&mut self, words: &[u32]) {
        self.gp0_words(words).unwrap();
    }
    fn control(&mut self, word: u32) {
        self.gp1(word).unwrap();
    }
    fn field(&mut self, field: bool) {
        self.set_field(field);
    }
}
