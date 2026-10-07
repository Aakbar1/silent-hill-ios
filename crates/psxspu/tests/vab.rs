use psxspu::{
    Spu,
    preview::PreviewSynth,
    sequence::Event,
    vab::{ToneRequest, Vab},
};
fn bank() -> Vec<u8> {
    let size = 0xa20 + 512 + 4096;
    let mut b = vec![0; size];
    b[..4].copy_from_slice(b"pBAV");
    b[12..16].copy_from_slice(&(size as u32).to_le_bytes());
    b[18..20].copy_from_slice(&1u16.to_le_bytes());
    b[20..22].copy_from_slice(&1u16.to_le_bytes());
    b[22..24].copy_from_slice(&1u16.to_le_bytes());
    b[24] = 127;
    b[25] = 64;
    // Sparse program 4; unused retained entries have no corresponding tones.
    b[32 + 4 * 16] = 1;
    b[32 + 4 * 16 + 1] = 127;
    b[32 + 4 * 16 + 4] = 64;
    b[32 + 12 * 16] = 16;
    b[0x820 + 2] = 127;
    b[0x820 + 3] = 64;
    b[0x820 + 4] = 60;
    b[0x820 + 6] = 0;
    b[0x820 + 7] = 127;
    b[0x820 + 16..0x820 + 18].copy_from_slice(&15u16.to_le_bytes());
    b[0x820 + 18..0x820 + 20].copy_from_slice(&0x1fc0u16.to_le_bytes());
    b[0x820 + 22..0x820 + 24].copy_from_slice(&1u16.to_le_bytes());
    b[0xa20 + 2..0xa20 + 4].copy_from_slice(&512u16.to_le_bytes());
    let start = 0xc20;
    b[start] = 8;
    b[start + 1] = 7;
    b[start + 2..start + 16].fill(0x11);
    b
}
#[test]
fn vab_sparse_programs_u16_sample_sizes_and_unused_headers() {
    let bytes = bank();
    let b = Vab::parse(&bytes).unwrap();
    assert_eq!(b.samples.len(), 1);
    assert_eq!(b.samples[0], 0..4096);
    assert_eq!(b.programs[4].tones.len(), 1);
    assert!(b.programs[12].tones.is_empty());
    let mut s = Spu::new();
    s.init();
    b.upload(&mut s, 0x1010).unwrap();
    let a = b
        .tone_attr(ToneRequest {
            program: 4,
            tone: 0,
            note: 60,
            fine: 0,
            voice: 23,
            base: 0x1010,
            volume: [1000; 2],
        })
        .unwrap();
    assert_eq!(a.address, 0x1010);
    assert_eq!(a.pitch, 4096);
    assert_eq!(a.voices, 1 << 23);
}
#[test]
fn vab_rejects_truncation_and_sample_extent() {
    let bytes = bank();
    for end in [0, 4, 31, 32, 2080, 3103, bytes.len() - 1] {
        assert!(Vab::parse(&bytes[..end]).is_err());
    }
    let mut bytes = bank();
    bytes[0xa20 + 2..0xa20 + 4].copy_from_slice(&65535u16.to_le_bytes());
    assert!(Vab::parse(&bytes).is_err());
}
#[test]
fn authored_muted_layers_need_external_game_control() {
    let bytes = bank();
    let b = Vab::parse(&bytes).unwrap();
    let mut s = Spu::new();
    s.init();
    b.upload(&mut s, 0x1010).unwrap();
    let mut p = PreviewSynth::default();
    for e in [
        Event::Program {
            channel: 1,
            program: 4,
        },
        Event::Control {
            channel: 1,
            controller: 7,
            value: 0,
        },
        Event::NoteOn {
            channel: 1,
            note: 60,
            velocity: 100,
        },
    ] {
        p.event(e, &mut s, &b, 0x1010).unwrap();
    }
    let mut output = [[0; 2]; 100];
    s.render(&mut output);
    assert!(output.iter().all(|x| *x == [0; 2]));
    p.set_channel_volume(1, 127, &mut s, &b).unwrap();
    s.render(&mut output);
    assert!(output.iter().any(|x| x[0] != 0));
}
