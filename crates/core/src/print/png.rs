//! A one-bit PNG, written by hand.
//!
//! The raster ticket's golden is a picture, because the thing being
//! reviewed is the shape of the Arabic: whether a letter took its joined
//! form, whether a ligature is right, whether the amounts line up. A text
//! dump of a bitmap says `<raster 576x1489>` and nothing else.
//!
//! Why this file exists rather than a crate: the workspace's lock carries
//! `png` only through the desktop's webview tree, so pulling it into the
//! core would add a dependency edge (and `miniz_oxide` under it) to buy one
//! function. What we need is the narrowest possible PNG — one bit per
//! pixel, greyscale, no interlace, no palette — and the format's compressed
//! stream is allowed to be *stored*: a deflate block with a length and the
//! bytes, no Huffman table, which is a dozen lines and cannot be subtly
//! wrong the way a hand-rolled compressor can. The file is therefore about
//! the size of the bitmap (72 bytes a row at 576 dots), which is nothing on
//! disk and compresses away again in git's own pack.

/// `rows` is the bitmap packed the way the head eats it: `width / 8` bytes
/// a row, the leftmost dot in the high bit, a set bit meaning a black dot.
/// PNG greyscale reads 0 as black and 1 as white, so every byte is flipped
/// on the way out.
pub(crate) fn one_bit(width: usize, height: usize, rows: &[u8]) -> Vec<u8> {
    let stride = width.div_ceil(8);
    let mut raw = Vec::with_capacity(height.saturating_mul(stride.saturating_add(1)));
    for y in 0..height {
        // Filter 0 (None). A filter that predicts from the row above would
        // pay off under a real compressor; under stored blocks it only
        // makes the bytes harder to read in a hex dump.
        raw.push(0);
        let start = y.saturating_mul(stride);
        let end = start.saturating_add(stride);
        match rows.get(start..end) {
            Some(row) => raw.extend(row.iter().map(|byte| !byte)),
            // A short `rows` is a caller bug, not a reason to panic in a
            // printing path: the missing rows come out white.
            None => raw.extend(std::iter::repeat_n(0xFF, stride)),
        }
    }

    let mut out = Vec::new();
    out.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&u32_be(width));
    ihdr.extend_from_slice(&u32_be(height));
    // depth 1, colour type 0 (greyscale), deflate, adaptive filtering, no
    // interlace: the four bytes that make this the narrowest legal PNG.
    ihdr.extend_from_slice(&[1, 0, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &zlib_stored(&raw));
    chunk(&mut out, b"IEND", &[]);
    out
}

fn u32_be(value: usize) -> [u8; 4] {
    u32::try_from(value).unwrap_or(u32::MAX).to_be_bytes()
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&u32_be(data.len()));
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc = Crc::new();
    crc.eat(kind);
    crc.eat(data);
    out.extend_from_slice(&crc.finish().to_be_bytes());
}

/// A zlib stream of stored deflate blocks: the 0x78 0x01 header (deflate,
/// 32 KB window, no preset dictionary, fastest), then `BFINAL`/`BTYPE=00`
/// blocks of at most 65 535 bytes each, then the Adler-32 of the raw bytes.
fn zlib_stored(raw: &[u8]) -> Vec<u8> {
    const BLOCK: usize = 65_535;
    let mut out = vec![0x78, 0x01];
    let mut chunks = raw.chunks(BLOCK).peekable();
    if raw.is_empty() {
        out.extend_from_slice(&[1, 0, 0, 0xFF, 0xFF]);
    }
    while let Some(block) = chunks.next() {
        let last = chunks.peek().is_none();
        out.push(u8::from(last));
        let len = u16::try_from(block.len()).unwrap_or(u16::MAX);
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(block);
    }
    out.extend_from_slice(&adler32(raw).to_be_bytes());
    out
}

fn adler32(data: &[u8]) -> u32 {
    const BASE: u32 = 65_521;
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for byte in data {
        a = a.wrapping_add(u32::from(*byte)) % BASE;
        b = b.wrapping_add(a) % BASE;
    }
    b.wrapping_shl(16) | a
}

struct Crc(u32);

impl Crc {
    const fn new() -> Self {
        Self(0xFFFF_FFFF)
    }

    fn eat(&mut self, data: &[u8]) {
        for byte in data {
            let index = usize::from((self.0 ^ u32::from(*byte)) as u8);
            self.0 = CRC_TABLE[index] ^ self.0.wrapping_shr(8);
        }
    }

    const fn finish(&self) -> u32 {
        self.0 ^ 0xFFFF_FFFF
    }
}

/// The CRC-32 table PNG's own specification prints the generator for
/// (polynomial 0xEDB88320), built at compile time so no test has to trust a
/// transcribed table.
const CRC_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut n = 0;
    while n < 256 {
        let mut c = n as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 == 1 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        table[n] = c;
        n += 1;
    }
    table
};

#[cfg(test)]
mod tests {
    use super::{adler32, one_bit, Crc};

    #[test]
    fn the_crc_and_the_adler_are_the_values_the_specifications_print() {
        // RFC 1950 and the PNG spec both use "123456789" as their check
        // string: CRC-32 is 0xCBF43926 and Adler-32 is 0x091E01DE. Written
        // from the specifications, not from this code.
        let mut crc = Crc::new();
        crc.eat(b"123456789");
        assert_eq!(crc.finish(), 0xCBF4_3926);
        assert_eq!(adler32(b"123456789"), 0x091E_01DE);
    }

    #[test]
    fn a_one_bit_png_has_the_header_a_reader_looks_for() {
        // Eight dots wide, two rows: the left half black on the first row.
        let png = one_bit(8, 2, &[0xF0, 0x00]);
        assert_eq!(
            &png[0..8],
            &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
        );
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(&png[16..24], &[0, 0, 0, 8, 0, 0, 0, 2]);
        // depth 1, greyscale, and the three zeroes after it.
        assert_eq!(&png[24..29], &[1, 0, 0, 0, 0]);
        assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
        // The whole compressed stream, written from RFC 1950 and RFC 1951
        // rather than read back out of the encoder. A substring search
        // passes a wrong NLEN, a block that forgot to mark itself final
        // and a bad zlib header alike, because none of those touch the
        // bytes it looks for.
        //
        // 78 01  zlib: deflate, 32 KB window, no preset dictionary.
        // 01     one block, BFINAL set, BTYPE 00 (stored).
        // 04 00  LEN 4, low byte first.
        // FB FF  NLEN, the one's complement of LEN (!0x0004 = 0xFFFB).
        // 00 0F  row 0: filter None, then 0xF0 inverted — a black dot is
        //        a 0 bit in a greyscale PNG.
        // 00 FF  row 1: filter None, then an all-white row.
        // 01 30 01 0F  Adler-32 of those four bytes, high half first.
        //        a = 1 + 0x00 + 0x0F + 0x00 + 0xFF = 271 = 0x010F.
        //        b = 1 + 16 + 16 + 271 = 304 = 0x0130.
        assert_eq!(
            idat(&png),
            vec![
                0x78, 0x01, 0x01, 0x04, 0x00, 0xFB, 0xFF, 0x00, 0x0F, 0x00, 0xFF, 0x01, 0x30, 0x01,
                0x0F,
            ]
        );
    }

    /// The bytes of the one IDAT chunk, found by walking the chunk list the
    /// way a decoder does: four bytes of length, four of name, the data,
    /// four of CRC.
    fn idat(png: &[u8]) -> Vec<u8> {
        let mut at = 8;
        while at + 8 <= png.len() {
            let len = u32::from_be_bytes([png[at], png[at + 1], png[at + 2], png[at + 3]]);
            let len = usize::try_from(len).unwrap_or(0);
            let name = &png[at + 4..at + 8];
            if name == b"IDAT" {
                return png[at + 8..at + 8 + len].to_vec();
            }
            at += 12 + len;
        }
        panic!("no IDAT chunk");
    }
}
