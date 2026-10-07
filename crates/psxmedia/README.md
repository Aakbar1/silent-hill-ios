# psxmedia

Native Rust STR/MDEC, XA-ADPCM and SPU/VAG decoding. The library has **zero runtime
dependencies**, no unsafe code, no emulated devices and no platform APIs. The
`png` dependency is only for the private dump example. GPL-3.0-only, with
LGPL-2.1-or-later FFmpeg table attribution in NOTICE.md.

From this directory:

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run --release --example dump
```

The crate is temporarily a standalone workspace. The director should add it to
the parent workspace and remove the empty `[workspace]` stanza at integration.

## Integration

```rust
use psxmedia::{Event, StreamConfig, StreamDecoder};

# fn demo(sectors: impl Iterator<Item = (u64, Vec<u8>)>) -> Result<(), psxmedia::Error> {
let mut decoder = StreamDecoder::new(StreamConfig {
    video_filter: Some((0, 1)),
    audio_filter: Some((1, 1)),
    ..StreamConfig::default() // Silent Hill intro: 15 frames/s
})?;
for (lba, bytes) in sectors {
    match decoder.feed_sector(&bytes, lba)? {
        Some(Event::Video(frame)) => {
            // Upload row-major RGBA8 and schedule using frame.pts and duration.
        }
        Some(Event::Audio(packet)) => {
            // Queue interleaved i16 PCM at packet.format.sample_rate / channels.
            // packet.pts describes the first sample, rather than its arrival.
        }
        None => {}
    }
}
decoder.finish()?;
# Ok(())
# }
```

Both tracks start at presentation time zero; sector arrival includes read-ahead
and should not determine playback time. Video uses exact rational frame numbers;
XA uses exact sample counts per file/channel, retaining predictor history across
interleaved sectors. STR does not encode frame rate: supply the rate for the
selected stream. Native 37.8/18.9 kHz PCM lets the host choose its device resampler.
Missing **video frame numbers** preserve the presentation gap. Incomplete chunks,
conflicting duplicates/headers, bad VLCs, invalid dimensions and predictors are
errors. The caller must reset or instantiate a decoder after a seek/discontinuity;
audio loss cannot be inferred from XA headers alone.

Input: Mode 2/2352 with sync/mode validation, Mode 2/2336 with duplicated XA
subheaders, or 2048-byte video user data. Select one video file/channel when
multiplexed video streams are present. XA channels can all be emitted independently.
Normal video chunks use 2016 bytes after each 32-byte STR header, including when
the sector is Form 2. Assembly is bounded to 256 chunks and 1024×512 pixels.

`MdecDecoder::decode` also accepts a complete BS v2/v3 frame directly. v3 DC
predictors reset for each frame. Coefficients saturate to signed 11 bits; qscale
zero uses the PSX unquantized ordering. The standard Q15 cosine matrix is applied
in two separable passes, with DC/sparse shortcuts and contiguous arrays for future
SIMD. YCbCr is 4:2:0, macroblocks are column-first, output is cropped to dimensions.
This is visually verified decoding, **not a claim of bit-exact MDEC silicon
rounding**. No intentional gameplay changes were introduced.

`XaDecoder` supports 4- and 8-bit XA, mono/stereo, both native rates; emphasis is
explicitly unsupported. `SpuDecoder` accepts raw 16-byte SPU blocks and reports
all three flags. `decode_vag` accepts big-endian-header mono `VAGp`, decodes through
the end flag and returns the loop sample range without expanding repetitions.
SPU playback pitch, interpolation, ADSR and mixing belong to the audio lane.

## Silent Hill USA 1.1 disc locations

Evidence: the read-only decomp at commit `d9e28f8`, `docs/File Formats.md:31-39`,
`src/main/filetable.c.USA.inc:2046-2075`, `include/main/fileinfo.h:105-108`,
`src/screens/stream/stream.c:54-73,115-147,323-345`, and
`src/bodyprog/sound/sd_call.c:929-958`. LBAs below are **zero-based raw-disc
indices**, not offsets relative to `HILL.`; do not add the 150-sector pregap to
the BIN seek. The decomp adds that pregap only for CD MSF commands.

`SILENT.` contains normal data/overlays, including `SND/*.VAB` sample banks and
`SND/*.KDT` sequences. `HILL.` contains `XA/*` music, voice and movies. Neither
container has its own directory: the executable file table supplies locations.
`STREAM.BIN` is the movie overlay, using PsyQ `DecDCTvlc`/`DecDCTin` and double-speed
XA streaming. Intro selection is C1 or C2 depending on the extra option; movie
opening selection is M1. The alternate intro reads C1 with a 2060-frame limit.

| Asset | Start LBA | Exclusive end LBA | Purpose |
|---|---:|---:|---|
| XA/05_02152 | 39359 | 41511 | Music/voice bank 1 |
| XA/10_04432 | 41511 | 45943 | Music/voice bank 2 |
| XA/15_07496 | 45943 | 53439 | Music/voice bank 3 |
| XA/20_06552 | 53439 | 59991 | Music/voice bank 4 |
| XA/25_03904 | 59991 | 63895 | Music/voice bank 5 |
| XA/30_04056 | 63895 | 67951 | Music/voice bank 6 |
| XA/35_26008 | 67951 | 93959 | Music/voice bank 7 |
| XA/40_10384 | 93959 | 104343 | Music/voice bank 8 |
| XA/45_28784 | 104343 | 133127 | Music/voice bank 9 |
| XA/C1_20670 | 133127 | 153797 | Default intro |
| XA/C2_20670 | 153797 | 174467 | Alternate intro selection |
| XA/M1_03500 | 174467 | 177967 | Opening movie |

Other FMV start LBAs from the same table: M2=177967, M3=179157, M4=181727,
M5=184217, M6=187357, M7=189469, M8=191005, M9=194044, MA=195774,
MB=199364, MC=204214, MD=206144, ME=209924, Z1=213224, Z3=229404,
Z4=231744, ZC=233334, ZZ=247726. Their end is the next entry's start;
ZZ is the last entry and needs the disc/container extent from the disc lane.

`g_FileXaLoc` lists the nine bank bases. `gSDXATable[727]` in
`src/bodyprog/sound/sound_data.c:201` supplies `xaFileIdx`, sector offset, audio
length and XA file/channel filter per music/voice item. `sd_call.c` computes
`bank_base + item.sector` and selects that item’s file/channel before reading.
The first bank's first 100 sectors were verified to interleave channels 0–7,
file 1, coding 1 (37.8 kHz stereo 4-bit). Do not concatenate all channels.

The intro's first 300 frames were verified on the private BIN: 320×208, BS v2,
video file/channel 0/1 every 10 sectors, audio 1/1 every 8 sectors, first audio
sector at start+7. At 150 sectors/s this gives exactly 15 fps and 2016 samples
per 8-sector interval. No disc bytes or generated images are included here.

## Verification artifacts

`examples/dump.rs` has a minimal reader; it does not replace the disc lane's reader.
It writes only to `C:/Claude Projects/Silent Hill iOS/private/work/media/`:
300 PNGs, exact 10-second and 20-second PCM WAVs, timeline CSV and speed results.
The speed test runs three single-thread, in-memory release passes with video and
XA decoding; file I/O and PNG compression are excluded. See PROJECT_STATE.md for
the measured result, visual review and independent-reference comparison.

Format references:

- [PsyQ streaming and bitstream specification, psx-spx](https://github.com/psx-spx/psx-spx.github.io/blob/master/docs/ps1/cdr/cdromfileformats/streaming.md)
- [MDEC quantization, IDCT and colour conversion, psx-spx](https://github.com/psx-spx/psx-spx.github.io/blob/master/docs/ps1/cpu/mdec/macroblockdecodermdec.md)
- [SPU blocks and loop flags, psx-spx](https://github.com/psx-spx/psx-spx.github.io/blob/master/docs/ps1/spu/soundprocessingunitspu.md)
- [Independent native video decoder, FFmpeg n7.1](https://github.com/FFmpeg/FFmpeg/blob/n7.1/libavcodec/mdec.c)
- [Independent XA decoder, FFmpeg n7.1](https://github.com/FFmpeg/FFmpeg/blob/n7.1/libavcodec/adpcm.c)
