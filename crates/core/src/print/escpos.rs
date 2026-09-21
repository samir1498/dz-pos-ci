//! The 80 mm ticket as ESC/POS bytes.
//!
//! Same document, same words as `render_ticket`. The HTML page is what a
//! driver prints on A4 paper and what the e2e photographs; this is what a
//! cheap thermal head eats. Tests pin a human-readable dump of those bytes
//! (`fixtures/print/ticket_80mm_escpos/`), the same golden rule as the HTML
//! tickets. A real printer is not required to know they are right.
//!
//! Two senders move the bytes without a driver: one to a file (a spool
//! file, a USB-serial device path), one over TCP to a network printer on
//! port 9100 or to `escpos-emulator` for a look without hardware. USB is
//! still not wired.

use crate::error::CoreError;
use crate::lang::Lang;
use crate::models::document::Document;
use crate::money::Regime;
use crate::print::png;
use crate::print::raster;
use crate::print::ticket::{self, Align, Item, TicketView};

const ESC: u8 = 0x1b;
const GS: u8 = 0x1d;

/// `GS v 0` carries the band's height in two bytes, so a band could be
/// 65 535 rows; the reason to keep it far below that is the head's own
/// buffer. Cheap heads take a few kilobytes at a time, and a band that
/// overruns the buffer prints a torn image. 60 000 bytes is under the
/// 64 KB the command can address and divides into whole rows at every
/// width we send.
const MAX_BAND_BYTES: usize = 60_000;

/// ESC/POS bytes for the 80 mm ticket. Same refusal as the HTML renderer
/// when an IFU document carries a TVA recap.
pub fn render_ticket_escpos(doc: &Document, lang: Lang) -> Result<Vec<u8>, CoreError> {
    if doc.regime == Regime::Ifu && !doc.totals.tva_by_rate.is_empty() {
        return Err(CoreError::render(
            "an IFU document carries a TVA recap and has no printable form",
        ));
    }
    Ok(encode(&ticket::view(doc, lang)))
}

/// The bytes as a reviewer reads them: commands on their own line, text
/// as it will sit on the paper, `<raster WxH>` for a band of dots.
/// Regenerated with `UPDATE_GOLDENS=1`.
pub fn dump_ticket_escpos(bytes: &[u8]) -> String {
    dump(bytes)
}

/// The raster bands in `bytes`, decoded back into a bitmap and written as a
/// PNG. `None` when the bytes carry no raster band, which is every ticket
/// down the text path.
///
/// This is the other half of the dump: `<raster 576x1000>` says a bitmap is
/// there and nothing about what it says, and the whole point of drawing
/// Arabic is that a reviewer can look at it. The PNG goes beside the `.txt`
/// goldens (`fixtures/print/ticket_80mm_escpos/ar.png`) and is opened, not
/// read as text.
pub fn dump_ticket_escpos_png(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut width = 0usize;
    let mut rows: Vec<u8> = Vec::new();
    let mut height = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if let Some((band_width, band_height, next, data)) = band_with_data(bytes, i) {
            if rows.is_empty() {
                width = band_width;
            } else if band_width != width {
                return None;
            }
            rows.extend_from_slice(data);
            height = height.saturating_add(band_height);
            i = next;
            continue;
        }
        i = i.saturating_add(1);
    }
    (!rows.is_empty()).then(|| png::one_bit(width, height, &rows))
}

/// A `GS v 0` band starting at `at`: its width in dots, its height in rows,
/// and where the next byte after it sits.
fn band_at(bytes: &[u8], at: usize) -> Option<(usize, usize, usize)> {
    band_with_data(bytes, at).map(|(width, height, next, _)| (width, height, next))
}

fn band_with_data(bytes: &[u8], at: usize) -> Option<(usize, usize, usize, &[u8])> {
    let header = bytes.get(at..at.checked_add(8)?)?;
    if header[0] != GS || header[1] != b'v' || header[2] != b'0' {
        return None;
    }
    let stride = usize::from(u16::from_le_bytes([header[4], header[5]]));
    let height = usize::from(u16::from_le_bytes([header[6], header[7]]));
    let start = at.checked_add(8)?;
    let end = start.checked_add(stride.checked_mul(height)?)?;
    let data = bytes.get(start..end)?;
    Some((stride.checked_mul(8)?, height, end, data))
}

/// Write the ticket bytes to a file: a spool file, a USB-serial device
/// path, or anything else that eats bytes from the filesystem. Fails
/// closed on a render refusal, so no file is left holding half a ticket:
/// the bytes are fully rendered before the first write.
pub fn write_ticket_escpos_to_file(
    doc: &Document,
    lang: Lang,
    path: &std::path::Path,
) -> Result<(), CoreError> {
    let bytes = render_ticket_escpos(doc, lang)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

/// Send the ticket bytes to a network printer over TCP, the way a receipt
/// is pushed to a printer on port 9100 or to `escpos-emulator` for a look
/// without hardware. The address is `host:port`; a refused connection is
/// an error, never a silent drop. Like the file sender, this renders first
/// and connects second, so a refused ticket never opens a connection.
pub fn send_ticket_escpos_tcp(doc: &Document, lang: Lang, addr: &str) -> Result<(), CoreError> {
    use std::io::Write as _;
    let bytes = render_ticket_escpos(doc, lang)?;
    let mut stream = std::net::TcpStream::connect(addr)?;
    stream.write_all(&bytes)?;
    Ok(())
}

/// The ticket's own list of things to print, one byte sequence per item.
/// The list is built in `ticket::items` and this only spells it out, which
/// is why the nine goldens did not move when the raster path landed.
fn encode(view: &TicketView) -> Vec<u8> {
    let mut out = Buf::new();
    out.init();
    for item in &ticket::items(view) {
        match item {
            Item::Align(align) => out.align(*align),
            Item::Bold(on) => out.bold(*on),
            Item::Line(text) => out.line(text),
            Item::Feed(lines) => out.feed(*lines),
            Item::Cut => out.cut(),
        }
    }
    out.into_bytes()
}

/// The same ticket as dots. A cheap head has no single-byte table for
/// Arabic and prints a box per byte, so the Arabic ticket is drawn from the
/// very same items and sent as a raster
/// (`context/plans/20260921-arabic-on-a-cheap-thermal-head.md`).
///
/// `width_dots` is the head's width: 576 on the common 80 mm head, 384 on a
/// 58 mm one. No codepage is selected because nothing here is text on the
/// wire.
pub fn render_ticket_escpos_raster(
    doc: &Document,
    lang: Lang,
    width_dots: u32,
) -> Result<Vec<u8>, CoreError> {
    let drawn = draw_ticket_raster(doc, lang, width_dots)?;
    let mut out = Buf::new();
    out.init_raster();
    out.raster(&drawn.bitmap)?;
    out.cut();
    Ok(out.into_bytes())
}

/// The bitmap on its own, beside the lines it was drawn from. The bytes
/// above are what a head is sent; this is what the tests read, because the
/// rule they keep is about the strings and the dots they took, not about
/// the command that carries them.
pub fn draw_ticket_raster(
    doc: &Document,
    lang: Lang,
    width_dots: u32,
) -> Result<raster::Drawn, CoreError> {
    if doc.regime == Regime::Ifu && !doc.totals.tva_by_rate.is_empty() {
        return Err(CoreError::render(
            "an IFU document carries a TVA recap and has no printable form",
        ));
    }
    raster::draw(&ticket::items(&ticket::view(doc, lang)), lang, width_dots)
}

struct Buf {
    bytes: Vec<u8>,
}

impl Buf {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    fn init(&mut self) {
        self.bytes.extend_from_slice(&[ESC, b'@']);
        // PC858 (code page 19) is the one the cheap 80 mm heads actually
        // carry for French: it is ISO 8859-15 with the eight bytes at
        // 0xA4/0xA6/0xA8/0xB4/0xB8/0xBC/0xBD/0xBE mapped to € Š š Ž ž Œ œ Ÿ
        // (see the emulator's ISO_8859_15_MAP and Epson's ESC t table).
        // The emulator ignores ESC t but a real head does not, and the dump
        // below skips the two bytes so the golden stays readable either way.
        self.bytes.extend_from_slice(&[ESC, b't', 19]);
    }

    /// The raster path selects no codepage: every dot it sends is a dot,
    /// and the one thing it does want is the left margin, so a bitmap the
    /// width of the head is not shifted by an alignment the last job left
    /// behind.
    fn init_raster(&mut self) {
        self.bytes.extend_from_slice(&[ESC, b'@']);
        self.align(Align::Left);
    }

    /// `GS v 0`: raster bit image, mode 0 (normal, no doubling). The four
    /// header bytes after the mode are the band's width in bytes and its
    /// height in rows, each little-endian, then the rows themselves, eight
    /// dots to a byte with the leftmost dot in the high bit and 1 meaning
    /// black — the same packing `raster::Bitmap` holds, so the bytes are
    /// copied and not rebuilt.
    fn raster(&mut self, bitmap: &raster::Bitmap) -> Result<(), CoreError> {
        let stride = bitmap.stride();
        let rows_per_band = MAX_BAND_BYTES.checked_div(stride).unwrap_or(0).max(1);
        let x = u16::try_from(stride)
            .map_err(|_| CoreError::render("a raster row wider than 65 535 bytes"))?;
        for band in bitmap.rows().chunks(stride.saturating_mul(rows_per_band)) {
            let height = band.len().checked_div(stride).unwrap_or(0);
            let y = u16::try_from(height)
                .map_err(|_| CoreError::render("a raster band taller than 65 535 rows"))?;
            self.bytes.extend_from_slice(&[GS, b'v', b'0', 0]);
            self.bytes.extend_from_slice(&x.to_le_bytes());
            self.bytes.extend_from_slice(&y.to_le_bytes());
            self.bytes.extend_from_slice(band);
        }
        Ok(())
    }

    fn align(&mut self, align: Align) {
        let n = match align {
            Align::Left => 0,
            Align::Center => 1,
        };
        self.bytes.extend_from_slice(&[ESC, b'a', n]);
    }

    fn bold(&mut self, on: bool) {
        self.bytes.extend_from_slice(&[ESC, b'E', u8::from(on)]);
    }

    fn feed(&mut self, lines: u8) {
        self.bytes.extend_from_slice(&[ESC, b'd', lines]);
    }

    fn cut(&mut self) {
        self.bytes.extend_from_slice(&[GS, b'V', 0]);
    }

    fn text(&mut self, s: &str) {
        // The head eats one byte per column (`ticket::WIDTH` = 42), not one UTF-8
        // scalar per column: "Café" is 4 columns, not 5 bytes. For French the
        // wire is ISO 8859-15 (the emulator's `String.fromCharCode(byte)` with
        // the eight 0xA4…0xBE overrides), so every char that fits there is one
        // byte; the narrow no-break space U+202F the money formatter uses is a
        // plain space. For Arabic script (and any other scalar outside 0xFF)
        // there is no single-byte table, so it is sent as UTF-8 — the dump
        // below decodes both, trying UTF-8 first and falling back to the
        // single-byte table, so "Café" stays 4 bytes and "قهوة" stays
        // readable.
        for ch in s.chars() {
            match ch {
                '\u{202f}' => self.bytes.push(b' '),
                '\u{20ac}' => self.bytes.push(0xA4),
                '\u{0160}' => self.bytes.push(0xA6),
                '\u{0161}' => self.bytes.push(0xA8),
                '\u{017d}' => self.bytes.push(0xB4),
                '\u{017e}' => self.bytes.push(0xB8),
                '\u{0152}' => self.bytes.push(0xBC),
                '\u{0153}' => self.bytes.push(0xBD),
                '\u{0178}' => self.bytes.push(0xBE),
                c if (c as u32) <= 0xFF => self.bytes.push(c as u8),
                c => {
                    let mut buf = [0u8; 4];
                    let encoded = c.encode_utf8(&mut buf);
                    self.bytes.extend_from_slice(encoded.as_bytes());
                }
            }
        }
    }

    fn lf(&mut self) {
        self.bytes.push(b'\n');
    }

    fn line(&mut self, s: &str) {
        self.text(s);
        self.lf();
    }
}

fn dump(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == ESC && i + 1 < bytes.len() {
            match bytes[i + 1] {
                b'@' => {
                    out.push_str("<init>\n");
                    i += 2;
                    continue;
                }
                b'a' if i + 2 < bytes.len() => {
                    let who = match bytes[i + 2] {
                        0 => "left",
                        1 => "center",
                        2 => "right",
                        n => {
                            out.push_str(&format!("<align {n}>\n"));
                            i += 3;
                            continue;
                        }
                    };
                    out.push_str(&format!("<align {who}>\n"));
                    i += 3;
                    continue;
                }
                b'E' if i + 2 < bytes.len() => {
                    out.push_str(if bytes[i + 2] == 0 {
                        "<bold off>\n"
                    } else {
                        "<bold on>\n"
                    });
                    i += 3;
                    continue;
                }
                b'd' if i + 2 < bytes.len() => {
                    out.push_str(&format!("<feed {}>\n", bytes[i + 2]));
                    i += 3;
                    continue;
                }
                b't' if i + 2 < bytes.len() => {
                    out.push_str(&format!("<codepage {}>\n", bytes[i + 2]));
                    i += 3;
                    continue;
                }
                _ => {}
            }
        }
        if bytes[i] == GS && i + 2 < bytes.len() && bytes[i + 1] == b'V' {
            out.push_str("<cut>\n");
            i += 3;
            continue;
        }
        // A raster band, named by the dots it covers and skipped whole.
        // This has to come before the text fallback below: a band's bytes
        // are dots, and a dot pattern that happens to be valid UTF-8 would
        // otherwise be read out as words nobody printed.
        if let Some((width, height, next)) = band_at(bytes, i) {
            out.push_str(&format!("<raster {width}x{height}>\n"));
            i = next;
            continue;
        }
        if bytes[i] == b'\n' {
            out.push('\n');
            i += 1;
            continue;
        }
        // The wire is mostly ISO 8859-15 (one byte per column, so "Café"
        // is 0x43 0x61 0x66 0xE9, not 0xC3 0xA9), but Arabic and any other
        // scalar outside 0xFF is sent as UTF-8. Try UTF-8 first so an Arabic
        // word decodes as one char; a single 0xE9 is invalid UTF-8 on its
        // own and falls through to the single-byte table, which is exactly
        // how the head and the emulator both see it. The first eyeball
        // showed the UTF-8 bytes split into "CafÃ©" because the French path
        // was still UTF-8.
        if let Ok(rest) = std::str::from_utf8(&bytes[i..]) {
            if let Some(ch) = rest.chars().next() {
                let len = ch.len_utf8();
                if len > 1 {
                    out.push(ch);
                    i += len;
                    continue;
                }
            }
        } else if let Err(err) = std::str::from_utf8(&bytes[i..]) {
            if err.valid_up_to() > 0 {
                if let Ok(rest) = std::str::from_utf8(&bytes[i..i + err.valid_up_to()]) {
                    if let Some(ch) = rest.chars().next() {
                        let len = ch.len_utf8();
                        if len > 1 {
                            out.push(ch);
                            i += len;
                            continue;
                        }
                    }
                }
            }
        }
        let ch = match bytes[i] {
            0xA4 => '€',
            0xA6 => 'Š',
            0xA8 => 'š',
            0xB4 => 'Ž',
            0xB8 => 'ž',
            0xBC => 'Œ',
            0xBD => 'œ',
            0xBE => 'Ÿ',
            b => b as char,
        };
        out.push(ch);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
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
}
