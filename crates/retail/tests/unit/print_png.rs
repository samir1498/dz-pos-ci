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
