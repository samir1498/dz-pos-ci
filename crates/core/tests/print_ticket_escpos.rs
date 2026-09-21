// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! ESC/POS dump of the 80 mm ticket, pinned as a golden the way the HTML
//! ticket is. There is no printer: the dump is the snapshot a reviewer
//! reads (`<init>`, the text, `<cut>`).
//!
//! Regenerated with `UPDATE_GOLDENS=1 cargo test -p dzpos-core --test
//! print_ticket_escpos`. That run fails on purpose.

use std::path::PathBuf;

use chrono::{Datelike, NaiveDate};
use dzpos_core::lang::Lang;
use dzpos_core::money::{compute_totals, Bps, Line, Money, PaymentMode, Regime, TotalsOptions};
use dzpos_core::print::strings::{text, Key};
use dzpos_core::print::{
    draw_ticket_raster, dump_ticket_escpos, dump_ticket_escpos_png, render_ticket_escpos,
    render_ticket_escpos_raster, send_ticket_escpos_tcp, write_ticket_escpos_to_file,
    HEAD_WIDTH_DOTS,
};
use dzpos_core::services::documents::{
    Document, DocumentKind, DocumentLine, DocumentStatus, SellerBlock,
};

const SHOP: i32 = 1;
const OWNER: i32 = 1;

const LINES: [(&str, i64, i64, i64, u32); 3] = [
    ("Café moulu 250 g", 2_000, 15_000, 0, 1900),
    ("Farine", 1_500, 32_000, 0, 900),
    ("Pain", 1_000, 8_000, 1_000, 0),
];
const GLOBAL_DISCOUNT: i64 = 2_000;
const TENDERED: i64 = 100_000;

#[derive(Clone, Copy)]
enum Case {
    Reel,
    Ifu,
    Card,
}

impl Case {
    const fn regime(self) -> Regime {
        match self {
            Case::Reel | Case::Card => Regime::Reel,
            Case::Ifu => Regime::Ifu,
        }
    }

    const fn payment_mode(self) -> PaymentMode {
        match self {
            Case::Reel | Case::Ifu => PaymentMode::Cash,
            Case::Card => PaymentMode::Card,
        }
    }

    const fn suffix(self) -> &'static str {
        match self {
            Case::Reel => "",
            Case::Ifu => "-ifu",
            Case::Card => "-card",
        }
    }
}

mod common;

fn goldens_dir() -> PathBuf {
    common::goldens_dir("ticket_80mm_escpos")
}

fn fixed_sale(case: Case) -> Document {
    let regime = case.regime();
    let money_lines: Vec<Line> = LINES
        .iter()
        .map(|(_, qty_milli, unit, line_discount, rate)| Line {
            qty_milli: *qty_milli,
            unit_price: Money::centimes(*unit),
            line_discount: Money::centimes(*line_discount),
            rate: Bps::new(*rate).unwrap(),
        })
        .collect();
    let totals = compute_totals(
        &money_lines,
        &TotalsOptions {
            global_discount: Money::centimes(GLOBAL_DISCOUNT),
            payment_mode: case.payment_mode(),
            stamp_enabled: true,
            regime,
        },
    )
    .unwrap();
    let lines = LINES
        .iter()
        .zip(&money_lines)
        .enumerate()
        .map(|(position, ((name, ..), line))| DocumentLine {
            id: i32::try_from(position).unwrap() + 1,
            position: i32::try_from(position).unwrap(),
            product_id: None,
            name: (*name).to_owned(),
            barcode: None,
            qty_milli: line.qty_milli,
            unit_price: line.unit_price,
            line_discount: line.line_discount,
            rate_bps: line.rate,
            line_total: line
                .unit_price
                .checked_mul_milli(line.qty_milli)
                .unwrap()
                .checked_sub(line.line_discount)
                .unwrap(),
            ref_line_id: None,
        })
        .collect();
    let issued_at = NaiveDate::from_ymd_opt(2026, 9, 9)
        .unwrap()
        .and_hms_opt(14, 5, 0)
        .unwrap();
    let tendered = match case.payment_mode() {
        PaymentMode::Cash => Some(Money::centimes(TENDERED)),
        PaymentMode::Card | PaymentMode::Credit => None,
    };
    Document {
        id: 1,
        shop_id: SHOP,
        kind: DocumentKind::Ticket,
        series: DocumentKind::Ticket.series_of_year(issued_at.year()),
        series_year: issued_at.year(),
        number: 123,
        issued_at,
        user_id: OWNER,
        regime,
        payment_mode: case.payment_mode(),
        seller: SellerBlock {
            name: "Mon magasin".to_owned(),
            rc: Some("16/00-1234567 B 25".to_owned()),
            nif: Some("000216001234567".to_owned()),
            nis: None,
            ai: None,
            address: Some("12 rue Didouche Mourad, Alger".to_owned()),
            phone: Some("0555 12 34 56".to_owned()),
        },
        customer_id: None,
        buyer: None,
        ref_document_id: None,
        balance: None,
        change: tendered.map(|t| t.checked_sub(totals.net_to_pay).unwrap()),
        tendered,
        totals,
        status: DocumentStatus::Issued,
        cancellation: None,
        lines,
        created_at: issued_at,
    }
}

fn golden_name(lang: Lang, case: Case) -> String {
    format!("{}{}.txt", lang.tag(), case.suffix())
}

fn golden(lang: Lang, case: Case, rendered: &str, updated: &mut Vec<String>) -> String {
    let name = golden_name(lang, case);
    let path = goldens_dir().join(&name);
    if std::env::var_os("UPDATE_GOLDENS").is_some() {
        std::fs::create_dir_all(goldens_dir()).unwrap();
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        if existing != rendered {
            std::fs::write(&path, rendered).unwrap();
            updated.push(name);
        }
        return rendered.to_owned();
    }
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}; run UPDATE_GOLDENS=1 to write it", path.display()))
}

fn each_language_of(case: Case) {
    let doc = fixed_sale(case);
    let mut updated = Vec::new();
    for lang in Lang::ALL {
        let bytes = render_ticket_escpos(&doc, lang).unwrap();
        let rendered = dump_ticket_escpos(&bytes);
        let expected = golden(lang, case, &rendered, &mut updated);
        assert_eq!(
            rendered,
            expected,
            "{} is not what the encoder dumps",
            golden_name(lang, case)
        );
        assert!(
            rendered.contains("TK-2026-000123"),
            "{lang:?} lost the ticket number"
        );
        assert!(
            rendered.contains(&format_amount(doc.totals.net_to_pay)),
            "{lang:?} dump does not carry the net to pay"
        );
    }
    assert!(
        updated.is_empty(),
        "rewrote {updated:?}: read the dump, then run again without UPDATE_GOLDENS"
    );
}

fn format_amount(amount: Money) -> String {
    dzpos_core::money::format::format_centimes(amount)
}

#[test]
fn the_reel_ticket_escpos_is_its_dump_in_every_language() {
    each_language_of(Case::Reel);
}

#[test]
fn the_ifu_ticket_escpos_is_its_dump_in_every_language() {
    each_language_of(Case::Ifu);
}

#[test]
fn the_card_ticket_escpos_is_its_dump_in_every_language() {
    each_language_of(Case::Card);
}

#[test]
fn an_ifu_escpos_ticket_names_no_tax() {
    let doc = fixed_sale(Case::Ifu);
    for lang in Lang::ALL {
        let dump = dump_ticket_escpos(&render_ticket_escpos(&doc, lang).unwrap());
        for forbidden in ["TVA", "VAT", "ت.ق.م"] {
            assert!(!dump.contains(forbidden), "{forbidden} on {lang:?}");
        }
        assert!(dump.contains(text(Key::Total, lang)), "{lang:?}");
        assert!(!dump.contains(text(Key::TotalHt, lang)), "{lang:?}");
    }
}

#[test]
fn a_card_escpos_ticket_has_no_stamp_and_no_change() {
    let doc = fixed_sale(Case::Card);
    for lang in Lang::ALL {
        let dump = dump_ticket_escpos(&render_ticket_escpos(&doc, lang).unwrap());
        assert!(!dump.contains(text(Key::Stamp, lang)), "{lang:?}");
        assert!(!dump.contains(text(Key::Tendered, lang)), "{lang:?}");
        assert!(!dump.contains(text(Key::Change, lang)), "{lang:?}");
        assert!(dump.contains(text(Key::Card, lang)), "{lang:?}");
    }
}

#[test]
fn a_ticket_written_to_a_file_is_the_rendered_bytes() {
    let doc = fixed_sale(Case::Reel);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ticket.bin");
    for lang in Lang::ALL {
        write_ticket_escpos_to_file(&doc, lang, &path).unwrap();
        let on_disk = std::fs::read(&path).unwrap();
        assert_eq!(
            on_disk,
            render_ticket_escpos(&doc, lang).unwrap(),
            "{lang:?} file is not the rendered bytes"
        );
    }
}

#[test]
fn a_ticket_sent_over_tcp_is_the_rendered_bytes() {
    use std::io::Read as _;
    let doc = fixed_sale(Case::Reel);
    let lang = Lang::Fr;
    let expected = render_ticket_escpos(&doc, lang).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let received = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).unwrap();
        buf
    });
    send_ticket_escpos_tcp(&doc, lang, &addr).unwrap();
    assert_eq!(received.join().unwrap(), expected);
}

#[test]
fn a_refused_ticket_writes_no_file_and_opens_no_connection() {
    let mut doc = fixed_sale(Case::Ifu);
    doc.totals.tva_by_rate.push(dzpos_core::money::TvaLine {
        rate: Bps::new(1900).unwrap(),
        base: Money::centimes(29_295),
        amount: Money::centimes(5_566),
    });
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ticket.bin");
    let err = write_ticket_escpos_to_file(&doc, Lang::Fr, &path).unwrap_err();
    assert_eq!(err.code(), "print");
    assert!(!path.exists(), "a refused ticket left a file behind");
    // A closed port would fail with an io error; the render refusal must
    // come first, before any connection is attempted.
    let err = send_ticket_escpos_tcp(&doc, Lang::Fr, "127.0.0.1:9").unwrap_err();
    assert_eq!(err.code(), "print");
}

#[test]
fn an_ifu_document_with_a_tva_recap_is_refused_as_escpos_too() {
    let mut doc = fixed_sale(Case::Ifu);
    doc.totals.tva_by_rate.push(dzpos_core::money::TvaLine {
        rate: Bps::new(1900).unwrap(),
        base: Money::centimes(29_295),
        amount: Money::centimes(5_566),
    });
    for lang in Lang::ALL {
        let err = render_ticket_escpos(&doc, lang).unwrap_err();
        assert_eq!(err.code(), "print", "{lang:?}");
    }
}

/// The lines a golden says the text path put on the paper: everything that
/// is not a `<command>`. The dump writes a ticket line and its newline, so
/// splitting is enough; the commands each sit on a line of their own.
fn text_lines(golden: &str) -> Vec<String> {
    golden
        .lines()
        .filter(|line| !line.is_empty() && !(line.starts_with('<') && line.ends_with('>')))
        .map(str::to_owned)
        .collect()
}

/// The narrow no-break space of "1 000,00" is a plain space on the wire
/// (`the_french_wire_is_one_byte_per_column_not_utf8` pins that), so the
/// golden carries a plain space where the line model carries U+202F. Both
/// paths start from the same character; only the comparison needs telling.
fn on_the_wire(line: &str) -> String {
    line.replace('\u{202f}', " ")
}

/// **The rule this whole task exists for.** The raster's source lines are
/// the text path's lines, string for string, so a number cannot be one
/// thing on a French ticket and another on the Arabic one beside it. The
/// left side is the committed `.txt` golden — a file, not a value this run
/// computed — and the right side is what `print::raster` says it drew.
///
/// If this ever goes red because an amount differs, the bug is that
/// something inside `raster.rs` formatted a number. Nothing in there is
/// allowed to.
#[test]
fn the_raster_draws_the_lines_the_text_path_prints() {
    for case in [Case::Reel, Case::Ifu, Case::Card] {
        let doc = fixed_sale(case);
        for lang in Lang::ALL {
            let name = golden_name(lang, case);
            let golden = std::fs::read_to_string(goldens_dir().join(&name))
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            let drawn = draw_ticket_raster(&doc, lang, HEAD_WIDTH_DOTS).unwrap();
            let raster: Vec<String> = drawn
                .lines
                .iter()
                .map(|line| on_the_wire(&line.text))
                .collect();
            assert_eq!(raster, text_lines(&golden), "{name}: the two paths differ");
            assert!(
                raster
                    .iter()
                    .any(|line| line.contains(&on_the_wire(&format_amount(doc.totals.net_to_pay)))),
                "{name}: the raster does not carry the net to pay"
            );
        }
    }
}

/// Nothing is trimmed to fit. A line wider than the head is an error from
/// `raster::draw`, so reaching this assertion at all means every line fit;
/// the assertion says by how much, and `notdef` says no character was
/// drawn as the box this task is about.
#[test]
fn no_raster_line_is_clipped_at_the_head_width() {
    for case in [Case::Reel, Case::Ifu, Case::Card] {
        let doc = fixed_sale(case);
        for lang in Lang::ALL {
            let drawn = draw_ticket_raster(&doc, lang, HEAD_WIDTH_DOTS).unwrap();
            let name = golden_name(lang, case);
            assert!(!drawn.lines.is_empty(), "{name}: nothing was drawn");
            for line in &drawn.lines {
                assert!(
                    line.advance <= HEAD_WIDTH_DOTS,
                    "{name}: {:?} took {} of {HEAD_WIDTH_DOTS} dots",
                    line.text,
                    line.advance
                );
                assert_eq!(
                    line.notdef, 0,
                    "{name}: {:?} has a character no vendored font carries",
                    line.text
                );
            }
            assert_eq!(drawn.bitmap.width(), HEAD_WIDTH_DOTS as usize, "{name}");
            assert!(drawn.bitmap.height() > 0, "{name}");
        }
    }
}

/// A 58 mm head is one argument away, and its narrower column budget is
/// the case where a line is most likely not to fit.
#[test]
fn the_same_ticket_draws_on_a_384_dot_head() {
    let doc = fixed_sale(Case::Reel);
    for lang in Lang::ALL {
        let drawn = draw_ticket_raster(&doc, lang, 384).unwrap();
        assert_eq!(drawn.bitmap.width(), 384, "{lang:?}");
        assert_eq!(drawn.cell, 384 / 42, "{lang:?}");
        for line in &drawn.lines {
            assert!(
                line.advance <= 384,
                "{lang:?}: {:?} took {} of 384 dots",
                line.text,
                line.advance
            );
        }
    }
    // A head that is not a whole number of bytes wide, or too narrow to
    // carry 42 columns, is refused rather than drawn wrong.
    let refused = |width| {
        draw_ticket_raster(&doc, Lang::Ar, width)
            .err()
            .map(|err| err.code())
    };
    assert_eq!(refused(577), Some("print"), "an odd head width was drawn");
    assert_eq!(refused(8), Some("print"), "an 8-dot head was drawn");
}

/// The shaper measures in font units and the rasteriser scales by the
/// face's height rather than its em, so the two agree only if the second
/// is told the em in the first's terms. When they do not agree the glyphs
/// are drawn at a different size from the advances that spaced them, and
/// the 42-dash rule is the line that shows it: it should measure exactly
/// 42 columns and its ink should reach both ends of that span.
#[test]
fn the_rule_line_measures_the_column_budget_it_was_built_from() {
    let doc = fixed_sale(Case::Reel);
    let drawn = draw_ticket_raster(&doc, Lang::Fr, HEAD_WIDTH_DOTS).unwrap();
    let budget = drawn.cell * 42;
    let rule = drawn
        .lines
        .iter()
        .find(|line| line.text.starts_with("-----"))
        .unwrap();
    assert_eq!(rule.advance, budget, "42 columns of the monospaced face");
    // The row the rule sits on: the first row whose ink reaches past three
    // quarters of the budget is one of the rules, and a dashed line of 42
    // dashes covers most of its span.
    let bitmap = &drawn.bitmap;
    let mut widest = 0;
    for y in 0..bitmap.height() {
        let ink: Vec<usize> = (0..bitmap.width())
            .filter(|x| bitmap.is_black(*x, y))
            .collect();
        if let (Some(first), Some(last)) = (ink.first(), ink.last()) {
            let span = last.saturating_sub(*first);
            if span > widest {
                widest = span;
            }
        }
    }
    assert!(
        widest >= (budget as usize) * 9 / 10 && widest <= HEAD_WIDTH_DOTS as usize,
        "the widest row of ink spans {widest} dots, not the {budget} the rule should"
    );
}

/// `GS v 0` and its four header bytes, read back. The dump names a band by
/// the dots it covers so a reviewer sees the shape of the job without a
/// printer, and the bands are split small enough for a cheap head's buffer.
#[test]
fn the_raster_ticket_is_bands_of_dots_and_nothing_else() {
    let doc = fixed_sale(Case::Reel);
    let bytes = render_ticket_escpos_raster(&doc, Lang::Ar, HEAD_WIDTH_DOTS).unwrap();
    let dump = dump_ticket_escpos(&bytes);
    assert!(dump.starts_with("<init>\n<align left>\n"), "{dump}");
    assert!(dump.ends_with("<cut>\n"), "{dump}");
    let bands: Vec<&str> = dump
        .lines()
        .filter(|line| line.starts_with("<raster "))
        .collect();
    assert!(!bands.is_empty(), "no raster band in {dump}");
    for band in &bands {
        assert!(band.starts_with("<raster 576x"), "{band}");
    }
    // No codepage and no text: the whole point is that the head is never
    // asked to spell anything.
    assert!(!dump.contains("<codepage"), "{dump}");
    assert!(!dump.contains('ت'), "Arabic text on a raster wire: {dump}");
    // Each band under the 64 KB the command addresses.
    let drawn = draw_ticket_raster(&doc, Lang::Ar, HEAD_WIDTH_DOTS).unwrap();
    let rows: usize = bands
        .iter()
        .filter_map(|band| band.trim_end_matches('>').split('x').next_back())
        .filter_map(|rows| rows.parse::<usize>().ok())
        .sum();
    assert_eq!(rows, drawn.bitmap.height(), "the bands lost rows");
    for band in &bands {
        let rows: usize = band
            .trim_end_matches('>')
            .split('x')
            .next_back()
            .and_then(|rows| rows.parse().ok())
            .unwrap();
        assert!(rows * drawn.bitmap.stride() < 65_536, "{band} is too tall");
    }
}

/// The three Arabic goldens a reviewer opens. They are the same bytes the
/// head is sent, decoded back out of the `GS v 0` bands, so a picture that
/// looks right is evidence about the wire and not about a second renderer.
#[test]
fn the_arabic_raster_goldens_are_what_the_head_is_sent() {
    let mut updated = Vec::new();
    for case in [Case::Reel, Case::Ifu, Case::Card] {
        let doc = fixed_sale(case);
        let bytes = render_ticket_escpos_raster(&doc, Lang::Ar, HEAD_WIDTH_DOTS).unwrap();
        let png = dump_ticket_escpos_png(&bytes).expect("the raster ticket carries no band");
        let name = format!("ar{}.png", case.suffix());
        let path = goldens_dir().join(&name);
        if std::env::var_os("UPDATE_GOLDENS").is_some() {
            if std::fs::read(&path).ok().as_deref() != Some(png.as_slice()) {
                std::fs::write(&path, &png).unwrap();
                updated.push(name);
            }
            continue;
        }
        let expected = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("{}: {e}; UPDATE_GOLDENS=1", path.display()));
        assert_eq!(png, expected, "{name} is not what the encoder draws");
    }
    assert!(
        updated.is_empty(),
        "rewrote {updated:?}: open them, then run again without UPDATE_GOLDENS"
    );
    // The text path carries no band at all, so nothing here can quietly
    // start writing a picture of a ticket that was sent as text.
    let text = render_ticket_escpos(&fixed_sale(Case::Reel), Lang::Ar).unwrap();
    assert!(dump_ticket_escpos_png(&text).is_none());
}

#[test]
fn the_french_wire_is_one_byte_per_column_not_utf8() {
    // "Café" is 4 columns: 0x43 0x61 0x66 0xE9, not 5 bytes 0x43 0x61 0x66
    // 0xC3 0xA9 that the first eyeball showed as "CafÃ©". The head eats one
    // byte per column (WIDTH = 42), so the wire must be ISO 8859-15.
    let doc = fixed_sale(Case::Reel);
    let bytes = render_ticket_escpos(&doc, Lang::Fr).unwrap();
    assert!(
        bytes.windows(2).any(|w| w == [0x43, 0x61]) && bytes.contains(&0xE9),
        "no single-byte é (0xE9) in the French wire"
    );
    assert!(
        !bytes.windows(2).any(|w| w == [0xC3, 0xA9]),
        "French wire still carries UTF-8 C3 A9 for é"
    );
    // Narrow NBSP U+202F the money formatter uses is 0xE2 0x80 0xAF in UTF-8;
    // the thermal wire must be 0x20, otherwise "19 % with 0x20" and "1 000"
    // drift from WIDTH and the emulator splits them.
    assert!(
        !bytes.windows(3).any(|w| w == [0xE2, 0x80, 0xAF]),
        "French wire still carries UTF-8 narrow NBSP"
    );
    assert!(
        bytes.windows(3).any(|w| w == *b"1 0"),
        "no 0x20 space where the narrow NBSP should be (1 000)"
    );
}

#[test]
fn the_arabic_wire_keeps_utf8_for_script_outside_the_table() {
    // Arabic "تذكرة" (Ticket) is outside 0xFF, so it is sent as UTF-8 and the
    // dump decodes it back; "?????" would be the old "?" fallback.
    let doc = fixed_sale(Case::Reel);
    let bytes = render_ticket_escpos(&doc, Lang::Ar).unwrap();
    let dump = dump_ticket_escpos(&bytes);
    assert!(dump.contains("تذكرة"), "Arabic Ticket title not in dump");
    assert!(
        !dump.contains("?????"),
        "Arabic still ?????: wire is ? fallback"
    );
    // The UTF-8 bytes themselves are on the wire, not single-byte ?
    assert!(
        bytes
            .windows(2)
            .any(|w| w == [0xD8, 0xAA] || w == [0xD8, 0xAA]),
        "no UTF-8 Arabic bytes on the wire"
    );
}
