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
use crate::print::ticket::{self, TicketView};

const ESC: u8 = 0x1b;
const GS: u8 = 0x1d;
const WIDTH: usize = 42;

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
/// as it will sit on the paper. Regenerated with `UPDATE_GOLDENS=1`.
pub fn dump_ticket_escpos(bytes: &[u8]) -> String {
    dump(bytes)
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

fn encode(view: &TicketView) -> Vec<u8> {
    let mut out = Buf::new();
    out.init();
    out.align(Align::Center);
    out.bold(true);
    out.line(view.title);
    out.bold(false);
    out.line(&view.seller.name);
    if let Some(address) = &view.seller.address {
        out.line(address);
    }
    if let Some(phone) = &view.seller.phone {
        out.line(phone);
    }
    for id in &view.seller.ids {
        out.line(&format!("{} {}", id.label, id.value));
    }
    out.line(&view.number);
    out.line(&view.issued_at);
    out.rule();
    out.align(Align::Left);
    for line in &view.lines {
        out.line(&line.name);
        let mut qty = format!("{} × {}", line.qty, line.unit_price);
        if let Some(rate) = &line.rate {
            qty = format!("{qty}  {rate}");
        }
        out.pair(&qty, &line.total);
        if let Some(discount) = &line.discount {
            out.pair(view.discount_label, &format!("-{discount}"));
        }
    }
    out.rule();
    out.pair(view.total_label, &view.total_amount);
    if let Some(discount) = &view.discount {
        out.pair(view.discount_label, &format!("-{discount}"));
    }
    for row in &view.tva_rows {
        out.pair(&format!("{} {}", row.label, row.rate), &row.amount);
    }
    if let Some(stamp) = &view.stamp {
        out.pair(view.stamp_label, stamp);
    }
    out.bold(true);
    out.pair(view.net_to_pay_label, &view.net_to_pay);
    out.bold(false);
    out.pair(view.payment_mode_label, view.payment_mode);
    if let Some(tendered) = &view.tendered {
        out.pair(view.tendered_label, tendered);
    }
    if let Some(change) = &view.change {
        out.pair(view.change_label, change);
    }
    if let Some(balance) = &view.balance {
        out.rule();
        out.line(balance.title);
        out.pair(balance.old_label, &balance.old);
        out.pair(balance.this_label, &balance.this);
        out.pair(balance.total_label, &balance.total);
    }
    out.align(Align::Center);
    out.feed(1);
    out.line(view.thank_you);
    out.line(view.currency);
    out.feed(2);
    out.cut();
    out.into_bytes()
}

struct Buf {
    bytes: Vec<u8>,
}

#[derive(Clone, Copy)]
enum Align {
    Left,
    Center,
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
        // The head eats one byte per column (WIDTH = 42), not one UTF-8
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

    fn rule(&mut self) {
        self.line(&"-".repeat(WIDTH));
    }

    fn pair(&mut self, label: &str, amount: &str) {
        let gap = WIDTH
            .saturating_sub(label.chars().count())
            .saturating_sub(amount.chars().count());
        let pad = if gap == 0 { 1 } else { gap };
        let mut line = String::new();
        line.push_str(label);
        for _ in 0..pad {
            line.push(' ');
        }
        line.push_str(amount);
        self.line(&line);
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
    use super::{dump, Buf};

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
}
