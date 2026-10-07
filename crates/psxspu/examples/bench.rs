use psxspu::{Spu, reverb::Preset, sdk::VoiceAttr};
use std::{hint::black_box, time::Instant};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut s = Spu::new();
    s.init();
    s.set_reverb_preset(Preset::Room, true);
    s.reverb.volume = [0x2800; 2];
    s.set_reverb(true);
    let mut block = [0x73; 16];
    block[0] = 0x24;
    block[1] = 7;
    s.upload(0x1010, &block)?;
    for i in 0..24 {
        s.key_on_with_attr(&VoiceAttr {
            voices: 1 << i,
            mask: 0x6009f,
            volume: [0x300; 2],
            pitch: 2048 + i as u16 * 127,
            address: 0x1010,
            adsr1: 15,
            adsr2: 0x1fc0,
            ..VoiceAttr::default()
        })?;
    }
    s.set_reverb_voice(true, 0xffffff);
    s.write_register(0x190, 0xaaaa)?;
    s.write_register(0x192, 0xaa)?;
    s.write_register(0x194, 0x1010)?;
    let mut block = [[0; 2]; 441];
    for _ in 0..100 {
        s.render(black_box(&mut block));
    }
    for pass in 1..=3 {
        let start = Instant::now();
        for _ in 0..1000 {
            s.render(black_box(&mut block));
        }
        let elapsed = start.elapsed().as_secs_f64();
        println!(
            "pass={pass} audio_seconds=10 elapsed_seconds={elapsed:.6} one_core_percent={:.3} voices=24 reverb=Room pmod=true noise=true decode_errors={}",
            elapsed * 10.0,
            s.decode_errors()
        );
    }
    for i in 0..24 {
        s.write_register(i * 16 + 4, 0x4000)?;
    }
    for pass in 1..=3 {
        let start = Instant::now();
        for _ in 0..1000 {
            s.render(black_box(&mut block));
        }
        let elapsed = start.elapsed().as_secs_f64();
        println!(
            "max_pitch_pass={pass} audio_seconds=10 elapsed_seconds={elapsed:.6} one_core_percent={:.3} voices=24 reverb=Room pmod=true noise=true decode_errors={}",
            elapsed * 10.0,
            s.decode_errors()
        );
    }
    Ok(())
}
