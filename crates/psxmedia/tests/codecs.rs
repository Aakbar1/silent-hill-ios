use psxmedia::*;

fn bs(version: u16, codes: &str) -> Vec<u8> {
    let mut data = vec![0, 0, 0, 0x38, 1, 0];
    data.extend(version.to_le_bytes());
    let bits: Vec<_> = codes.bytes().filter(|&b| b == b'0' || b == b'1').collect();
    for chunk in bits.chunks(16) {
        let mut word = 0u16;
        for (i, &b) in chunk.iter().enumerate() {
            word |= ((b - b'0') as u16) << (15 - i);
        }
        data.extend(word.to_le_bytes());
    }
    data
}
fn dc10(value: i32) -> String {
    format!("{:010b}10", value & 1023)
}
fn solid_v2(dc: [i32; 6]) -> Vec<u8> {
    bs(2, &dc.into_iter().map(dc10).collect::<String>())
}
fn str_sector(number: u32, chunk: u16, chunks: u16, data: &[u8], size: usize) -> Vec<u8> {
    let mut sector = vec![0; 2048];
    sector[..4].copy_from_slice(&[0x60, 1, 1, 0x80]);
    sector[4..6].copy_from_slice(&chunk.to_le_bytes());
    sector[6..8].copy_from_slice(&chunks.to_le_bytes());
    sector[8..12].copy_from_slice(&number.to_le_bytes());
    sector[12..16].copy_from_slice(&(size as u32).to_le_bytes());
    sector[16..18].copy_from_slice(&16u16.to_le_bytes());
    sector[18..20].copy_from_slice(&16u16.to_le_bytes());
    sector[32..32 + data.len()].copy_from_slice(data);
    sector
}
fn xa_data(bits: u8, value: u8, parameter: u8) -> Vec<u8> {
    let mut data = vec![0; 2304];
    for group in data.as_chunks_mut::<128>().0 {
        group[..16].fill(parameter);
        group[16..].fill(if bits == 4 { value | value << 4 } else { value });
    }
    data
}
fn xa_sector(file: u8, channel: u8, coding: u8, data: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0; 2336];
    bytes[..4].copy_from_slice(&[file, channel, 0x64, coding]);
    bytes[4..8].copy_from_slice(&[file, channel, 0x64, coding]);
    bytes[8..8 + data.len()].copy_from_slice(data);
    bytes
}

#[test]
fn v2_black_white_color_and_cropped_dimensions() {
    for (dc, expected) in [
        ([0, 0, -512, -512, -512, -512], [0, 0, 0, 255]),
        ([0, 0, 508, 508, 508, 508], [255, 255, 255, 255]),
        ([64, -64, 0, 0, 0, 0], [150, 122, 100, 255]),
    ] {
        let pixels = MdecDecoder.decode(&solid_v2(dc), 13, 9).unwrap();
        assert_eq!(pixels.len(), 13 * 9 * 4);
        assert!(pixels.as_chunks::<4>().0.iter().all(|&p| p == expected));
    }
}
#[test]
fn v3_differential_dc_components_and_frame_reset() {
    // Cr +1, Cb -1, Y +1, then three zero differences; DC precision is four.
    let data = bs(3, "01 1 10  01 0 10  00 1 10  100 10 100 10 100 10");
    let pixels = MdecDecoder.decode(&data, 16, 16).unwrap();
    assert!(
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .all(|&p| p == [130, 129, 127, 255])
    );
    assert_eq!(pixels, MdecDecoder.decode(&data, 16, 16).unwrap());
}
#[test]
fn v2_ac_escape_and_vlc_signs() {
    // Y1 negative AC at zigzag index 1, followed by EOB; the other blocks are flat.
    let prefix = dc10(0).repeat(2);
    let escaped = format!(
        "{prefix}0000000000 000001 000000 1111000000 10 {}",
        dc10(0).repeat(3)
    );
    let pixels = MdecDecoder.decode(&bs(2, &escaped), 16, 16).unwrap();
    assert!(pixels[0] < 128 && pixels[7 * 4] > 128);
    assert_eq!(&pixels[8 * 4..8 * 4 + 4], &[128, 128, 128, 255]);
    let positive = format!("{prefix}0000000000 11 0 10 {}", dc10(0).repeat(3));
    assert!(MdecDecoder.decode(&bs(2, &positive), 16, 16).is_ok());
}

#[test]
fn quantizer_zero_uses_natural_coefficient_order() {
    let prefix = dc10(0).repeat(2);
    // The third AC coefficient is horizontal frequency 3 without zigzag,
    // and vertical frequency 2 with zigzag.
    let codes = format!(
        "{prefix}0000000000 000001 000010 0001000000 10 {}",
        dc10(0).repeat(3)
    );
    let mut raw = bs(2, &codes);
    raw[4] = 0;
    let natural = MdecDecoder.decode(&raw, 16, 16).unwrap();
    let zigzag = MdecDecoder.decode(&bs(2, &codes), 16, 16).unwrap();
    assert_ne!(natural[0], natural[4]);
    assert_eq!(natural[0], natural[16 * 4]);
    assert_eq!(zigzag[0], zigzag[4]);
    assert_ne!(zigzag[0], zigzag[16 * 4]);
}

#[test]
fn malformed_mdec_input_is_bounded_and_never_panics() {
    let mut seed = 0x1a2b3c4du32;
    for _ in 0..256 {
        let mut data = vec![0; 128];
        for value in &mut data {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            *value = seed as u8;
        }
        data[2..4].copy_from_slice(&0x3800u16.to_le_bytes());
        data[4..6].copy_from_slice(&31u16.to_le_bytes());
        data[6..8].copy_from_slice(&2u16.to_le_bytes());
        if let Ok(pixels) = MdecDecoder.decode(&data, 16, 16) {
            assert_eq!(pixels.len(), 1024);
        }
    }
}
#[test]
fn mdec_rejects_truncation_versions_runs_and_dc_overflow() {
    let good = solid_v2([0; 6]);
    for length in 0..good.len() {
        assert!(MdecDecoder.decode(&good[..length], 16, 16).is_err());
    }
    assert!(matches!(
        MdecDecoder.decode(&bs(1, "0"), 16, 16),
        Err(Error::Unsupported(_))
    ));
    let run = format!(
        "0000000000 000001 111111 0000000001 10 {}",
        dc10(0).repeat(5)
    );
    assert_eq!(
        MdecDecoder.decode(&bs(2, &run), 16, 16),
        Err(Error::Invalid("AC run overflow"))
    );
    assert!(
        MdecDecoder
            .decode(&bs(3, "11111110 11111111"), 16, 16)
            .is_err()
    );
}
#[test]
fn spu_nibble_order_signed_predictors_and_reset() {
    let mut decoder = SpuDecoder::default();
    let mut block = [0; 16];
    block[0] = 12;
    block[2..].fill(0xf7);
    let pcm = decoder.decode_block(&block).unwrap().pcm;
    assert_eq!(&pcm[..4], &[7, -1, 7, -1]);
    block[0] = 0x1c;
    block[2..].fill(0);
    assert_eq!(decoder.decode_block(&block).unwrap().pcm[0], -1);
    decoder.reset();
    assert_eq!(decoder.decode_block(&block).unwrap().pcm, [0; 28]);
    for filter in 0..5 {
        block[0] = filter << 4;
        assert!(decoder.decode_block(&block).is_ok());
    }
    block[0] = 0x50;
    assert!(decoder.decode_block(&block).is_err());
    assert!(decoder.decode_block(&block[..15]).is_err());
}
#[test]
fn spu_saturates_and_reports_all_flags() {
    let mut decoder = SpuDecoder::default();
    let mut block = [0x77; 16];
    block[0] = 0x20;
    block[1] = 7;
    let result = decoder.decode_block(&block).unwrap();
    assert!(result.end && result.repeat && result.loop_start);
    assert_eq!(result.pcm[27], 32767);
}
#[test]
fn vag_big_endian_header_and_loop_sample_range() {
    let mut vag = vec![0; 48 + 48];
    vag[..4].copy_from_slice(b"VAGp");
    vag[12..16].copy_from_slice(&48u32.to_be_bytes());
    vag[16..20].copy_from_slice(&22050u32.to_be_bytes());
    vag[49] = 4;
    vag[65] = 3;
    let sample = decode_vag(&vag).unwrap();
    assert_eq!(sample.sample_rate, 22050);
    assert_eq!(sample.pcm.len(), 56);
    assert_eq!(sample.loop_range, Some(0..56));
    assert!(sample.ended);
    assert!(decode_vag(&vag[..79]).is_err());
}
#[test]
fn xa_mono_stereo_both_rates_and_both_bit_depths() {
    for coding in [0, 1, 4, 5, 16, 17, 20, 21] {
        let format = AudioFormat::from_xa_coding(coding).unwrap();
        let data = xa_data(
            format.adpcm_bits_per_sample,
            7,
            if format.adpcm_bits_per_sample == 4 {
                12
            } else {
                8
            },
        );
        let pcm = XaDecoder::default().decode(&data, format).unwrap();
        assert_eq!(
            pcm.len(),
            if format.adpcm_bits_per_sample == 4 {
                4032
            } else {
                2016
            }
        );
        assert!(pcm.iter().all(|&s| s == 7));
        assert_eq!(
            format.sample_rate,
            if coding & 4 == 0 { 37800 } else { 18900 }
        );
    }
}
#[test]
fn xa_stereo_deinterleave_and_mono_unit_order() {
    let mut data = xa_data(4, 0, 12);
    for group in data.as_chunks_mut::<128>().0 {
        for row in group[16..].as_chunks_mut::<4>().0 {
            row.copy_from_slice(&[0x21, 0x43, 0x65, 0x87]);
        }
    }
    let stereo = XaDecoder::default()
        .decode(&data, AudioFormat::from_xa_coding(1).unwrap())
        .unwrap();
    assert_eq!(&stereo[..4], &[1, 2, 1, 2]);
    assert_eq!(&stereo[56..60], &[3, 4, 3, 4]);
    assert_eq!(&stereo[168..172], &[7, -8, 7, -8]);
    let mono = XaDecoder::default()
        .decode(&data, AudioFormat::from_xa_coding(0).unwrap())
        .unwrap();
    assert_eq!(&mono[28..32], &[2; 4]);
}
#[test]
fn xa_invalid_sector_does_not_corrupt_predictors() {
    let format = AudioFormat::from_xa_coding(1).unwrap();
    let mut decoder = XaDecoder::default();
    let good = xa_data(4, 7, 12);
    decoder.decode(&good, format).unwrap();
    let mut bad = xa_data(4, 0, 0x1c);
    bad[2304 - 128 + 4] = 0x40;
    assert!(decoder.decode(&bad, format).is_err());
    let good = xa_data(4, 0, 0x1c);
    assert_eq!(decoder.decode(&good, format).unwrap()[0], 7);
    assert!(decoder.decode(&good[..100], format).is_err());
    assert!(AudioFormat::from_xa_coding(2).is_err());
    assert!(AudioFormat::from_xa_coding(64).is_err());
}
#[test]
fn sector_layouts_and_duplicate_subheaders() {
    let user = str_sector(1, 0, 1, &solid_v2([0; 6]), solid_v2([0; 6]).len());
    assert!(
        Sector::parse(&user)
            .unwrap()
            .str_header()
            .unwrap()
            .is_some()
    );
    let short = xa_sector(1, 2, 1, &xa_data(4, 0, 12));
    let mut raw = vec![0; 2352];
    raw[1..11].fill(255);
    raw[15] = 2;
    raw[16..].copy_from_slice(&short);
    assert!(Sector::parse(&raw).unwrap().is_audio());
    assert_eq!(Sector::parse(&short).unwrap().payload.len(), 2324);
    raw[20] = 9;
    assert!(Sector::parse(&raw).is_err());
    assert!(Sector::parse(&[0; 2352]).is_err());
}
#[test]
fn str_chunks_out_of_order_duplicate_and_exact_pts() {
    let data = solid_v2([0; 6]);
    let first = str_sector(1, 0, 2, &data, data.len());
    let second = str_sector(1, 1, 2, &[], data.len());
    let mut decoder = StreamDecoder::new(StreamConfig::default()).unwrap();
    assert!(decoder.feed_sector(&second, 11).unwrap().is_none());
    assert!(decoder.feed_sector(&second, 11).unwrap().is_none());
    let Some(Event::Video(frame)) = decoder.feed_sector(&first, 10).unwrap() else {
        panic!()
    };
    assert_eq!(frame.pts.ticks, 0);
    assert_eq!(frame.duration.seconds(), 1.0 / 15.0);
    assert_eq!(frame.first_sector, 10);
    assert_eq!(frame.last_sector, 11);
    let next = str_sector(3, 0, 1, &data, data.len());
    let Some(Event::Video(frame)) = decoder.feed_sector(&next, 30).unwrap() else {
        panic!()
    };
    assert_eq!(frame.pts.ticks, 2); // Preserve a missing frame's time instead of speeding playback.
    assert!(decoder.finish().is_ok());
    assert!(decoder.feed_sector(&next, 31).is_err());
}
#[test]
fn str_incomplete_and_conflicting_headers_are_errors() {
    let data = solid_v2([0; 6]);
    let sector = str_sector(1, 0, 2, &data, data.len());
    let mut decoder = StreamDecoder::new(StreamConfig::default()).unwrap();
    decoder.feed_sector(&sector, 1).unwrap();
    assert_eq!(decoder.finish(), Err(Error::IncompleteFrame(1)));
    let mut bad = sector.clone();
    bad[32] ^= 1;
    assert!(decoder.feed_sector(&bad, 1).is_err());
    let next = str_sector(2, 0, 1, &data, data.len());
    assert!(matches!(
        decoder.feed_sector(&next, 2),
        Err(Error::IncompleteFrame(1))
    ));
    decoder.reset();
    assert!(decoder.feed_sector(&next, 2).is_ok());
    bad[6..8].copy_from_slice(&257u16.to_le_bytes());
    assert!(StrHeader::parse(&bad).is_err());
}
#[test]
fn xa_stream_channels_independent_and_timestamps_continuous() {
    let mut decoder = StreamDecoder::new(StreamConfig::default()).unwrap();
    let first = xa_sector(1, 0, 1, &xa_data(4, 7, 12));
    let other = xa_sector(1, 1, 1, &xa_data(4, 0, 0x1c));
    let next = xa_sector(1, 0, 1, &xa_data(4, 0, 0x1c));
    decoder.feed_sector(&first, 0).unwrap();
    let Some(Event::Audio(a)) = decoder.feed_sector(&other, 1).unwrap() else {
        panic!()
    };
    assert_eq!(a.pcm[0], 0);
    assert_eq!(a.pts.ticks, 0);
    let Some(Event::Audio(a)) = decoder.feed_sector(&next, 8).unwrap() else {
        panic!()
    };
    assert_eq!(a.pcm[0], 7);
    assert_eq!(a.pts.ticks, 2016);
    assert_eq!(a.pts.timescale, 37800);
    assert_eq!(a.pts.seconds(), 8.0 / 150.0);
}
#[test]
fn stream_filters_and_nonzero_rational_rate() {
    let config = StreamConfig {
        audio_filter: Some((2, 3)),
        frame_rate: FrameRate {
            numerator: 30000,
            denominator: 1001,
        },
        ..StreamConfig::default()
    };
    let mut decoder = StreamDecoder::new(config).unwrap();
    assert!(
        decoder
            .feed_sector(&xa_sector(1, 3, 1, &xa_data(4, 0, 12)), 0)
            .unwrap()
            .is_none()
    );
    let data = solid_v2([0; 6]);
    let Some(Event::Video(f)) = decoder
        .feed_sector(&str_sector(1, 0, 1, &data, data.len()), 0)
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(f.duration.ticks, 1001);
    assert_eq!(f.duration.timescale, 30000);
    assert!(
        StreamDecoder::new(StreamConfig {
            frame_rate: FrameRate {
                numerator: 0,
                denominator: 1
            },
            ..config
        })
        .is_err()
    );
}
