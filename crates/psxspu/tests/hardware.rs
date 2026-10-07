use psxspu::{
    Adsr, AdsrPhase, Spu, Volume,
    reverb::{Preset, Reverb},
    sdk::{Allocator, KeyStatus, VoiceAttr},
    vab::note_to_pitch,
};

fn voice(pitch: u16, flags: u8) -> Spu {
    let mut spu = Spu::new();
    spu.init();
    let mut block = [0x11; 16];
    block[0] = 8;
    block[1] = flags;
    spu.upload(0x1010, &block).unwrap();
    spu.key_on_with_attr(&VoiceAttr {
        voices: 1,
        mask: 0x6009f,
        volume: [0x3fff; 2],
        pitch,
        address: 0x1010,
        adsr1: 15,
        adsr2: 0x1fc0,
        ..VoiceAttr::default()
    })
    .unwrap();
    spu
}
#[test]
fn adsr_fast_linear_attack_and_exponential_decay_goldens() {
    let mut a = Adsr::default();
    a.key_on();
    let values = std::array::from_fn::<_, 7, _>(|_| a.tick(0, 0));
    assert_eq!(values, [14336, 28672, 32767, 16383, 8191, 4095, 2047]);
    assert_eq!(a.phase, AdsrPhase::Sustain);
}
#[test]
fn adsr_linear_release_goldens() {
    let mut a = Adsr::default();
    a.key_on();
    a.level = 32767;
    a.key_off();
    assert_eq!(a.tick(0, 0), 16383);
    assert_eq!(a.tick(0, 0), 0);
    assert_eq!(a.phase, AdsrPhase::Off);
}
#[test]
fn adsr_exponential_release_goldens() {
    let mut a = Adsr::default();
    a.key_on();
    a.level = 16384;
    a.key_off();
    assert_eq!(
        [a.tick(0, 32), a.tick(0, 32), a.tick(0, 32)],
        [8192, 4096, 2048]
    );
}
#[test]
fn adsr_frozen_attack_and_release() {
    let mut a = Adsr::default();
    a.key_on();
    for _ in 0..100000 {
        assert_eq!(a.tick(0x7f00, 31), 0);
    }
    a.level = 12345;
    a.key_off();
    for _ in 0..100000 {
        assert_eq!(a.tick(0, 31), 12345);
    }
}
#[test]
fn adsr_exponential_attack_knee() {
    let mut a = Adsr::default();
    a.key_on();
    a.level = 25000;
    assert_eq!(a.tick(0x8000, 0), 28584);
}
#[test]
fn adsr_fractional_counter_period() {
    let mut a = Adsr::default();
    a.key_on();
    assert_eq!(a.tick(12 << 10, 0), 0);
    assert_eq!(a.tick(12 << 10, 0), 7);
}
#[test]
fn signed_direct_volume_and_sweeps() {
    let mut v = Volume::default();
    v.write(0x3fff);
    assert_eq!(v.tick(), 32766);
    v.write(0x4000);
    assert_eq!(v.tick(), -32768);
    v.write(0);
    v.write(0x8000);
    assert_eq!(v.tick(), 14336);
    v.write(0xa000);
    assert_eq!(v.tick(), 0);
}
#[test]
fn negative_phase_sweep_and_freeze() {
    let mut v = Volume::default();
    v.write(0);
    v.write(0x9000);
    assert_eq!(v.tick(), -16384);
    v.write(0x907f);
    for _ in 0..10000 {
        assert_eq!(v.tick(), -16384);
    }
}
#[test]
fn pitch_integer_octaves_and_fine_table() {
    assert_eq!(note_to_pitch(60, 0, 60, 0), 0x1000);
    assert_eq!(note_to_pitch(72, 0, 60, 0), 0x2000);
    assert_eq!(note_to_pitch(48, 0, 60, 0), 0x800);
    assert_eq!(note_to_pitch(60, 1, 60, 0), 0x1001);
    assert_eq!(note_to_pitch(60, 0, 60, 1), 0x1001);
}
#[test]
fn pitch_half_unity_and_double_sample_counters() {
    for (pitch, expected) in [(0x800, 14), (0x1000, 28), (0x2000, 56)] {
        let mut s = voice(pitch, 7);
        for _ in 0..28 {
            s.next_frame([0; 2], [0; 2]);
        }
        let v = s.voice(0).unwrap();
        assert_eq!(
            v.pitch_counter,
            if expected > 28 {
                (expected - 28) << 12
            } else {
                expected << 12
            }
        );
    }
}
#[test]
fn pitch_clamps_at_four_samples_and_zero_holds() {
    let mut s = voice(65535, 7);
    s.next_frame([0; 2], [0; 2]);
    assert_eq!(s.voice(0).unwrap().pitch_counter, 0x4000);
    let mut s = voice(0, 7);
    for _ in 0..100 {
        s.next_frame([0; 2], [0; 2]);
    }
    assert_eq!(s.voice(0).unwrap().pitch_counter, 0);
}
#[test]
fn loop_start_loop_end_and_endx_rekey() {
    let mut s = voice(4096, 7);
    for _ in 0..28 {
        s.next_frame([0; 2], [0; 2]);
    }
    assert_eq!(s.endx(), 0);
    s.next_frame([0; 2], [0; 2]);
    assert_eq!(s.endx(), 1);
    assert_eq!(s.voice(0).unwrap().registers[7], 0x1010 / 8);
    s.key_on(1);
    assert_eq!(s.endx(), 0);
    assert_eq!(s.key_status(1), KeyStatus::On);
}
#[test]
fn one_shot_stops_at_block_boundary() {
    let mut s = voice(4096, 1);
    for _ in 0..29 {
        s.next_frame([0; 2], [0; 2]);
    }
    assert_eq!(s.endx(), 1);
    assert_eq!(s.voice(0).unwrap().phase, AdsrPhase::Off);
    assert_eq!(s.voice(0).unwrap().envelope, 0);
}
#[test]
fn keyoff_reports_release_without_waiting_for_callback() {
    let mut s = voice(4096, 7);
    s.render(&mut [[0; 2]; 16]);
    s.key_off(1);
    assert_eq!(s.key_status(1), KeyStatus::OffEnvelopeOn);
    assert_eq!(s.voice(0).unwrap().phase, AdsrPhase::Release);
}
#[test]
fn key_status_invalid_masks_and_silent_active_envelope() {
    let mut s = voice(4096, 7);
    assert_eq!(s.key_status(0), KeyStatus::InvalidMask);
    assert_eq!(s.key_status(1 << 24), KeyStatus::InvalidMask);
    s.write_register(8, 0x7f00).unwrap();
    s.next_frame([0; 2], [0; 2]);
    assert_eq!(s.key_status(1), KeyStatus::OnEnvelopeOff);
}
#[test]
fn manual_and_dma_wrap_readback() {
    let mut s = Spu::new();
    s.set_transfer_address(524280).unwrap();
    s.dma_write(&[7; 16]);
    assert_eq!(s.transfer_address(), 8);
    s.set_transfer_address(524280).unwrap();
    let mut b = [0; 16];
    s.dma_read(&mut b);
    assert_eq!(b, [7; 16]);
    s.write_register(0x1a6, 0x400).unwrap();
    s.write_register(0x1a8, 0xabcd).unwrap();
    assert_eq!(&s.ram()[8192..8194], &[0xcd, 0xab]);
    assert_eq!(s.read_register(0x1a6).unwrap(), 0x400);
}
#[test]
fn bad_registers_and_upload_are_rejected() {
    let mut s = Spu::new();
    assert!(s.write_register(1, 0).is_err());
    assert!(s.read_register(0x1f802000).is_err());
    assert!(s.upload(524288, &[1]).is_err());
    assert!(s.upload(usize::MAX, &[1]).is_err());
}
#[test]
fn active_adpcm_irq_and_ack() {
    let mut s = voice(4096, 7);
    s.write_register(0x1a4, 0x1010 / 8).unwrap();
    s.write_register(0x1aa, 0xc040).unwrap();
    s.next_frame([0; 2], [0; 2]);
    assert!(s.irq_pending());
    assert_ne!(s.read_register(0x1ae).unwrap() & 64, 0);
    s.write_register(0x1aa, 0xc000).unwrap();
    assert!(!s.irq_pending());
}
#[test]
fn cd_input_signed_volume_and_capture() {
    let mut s = Spu::new();
    s.init();
    s.write_register(0x1aa, 1).unwrap();
    s.write_register(0x1b0, 0x4000).unwrap();
    s.write_register(0x1b2, 0xc000).unwrap();
    assert_eq!(s.next_frame([10000, 10000], [0; 2]), [4999, -5000]);
    assert_eq!(&s.ram()[0..2], &10000i16.to_le_bytes());
    assert_eq!(&s.ram()[1024..1026], &10000i16.to_le_bytes());
}
#[test]
fn noise_is_shared_and_independent_of_pitch() {
    let mut s = voice(0, 7);
    s.write_register(0x194, 10).unwrap();
    s.write_register(0x1aa, 0xfc00).unwrap();
    let attr = s.get_voice_attr(0).unwrap();
    let mut second = attr;
    second.voices = 2;
    second.pitch = 4096;
    s.key_on_with_attr(&second).unwrap();
    second.voices = 8;
    second.pitch = 0;
    s.key_on_with_attr(&second).unwrap();
    for _ in 0..200 {
        s.next_frame([0; 2], [0; 2]);
    }
    assert_eq!(s.decode_errors(), 0);
    assert_eq!(&s.ram()[0x800..0x800 + 400], &s.ram()[0xc00..0xc00 + 400]);
}
#[test]
fn reverb_reflection_q15_known_values() {
    let mut r = Reverb::default();
    r.set_base(0xf000);
    r.registers[2] = 0x4000;
    r.registers[30] = 0x4000;
    r.registers[10] = 4;
    r.registers[18] = 8;
    let mut ram = vec![0; 524288];
    r.tick_channel(&mut ram, 16384, 0, true);
    let base = 0xf000 * 8;
    assert_eq!(
        i16::from_le_bytes(ram[base + 32..base + 34].try_into().unwrap()),
        4096
    );
    assert_eq!(
        i16::from_le_bytes(ram[base + 64..base + 66].try_into().unwrap()),
        4096
    );
}
#[test]
fn reverb_cross_reflection_reads_opposite_channel() {
    let mut r = Reverb::default();
    r.set_base(0xf000);
    r.registers[2] = 0x4000;
    r.registers[7] = 0x4000;
    r.registers[18] = 8;
    r.registers[25] = 3;
    let mut ram = vec![0; 524288];
    let base = 0xf000 * 8;
    ram[base + 24..base + 26].copy_from_slice(&16000i16.to_le_bytes());
    r.tick_channel(&mut ram, 0, 0, true);
    assert_eq!(
        i16::from_le_bytes(ram[base + 64..base + 66].try_into().unwrap()),
        4000
    );
}
#[test]
fn disabled_reverb_keeps_output_and_does_not_write() {
    let mut r = Reverb::default();
    r.set_base(0xf000);
    r.registers[28] = 1;
    r.volume = [0x4000; 2];
    let mut ram = vec![0; 524288];
    let a = 0xf000 * 8 + 8;
    ram[a..a + 2].copy_from_slice(&12345i16.to_le_bytes());
    let before = ram.clone();
    assert_eq!(r.tick_channel(&mut ram, 32767, 0, false), 6172);
    assert_eq!(ram, before);
}
#[test]
fn reverb_cursor_half_rate_wrap_and_presets() {
    for id in 0..10 {
        let p = Preset::from_id(id).unwrap();
        assert_eq!(usize::from(p.base()) * 8 + p.work_bytes(), 524288);
    }
    let mut r = Reverb::default();
    r.set_base(65535);
    let mut ram = vec![0; 524288];
    let base = r.cursor();
    r.process(&mut ram, [0; 2], false);
    assert_eq!(r.cursor(), base);
    r.process(&mut ram, [0; 2], false);
    assert_eq!(r.cursor(), base + 2);
    for _ in 0..6 {
        r.process(&mut ram, [0; 2], false);
    }
    assert_eq!(r.cursor(), base);
}
#[test]
fn allocator_reservation_fragmentation_and_exhaustion() {
    let mut a = Allocator::new(2).unwrap();
    let x = a.allocate(17).unwrap();
    let y = a.allocate(16).unwrap();
    assert_eq!(y, x + 24);
    assert!(a.allocate(8).is_err());
    assert!(a.free(x));
    assert_eq!(a.allocate(8).unwrap(), x);
    assert!(!a.free(0));
    let mut a = Allocator::new(16).unwrap();
    a.allocate_at(500000, 2000).unwrap();
    assert!(a.reserve_reverb(Preset::Hall, true).is_err());
}
