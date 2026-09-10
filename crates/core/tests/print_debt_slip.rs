// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The 80 mm debt slip against its golden files, one per language
//! (features.md §4: "Golden-file test for every template × language against
//! fixed fixtures. A template change is a reviewed golden diff").
//!
//! Regenerate with `UPDATE_GOLDENS=1 cargo test -p dzpos-core --test
//! print_debt_slip`. The run that rewrites them fails on purpose: a golden
//! nobody looked at must never be green in the same run that wrote it.
//!
//! Every amount in each golden is parsed back out of the file and compared to
//! the slip the page was rendered from: the balance, each row's debit or
//! credit, and each running balance. The checks call no renderer, so a golden
//! that has drifted from the money cannot be accepted by regenerating it.
//!
//! The fixture carries twelve movements and the slip prints ten, which is the
//! check that matters most here: the balance is the whole ledger's and the
//! rows are a window on it, so a page that summed what it showed would hand a
//! customer a figure smaller than what they owe.

use std::path::PathBuf;

use chrono::{NaiveDate, NaiveDateTime};
use dzpos_core::lang::Lang;
use dzpos_core::models::document::SellerBlock;
use dzpos_core::money::words::amount_in_words;
use dzpos_core::money::Money;
use dzpos_core::print::debt_slip::MOVEMENTS;
use dzpos_core::print::render_debt_slip;
use dzpos_core::print::strings::{text, Key};
use dzpos_core::services::customers::{Customer, PartyKind};
use dzpos_core::services::debt::{
    DebtEntry, DebtKind, DocumentRef, PaymentMethod, RecentStatement, StatementEntry,
};
use dzpos_core::services::documents::DocumentKind;

const SHOP: i32 = 1;
const OWNER: i32 = 1;
const CUSTOMER: i32 = 7;

/// What the customer owes when the slip is printed: 1 750,00 DA. Written out
/// rather than computed, so the fixture pins the figure instead of repeating
/// whatever produced it.
const BALANCE: i64 = 175_000;

mod common;

fn goldens_dir() -> PathBuf {
    common::goldens_dir("debt_slip_80mm")
}

fn day(d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, d).unwrap()
}

fn at(d: u32, hour: u32) -> NaiveDateTime {
    day(d).and_hms_opt(hour, 30, 0).unwrap()
}

/// The moment the paper came out of the printer.
fn printed_at() -> NaiveDateTime {
    at(13, 17)
}

fn a_shop() -> SellerBlock {
    SellerBlock {
        name: "Alimentation Générale El Baraka".to_string(),
        rc: Some("16/00-7654321 B 21".to_string()),
        nif: Some("000916007654321".to_string()),
        nis: Some("000916001112223".to_string()),
        ai: Some("16987654321".to_string()),
        address: Some("5 boulevard Amirouche, Alger".to_string()),
        phone: Some("021 63 44 55".to_string()),
    }
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
        number,
    })
}

/// A document's number as the slip prints it: `FA-000042`, `AV-000003`.
fn document_number(document: DocumentRef) -> String {
    format!("{}-{:06}", document.kind.number_prefix(), document.number)
}

/// One fixed row: the movement's id, why the debt moved, its two columns, the
/// document it cites, the day it landed and the balance it left behind.
struct Row {
    id: i32,
    kind: DebtKind,
    debit: i64,
    credit: i64,
    document: Option<DocumentRef>,
    day: u32,
    balance: i64,
}

/// Twelve movements, oldest first, and the balance each one left. Every
/// running balance is written out rather than computed, so the fixture proves
/// the column instead of repeating whatever built it.
fn twelve_movements() -> [Row; 12] {
    [
        Row {
            id: 1,
            kind: DebtKind::Opening,
            debit: 150_000,
            credit: 0,
            document: None,
            day: 1,
            balance: 150_000,
        },
        Row {
            id: 2,
            kind: DebtKind::Sale,
            debit: 200_000,
            credit: 0,
            document: facture(42),
            day: 2,
            balance: 350_000,
        },
        Row {
            id: 3,
            kind: DebtKind::Payment,
            debit: 0,
            credit: 250_000,
            document: None,
            day: 3,
            balance: 100_000,
        },
        Row {
            id: 4,
            kind: DebtKind::Sale,
            debit: 50_000,
            credit: 0,
            document: facture(43),
            day: 4,
            balance: 150_000,
        },
        Row {
            id: 5,
            kind: DebtKind::Adjustment,
            debit: 0,
            credit: 10_000,
            document: None,
            day: 5,
            balance: 140_000,
        },
        Row {
            id: 6,
            kind: DebtKind::Sale,
            debit: 30_000,
            credit: 0,
            document: facture(44),
            day: 6,
            balance: 170_000,
        },
        Row {
            id: 7,
            kind: DebtKind::Payment,
            debit: 0,
            credit: 20_000,
            document: None,
            day: 7,
            balance: 150_000,
        },
        Row {
            id: 8,
            kind: DebtKind::Sale,
            debit: 25_000,
            credit: 0,
            document: facture(45),
            day: 8,
            balance: 175_000,
        },
        Row {
            id: 9,
            kind: DebtKind::Avoir,
            debit: 0,
            credit: 15_000,
            document: Some(DocumentRef {
                kind: DocumentKind::Avoir,
                number: 3,
            }),
            day: 9,
            balance: 160_000,
        },
        Row {
            id: 10,
            kind: DebtKind::Sale,
            debit: 40_000,
            credit: 0,
            document: facture(46),
            day: 10,
            balance: 200_000,
        },
        Row {
            id: 11,
            kind: DebtKind::Payment,
            debit: 0,
            credit: 60_000,
            document: None,
            day: 11,
            balance: 140_000,
        },
        Row {
            id: 12,
            kind: DebtKind::Sale,
            debit: 35_000,
            credit: 0,
            document: facture(47),
            day: 12,
            balance: 175_000,
        },
    ]
}

/// The account as `services::debt::recent` answers it: newest first, and the
/// balance the whole ledger sums to rather than the sum of the rows carried.
fn a_long_account() -> RecentStatement {
    let mut entries: Vec<StatementEntry> = twelve_movements()
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
        .collect();
    entries.reverse();
    RecentStatement {
        balance: Money::centimes(BALANCE),
        entries,
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
/// The facture and statement suites have the same parser, copied rather than
/// shared for the reason their headers give: two golden suites checking each
/// other's files through one helper can both be made green by editing the
/// helper once.
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

/// The rows the slip prints: the newest ten of whatever it was handed.
fn shown(slip: &RecentStatement) -> Vec<&StatementEntry> {
    slip.entries.iter().take(MOVEMENTS).collect()
}

/// Every amount in the golden, against what the slip holds.
fn the_golden_says_what_the_slip_holds(html: &str, slip: &RecentStatement, lang: Lang) {
    assert_eq!(
        centimes(&one_amount(html, "balance")),
        slip.balance.as_centimes(),
        "the balance"
    );
    // The running column, row by row and in the order the slip shows them.
    assert_eq!(
        amounts(html, "running")
            .iter()
            .map(|printed| centimes(printed))
            .collect::<Vec<i64>>(),
        shown(slip)
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
        shown(slip)
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
        shown(slip)
            .iter()
            .map(|line| line.entry.credit.as_centimes())
            .filter(|c| *c != 0)
            .collect::<Vec<i64>>(),
        "the credit column"
    );
    // The words are the balance and nothing else, and they are written by the
    // money module rather than by this page.
    assert_eq!(
        in_words(html),
        amount_in_words(slip.balance, lang).unwrap(),
        "the words line is not the balance written out"
    );
    for line in shown(slip) {
        if let Some(document) = line.document {
            let printed = document_number(document);
            assert!(
                html.contains(&printed),
                "the row citing {printed} does not print its number"
            );
        }
    }
    assert!(html.contains(text(Key::DebtSlip, lang)));
    assert!(html.contains(text(Key::LastMovements, lang)));
}

#[test]
fn the_debt_slip_is_its_golden_in_every_language() {
    let slip = a_long_account();
    let shop = a_shop();
    let customer = a_company();
    let mut updated = Vec::new();
    for lang in Lang::ALL {
        let rendered = render_debt_slip(&shop, &customer, &slip, printed_at(), lang).unwrap();
        let expected = golden(lang, &rendered, &mut updated);
        assert_eq!(rendered, expected, "the {lang:?} slip differs");
        the_golden_says_what_the_slip_holds(&expected, &slip, lang);
    }
    refuse_a_silent_regeneration(&updated);
}

#[test]
fn the_slip_prints_the_newest_ten_movements_and_a_balance_that_counts_them_all() {
    let slip = a_long_account();

    let page = render_debt_slip(&a_shop(), &a_company(), &slip, printed_at(), Lang::Fr).unwrap();

    // The twelve are the fixture's, oldest first, and the window is the last
    // ten of them: the two oldest are the ones that fall off.
    let all = twelve_movements();
    let (dropped, kept) = all.split_at(all.len() - MOVEMENTS);
    assert_eq!(
        dropped.len(),
        2,
        "the fixture stopped being longer than a page"
    );

    // Named row by row rather than counted. The running column is the whole
    // of the window in the order the slip shows it, so a page that dropped a
    // different pair, kept eleven, or reversed them fails here; a count alone
    // would pass on any ten rows.
    assert_eq!(
        amounts(&page, "running")
            .iter()
            .map(|printed| centimes(printed))
            .collect::<Vec<i64>>(),
        kept.iter()
            .rev()
            .map(|row| row.balance)
            .collect::<Vec<i64>>(),
        "the slip does not print exactly the newest ten movements"
    );
    // And the same window read off the documents: every row on the page that
    // cites one prints its number, and neither row that fell off does.
    for row in kept {
        if let Some(document) = row.document {
            let printed = document_number(document);
            assert!(
                page.contains(&printed),
                "the row citing {printed} is not on the page"
            );
        }
    }
    for row in dropped {
        if let Some(document) = row.document {
            let printed = document_number(document);
            assert!(
                !page.contains(&printed),
                "{printed} is older than the newest ten and reached the paper"
            );
        }
    }
    // And the figure the customer is asked for counts all twelve, not the ten
    // shown: the newest movement's running balance and the slip's balance are
    // the same figure here only because that movement is the newest one.
    assert_eq!(centimes(&one_amount(&page, "balance")), BALANCE);
}

#[test]
fn the_slip_prints_the_identifiers_of_a_company_and_never_a_consumers() {
    let slip = a_long_account();
    let company = render_debt_slip(&a_shop(), &a_company(), &slip, printed_at(), Lang::Fr).unwrap();
    let consumer =
        render_debt_slip(&a_shop(), &a_consumer(), &slip, printed_at(), Lang::Fr).unwrap();

    assert!(company.contains("000916001234567"), "the NIF is missing");
    assert!(company.contains("16/00-1234567 B 09"));
    assert!(!consumer.contains("000916001234567"));
    assert!(!consumer.contains("16/00-1234567 B 09"));
    assert!(consumer.contains("Ahmed Cherif"));
    assert!(consumer.contains("12 rue Didouche Mourad, Alger"));
    // The shop's own identifiers are on both: the header names who handed the
    // paper over whoever it was handed to.
    assert!(consumer.contains("000916007654321"));
}

#[test]
fn a_customer_who_owes_nothing_yet_gets_a_slip_that_says_so() {
    let slip = RecentStatement {
        balance: Money::ZERO,
        entries: Vec::new(),
    };

    let page = render_debt_slip(&a_shop(), &a_company(), &slip, printed_at(), Lang::Fr).unwrap();

    assert!(page.contains(text(Key::NoMovement, Lang::Fr)));
    assert_eq!(centimes(&one_amount(&page, "balance")), 0);
    assert!(
        amounts(&page, "running").is_empty(),
        "an empty ledger printed a movement row"
    );
}

#[test]
fn a_balance_the_shop_owes_prints_the_amount_and_says_whose_way_it_goes() {
    // The shop holding money for a customer is what an avoir leaves behind
    // (T6). The words carry no sign, so the page has to say the direction in
    // words of its own or it reads as the opposite of what it means.
    let slip = RecentStatement {
        balance: Money::centimes(-20_000),
        entries: vec![StatementEntry {
            entry: movement(21, DebtKind::Avoir, 0, 20_000, facture(43), at(9, 11)),
            balance_after: Money::centimes(-20_000),
            document: facture(43),
        }],
    };

    let page = render_debt_slip(&a_shop(), &a_company(), &slip, printed_at(), Lang::Fr).unwrap();

    assert_eq!(centimes(&one_amount(&page, "balance")), -20_000);
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
fn the_slip_says_on_its_face_that_it_proves_nothing() {
    // A counter paper with a shop's name, a customer's name and an amount on
    // it is exactly the shape a comptable would file. It carries no number
    // from a series, no stamp and no TVA, so it has to say so: the facture and
    // the statement are the papers that prove something.
    for lang in Lang::ALL {
        let page = render_debt_slip(
            &a_shop(),
            &a_company(),
            &a_long_account(),
            printed_at(),
            lang,
        )
        .unwrap();
        assert!(
            page.contains(text(Key::NoFiscalValue, lang)),
            "the {lang:?} slip does not say it has no fiscal value"
        );
        assert!(
            !page.contains(text(Key::Stamp, lang)),
            "the {lang:?} slip names the stamp duty"
        );
        assert!(
            !page.contains(text(Key::Tva, lang)),
            "the {lang:?} slip names the TVA"
        );
    }
}

#[test]
fn the_arabic_slip_reads_right_to_left_and_says_it_is_unreviewed() {
    let page = render_debt_slip(
        &a_shop(),
        &a_company(),
        &a_long_account(),
        printed_at(),
        Lang::Ar,
    )
    .unwrap();
    assert!(page.contains("dir=\"rtl\""));
    assert!(page.contains("unreviewed by a native speaker"));
}

#[test]
fn the_digits_are_western_in_every_language() {
    for lang in Lang::ALL {
        let page = render_debt_slip(
            &a_shop(),
            &a_company(),
            &a_long_account(),
            printed_at(),
            lang,
        )
        .unwrap();
        assert!(
            page.contains("1\u{202f}750,00"),
            "the {lang:?} slip does not carry the balance in Western digits"
        );
        assert!(
            !page.chars().any(|c| ('\u{0660}'..='\u{0669}').contains(&c)),
            "the {lang:?} slip carries Eastern Arabic digits"
        );
    }
}
