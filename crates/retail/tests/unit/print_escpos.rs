use super::{dump, raster, Buf};

#[test]
fn the_dump_names_the_commands_the_buffer_wrote() {
    let mut buf = Buf::new();
    buf.init();
    buf.align(super::Align::Center);
    buf.bold(true);
    buf.line("Café");
    buf.bold(false);
    buf.cut();
    assert_eq!(
        dump(&buf.into_bytes()),
        "<init>\n<codepage 19>\n<align center>\n<bold on>\nCafé\n<bold off>\n<cut>\n"
    );
}

/// The raster header, byte for byte, written from Epson's command
/// reference and not from the encoder.
///
/// `GS v 0` is `1D 76 30 m xL xH yL yH`: the mode, then the band's
/// width **in bytes** and its height **in rows**, each of them two
/// bytes, low byte first. Every one of those four numbers can be
/// swapped for another and still survive a round trip through the
/// decoder beside it, which is why the expectation here is a literal
/// and not a re-read: 16 dots wide is 2 bytes a row, so `xL xH` is
/// `02 00` and not `10 00`, and 3 rows is `03 00` and not `00 03`.
#[test]
fn a_raster_band_carries_its_width_in_bytes_and_its_height_in_rows() {
    let mut bitmap = raster::Bitmap::new(16, 3);
    // A diagonal: one dot in the left byte, one in the right, one back
    // in the left, so a row that swapped ends would show.
    bitmap.set(0, 0);
    bitmap.set(8, 1);
    bitmap.set(1, 2);
    let mut buf = Buf::new();
    let Ok(()) = buf.raster(&bitmap) else {
        panic!("a 16 by 3 bitmap is not too big for one band");
    };
    assert_eq!(
        buf.into_bytes(),
        vec![
            0x1D, 0x76, 0x30, 0x00, // GS v 0, mode 0
            0x02, 0x00, // two bytes across
            0x03, 0x00, // three rows down
            0x80, 0x00, // row 0: the leftmost dot of the left byte
            0x00, 0x80, // row 1: the leftmost dot of the right byte
            0x40, 0x00, // row 2: the second dot of the left byte
        ]
    );
}
