// SPDX-License-Identifier: GPL-3.0-only
use psxgpu::{capture, Error, Processor, SoftwareRenderer, WgpuRenderer};
fn main() -> psxgpu::Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| Error("usage: replay PRIVATE_CAPTURE.psxcap".into()))?;
    let mut software = Processor::new(SoftwareRenderer::new());
    let mut accelerated = Processor::new(pollster::block_on(WgpuRenderer::new(1))?);
    let stats = capture::compare_replay(
        &mut std::fs::File::open(&path)?,
        &mut software,
        &mut accelerated,
        1024 * 1024 * 1024,
    )?;
    println!(
        "adapter={:?} {stats:?} mismatches=0/524288 at every frame and EOF",
        accelerated.renderer().adapter_info()
    );
    Ok(())
}
