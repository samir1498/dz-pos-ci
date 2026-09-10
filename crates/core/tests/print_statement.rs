// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The A4 statement against its golden files, one per language (features.md
//! §4: "Golden-file test for every template × language against fixed
//! fixtures. A template change is a reviewed golden diff").
//!
//! Regenerate with `UPDATE_GOLDENS=1 cargo test -p dzpos-core --test
//! print_statement`. The run that rewrites them fails on purpose: a golden
//! nobody looked at must never be green in the same run that wrote it.
//!
//! Every amount in each golden is parsed back out of the file and compared to
//! the statement the page was rendered from: the opening balance, each row's
//! debit or credit, each running balance and the closing one. The checks call
//! no renderer, so a golden that has drifted from the money cannot be
//! accepted by regenerating it. The running balances matter most: they are
//! the core's column, and a page that quietly recomputed them would be a
//! second answer to what the customer owes.

use std::path::PathBuf;

use chrono::{NaiveDate, NaiveDateTime};
use dzpos_core::lang::Lang;
use dzpos_core::money::words::amount_in_words;
use dzpos_core::money::Money;
use dzpos_core::print::strings::{text, Key};
use dzpos_core::print::{render_statement, Paper};
use dzpos_core::services::customers::{Customer, PartyKind};
use dzpos_core::services::debt::{
    DebtEntry, DebtKind, DocumentRef, PaymentMethod, RangedStatement, StatementEntry,
};
use dzpos_core::services::documents::DocumentKind;

const SHOP: i32 = 1;
const OWNER: i32 = 1;
const CUSTOMER: i32 = 7;

/// What the customer owed on the morning of the first day: 1 500,00 DA.
const OPENING: i64 = 150_000;

mod common;

fn goldens_dir() -> PathBuf {
    common::goldens_dir("statement_a4")
}

fn day(d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, d).unwrap()
}

fn at(d: u32, hour: u32) -> NaiveDateTime {
    day(d).and_hms_opt(hour, 30, 0).unwrap()
}

fn a_company() -> Customer {
    Customer {
        id: CUSTOMER,
        shop_id: SHOP,
        name: "Entreprise Benali & Fils".to_string(),
        party_kind: PartyKind::Company,
        phone: Some("0770 11 22 33".to_string()),
        address: Some("12 rue Didouche Mourad, Alger".to_string()),
        rc: Some("16/00-1234567 B 09".to_string()),
        nif: Some("000916001234567".to_string()),
        nis: Some("000916009876543".to_string()),
        ai: Some("16123456789".to_string()),
        credit_limit: Some(Money::centimes(500_000)),
        warn_threshold: Some(Money::centimes(400_000)),
        notes: None,
        active: true,
        created_at: at(1, 9),
        updated_at: at(1, 9),
    }
}

/// A consumer fiche carrying identifiers it should never print: décret 05-468
/// art. 3-2, last alinéa, gives a private buyer their name and address and
/// nothing else, and a fiche that once held an RC does not turn them into a
/// company.
fn a_consumer() -> Customer {
    Customer {
        party_kind: PartyKind::Consumer,
        name: "Ahmed Cherif".to_string(),
        ..a_company()
    }
}

fn movement(
    id: i32,
    kind: DebtKind,
    debit: i64,
    credit: i64,
    document: Option<DocumentRef>,
    at: NaiveDateTime,
) -> DebtEntry {
    DebtEntry {
        id,
        shop_id: SHOP,
        customer_id: CUSTOMER,
        document_id: document.map(|_| id),
        kind,
        debit: Money::centimes(debit),
        credit: Money::centimes(credit),
        user_id: OWNER,
        note: None,
        payment_mode: (kind == DebtKind::Payment).then_some(PaymentMethod::Cash),
        created_at: at,
    }
}

fn facture(number: i64) -> Option<DocumentRef> {
    Some(DocumentRef {
        kind: DocumentKind::Facture,
        year: 2026,
        number,
    })
}

/// The fixed month: an opening balance carried in from before the range, a
/// facture on credit, a payment that settled part of it, and a correction.
/// Every running balance below is written out rather than computed, so the
/// fixture proves the column instead of repeating whatever built it.
fn a_month() -> RangedStatement {
    /// One fixed row: the movement's id, why the debt moved, its two columns,
    /// the document it cites, the day it landed and the balance it left.
    struct Row {
        id: i32,
        kind: DebtKind,
        debit: i64,
        credit: i64,
        document: Option<DocumentRef>,
        day: u32,
        balance: i64,
    }
    let rows = [
        Row {
            id: 11,
            kind: DebtKind::Sale,
            debit: 200_000,
            credit: 0,
            document: facture(42),
            day: 5,
            balance: 350_000,
        },
        Row {
            id: 12,
            kind: DebtKind::Payment,
            debit: 0,
            credit: 250_000,
            document: None,
            day: 12,
            balance: 100_000,
        },
        Row {
            id: 13,
            kind: DebtKind::Adjustment,
            debit: 0,
            credit: 10_000,
            document: None,
            day: 20,
            balance: 90_000,
        },
    ];
    RangedStatement {
        from: day(1),
        to: day(30),
        opening: Money::centimes(OPENING),
        entries: rows
            .into_iter()
            .map(|row| StatementEntry {
                entry: movement(
                    row.id,
                    row.kind,
                    row.debit,
                    row.credit,
                    row.document,
                    at(row.day, 10),
                ),
                balance_after: Money::centimes(row.balance),
                document: row.document,
            })
            .collect(),
        closing: Money::centimes(90_000),
    }
}

fn golden(lang: Lang, rendered: &str, updated: &mut Vec<String>) -> String {
    let name = format!("{}.html", lang.tag());
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

fn refuse_a_silent_regeneration(updated: &[String]) {
    assert!(
        updated.is_empty(),
        "rewrote {updated:?}: read the diff, then run the test again without UPDATE_GOLDENS"
    );
}

/// The text of every `<span class="amount amount-{marker}">` in the file.
/// Reads the golden, never the renderer: that is what makes the check worth
/// running against a file a regeneration just wrote.
///
/// The facture suite has the same parser, copied rather than shared for the
/// reason its own header gives: two golden suites checking each other's files
/// through one helper can both be made green by editing the helper once.
fn amounts(html: &str, marker: &str) -> Vec<String> {
    let opening = format!("<span class=\"amount amount-{marker}\">");
    html.split(&opening)
        .skip(1)
        .map(|rest| {
            let end = rest.find("</span>").expect("an amount span never closes");
            rest[..end].to_owned()
        })
        .collect()
}

fn one_amount(html: &str, marker: &str) -> String {
    let found = amounts(html, marker);
    assert_eq!(found.len(), 1, "expected one {marker} row, got {found:?}");
    found.into_iter().next().unwrap_or_default()
}

/// "1 234,56" back to 123456 centimes. The golden's own digits, read by a
/// parser that shares no code with the formatter that wrote them.
fn centimes(printed: &str) -> i64 {
    let digits: String = printed
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ',' || *c == '-')
        .collect();
    let (whole, rest) = digits
        .split_once(',')
        .unwrap_or_else(|| panic!("{printed} is not an amount"));
    assert_eq!(rest.len(), 2, "{printed} does not carry two centime digits");
    let sign = if whole.starts_with('-') { -1 } else { 1 };
    let whole: i64 = whole.trim_start_matches('-').parse().unwrap();
    let rest: i64 = rest.parse().unwrap();
    sign * (whole * 100 + rest)
}

fn in_words(html: &str) -> String {
    let opening = "<strong class=\"in-words\">";
    let rest = html
        .split_once(opening)
        .unwrap_or_else(|| panic!("the page carries no words line"))
        .1;
    let end = rest.find("</strong>").expect("the words line never closes");
    rest[..end].to_owned()
}

/// Every amount in the golden, against what the statement holds.
fn the_golden_says_what_the_statement_holds(html: &str, statement: &RangedStatement, lang: Lang) {
    assert_eq!(
        centimes(&one_amount(html, "opening")),
        statement.opening.as_centimes(),
        "the opening balance"
    );
    assert_eq!(
        centimes(&one_amount(html, "closing")),
        statement.closing.as_centimes(),
        "the closing balance"
    );
    // The running column, row by row and in order. This is the check the page
    // exists for: a statement whose balances do not follow its own movements
    // is a page a customer can prove wrong at the counter.
    assert_eq!(
        amounts(html, "running")
            .iter()
            .map(|printed| centimes(printed))
            .collect::<Vec<i64>>(),
        statement
            .entries
            .iter()
            .map(|line| line.balance_after.as_centimes())
            .collect::<Vec<i64>>(),
        "the running balances"
    );
    // A movement raises the debt or lowers it, never both, so each column
    // carries only the rows that used it and no zeroes.
    assert_eq!(
        amounts(html, "debit")
            .iter()
            .map(|printed| centimes(printed))
            .collect::<Vec<i64>>(),
        statement
            .entries
            .iter()
            .map(|line| line.entry.debit.as_centimes())
            .filter(|c| *c != 0)
            .collect::<Vec<i64>>(),
        "the debit column"
    );
    assert_eq!(
        amounts(html, "credit")
            .iter()
            .map(|printed| centimes(printed))
            .collect::<Vec<i64>>(),
        statement
            .entries
            .iter()
            .map(|line| line.entry.credit.as_centimes())
            .filter(|c| *c != 0)
            .collect::<Vec<i64>>(),
        "the credit column"
    );
    // The words are the closing balance and nothing else, and they are
    // written by the money module rather than by this page.
    assert_eq!(
        in_words(html),
        amount_in_words(statement.closing, lang).unwrap(),
        "the words line is not the closing balance written out"
    );
    // Every movement names its kind in the print dictionary's words, and a
    // document the movement cites is printed under the number a customer
    // quotes.
    for line in &statement.entries {
        if let Some(document) = line.document {
            let printed = format!(
                "{}-{}-{:06}",
                document.kind.number_prefix(),
                document.year,
                document.number
            );
            assert!(
                html.contains(&printed),
                "the row citing {printed} does not print its number"
            );
        }
    }
    assert!(html.contains(text(Key::Statement, lang)));
    assert!(html.contains(text(Key::KindPayment, lang)));
}

#[test]
fn the_statement_is_its_golden_in_every_language() {
    let statement = a_month();
    let customer = a_company();
    let mut updated = Vec::new();
    for lang in Lang::ALL {
        let rendered = render_statement(&customer, &statement, lang, Paper::A4).unwrap();
        let expected = golden(lang, &rendered, &mut updated);
        assert_eq!(rendered, expected, "the {lang:?} statement differs");
        the_golden_says_what_the_statement_holds(&expected, &statement, lang);
    }
    refuse_a_silent_regeneration(&updated);
}

#[test]
fn the_page_prints_the_identifiers_of_a_company_and_never_a_consumers() {
    let statement = a_month();
    let company = render_statement(&a_company(), &statement, Lang::Fr, Paper::A4).unwrap();
    let consumer = render_statement(&a_consumer(), &statement, Lang::Fr, Paper::A4).unwrap();

    assert!(company.contains("000916001234567"), "the NIF is missing");
    assert!(company.contains("16/00-1234567 B 09"));
    // The same fiche fields, and a private buyer prints their name and their
    // address and nothing else.
    assert!(!consumer.contains("000916001234567"));
    assert!(!consumer.contains("16/00-1234567 B 09"));
    assert!(consumer.contains("Ahmed Cherif"));
    assert!(consumer.contains("12 rue Didouche Mourad, Alger"));
}

#[test]
fn a_range_with_no_movement_says_so_and_still_carries_both_balances() {
    let statement = RangedStatement {
        from: day(1),
        to: day(30),
        opening: Money::centimes(OPENING),
        entries: Vec::new(),
        // Nothing moved, so the day closes where it opened.
        closing: Money::centimes(OPENING),
    };

    let page = render_statement(&a_company(), &statement, Lang::Fr, Paper::A4).unwrap();

    assert!(page.contains(text(Key::NoMovement, Lang::Fr)));
    assert_eq!(centimes(&one_amount(&page, "opening")), OPENING);
    assert_eq!(centimes(&one_amount(&page, "closing")), OPENING);
    assert!(
        amounts(&page, "running").is_empty(),
        "a range with no movement printed a movement row"
    );
}

#[test]
fn a_closing_balance_the_shop_owes_prints_the_amount_and_says_whose_way_it_goes() {
    // The shop holding money for a customer is what an avoir leaves behind.
    // The words carry no sign, so the page has to say the direction in
    // words of its own or it reads as the opposite of what it means.
    let statement = RangedStatement {
        from: day(1),
        to: day(30),
        opening: Money::ZERO,
        entries: vec![StatementEntry {
            entry: movement(21, DebtKind::Avoir, 0, 20_000, facture(43), at(9, 11)),
            balance_after: Money::centimes(-20_000),
            document: facture(43),
        }],
        closing: Money::centimes(-20_000),
    };

    let page = render_statement(&a_company(), &statement, Lang::Fr, Paper::A4).unwrap();

    assert_eq!(centimes(&one_amount(&page, "closing")), -20_000);
    assert_eq!(
        in_words(&page),
        amount_in_words(Money::centimes(20_000), Lang::Fr).unwrap(),
        "the words of a negative balance are the words of the amount"
    );
    assert!(
        page.contains(text(Key::InFavourOfCustomer, Lang::Fr)),
        "the page says the amount is owed and not which way"
    );
}

#[test]
fn the_arabic_statement_reads_right_to_left_and_says_it_is_unreviewed() {
    let page = render_statement(&a_company(), &a_month(), Lang::Ar, Paper::A4).unwrap();
    assert!(page.contains("dir=\"rtl\""));
    assert!(page.contains("unreviewed by a native speaker"));
}

#[test]
fn the_digits_are_western_in_every_language() {
    for lang in Lang::ALL {
        let page = render_statement(&a_company(), &a_month(), lang, Paper::A4).unwrap();
        assert!(
            page.contains("1\u{202f}500,00"),
            "the {lang:?} page does not carry the opening balance in Western digits"
        );
        assert!(
            !page.chars().any(|c| ('\u{0660}'..='\u{0669}').contains(&c)),
            "the {lang:?} page carries Eastern Arabic digits"
        );
    }
}

#[test]
fn the_a5_statement_differs_from_the_a4_in_the_page_size_line_only() {
    let a4 = render_statement(&a_company(), &a_month(), Lang::Fr, Paper::A4).unwrap();
    let a5 = render_statement(&a_company(), &a_month(), Lang::Fr, Paper::A5).unwrap();
    assert_eq!(
        a4.replace("size: A4", "size: A5"),
        a5,
        "the smaller sheet is a second layout and not the same page"
    );
}
