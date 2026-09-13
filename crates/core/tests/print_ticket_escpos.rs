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
use dzpos_core::print::{dump_ticket_escpos, render_ticket_escpos};
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
