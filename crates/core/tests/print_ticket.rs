// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The 80 mm ticket against its golden files, one per language and one per
//! régime (features.md §4: "Golden-file test for every template × language
//! against fixed fixtures. A template change is a reviewed golden diff").
//!
//! Regenerate with `UPDATE_GOLDENS=1 cargo test -p dzpos-core --test
//! print_ticket`. The run that rewrites them fails on purpose: a golden
//! nobody looked at must never be green in the same run that wrote it.
//!
//! The amounts in each golden are parsed back out of the file and compared
//! to the document's stored totals. That check never calls the renderer, so
//! a golden that drifts from the totals cannot be accepted by regenerating
//! it.

use std::path::PathBuf;

use chrono::NaiveDate;
use dzpos_core::lang::Lang;
use dzpos_core::money::{
    compute_totals, Bps, Line, Money, PaymentMode, Regime, TotalsOptions, TvaLine,
};
use dzpos_core::print::render_ticket;
use dzpos_core::print::strings::{text, Key};
use dzpos_core::services::documents::{
    BalanceTriple, Document, DocumentKind, DocumentLine, DocumentStatus, PartyBlock, PartyKind,
    SellerBlock,
};

const SHOP: i32 = 1;
const OWNER: i32 = 1;

/// The three lines of the fixed sale, as thousandths, centimes and basis
/// points: 2 × 150,00 at 19 %, 1,5 kg × 320,00 at 9 %, and 1 × 80,00 at
/// 0 % with a 10,00 line discount. Written out rather than computed so a
/// change to the fixture is visible in a diff.
const LINES: [(&str, i64, i64, i64, u32); 3] = [
    ("Café moulu 250 g", 2_000, 15_000, 0, 1900),
    ("Farine", 1_500, 32_000, 0, 900),
    ("Pain", 1_000, 8_000, 1_000, 0),
];

/// 20,00 DA off the whole basket.
const GLOBAL_DISCOUNT: i64 = 2_000;
/// A thousand dinars handed over in cash.
const TENDERED: i64 = 100_000;

/// What the credit customer owed before this ticket: 2 500,00 DA. Written
/// out rather than derived, so the triple on the paper is checked against a
/// number a reader chose and not against one the code worked out.
const OLD_BALANCE: i64 = 250_000;

/// What the shop was holding for the other credit customer before this
/// ticket: 5 000,00 DA the wrong way round. Large enough that the basket
/// cannot take the closing balance back above zero, so the paper has to name
/// the customer a creditor and not a debtor.
const CREDIT_HELD: i64 = -500_000;

/// The same basket, sold three ways. Each is a golden per language: the
/// régime decides the TVA half of the paper and the payment mode decides
/// the money half, and neither is a variation of the other.
#[derive(Debug, Clone, Copy)]
enum Case {
    /// Réel, cash: the whole ticket, recap and stamp and change included.
    Reel,
    /// IFU, cash: no recap, no rate on a line, no "HT" on the total row.
    Ifu,
    /// Réel, card: the droit de timbre is cash only (Code du timbre 2026
    /// art. 100-I), and a card takes no note and gives no coins back, so
    /// three rows that are on every other ticket are absent from this one.
    Card,
    /// Réel, credit: nothing tendered and no stamp, like the card, plus the
    /// three rows of the debt block. The customer owed 2 500,00 before this
    /// basket and owes that plus the net to pay after it.
    Credit,
    /// Réel, credit, to a customer the shop is holding money for: the same
    /// three rows, closing below zero. The facture calls that a credit and
    /// the ticket has to call it the same thing, or one basket rung up twice
    /// tells the customer two different stories.
    CreditHeld,
}

impl Case {
    const ALL: [Case; 5] = [
        Case::Reel,
        Case::Ifu,
        Case::Card,
        Case::Credit,
        Case::CreditHeld,
    ];

    const fn regime(self) -> Regime {
        match self {
            Case::Reel | Case::Card | Case::Credit | Case::CreditHeld => Regime::Reel,
            Case::Ifu => Regime::Ifu,
        }
    }

    const fn payment_mode(self) -> PaymentMode {
        match self {
            Case::Reel | Case::Ifu => PaymentMode::Cash,
            Case::Card => PaymentMode::Card,
            Case::Credit | Case::CreditHeld => PaymentMode::Credit,
        }
    }

    /// What the customer's account stood at before this basket, on the cases
    /// that name a customer at all.
    const fn old_balance(self) -> Option<i64> {
        match self {
            Case::Reel | Case::Ifu | Case::Card => None,
            Case::Credit => Some(OLD_BALANCE),
            Case::CreditHeld => Some(CREDIT_HELD),
        }
    }

    /// What the golden's name carries after the language.
    const fn suffix(self) -> &'static str {
        match self {
            Case::Reel => "",
            Case::Ifu => "-ifu",
            Case::Card => "-card",
            Case::Credit => "-credit",
            Case::CreditHeld => "-credit-held",
        }
    }
}

mod common;

fn goldens_dir() -> PathBuf {
    common::goldens_dir("ticket_80mm")
}

/// The one document every golden renders. Its totals come from
/// `compute_totals`, not from hand-typed numbers: a golden pins the
/// template, and the money module is what pins the money.
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
            // On for the shop in every case: the stamp being absent from
            // the card ticket has to be the rule doing it, not a setting.
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

    // 9 September 2026, 14:05, already on the shop's calendar: a document's
    // `issued_at` is written through services::clock (features.md, the
    // shop's clock is UTC+1), so printing it needs no conversion.
    let issued_at = NaiveDate::from_ymd_opt(2026, 9, 9)
        .unwrap()
        .and_hms_opt(14, 5, 0)
        .unwrap();
    // Cash is the only mode that tenders anything, so the other modes store
    // neither half and the ticket shows neither row.
    let tendered = match case.payment_mode() {
        PaymentMode::Cash => Some(Money::centimes(TENDERED)),
        PaymentMode::Card | PaymentMode::Credit => None,
    };
    // Only the credit sale names a customer here, so only it carries a buyer
    // block and a balance. The triple is the ledger's answer at issue time:
    // what was owed, what this document leaves unpaid, what is owed now.
    let balance = case.old_balance().map(|old| BalanceTriple {
        old_balance: Money::centimes(old),
        remaining_debt: totals.net_to_pay,
        total_debt: Money::centimes(old).checked_add(totals.net_to_pay).unwrap(),
    });
    let buyer = case.old_balance().map(|_| PartyBlock {
        name: "Entreprise Amrani".to_owned(),
        party_kind: PartyKind::Company,
        rc: Some("16/00-7654321 B 25".to_owned()),
        nif: Some("000216007654321".to_owned()),
        nis: None,
        ai: None,
        address: Some("7 rue Larbi Ben M'hidi, Alger".to_owned()),
    });

    Document {
        id: 1,
        shop_id: SHOP,
        kind: DocumentKind::Ticket,
        series: DocumentKind::Ticket.series().to_owned(),
        number: 123,
        issued_at,
        user_id: OWNER,
        regime,
        payment_mode: case.payment_mode(),
        // NIS and AI are left out on purpose: a ticket prints the seller
        // identifiers it has and no empty rows for the ones it has not
        // (features.md, party identifiers row).
        seller: SellerBlock {
            name: "Mon magasin".to_owned(),
            rc: Some("16/00-1234567 B 25".to_owned()),
            nif: Some("000216001234567".to_owned()),
            nis: None,
            ai: None,
            address: Some("12 rue Didouche Mourad, Alger".to_owned()),
            phone: Some("0555 12 34 56".to_owned()),
        },
        customer_id: case.old_balance().map(|_| 7),
        // A till ticket is sold to whoever walked in unless the sale is on
        // credit, and then it is owed by somebody the paper has to name.
        buyer,
        ref_document_id: None,
        balance,
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
    format!("{}{}.html", lang.tag(), case.suffix())
}

/// The golden as it stands on disk, or a rewritten one under
/// `UPDATE_GOLDENS=1`. Rewriting is not a pass: `updated` says so and the
/// test that called this fails at the end of the run.
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

fn refuse_a_silent_regeneration(updated: &[String]) {
    assert!(
        updated.is_empty(),
        "rewrote {updated:?}: read the diff, then run the test again without UPDATE_GOLDENS"
    );
}

/// The text of the one `<span class="amount amount-{marker}">` in the file.
/// Reads the golden, never the renderer: that is what makes the check worth
/// running against a file a regeneration just wrote.
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

/// A row a ticket carries only sometimes: the stamp on a cash sale, the two
/// halves of the change. Absent is an answer, so it comes back as `None`
/// rather than as a zero nobody printed.
fn optional_amount(html: &str, marker: &str) -> Option<i64> {
    let found = amounts(html, marker);
    assert!(found.len() <= 1, "more than one {marker} row: {found:?}");
    found.first().map(|printed| centimes(printed))
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

/// The text of every `<span class="rate rate-{marker}">` in the file: the
/// rate cells, which carry a figure no amount parser would catch. The facture
/// suite has the same pair of helpers, copied rather than shared for the
/// reason its own header gives.
fn rates(html: &str, marker: &str) -> Vec<String> {
    let opening = format!("<span class=\"rate rate-{marker}\">");
    html.split(&opening)
        .skip(1)
        .map(|rest| {
            let end = rest.find("</span>").expect("a rate span never closes");
            rest[..end].to_owned()
        })
        .collect()
}

/// "9,5 %" back to 950 basis points, and "19 %" to 1900. The golden's own
/// digits, read by a parser that shares no code with the one that printed
/// them.
fn bps(printed: &str) -> u32 {
    let digits: String = printed
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ',')
        .collect();
    let (whole, rest) = digits.split_once(',').unwrap_or((digits.as_str(), ""));
    let whole: u32 = whole.parse().unwrap();
    let rest: u32 = format!("{rest:0<2}").parse().unwrap();
    whole * 100 + rest
}

/// Every amount in the golden, against the totals the document stores.
fn the_golden_says_what_the_document_stores(html: &str, doc: &Document) {
    let totals = &doc.totals;
    assert_eq!(
        centimes(&one_amount(html, "total")),
        totals.total_ht.as_centimes(),
        "the total row"
    );
    // The discount is stored as what it takes off and printed as what it
    // does to the column, so the paper carries the sign the document does
    // not: -20,00 under a total of 850,00.
    assert_eq!(
        centimes(&one_amount(html, "discount")),
        -totals.discount.as_centimes(),
        "the discount row"
    );
    // Nothing due prints no row: a stamp of 0,00 on a card sale would say
    // the tax was charged and came to nothing.
    assert_eq!(
        optional_amount(html, "stamp"),
        (totals.stamp != Money::ZERO).then(|| totals.stamp.as_centimes()),
        "the stamp row"
    );
    assert_eq!(
        centimes(&one_amount(html, "net-to-pay")),
        totals.net_to_pay.as_centimes(),
        "the net to pay row"
    );

    let tva = amounts(html, "tva");
    assert_eq!(
        tva.len(),
        totals.tva_by_rate.len(),
        "one TVA row per rate the document carries"
    );
    for (printed, row) in tva.iter().zip(&totals.tva_by_rate) {
        assert_eq!(centimes(printed), row.amount.as_centimes(), "a TVA row");
    }

    // Each recap row names the rate its tax belongs to, and the paper is read
    // for it the way the facture's is: an amount that landed under the wrong
    // rate adds up on the page and is wrong on it.
    let recap_rates = rates(html, "tva");
    assert_eq!(
        recap_rates.len(),
        totals.tva_by_rate.len(),
        "one rate per recap row"
    );
    for (printed, row) in recap_rates.iter().zip(&totals.tva_by_rate) {
        assert_eq!(bps(printed), row.rate.as_u32(), "a recap row's rate");
    }

    let lines = amounts(html, "line");
    assert_eq!(lines.len(), doc.lines.len(), "one total per sold line");
    for (printed, line) in lines.iter().zip(&doc.lines) {
        assert_eq!(centimes(printed), line.line_total.as_centimes(), "a line");
    }

    // The rate on a line is the rate the line was sold at, réel only: under
    // the IFU there is no rate column at all, not a column of zeroes
    // (`an_ifu_ticket_names_no_tax_in_any_language`, below).
    let line_rates = rates(html, "line");
    let expected: Vec<u32> = match doc.regime {
        Regime::Reel => doc.lines.iter().map(|l| l.rate_bps.as_u32()).collect(),
        Regime::Ifu => Vec::new(),
    };
    assert_eq!(
        line_rates.iter().map(|r| bps(r)).collect::<Vec<u32>>(),
        expected,
        "the rate cells of the lines"
    );

    // A line discount is an amount on the paper like any other, so it is
    // read back like any other. A line that carries none prints no row.
    let discounted: Vec<&DocumentLine> = doc
        .lines
        .iter()
        .filter(|l| l.line_discount != Money::ZERO)
        .collect();
    let printed = amounts(html, "line-discount");
    assert_eq!(
        printed.len(),
        discounted.len(),
        "one row per discounted line"
    );
    for (printed, line) in printed.iter().zip(discounted) {
        assert_eq!(
            centimes(printed),
            -line.line_discount.as_centimes(),
            "a line discount"
        );
    }

    assert_eq!(
        optional_amount(html, "tendered"),
        doc.tendered.map(Money::as_centimes),
        "the tendered row"
    );
    assert_eq!(
        optional_amount(html, "change"),
        doc.change.map(Money::as_centimes),
        "the change row"
    );

    // The debt block, the three amounts together or none at all. Read off
    // the paper and compared with the triple the document stores, so a
    // regenerated golden cannot quietly print a balance the document never
    // held.
    for (marker, amount) in [
        ("old-balance", doc.balance.map(|b| b.old_balance)),
        ("this-document", doc.balance.map(|b| b.remaining_debt)),
        ("total-debt", doc.balance.map(|b| b.total_debt)),
    ] {
        assert_eq!(
            optional_amount(html, marker),
            amount.map(Money::as_centimes),
            "the {marker} row"
        );
    }
}

/// One case against its three goldens: the render is the file, and the file
/// says what the document stores.
fn each_language_of(case: Case) {
    let doc = fixed_sale(case);
    let mut updated = Vec::new();
    for lang in Lang::ALL {
        let rendered = render_ticket(&doc, lang).unwrap();
        let expected = golden(lang, case, &rendered, &mut updated);
        assert_eq!(
            rendered,
            expected,
            "{} is not what the template renders",
            golden_name(lang, case)
        );
        the_golden_says_what_the_document_stores(&expected, &doc);
    }
    refuse_a_silent_regeneration(&updated);
}

#[test]
fn the_reel_ticket_is_its_golden_in_every_language() {
    each_language_of(Case::Reel);
}

#[test]
fn the_ifu_ticket_is_its_golden_in_every_language() {
    each_language_of(Case::Ifu);
}

#[test]
fn the_card_ticket_is_its_golden_in_every_language() {
    each_language_of(Case::Card);
}

/// A card sale is missing three rows every cash ticket has, and each is
/// missing for its own reason: the droit de timbre is due on cash only
/// (Code du timbre 2026 art. 100-I), and nothing is tendered or given back.
/// The cash ticket is checked in the same test so a template that dropped
/// all three for everyone could not pass this.
#[test]
fn the_credit_ticket_is_its_golden_in_every_language() {
    each_language_of(Case::Credit);
}

#[test]
fn the_credit_held_ticket_is_its_golden_in_every_language() {
    each_language_of(Case::CreditHeld);
}

/// A balance that closes below zero is money the shop is holding, and the
/// facture has always named it that way (`the_balance_block_says_credit_when
/// _the_avoir_closes_below_zero`). The ticket printed "total dû" over the
/// same figure, so one customer's account had two names depending on which
/// paper they were handed. The sign decides it and not the payment mode: the
/// debtor's ticket beside it is what keeps a template that renamed the row
/// for everybody from passing.
#[test]
fn a_ticket_that_closes_below_zero_names_the_customer_a_creditor() {
    let held = fixed_sale(Case::CreditHeld);
    let owed = fixed_sale(Case::Credit);
    let closing = held
        .balance
        .expect("the credit-held ticket carries a balance")
        .total_debt;
    assert!(
        closing.is_negative(),
        "the fixture no longer closes below zero: {closing:?}"
    );

    for lang in Lang::ALL {
        let html = render_ticket(&held, lang).unwrap();
        assert!(
            html.contains(text(Key::TotalCredit, lang)),
            "the {lang:?} ticket does not name the credit"
        );
        assert!(
            !html.contains(text(Key::TotalDebt, lang)),
            "the {lang:?} ticket still calls the credit a debt"
        );
        assert_eq!(
            centimes(&one_amount(&html, "total-debt")),
            closing.as_centimes(),
            "the {lang:?} ticket prints a figure the document does not store"
        );

        let debtor = render_ticket(&owed, lang).unwrap();
        assert!(
            debtor.contains(text(Key::TotalDebt, lang)),
            "the {lang:?} ticket stopped naming a debt a debt"
        );
        assert!(
            !debtor.contains(text(Key::TotalCredit, lang)),
            "the {lang:?} ticket calls a debt a credit"
        );
    }
}

/// The paper a customer takes away when they have paid nothing: it says how
/// it was paid, it carries none of the three cash rows, and it closes on
/// what is now owed. The three balance amounts add up on the paper itself,
/// so a reader can check the closing figure against the opening one without
/// knowing what the till computed.
#[test]
fn a_credit_ticket_says_credit_carries_no_cash_row_and_closes_on_the_debt() {
    let doc = fixed_sale(Case::Credit);
    assert_eq!(doc.totals.stamp, Money::ZERO, "the stamp is cash only");
    assert_eq!(doc.tendered, None);
    assert_eq!(doc.change, None);

    for lang in Lang::ALL {
        let html = render_ticket(&doc, lang).unwrap();
        assert!(html.contains(text(Key::Credit, lang)), "{lang:?}");
        for absent in ["stamp", "tendered", "change"] {
            assert!(
                amounts(&html, absent).is_empty(),
                "the {lang:?} credit ticket keeps a {absent} row"
            );
        }
        for label in [
            Key::Balance,
            Key::OldBalance,
            Key::ThisDocument,
            Key::TotalDebt,
        ] {
            assert!(html.contains(text(label, lang)), "{lang:?} {label:?}");
        }
        let old = centimes(&one_amount(&html, "old-balance"));
        let this = centimes(&one_amount(&html, "this-document"));
        let total = centimes(&one_amount(&html, "total-debt"));
        assert_eq!(old, OLD_BALANCE);
        assert_eq!(this, doc.totals.net_to_pay.as_centimes());
        assert_eq!(old + this, total, "the block does not add up on the paper");
    }

    // A ticket that names nobody prints none of it: three rows of zeroes
    // would tell a walk-in customer they owe nothing they never owed.
    let anonymous = render_ticket(&fixed_sale(Case::Reel), Lang::Fr).unwrap();
    for absent in ["old-balance", "this-document", "total-debt"] {
        assert!(
            amounts(&anonymous, absent).is_empty(),
            "an anonymous ticket carries a {absent} row"
        );
    }
    assert!(!anonymous.contains(text(Key::TotalDebt, Lang::Fr)));
}

#[test]
fn a_card_ticket_carries_no_stamp_and_neither_half_of_the_change() {
    let card = fixed_sale(Case::Card);
    assert_eq!(card.totals.stamp, Money::ZERO, "the stamp is cash only");
    assert_eq!(card.tendered, None);
    assert_eq!(card.change, None);

    for lang in Lang::ALL {
        let html = render_ticket(&card, lang).unwrap();
        for absent in ["stamp", "tendered", "change"] {
            assert!(
                amounts(&html, absent).is_empty(),
                "the {lang:?} card ticket keeps a {absent} row"
            );
        }
        assert!(!html.contains(text(Key::Stamp, lang)), "{lang:?}");
        assert!(!html.contains(text(Key::Tendered, lang)), "{lang:?}");
        assert!(!html.contains(text(Key::Change, lang)), "{lang:?}");
        // It says how it was paid, and it is still a réel ticket.
        assert!(html.contains(text(Key::Card, lang)), "{lang:?}");
        assert_eq!(amounts(&html, "tva").len(), 3, "{lang:?}");
    }

    let cash = render_ticket(&fixed_sale(Case::Reel), Lang::Fr).unwrap();
    for present in ["stamp", "tendered", "change"] {
        assert_eq!(
            amounts(&cash, present).len(),
            1,
            "the cash ticket lost its {present} row"
        );
    }
}

/// The IFU rule read off the paper: the word is not on it,
/// in any of the three languages it could be on it in. CTCA 2026 art. 64
/// forbids an IFU document from mentioning the tax at all, so this is the
/// rule and not a layout preference.
#[test]
fn an_ifu_ticket_names_no_tax_in_any_language() {
    let doc = fixed_sale(Case::Ifu);
    for lang in Lang::ALL {
        let html = render_ticket(&doc, lang).unwrap();
        for forbidden in ["TVA", "VAT", "ت.ق.م"] {
            assert!(
                !html.contains(forbidden),
                "{forbidden} is on the {lang:?} IFU ticket"
            );
        }
        assert!(
            !html.contains("amount-tva"),
            "the {lang:?} IFU ticket keeps a TVA row"
        );
        // Nor a rate cell anywhere: not in the recap it has none of, and not
        // on a line either. A rate is a tax figure whatever the row it sits
        // on, and a column of 0 % would name the tax as surely as the word.
        assert!(
            rates(&html, "tva").is_empty() && rates(&html, "line").is_empty(),
            "the {lang:?} IFU ticket keeps a rate cell"
        );
        // "hors taxe" names a tax too, so the total row changes word under
        // the IFU rather than only losing the recap below it.
        assert!(
            !html.contains(text(Key::TotalHt, lang)),
            "the {lang:?} IFU ticket still says {}",
            text(Key::TotalHt, lang)
        );
        assert!(html.contains(text(Key::Total, lang)), "{lang:?}");
    }
    assert!(
        doc.totals.tva_by_rate.is_empty(),
        "an IFU document stores no TVA recap"
    );
}

/// The réel ticket carries the recap the IFU one must not: one row per rate
/// present in the lines, 0 % included, because the rate groups are a field
/// of the document and not a summary of the lines.
#[test]
fn a_reel_ticket_carries_one_tva_row_per_rate() {
    let doc = fixed_sale(Case::Reel);
    assert_eq!(doc.totals.tva_by_rate.len(), 3, "0 %, 9 % and 19 %");
    for lang in Lang::ALL {
        let html = render_ticket(&doc, lang).unwrap();
        assert_eq!(amounts(&html, "tva").len(), 3, "{lang:?}");
    }
}

#[test]
fn the_arabic_ticket_reads_right_to_left_and_says_it_is_unreviewed() {
    let doc = fixed_sale(Case::Reel);
    let ar = render_ticket(&doc, Lang::Ar).unwrap();
    assert!(ar.contains("dir=\"rtl\""), "the Arabic ticket is not RTL");
    assert!(ar.contains("lang=\"ar\""));
    assert!(
        ar.contains("unreviewed by a native speaker, like words_ar"),
        "the Arabic ticket does not disclose that its wording is unreviewed"
    );
    for other in [Lang::Fr, Lang::En] {
        let html = render_ticket(&doc, other).unwrap();
        assert!(html.contains("dir=\"ltr\""), "{other:?}");
        assert!(
            !html.contains("unreviewed by a native speaker"),
            "{other:?}"
        );
    }
}

/// Western digits in every language, the shop's number format: an Arabic
/// ticket a comptable reads carries the same figures as the French one.
#[test]
fn the_digits_are_western_in_every_language() {
    for case in Case::ALL {
        let doc = fixed_sale(case);
        for lang in Lang::ALL {
            let html = render_ticket(&doc, lang).unwrap();
            assert!(
                !html.chars().any(|c| ('\u{0660}'..='\u{0669}').contains(&c)),
                "{lang:?} {case:?} carries Arabic-Indic digits"
            );
            assert_eq!(
                centimes(&one_amount(&html, "net-to-pay")),
                doc.totals.net_to_pay.as_centimes(),
                "{lang:?} {case:?}"
            );
        }
    }
}

/// A stored IFU document that carries a TVA recap contradicts the régime it
/// was issued under: a restored file, a repaired row, an import. There is no
/// honest ticket for it. Printing the recap would name a tax the document
/// must not name (CTCA 2026 art. 64) and dropping it silently would hand the
/// customer a total whose parts do not add up, so the printer refuses.
#[test]
fn an_ifu_document_carrying_a_tva_recap_is_refused_not_quietly_stripped() {
    let mut doc = fixed_sale(Case::Ifu);
    doc.totals.tva_by_rate.push(TvaLine {
        rate: Bps::new(1900).unwrap(),
        base: Money::centimes(29_295),
        amount: Money::centimes(5_566),
    });
    for lang in Lang::ALL {
        let err = render_ticket(&doc, lang).unwrap_err();
        assert_eq!(err.code(), "print", "{lang:?}: {err:?}");
    }
}

/// The refusal above is about the régime, not about the recap being there:
/// a réel document is printable whatever its recap holds. An empty one is a
/// réel sale of nothing taxable, and a 0 % only recap is the ordinary shape
/// of a basket of exempt goods; both still print.
#[test]
fn a_reel_document_prints_with_an_empty_recap_and_with_a_zero_rate_one() {
    let mut empty = fixed_sale(Case::Reel);
    empty.totals.tva_by_rate.clear();
    let mut exempt = fixed_sale(Case::Reel);
    exempt.totals.tva_by_rate = vec![TvaLine {
        rate: Bps::ZERO,
        base: exempt.totals.subtotal_ht,
        amount: Money::ZERO,
    }];
    for lang in Lang::ALL {
        assert_eq!(
            amounts(&render_ticket(&empty, lang).unwrap(), "tva").len(),
            0
        );
        assert_eq!(
            amounts(&render_ticket(&exempt, lang).unwrap(), "tva").len(),
            1
        );
    }
}

/// The number a customer quotes when they come back. `TK-000123`, not the
/// counter's own `doc_ticket`.
#[test]
fn the_ticket_number_is_the_series_prefix_and_six_digits() {
    let doc = fixed_sale(Case::Reel);
    for lang in Lang::ALL {
        assert!(
            render_ticket(&doc, lang).unwrap().contains("TK-000123"),
            "{lang:?}"
        );
    }
}
