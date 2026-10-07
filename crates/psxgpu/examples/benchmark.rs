// SPDX-License-Identifier: GPL-3.0-only
use psxgpu::{Processor, Rect, Renderer, WgpuRenderer};
fn xy(x: u32, y: u32) -> u32 {
    x | (y << 16)
}
fn uv(u: u32, v: u32) -> u32 {
    u | (v << 8)
}
fn main() -> psxgpu::Result<()> {
    let feedback = std::env::args().nth(2).is_some_and(|arg| arg == "feedback");
    let frames = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(240)
        .clamp(1, 10000);
    let scale = std::env::args()
        .nth(3)
        .map(|s| s.parse::<u32>())
        .transpose()
        .map_err(|_| psxgpu::Error("scale must be an integer in 1..=8".into()))?
        .unwrap_or(4);
    let mut gpu = Processor::new(pollster::block_on(WgpuRenderer::new(scale))?);
    let display = psxgpu::Display {
        disabled: false,
        mode: 1,
        ..Default::default()
    };
    let target = gpu.renderer().create_scanout_texture(display)?;
    gpu.gp0_words(&[0xe3000000, 0xe4000000 | 319 | (239 << 10), 0xe100010a])?;
    let pixels: Vec<u16> = (0..256 * 256)
        .map(|i| (i as u16 & 0x7fff) | 0x8000)
        .collect();
    gpu.renderer_mut()
        .upload(Rect::new(640, 0, 256, 256), &pixels, 0)?;
    let mut words = Vec::new();
    for i in 0..1200u32 {
        let x = (i * 17) % 280;
        let y = (i * 13) % 208;
        let op = if i % 3 == 0 { 0x36808080 } else { 0x34808080 };
        words.extend([
            op,
            xy(x, y),
            uv(i & 255, (i * 3) & 255),
            0x809090,
            xy(x + 32, y + 2),
            uv((i + 32) & 255, (i * 3) & 255) | (0x10a << 16),
            0x808080,
            xy(x + 8, y + 30),
            uv((i + 8) & 255, (i * 3 + 30) & 255),
        ]);
    }
    for _ in 0..8 {
        gpu.renderer_mut().fill(Rect::new(0, 0, 320, 240), 0)?;
        gpu.gp0_words(&words)?;
        gpu.renderer_mut().flush()?;
        if feedback {
            gpu.renderer_mut()
                .copy(Rect::new(0, 0, 256, 240), 0, 256, 0)?;
            gpu.gp0_words(&[0xe1000110, 0x66608080, 0, 0, xy(256, 240)])?;
        }
        gpu.renderer_mut().render_scanout(display, false, &target)?;
    }
    gpu.renderer().wait_idle();
    let start = std::time::Instant::now();
    let mut submit_seconds = 0.0;
    let mut wait_seconds = 0.0;
    let mut decode_seconds = 0.0;
    let mut frame_ms = Vec::with_capacity(frames as usize);
    for _ in 0..frames {
        let frame_start = std::time::Instant::now();
        let t = std::time::Instant::now();
        gpu.renderer_mut().fill(Rect::new(0, 0, 320, 240), 0)?;
        gpu.gp0_words(&words)?;
        decode_seconds += t.elapsed().as_secs_f64();
        gpu.renderer_mut().flush()?;
        if feedback {
            gpu.renderer_mut()
                .copy(Rect::new(0, 0, 256, 240), 0, 256, 0)?;
            gpu.gp0_words(&[0xe1000110, 0x66608080, 0, 0, xy(256, 240)])?;
        }
        gpu.renderer_mut().render_scanout(display, false, &target)?;
        submit_seconds += t.elapsed().as_secs_f64();
        let t = std::time::Instant::now();
        gpu.renderer().wait_idle();
        wait_seconds += t.elapsed().as_secs_f64();
        frame_ms.push(frame_start.elapsed().as_secs_f64() * 1000.0);
    }
    let elapsed = start.elapsed().as_secs_f64();
    println!("adapter={:?}\nscale={scale} frames={frames} triangles/frame=1200 (textured, gouraud, 400 transparent), native draw area=320x240\ncompleted seconds={elapsed:.6} fps={:.2} ms/frame={:.3}",
        gpu.renderer().adapter_info(),f64::from(frames)/elapsed,elapsed*1000.0/f64::from(frames));
    println!(
        "submit ms/frame={:.3} completion wait ms/frame={:.3}",
        submit_seconds * 1000.0 / f64::from(frames),
        wait_seconds * 1000.0 / f64::from(frames)
    );
    frame_ms.sort_by(f64::total_cmp);
    println!(
        "completed p50={:.3} p95={:.3} p99={:.3} max={:.3} ms",
        frame_ms[frame_ms.len() / 2],
        frame_ms[((frame_ms.len() - 1) as f64 * 0.95).ceil() as usize],
        frame_ms[((frame_ms.len() - 1) as f64 * 0.99).ceil() as usize],
        frame_ms[frame_ms.len() - 1]
    );
    println!(
        "scanout={}x{} rgba8unorm; framebuffer copy+blend={feedback}; clear each frame",
        320 * scale,
        240 * scale
    );
    println!(
        "decode ms/frame={:.3} flush ms/frame={:.3}",
        decode_seconds * 1000.0 / f64::from(frames),
        (submit_seconds - decode_seconds) * 1000.0 / f64::from(frames)
    );
    // Force inspection of rendered output so the measured workload is observable.
    let vram = gpu.renderer_mut().read_vram()?;
    println!(
        "nonzero native framebuffer pixels={}",
        vram[..240 * 1024]
            .as_chunks::<1024>()
            .0
            .iter()
            .flat_map(|row| &row[..320])
            .filter(|&&p| p != 0)
            .count()
    );
    Ok(())
}
