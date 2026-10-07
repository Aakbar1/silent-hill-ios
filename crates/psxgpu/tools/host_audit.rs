// SPDX-License-Identifier: GPL-3.0-only
// Compiled by audit_host.py with private copies of the selected host Raster.
use psxgpu::{Processor, Rect, Renderer, SoftwareRenderer, WgpuRenderer};
use std::{
    hash::{Hash, Hasher},
    io::Write,
};

fn words_equal(a: &[u16], b: &[u16]) -> usize {
    a.iter().zip(b).filter(|(a, b)| a != b).count()
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut gpu = Processor::new(psxgpu::block_on(WgpuRenderer::new(1))?);
    let mut failures = 0;
    for case in fixtures::cases() {
        let mut sw = Processor::new(SoftwareRenderer::new());
        let mut host = host_raster::Raster::default();
        gpu.gp1(0)?;
        gpu.renderer_mut().fill(Rect::new(0, 0, 1024, 512), 0)?;
        fixtures::apply(&case, &mut sw);
        fixtures::apply(&case, &mut gpu);
        for (rect, pixels) in &case.uploads {
            host.load(
                rect.x.into(),
                rect.y.into(),
                rect.width.into(),
                rect.height.into(),
                pixels,
            );
        }
        for packet in &case.packets {
            let mut tagged = vec![0];
            tagged.extend(packet);
            host.packet(&tagged);
        }
        fixtures::check(&case, sw.renderer().vram());
        let accelerated = gpu.renderer_mut().read_vram()?;
        let host = host.read([0, 0, 1024, 512]);
        let gpu_diff = words_equal(sw.renderer().vram(), &accelerated);
        let host_diff = words_equal(sw.renderer().vram(), &host);
        let mut host_rgb_pixels = 0;
        let mut maximum = 0;
        for (a, b) in sw.renderer().vram().iter().zip(&host) {
            let a = psxgpu::rgb888(*a);
            let b = psxgpu::rgb888(*b);
            if a != b {
                host_rgb_pixels += 1;
            }
            for c in 0..3 {
                maximum = maximum.max(a[c].abs_diff(b[c]));
            }
        }
        println!("case={} host_words={host_diff} host_rgb_pixels={host_rgb_pixels} max_rgb8_delta={maximum} wgpu_words={gpu_diff}", case.name);
        if gpu_diff != 0
            || (std::env::var_os("AUDIT_REQUIRE_HOST_EXACT").is_some() && host_diff != 0)
        {
            failures += 1;
        }
    }
    if let Some(input) = std::env::args().nth(1) {
        let bytes = std::fs::read(input)?;
        let output = std::env::args().nth(2).ok_or("missing output prefix")?;
        let mut csv = std::fs::File::create(format!("{output}.csv"))?;
        writeln!(csv,"frame,host_rgb_pixels,max_rgb8_delta,host_vram_words,wgpu_vram_words,recorded_hash_match")?;
        let mut sw = Processor::new(SoftwareRenderer::new());
        let mut host = host_raster::Raster::default();
        gpu.gp1(0)?;
        gpu.renderer_mut().fill(Rect::new(0, 0, 1024, 512), 0)?;
        // Host startup drawing area is full VRAM before the first env call.
        sw.draw_state.area = [0, 0, 1023, 511];
        gpu.draw_state.area = sw.draw_state.area;
        let mut at = 0;
        let mut frame = 0;
        let mut load = None;
        let mut recorded_hash_matches = 0;
        let mut last = (0, 0, 0, 0);
        let mut total_gpu_diffs = 0;
        while at < bytes.len() {
            let read = |offset| u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
            if bytes.len() - at < 8 {
                return Err("truncated trace header".into());
            }
            let kind = read(at);
            let count = read(at + 4) as usize;
            at += 8;
            if count > (bytes.len() - at) / 4 {
                return Err("truncated trace words".into());
            }
            let w: Vec<u32> = (0..count).map(|i| read(at + i * 4)).collect();
            at += count * 4;
            match kind {
                1 => {
                    sw.gp0_words(&w)?;
                    gpu.gp0_words(&w)?;
                    let mut tag = vec![0];
                    tag.extend(w);
                    host.packet(&tag);
                }
                2 => {
                    if w.len() != 4 {
                        return Err("bad load".into());
                    }
                    load = Some(w);
                }
                22 => {
                    let r = load.take().ok_or("pixels without rect")?;
                    let pixels: Vec<u16> = w.iter().map(|p| *p as u16).collect();
                    let rect = Rect::new(r[0] as u16, r[1] as u16, r[2] as u16, r[3] as u16);
                    sw.renderer_mut().upload(rect, &pixels, 0)?;
                    gpu.renderer_mut().upload(rect, &pixels, 0)?;
                    host.load(r[0] as i32, r[1] as i32, r[2] as i32, r[3] as i32, &pixels);
                }
                3 => {
                    if w.len() != 7 {
                        return Err("bad clear".into());
                    }
                    let r = w[..4].iter().map(|v| *v as i32).collect::<Vec<_>>();
                    let color = [w[4] as u8, w[5] as u8, w[6] as u8];
                    host.clear(r.clone().try_into().unwrap(), color);
                    let left = r[0].max(0);
                    let top = r[1].max(0);
                    let right = (r[0] + r[2]).min(1024);
                    let bottom = (r[1] + r[3]).min(512);
                    if right > left && bottom > top {
                        let rect = Rect::new(
                            left as u16,
                            top as u16,
                            (right - left) as u16,
                            (bottom - top) as u16,
                        );
                        let [r, g, b] = color.map(|c| u16::from(c >> 3));
                        let p = r | (g << 5) | (b << 10);
                        sw.renderer_mut().fill(rect, p)?;
                        gpu.renderer_mut().fill(rect, p)?;
                    }
                }
                4 => {
                    if w.len() != 6 {
                        return Err("bad env".into());
                    }
                    let r = [w[0] as i32, w[1] as i32, w[2] as i32, w[3] as i32];
                    let off = [w[4] as i32, w[5] as i32];
                    host.env(r, off);
                    let area = [
                        r[0].max(0),
                        r[1].max(0),
                        (r[0] + r[2]).min(1024) - 1,
                        (r[1] + r[3]).min(512) - 1,
                    ];
                    sw.draw_state.area = area;
                    gpu.draw_state.area = area;
                    sw.draw_state.offset = off;
                    gpu.draw_state.offset = off;
                }
                5 => {
                    if w.len() != 5 {
                        return Err("bad scanout".into());
                    }
                    frame += 1;
                    let native = gpu.renderer_mut().read_vram()?;
                    let diff = words_equal(sw.renderer().vram(), &native);
                    total_gpu_diffs += diff;
                    let legacy = host.read([0, 0, 1024, 512]);
                    let mut pixels = 0;
                    let mut maximum = 0;
                    if w[4] == 0 {
                        let mut rgb = Vec::new();
                        for y in 0..w[3] {
                            for x in 0..w[2] {
                                let i = (((w[1] + y) & 511) * 1024 + ((w[0] + x) & 1023)) as usize;
                                let a = psxgpu::rgb888(sw.renderer().vram()[i]);
                                let b = psxgpu::rgb888(legacy[i]);
                                if a != b {
                                    pixels += 1;
                                }
                                for c in 0..3 {
                                    maximum = maximum.max(a[c].abs_diff(b[c]));
                                }
                                rgb.extend(a);
                            }
                        }
                        std::fs::write(format!("{output}.rgb"), rgb)?;
                    }
                    last = (
                        pixels,
                        maximum,
                        words_equal(sw.renderer().vram(), &legacy),
                        diff,
                    );
                }
                6 => {
                    if w.len() != 2 {
                        return Err("bad hash".into());
                    }
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    sw.renderer().vram().hash(&mut h);
                    let matches = h.finish() == (u64::from(w[0]) | (u64::from(w[1]) << 32));
                    recorded_hash_matches += usize::from(matches);
                    writeln!(
                        csv,
                        "{frame},{},{},{},{},{matches}",
                        last.0, last.1, last.2, last.3
                    )?;
                }
                _ => return Err(format!("unknown trace record {kind}").into()),
            }
        }
        println!("trace frames={frame} wgpu_differing_words={total_gpu_diffs} recorded_hash_matches={recorded_hash_matches} last_host_rgb_pixels={} last_max_delta={}",last.0,last.1);
        if total_gpu_diffs != 0 {
            failures += 1;
        }
        if std::env::var_os("AUDIT_REQUIRE_HOST_EXACT").is_some() {
            // Every CSV row, including non-displayed VRAM, must pass the patched-host gate.
            let rows = std::fs::read_to_string(format!("{output}.csv"))?;
            if rows
                .lines()
                .skip(1)
                .any(|r| r.split(',').nth(3) != Some("0"))
            {
                failures += 1;
            }
        }
    }
    if failures != 0 {
        return Err(format!("{failures} failed comparisons").into());
    }
    Ok(())
}
