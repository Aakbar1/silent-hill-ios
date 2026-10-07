use crate::{Error, le16, tables::*};
use std::sync::OnceLock;

const ZIGZAG: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20,
    13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59,
    52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];

// Standard PSX cosine matrix (signed Q15); see psx-spx MDEC specifications.
const COS: [[i64; 8]; 8] = [
    [23170, 23170, 23170, 23170, 23170, 23170, 23170, 23170],
    [32138, 27245, 18204, 6392, -6393, -18205, -27246, -32139],
    [30273, 12539, -12540, -30274, -30274, -12540, 12539, 30273],
    [27245, -6393, -32139, -18205, 18204, 32138, 6392, -27246],
    [23170, -23171, -23171, 23170, 23170, -23171, -23171, 23170],
    [18204, -32139, 6392, 27245, -27246, -6393, 32138, -18205],
    [12539, -30274, 30273, -12540, -12540, 30273, -30274, 12539],
    [6392, -18205, 27245, -32139, 32138, -27246, 18204, -6393],
];

#[derive(Clone, Copy, Default)]
struct Vlc {
    bits: u8,
    run: u8,
    level: i16,
}
fn lookup() -> &'static [Vlc] {
    static TABLE: OnceLock<Box<[Vlc]>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = vec![Vlc::default(); 65536];
        for (i, &(code, bits)) in AC_VLC.iter().enumerate() {
            let start = (code as usize) << (16 - bits);
            let v = Vlc {
                bits,
                run: if i < 111 { AC_RUN[i] as u8 } else { 0 },
                level: if i < 111 {
                    AC_LEVEL[i] as i16
                } else if i == 111 {
                    -1
                } else {
                    -2
                },
            };
            table[start..start + (1usize << (16 - bits))].fill(v);
        }
        table.into_boxed_slice()
    })
}

struct Bits<'a> {
    data: &'a [u8],
    position: usize,
}
impl Bits<'_> {
    // MSB-first bits in little-endian 16-bit words. Peek zero-pads, read does not.
    fn peek(&self, count: u8) -> u32 {
        let offset = (self.position / 16) * 2;
        let word = |at| self.data.get(at..at + 2).map_or(0, |b| le16(b, 0) as u32);
        let value = (word(offset) << 16) | word(offset + 2);
        (value >> (32 - (self.position % 16) - count as usize)) & ((1u32 << count) - 1)
    }
    fn skip(&mut self, count: u8) -> Result<(), Error> {
        if self.position + count as usize > self.data.len() * 8 {
            return Err(Error::Truncated);
        }
        self.position += count as usize;
        Ok(())
    }
    fn read(&mut self, count: u8) -> Result<u32, Error> {
        let value = self.peek(count);
        self.skip(count)?;
        Ok(value)
    }
    fn signed(&mut self, count: u8) -> Result<i32, Error> {
        Ok((self.read(count)? as i32) << (32 - count) >> (32 - count))
    }
    fn dc(&mut self, luma: bool) -> Result<i32, Error> {
        let (codes, lengths) = if luma {
            (&DC_LUMA_CODE, &DC_LUMA_BITS)
        } else {
            (&DC_CHROMA_CODE, &DC_CHROMA_BITS)
        };
        for (size, (&code, &length)) in codes.iter().zip(lengths).enumerate() {
            if self.peek(length) == code as u32 {
                self.skip(length)?;
                if size == 0 {
                    return Ok(0);
                }
                let value = self.read(size as u8)? as i32;
                return Ok(if value & (1 << (size - 1)) != 0 {
                    value
                } else {
                    value - ((1 << size) - 1)
                } * 4);
            }
        }
        Err(Error::Invalid("v3 DC VLC"))
    }
}

/// Scalar, separable integer IDCT with contiguous output rows, suitable for SIMD.
/// Uses the standard PSX cosine matrix and rounds once after both Q15 passes.
fn idct(input: &[i32; 64]) -> [i32; 64] {
    // PORT: Standard cosine IDCT rounds after both passes; exact MDEC silicon
    // rounding is not established. Pixel differences are documented in state.
    if input[1..].iter().all(|&v| v == 0) {
        return [((input[0] as i64 * COS[0][0] * COS[0][0] + (1 << 31)) >> 32) as i32; 64];
    }
    let mut temp = [[0i64; 8]; 8];
    for k in 0..8 {
        for x in 0..8 {
            let value = input[x + k * 8] as i64;
            if value != 0 {
                for (y, row) in temp.iter_mut().enumerate() {
                    row[x] += value * COS[k][y];
                }
            }
        }
    }
    let mut output = [0; 64];
    for (y, row) in temp.iter().enumerate() {
        for x in 0..8 {
            let sum: i64 = (0..8).map(|k| row[k] * COS[k][x]).sum();
            output[y * 8 + x] = ((sum + (1 << 31)) >> 32) as i32;
        }
    }
    output
}

/// Decoder for Sony BS v2/v3 bitstreams embedded in STR frames.
/// Each call resets v3 DC predictors; frame decoding requires no other frames.
#[derive(Debug, Default)]
pub struct MdecDecoder;
impl MdecDecoder {
    /// Return row-major RGBA8. Macroblocks are stored column-first on disc.
    pub fn decode(&self, data: &[u8], width: u16, height: u16) -> Result<Vec<u8>, Error> {
        if data.len() < 8 {
            return Err(Error::Truncated);
        }
        if le16(data, 2) != 0x3800 {
            return Err(Error::Invalid("BS magic"));
        }
        let qscale = le16(data, 4) as i32;
        let version = le16(data, 6);
        if !matches!(version, 2 | 3) {
            return Err(Error::Unsupported("BS version (expected v2/v3)"));
        }
        if qscale > 63
            || width == 0
            || height == 0
            || width > 1024
            || height > 512
            || !data.len().is_multiple_of(2)
        {
            return Err(Error::Invalid("BS dimensions/quantizer/length"));
        }
        let mut bits = Bits {
            data: &data[8..],
            position: 0,
        };
        let table = lookup();
        let mut previous_dc = [0; 3];
        let mut rgba = vec![0; width as usize * height as usize * 4];
        for mx in 0..(width as usize).div_ceil(16) {
            for my in 0..(height as usize).div_ceil(16) {
                let mut blocks = [[0; 64]; 6];
                for (n, block) in blocks.iter_mut().enumerate() {
                    let dc = if version == 2 {
                        bits.signed(10)?
                    } else {
                        let component = n.min(2);
                        previous_dc[component] += bits.dc(n >= 2)?;
                        if !(-512..512).contains(&previous_dc[component]) {
                            return Err(Error::Invalid("v3 DC overflow"));
                        }
                        previous_dc[component]
                    };
                    let mut coefficients = [0; 64];
                    coefficients[0] = (dc * 2).clamp(-1024, 1023);
                    let mut position = 0usize;
                    loop {
                        let vlc = table[bits.peek(16) as usize];
                        if vlc.bits == 0 {
                            return Err(Error::Invalid("AC VLC"));
                        }
                        bits.skip(vlc.bits)?;
                        if vlc.level == -2 {
                            break;
                        }
                        let (run, level) = if vlc.level == -1 {
                            (bits.read(6)? as usize, bits.signed(10)?)
                        } else {
                            (
                                vlc.run as usize,
                                if bits.read(1)? == 0 {
                                    vlc.level as i32
                                } else {
                                    -(vlc.level as i32)
                                },
                            )
                        };
                        position += run + 1;
                        if position > 63 {
                            return Err(Error::Invalid("AC run overflow"));
                        }
                        let index = if qscale == 0 {
                            position
                        } else {
                            ZIGZAG[position]
                        };
                        coefficients[index] = if qscale == 0 {
                            level * 2
                        } else {
                            (level * QUANT[index] * qscale + 4) >> 3
                        }
                        .clamp(-1024, 1023);
                    }
                    *block = idct(&coefficients);
                }
                for y in 0..16 {
                    let py = my * 16 + y;
                    if py >= height as usize {
                        break;
                    }
                    for x in 0..16 {
                        let px = mx * 16 + x;
                        if px >= width as usize {
                            break;
                        }
                        let chroma = (y / 2) * 8 + x / 2;
                        let cr = blocks[0][chroma];
                        let cb = blocks[1][chroma];
                        let yy = blocks[2 + (y / 8) * 2 + x / 8][(y % 8) * 8 + x % 8];
                        let rgb = [
                            yy + ((91881 * cr + 32768) >> 16),
                            yy + ((-22525 * cb - 46812 * cr + 32768) >> 16),
                            yy + ((116130 * cb + 32768) >> 16),
                        ];
                        let at = (py * width as usize + px) * 4;
                        for (channel, value) in rgb.into_iter().enumerate() {
                            // MDEC narrows to signed 9 bits before saturating to 8.
                            let signed9 = (value << 23) >> 23;
                            rgba[at + channel] = (signed9.clamp(-128, 127) + 128) as u8;
                        }
                        rgba[at + 3] = 255;
                    }
                }
            }
        }
        Ok(rgba)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn idct_dc_and_ac() {
        let mut block = [0; 64];
        block[0] = 800;
        assert_eq!(idct(&block), [100; 64]);
        block[1] = 64;
        let output = idct(&block);
        assert!(output[0] > 100 && output[7] < 100);
        assert_eq!(&output[..8], &output[8..16]);
    }
    #[test]
    fn bit_order_and_truncation() {
        let mut bits = Bits {
            data: &[0x34, 0x12, 0xcd, 0xab],
            position: 0,
        };
        assert_eq!(bits.read(12), Ok(0x123));
        assert_eq!(bits.read(12), Ok(0x4ab));
        assert_eq!(bits.read(8), Ok(0xcd));
        assert_eq!(bits.read(1), Err(Error::Truncated));
    }
}
