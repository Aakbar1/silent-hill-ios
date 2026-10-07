use psxmedia::AudioFormat;
use psxspu::{
    sequence::{Event, Sequence, SequenceClock, convert_delta},
    xa::XaResampler,
};
#[test]
fn original_housekeeping_first_and_recurring_intervals() {
    let mut c = psxspu::sequence::HousekeepingClock::default();
    let due = (1..=35).filter(|_| c.tick()).collect::<Vec<_>>();
    assert_eq!(due, [12, 23, 34]);
}
fn kdt(track: &[u8]) -> Vec<u8> {
    let mut b = b"KDT1".to_vec();
    b.extend_from_slice(&(18u32 + track.len() as u32).to_le_bytes());
    b.extend_from_slice(&480u32.to_le_bytes());
    b.extend_from_slice(&1u32.to_le_bytes());
    b.extend_from_slice(&(track.len() as u16).to_le_bytes());
    b.extend_from_slice(track);
    b
}
#[test]
fn sequence_nominal_timer_frequency_and_no_drift() {
    let mut c = SequenceClock::default();
    for _ in 0..44100 {
        c.advance_frame();
    }
    assert_eq!(c.ticks, 577);
    for _ in 0..44100 * 9 {
        c.advance_frame();
    }
    assert_eq!(c.ticks, 5777);
}
#[test]
fn delta_conversion_retains_fraction_and_original_u16_wrap() {
    let mut r = 0;
    assert_eq!(
        std::array::from_fn::<_, 8, _>(|_| convert_delta(1, 480, &mut r)),
        [0, 0, 0, 1, 0, 0, 0, 1]
    );
    let mut r = 0;
    assert_eq!(convert_delta(65535, 48, &mut r), 16381);
    assert_eq!(r, 2);
}
#[test]
fn kdt_chaining_channel_program_note_and_short_off_goldens() {
    let bytes = kdt(&[0, 0xc6, 0x81, 0xc9, 0x80, 60, 100, 4, 0xca, 0, 0xff, 0]);
    let mut s = Sequence::parse(&bytes).unwrap();
    let mut events = Vec::new();
    s.tick(|e| events.push(e)).unwrap();
    assert!(events.is_empty());
    s.tick(|e| events.push(e)).unwrap();
    assert_eq!(
        events,
        [
            Event::Program {
                channel: 1,
                program: 0
            },
            Event::NoteOn {
                channel: 1,
                note: 60,
                velocity: 100
            }
        ]
    );
    for _ in 0..10 {
        s.tick(|e| events.push(e)).unwrap();
    }
    assert!(s.ended());
    assert_eq!(
        events[2],
        Event::NoteOff {
            channel: 1,
            note: 60
        }
    );
    assert_eq!(events[3], Event::End { track: 0 });
}
#[test]
fn kdt_tempo_and_pause_keep_fractional_clock() {
    let bytes = kdt(&[0, 0xc7, 0xc0, 60, 100, 4, 0xff, 0]);
    let mut s = Sequence::parse(&bytes).unwrap();
    s.paused = true;
    for _ in 0..5 {
        s.tick(|_| panic!()).unwrap();
    }
    assert_eq!(s.interrupts, 0);
    s.paused = false;
    let mut events = Vec::new();
    for _ in 0..2 {
        s.tick(|e| events.push(e)).unwrap();
    }
    assert_eq!(events[0], Event::Tempo(130));
    assert_eq!(events.len(), 2);
}
#[test]
fn malformed_sequence_and_bounded_event_chains() {
    for b in [&b"KDT1"[..], &b"nope"[..], &b"MThd"[..], &b"pQES"[..]] {
        assert!(Sequence::parse(b).is_err());
    }
    let bytes = kdt(&[0, 0x80]);
    let mut s = Sequence::parse(&bytes).unwrap();
    assert!(s.tick(|_| {}).is_ok());
    assert!(s.tick(|_| {}).is_err());
}
#[test]
fn midi_running_status_and_meta_end() {
    let track = [0, 0x90, 60, 100, 4, 60, 0, 0, 0xff, 0x2f, 0];
    let mut bytes = b"MThd".to_vec();
    bytes.extend_from_slice(&[0, 0, 0, 6, 0, 0, 0, 1, 1, 224]);
    bytes.extend_from_slice(b"MTrk");
    bytes.extend_from_slice(&(track.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&track);
    let mut s = Sequence::parse(&bytes).unwrap();
    let mut events = Vec::new();
    for _ in 0..20 {
        s.tick(|e| events.push(e)).unwrap();
    }
    assert_eq!(
        events,
        [
            Event::NoteOn {
                channel: 0,
                note: 60,
                velocity: 100
            },
            Event::NoteOff {
                channel: 0,
                note: 60
            },
            Event::End { track: 0 }
        ]
    );
    assert!(s.ended());
}
#[test]
fn seq_header_tempo_and_end() {
    let mut bytes = b"pQES\0\0\0\x01\x01\xe0\x07\xa1\x20\x04\x02".to_vec();
    bytes.extend_from_slice(&[0, 0x90, 60, 100, 4, 0x80, 60, 0, 0, 0xff, 0x2f, 0]);
    let mut s = Sequence::parse(&bytes).unwrap();
    let mut events = Vec::new();
    for _ in 0..20 {
        s.tick(|e| events.push(e)).unwrap();
    }
    assert_eq!(events.len(), 3);
    assert!(s.ended());
}
#[test]
fn xa_zigzag_exact_rate_chunk_independence_and_mono() {
    let format = AudioFormat {
        sample_rate: 37800,
        channels: 1,
        adpcm_bits_per_sample: 4,
    };
    let input = vec![1000; 2016];
    let mut a = XaResampler::default();
    let mut all = Vec::new();
    a.process(&input, format, &mut all).unwrap();
    assert_eq!(all.len(), 2352);
    assert!(all.iter().all(|x| x[0] == x[1]));
    let mut b = XaResampler::default();
    let mut split = Vec::new();
    for chunk in input.chunks(13) {
        b.process(chunk, format, &mut split).unwrap();
    }
    assert_eq!(all, split);
}
#[test]
fn xa_half_rate_and_impulse_known_coefficients() {
    let mut a = XaResampler::default();
    let mut out = Vec::new();
    let format = AudioFormat {
        sample_rate: 18900,
        channels: 1,
        adpcm_bits_per_sample: 4,
    };
    a.process(&vec![0; 2016], format, &mut out).unwrap();
    assert_eq!(out.len(), 4704);
    let mut a = XaResampler::default();
    let mut out = Vec::new();
    let format = AudioFormat {
        sample_rate: 37800,
        ..format
    };
    a.process(&[0, 0, 0, 0, 0, 32767], format, &mut out)
        .unwrap();
    assert_eq!(
        out.iter().map(|x| x[0]).collect::<Vec<_>>(),
        [0, 0, 0, 0, -1, 1, -5]
    );
}
