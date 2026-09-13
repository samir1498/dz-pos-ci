//! The 80 mm ticket as ESC/POS bytes.
//!
//! Same document, same words as `render_ticket`. The HTML page is what a
//! driver prints on A4 paper and what the e2e photographs; this is what a
//! cheap thermal head eats. There is no USB in this crate: the caller gets
//! the bytes. Tests pin a human-readable dump of those bytes
//! (`fixtures/print/ticket_80mm_escpos/`), the same golden rule as the HTML
//! tickets. A real printer is not required to know they are right.

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
        self.bytes.extend_from_slice(s.as_bytes());
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
        // Take one UTF-8 scalar, so a golden of Café is not three replacement
        // characters. A byte that is not UTF-8 is shown as hex so a protocol
        // slip cannot hide inside a dump a reviewer reads as text.
        match std::str::from_utf8(&bytes[i..]) {
            Ok(rest) => {
                if let Some(ch) = rest.chars().next() {
                    out.push(ch);
                    i += ch.len_utf8();
                    continue;
                }
            }
            Err(err) if err.valid_up_to() > 0 => {
                if let Ok(rest) = std::str::from_utf8(&bytes[i..i + err.valid_up_to()]) {
                    if let Some(ch) = rest.chars().next() {
                        out.push(ch);
                        i += ch.len_utf8();
                        continue;
                    }
                }
            }
            Err(_) => {}
        }
        out.push_str(&format!("<{:02x}>", bytes[i]));
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
            "<init>\n<align center>\n<bold on>\nCafé\n<bold off>\n<cut>\n"
        );
    }
}
