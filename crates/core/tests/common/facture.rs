//! The facture fixtures and the golden-file parser both facture suites read.
//!
//! Shared between `print_facture.rs` and `print_facture_compact.rs` on
//! purpose, and only between those two. The warning the facture suite has
//! carried since it was written is that two golden suites checking each
//! other's files through one helper can both be made green by editing the
//! helper once, which is why the ticket suite keeps its own copy of this
//! parser. Two layouts of the same document are the exception it was written
//! against: `the_compact_layout_says_everything_the_standard_one_says`
//! compares the two renders field by field, so a helper edited to make one
//! suite green makes that comparison fail rather than hiding anything.
//!
//! Nothing here renders anything. The parsers read a file off disk and say
//! what is in it, which is what makes them worth running against a golden a
//! regeneration just wrote.

use std::path::PathBuf;

use chrono::{Datelike, NaiveDate};
use dzpos_core::lang::Lang;
use dzpos_core::money::words::amount_in_words;
use dzpos_core::money::{compute_totals, Bps, Line, Money, PaymentMode, Regime, TotalsOptions};
use dzpos_core::print::{
    render_facture_with, Cancellation, FactureInput, FactureLayout, Page, Paper,
};
use dzpos_core::services::documents::{
    BalanceTriple, Document, DocumentKind, DocumentLine, DocumentStatus, PartyBlock, PartyKind,
    SellerBlock,
};

pub(crate) const SHOP: i32 = 1;
pub(crate) const OWNER: i32 = 1;
pub(crate) const CUSTOMER: i32 = 7;

/// The three lines of the fixed sale, the ticket fixture's amounts: 2 ×
/// 150,00 at 19 %, 1,5 kg × 320,00 at 9 %, and 1 × 80,00 at 0 % with a
/// 10,00 line discount.
///
/// The first name is not the ticket's. A product name is typed by the shop
/// and lands in the page as it was typed, so one fixture name carries an
/// ampersand and a pair of angle brackets: the goldens then pin askama's
/// escaping, and a template that ever rendered a name raw would show it as
/// a golden diff rather than as a broken facture at a customer's desk.
pub(crate) const LINES: [(&str, i64, i64, i64, u32); 3] = [
    ("Huile <Elio> & Co 5 L", 2_000, 15_000, 0, 1900),
    ("Farine", 1_500, 32_000, 0, 900),
    ("Pain", 1_000, 8_000, 1_000, 0),
];

/// The two lines the avoir takes back, a part of the basket and not all of
/// it: one of the two bottles and half of the flour. A partial avoir is
/// what the avoir rule allows, and an avoir printing the facture's own
/// totals is the mistake these goldens have to be able to catch, so its
/// lines and its amounts are none of the facture's.
///
/// The first name carries the markup the facture's does: a reprint of an
/// avoir escapes a product name or it does not, and the avoir goldens say
/// which.
pub(crate) const AVOIR_LINES: [(&str, i64, i64, i64, u32); 2] = [
    ("Huile <Elio> & Co 5 L", 1_000, 15_000, 0, 1900),
    ("Farine", 500, 32_000, 0, 900),
];

/// 20,00 DA off the whole basket.
pub(crate) const GLOBAL_DISCOUNT: i64 = 2_000;
/// What the customer owed before this facture: 1 500,00 DA.
pub(crate) const OLD_BALANCE: i64 = 150_000;
/// What the customer owed before the avoir: 300,00 DA, less than the avoir
/// gives back, so the triple closes below zero and the block has to say
/// credit rather than debt.
pub(crate) const OLD_BALANCE_BEFORE_THE_AVOIR: i64 = 30_000;

/// The day the fixed sale was made, the later day the avoir is written and
/// the later day the facture is cancelled. They differ on purpose: a
/// reference line or a cancellation line that printed the document's own
/// date instead of the one it was handed would read the same on 9 September
/// and be wrong.
pub(crate) const ISSUED: (i32, u32, u32, u32, u32) = (2026, 9, 9, 14, 5);
pub(crate) const AVOIR_ISSUED: (i32, u32, u32, u32, u32) = (2026, 9, 12, 9, 20);
pub(crate) const CANCELLED_AT: (i32, u32, u32, u32, u32) = (2026, 9, 12, 10, 30);

/// Typed by the shop into a free-text field and printed on the paper, so it
/// carries the markup a product name carries and for the same reason: a
/// reason that could close a tag would break the page it explains.
pub(crate) const CANCEL_REASON: &str = "Erreur de saisie <quantité> & prix";

/// The same basket, invoiced three ways. The régime decides the TVA half of
/// the paper, the payment mode decides the money half and the party kind
/// decides the buyer block; the IFU case is the credit one with the régime
/// flipped, so its golden diff is the tax and nothing else.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Case {
    /// Réel, on credit, to a company: the balance triple, no droit de
    /// timbre (it is cash only, Code du timbre 2026 art. 100-I).
    Credit,
    /// Réel, cash, to a consumer: the stamp, the buyer's name and address
    /// only (décret 05-468 art. 3-2, last alinéa), no balance block.
    Cash,
    /// IFU, on credit, to a company: no rate column, no recap, no "HT" and
    /// no "TTC" anywhere on the page.
    Ifu,
    /// A partial avoir written against the credit facture: two lines of the
    /// three, its own later date, no droit de timbre, and a triple that
    /// leaves the customer holding a credit.
    Avoir,
    /// The same basket quoted rather than sold: no balance block, a wording
    /// saying the page settles nothing, and the words kept.
    Proforma,
    /// The credit facture reprinted after it was cancelled: the same money
    /// to the centime, under the cancelled heading and behind the mark.
    Cancelled,
}

impl Case {
    pub(crate) const ALL: [Case; 6] = [
        Case::Credit,
        Case::Cash,
        Case::Ifu,
        Case::Avoir,
        Case::Proforma,
        Case::Cancelled,
    ];

    pub(crate) const fn regime(self) -> Regime {
        match self {
            Case::Credit | Case::Cash | Case::Avoir | Case::Proforma | Case::Cancelled => {
                Regime::Reel
            }
            Case::Ifu => Regime::Ifu,
        }
    }

    pub(crate) const fn payment_mode(self) -> PaymentMode {
        match self {
            Case::Credit | Case::Ifu | Case::Avoir | Case::Proforma | Case::Cancelled => {
                PaymentMode::Credit
            }
            Case::Cash => PaymentMode::Cash,
        }
    }

    pub(crate) const fn party_kind(self) -> PartyKind {
        match self {
            Case::Credit | Case::Ifu | Case::Avoir | Case::Proforma | Case::Cancelled => {
                PartyKind::Company
            }
            Case::Cash => PartyKind::Consumer,
        }
    }

    pub(crate) const fn kind(self) -> DocumentKind {
        match self {
            Case::Credit | Case::Cash | Case::Ifu | Case::Cancelled => DocumentKind::Facture,
            Case::Avoir => DocumentKind::Avoir,
            Case::Proforma => DocumentKind::Proforma,
        }
    }

    /// A cancelled facture keeps the number it burned (features.md,
    /// Numbering row); every other face is a document in its own state.
    pub(crate) const fn status(self) -> DocumentStatus {
        match self {
            Case::Cancelled => DocumentStatus::Cancelled,
            _ => DocumentStatus::Issued,
        }
    }

    /// The number in the kind's own series. An avoir and a proforma each
    /// burn their own, so no two of the three faces print the same figure
    /// and a golden cannot pass by carrying the facture's.
    pub(crate) const fn number(self) -> i64 {
        match self {
            Case::Avoir => 3,
            Case::Proforma => 5,
            _ => 42,
        }
    }

    /// The lines the document carries. The avoir takes back a part of the
    /// basket; every other face sells all of it.
    pub(crate) const fn rows(self) -> &'static [(&'static str, i64, i64, i64, u32)] {
        match self {
            Case::Avoir => &AVOIR_LINES,
            _ => &LINES,
        }
    }

    /// A global discount is a thing the seller granted on the day. An avoir
    /// hands back the lines it names and grants nothing, so it carries none
    /// and prints neither the discount row nor the subtotal.
    pub(crate) const fn global_discount(self) -> i64 {
        match self {
            Case::Avoir => 0,
            _ => GLOBAL_DISCOUNT,
        }
    }

    pub(crate) const fn issued_at(self) -> (i32, u32, u32, u32, u32) {
        match self {
            Case::Avoir => AVOIR_ISSUED,
            _ => ISSUED,
        }
    }

    /// What the golden's name carries after the language.
    pub(crate) const fn suffix(self) -> &'static str {
        match self {
            Case::Credit => "",
            Case::Cash => "-cash",
            Case::Ifu => "-ifu",
            Case::Avoir => "-avoir",
            Case::Proforma => "-proforma",
            Case::Cancelled => "-annulee",
        }
    }
}

pub(crate) fn goldens_dir() -> PathBuf {
    super::goldens_dir("facture_a4")
}

pub(crate) fn compact_goldens_dir() -> PathBuf {
    super::goldens_dir("facture_compact_a4")
}

pub(crate) fn half_sheet_goldens_dir() -> PathBuf {
    super::goldens_dir("facture_half_sheet")
}

/// The roll down the other wire: the ESC/POS dump a thermal head is sent,
/// and the pictures of the Arabic one. Beside the HTML roll's goldens and
/// not inside them, because a `.txt` dump and an `.html` page are read by
/// different eyes.
pub(crate) fn escpos_roll_goldens_dir() -> PathBuf {
    super::goldens_dir("facture_roll_80mm_escpos")
}

pub(crate) fn roll_goldens_dir() -> PathBuf {
    super::goldens_dir("facture_roll_80mm")
}

/// The seller as a facture prints them: the four identifiers décret 05-468
/// art. 3 asks of the issuer, unlike the ticket fixture which carries two.
pub(crate) fn seller() -> SellerBlock {
    SellerBlock {
        name: "Mon magasin".to_owned(),
        rc: Some("16/00-1234567 B 25".to_owned()),
        nif: Some("000216001234567".to_owned()),
        nis: Some("000216001234567 00".to_owned()),
        ai: Some("16001234567".to_owned()),
        address: Some("12 rue Didouche Mourad, Alger".to_owned()),
        phone: Some("0555 12 34 56".to_owned()),
    }
}

pub(crate) fn company_buyer() -> PartyBlock {
    PartyBlock {
        name: "Sarl Amine Distribution".to_owned(),
        party_kind: PartyKind::Company,
        rc: Some("16/00-7654321 B 20".to_owned()),
        nif: Some("000216007654321".to_owned()),
        nis: Some("000216007654321 00".to_owned()),
        ai: Some("16007654321".to_owned()),
        address: Some("05 boulevard Krim Belkacem, Alger".to_owned()),
    }
}

pub(crate) fn consumer_buyer() -> PartyBlock {
    PartyBlock {
        name: "Yacine Meziane".to_owned(),
        party_kind: PartyKind::Consumer,
        rc: None,
        nif: None,
        nis: None,
        ai: None,
        address: Some("14 rue des Frères Bouadou, Bir Mourad Raïs".to_owned()),
    }
}

/// The one document every golden renders. Its totals come from
/// `compute_totals`, not from hand-typed numbers: a golden pins the
/// template, and the money module is what pins the money.
pub(crate) fn fixed_facture(case: Case) -> Document {
    let regime = case.regime();
    let rows = case.rows();
    let money_lines: Vec<Line> = rows
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
            global_discount: Money::centimes(case.global_discount()),
            payment_mode: case.payment_mode(),
            // On for the shop in every case: the stamp being absent from
            // the credit facture has to be the rule doing it, not a
            // setting.
            stamp_enabled: true,
            regime,
        },
    )
    .unwrap();

    let lines = rows
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

    // 9 September 2026, already on the shop's calendar (services::clock
    // writes it there, UTC+1 all year), so printing it needs no conversion.
    // A facture prints the day and not the minute.
    let issued_at = at(case.issued_at());

    // A credit facture carries the ledger as it stood when it was issued:
    // what was owed before, what this document adds, what is left. A cash
    // one adds nothing to any ledger and carries no block at all.
    let credit = case.payment_mode() == PaymentMode::Credit;
    let balance = match case {
        // An avoir moves the debt the other way, so what it adds is the
        // negative of what it hands back, and a customer given back more
        // than they owed ends the day holding a credit.
        Case::Avoir => {
            let old_balance = Money::centimes(OLD_BALANCE_BEFORE_THE_AVOIR);
            let this = Money::ZERO.checked_sub(totals.net_to_pay).unwrap();
            Some(BalanceTriple {
                old_balance,
                remaining_debt: this,
                total_debt: old_balance.checked_add(this).unwrap(),
            })
        }
        // A proforma creates no debt at all, and the triple it stores
        // says so in three zeroes. The page drops the block rather than
        // print a debt of nothing three times.
        Case::Proforma => Some(BalanceTriple {
            old_balance: Money::ZERO,
            remaining_debt: Money::ZERO,
            total_debt: Money::ZERO,
        }),
        _ => {
            let old_balance = Money::centimes(OLD_BALANCE);
            credit.then(|| BalanceTriple {
                old_balance,
                remaining_debt: totals.net_to_pay,
                total_debt: old_balance.checked_add(totals.net_to_pay).unwrap(),
            })
        }
    };

    Document {
        // The avoir is a second row in the table and the facture it names is
        // the first: an avoir carrying the id it references would let the
        // reference check pass on a document that referenced itself.
        id: if matches!(case, Case::Avoir) { 2 } else { 1 },
        shop_id: SHOP,
        kind: case.kind(),
        series: case.kind().series_of_year(issued_at.year()),
        series_year: issued_at.year(),
        number: case.number(),
        issued_at,
        user_id: OWNER,
        regime,
        payment_mode: case.payment_mode(),
        seller: seller(),
        customer_id: credit.then_some(CUSTOMER),
        buyer: Some(match case.party_kind() {
            PartyKind::Company => company_buyer(),
            PartyKind::Consumer => consumer_buyer(),
        }),
        // The avoir names the facture it is written against; nothing else
        // this template prints names another document.
        ref_document_id: matches!(case, Case::Avoir).then_some(1),
        balance,
        totals,
        // A facture states what is due and how it is settled; the note
        // handed over and the coins given back are the till's business and
        // the ticket's.
        tendered: None,
        change: None,
        status: case.status(),
        // The annulée face reads its date and its reason from the render
        // input beside the document, so the fixture leaves the stored block
        // empty and the cases that need one hand it over there.
        cancellation: None,
        lines,
        created_at: issued_at,
    }
}

/// A date on the shop's calendar, from the tuple the case names it by.
pub(crate) fn at(when: (i32, u32, u32, u32, u32)) -> chrono::NaiveDateTime {
    let (year, month, day, hour, minute) = when;
    NaiveDate::from_ymd_opt(year, month, day)
        .unwrap()
        .and_hms_opt(hour, minute, 0)
        .unwrap()
}

/// The document a case renders and everything the page needs beside it: the
/// facture an avoir names, and the day and the reason a cancelled facture
/// was cancelled. Neither is on the document's own row (the reference is an
/// id there and a number on paper; the cancellation columns belong to the
/// document's own cancellation), so the fixture hands them over the way a
/// caller will.
pub(crate) struct Fixture {
    pub(crate) doc: Document,
    referenced: Option<Document>,
    cancellation: Option<(chrono::NaiveDateTime, String)>,
}

impl Fixture {
    pub(crate) fn of(case: Case) -> Fixture {
        Fixture {
            doc: fixed_facture(case),
            referenced: matches!(case, Case::Avoir).then(|| fixed_facture(Case::Credit)),
            cancellation: matches!(case, Case::Cancelled)
                .then(|| (at(CANCELLED_AT), CANCEL_REASON.to_owned())),
        }
    }

    pub(crate) fn input(&self) -> FactureInput<'_> {
        FactureInput {
            referenced: self.referenced.as_ref(),
            cancellation: self
                .cancellation
                .as_ref()
                .map(|(at, reason)| Cancellation { at: *at, reason }),
        }
    }

    pub(crate) fn render(&self, lang: Lang, paper: Paper) -> String {
        self.render_in(lang, paper, FactureLayout::Standard)
    }

    pub(crate) fn render_in(&self, lang: Lang, paper: Paper, layout: FactureLayout) -> String {
        render_facture_with(&self.doc, &self.input(), lang, Page { paper, layout }).unwrap()
    }

    /// The balance the page is expected to carry. A proforma stores a triple
    /// of zeroes and prints no block, so what the document holds and what
    /// the paper says are two different answers here and only here.
    pub(crate) fn printed_balance(&self) -> Option<BalanceTriple> {
        match self.doc.kind {
            DocumentKind::Proforma => None,
            _ => self.doc.balance,
        }
    }
}

pub(crate) fn golden_name(lang: Lang, case: Case) -> String {
    format!("{}{}.html", lang.tag(), case.suffix())
}

/// The golden as it stands on disk, or a rewritten one under
/// `UPDATE_GOLDENS=1`. Rewriting is not a pass: `updated` says so and the
/// test that called this fails at the end of the run.
pub(crate) fn golden(lang: Lang, case: Case, rendered: &str, updated: &mut Vec<String>) -> String {
    golden_in(goldens_dir(), lang, case, rendered, updated)
}

pub(crate) fn golden_in(
    dir: PathBuf,
    lang: Lang,
    case: Case,
    rendered: &str,
    updated: &mut Vec<String>,
) -> String {
    let name = golden_name(lang, case);
    let path = dir.join(&name);
    if std::env::var_os("UPDATE_GOLDENS").is_some() {
        std::fs::create_dir_all(&dir).unwrap();
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

pub(crate) fn refuse_a_silent_regeneration(updated: &[String]) {
    assert!(
        updated.is_empty(),
        "rewrote {updated:?}: read the diff, then run the test again without UPDATE_GOLDENS"
    );
}

/// The text of every `<span class="amount amount-{marker}">` in the file.
/// Reads the golden, never the renderer: that is what makes the check worth
/// running against a file a regeneration just wrote.
pub(crate) fn amounts(html: &str, marker: &str) -> Vec<String> {
    let opening = format!("<span class=\"amount amount-{marker}\">");
    html.split(&opening)
        .skip(1)
        .map(|rest| {
            let end = rest.find("</span>").expect("an amount span never closes");
            rest[..end].to_owned()
        })
        .collect()
}

pub(crate) fn one_amount(html: &str, marker: &str) -> String {
    let found = amounts(html, marker);
    assert_eq!(found.len(), 1, "expected one {marker} row, got {found:?}");
    found.into_iter().next().unwrap_or_default()
}

/// A row a facture carries only sometimes: the stamp on a cash sale, the
/// TTC total under the réel, the three balance amounts. Absent is an
/// answer, so it comes back as `None` rather than as a zero nobody printed.
pub(crate) fn optional_amount(html: &str, marker: &str) -> Option<i64> {
    let found = amounts(html, marker);
    assert!(found.len() <= 1, "more than one {marker} row: {found:?}");
    found.first().map(|printed| centimes(printed))
}

/// "1 234,56" back to 123456 centimes. The golden's own digits, read by a
/// parser that shares no code with the formatter that wrote them.
pub(crate) fn centimes(printed: &str) -> i64 {
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

/// The text of every `<span class="qty qty-line">` in the file: the
/// quantity cells. A quantity is a figure like any other on this page, and
/// the one an avoir hands back is not the one the facture sold, so it is
/// read back off the golden too.
pub(crate) fn qtys(html: &str) -> Vec<String> {
    let opening = "<span class=\"qty qty-line\">";
    html.split(opening)
        .skip(1)
        .map(|rest| {
            let end = rest.find("</span>").expect("a quantity span never closes");
            rest[..end].to_owned()
        })
        .collect()
}

/// "1,5" back to 1500 thousandths, and "2" to 2000. The golden's own
/// digits, read by a parser that shares no code with the formatter.
pub(crate) fn milli(printed: &str) -> i64 {
    let digits: String = printed
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ',' || *c == '-')
        .collect();
    let (whole, rest) = digits.split_once(',').unwrap_or((digits.as_str(), ""));
    let sign = if whole.starts_with('-') { -1 } else { 1 };
    let whole: i64 = whole.trim_start_matches('-').parse().unwrap();
    assert!(rest.len() <= 3, "{printed} carries more than thousandths");
    let rest: i64 = format!("{rest:0<3}").parse().unwrap();
    sign * (whole * 1_000 + rest)
}

/// The text of every `<span class="rate rate-{marker}">` in the file: the
/// rate cells, which carry a figure no amount parser would catch.
pub(crate) fn rates(html: &str, marker: &str) -> Vec<String> {
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
/// them, so a recap label that slid onto the wrong row is caught by the
/// rate and not only by the amount beside it.
pub(crate) fn bps(printed: &str) -> u32 {
    let digits: String = printed
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ',')
        .collect();
    let (whole, rest) = digits.split_once(',').unwrap_or((digits.as_str(), ""));
    let whole: u32 = whole.parse().unwrap();
    let rest: u32 = format!("{rest:0<2}").parse().unwrap();
    whole * 100 + rest
}

/// The reference line, as the file carries it: the whole sentence, so a test
/// can read the number and the day in it together rather than find each of
/// them somewhere on the page.
pub(crate) fn reference_line(html: &str) -> String {
    let opening = "<div class=\"reference\">";
    let rest = html
        .split_once(opening)
        .unwrap_or_else(|| panic!("the page carries no reference line"))
        .1;
    let end = rest
        .find("</div>")
        .expect("the reference line never closes");
    rest[..end].to_owned()
}

/// The diagonal mark of a cancelled reprint, as the file carries it: the
/// contents of its own `div`, so a test reads the word where the mark is
/// drawn and not anywhere on a page whose heading already says "annulée".
pub(crate) fn mark_block(html: &str) -> Option<String> {
    let opening = "<div class=\"annulee\">";
    let rest = html.split_once(opening)?.1;
    let end = rest.find("</div>").expect("the mark block never closes");
    Some(rest[..end].to_owned())
}

/// The stylesheet rule that draws the mark across the page. Read out of the
/// same file, because the mark is a rule and a `div` together: either one
/// without the other is a word sitting in the corner of a facture.
pub(crate) fn mark_rule(html: &str) -> String {
    let opening = ".annulee span {";
    let rest = html
        .split_once(opening)
        .unwrap_or_else(|| panic!("the page carries no rule for the mark"))
        .1;
    let end = rest.find('}').expect("the mark rule never closes");
    rest[..end].to_owned()
}

/// The page from the opening of the lines table to the end of the words
/// line, in one string: the lines with their quantities, prices and rates,
/// the totals table down to the net row, and the sentence writing that
/// figure out. Two renders can be compared for all of it at once, without
/// the comparison naming each amount and forgetting one.
///
/// What it does not cover is the rest of the page: the framed blocks, the
/// signatures, and the heading above the parties. A test comparing two
/// renders reads those separately (`a_cancelled_reprint_differs_from_the_
/// live_facture_in_the_cancellation_only` does it line by line, in both
/// directions).
pub(crate) fn money_block(html: &str) -> String {
    let opening = "<div class=\"lines\">";
    let rest = html
        .split_once(opening)
        .unwrap_or_else(|| panic!("the page carries no lines table"))
        .1;
    let end = rest
        .find("</p>")
        .expect("the words line never closes the money block");
    rest[..end].to_owned()
}

/// The heading, as the file carries it: the contents of the `h1`. Read as
/// the element and not as text anywhere on the page, because a title is a
/// word the page repeats elsewhere: the Arabic proforma's notice opens with
/// the very words of its heading, so a page titled "facture" would carry
/// "فاتورة أولية" all the same.
pub(crate) fn heading(html: &str) -> String {
    let rest = html
        .split_once("<h1>")
        .unwrap_or_else(|| panic!("the page carries no heading"))
        .1;
    let end = rest.find("</h1>").expect("the heading never closes");
    rest[..end].to_owned()
}

/// The words line, as the file carries it.
pub(crate) fn in_words(html: &str) -> String {
    let opening = "<strong class=\"in-words\">";
    let rest = html
        .split_once(opening)
        .unwrap_or_else(|| panic!("the page carries no words line"))
        .1;
    let end = rest.find("</strong>").expect("the words line never closes");
    rest[..end].to_owned()
}

/// Every amount in the golden, against what the document stores.
///
/// `balance` is what the page is expected to print, which is the document's
/// own triple everywhere but on a proforma: that one stores three zeroes and
/// prints no block, and passing the document's field here would let a page
/// printing a debt of nothing pass.
pub(crate) fn the_golden_says_what_the_document_stores(
    html: &str,
    doc: &Document,
    balance: Option<BalanceTriple>,
    lang: Lang,
) {
    let totals = &doc.totals;
    assert_eq!(
        centimes(&one_amount(html, "total")),
        totals.total_ht.as_centimes(),
        "the total HT row"
    );
    // The discount is stored as what it takes off and printed as what it
    // does to the column, so the paper carries the sign the document does
    // not: -20,00 under a total of 850,00. Nothing granted prints no row.
    assert_eq!(
        optional_amount(html, "discount"),
        (totals.discount != Money::ZERO).then(|| -totals.discount.as_centimes()),
        "the discount row"
    );
    // No discount, no subtotal row: it would repeat the total above it.
    assert_eq!(
        optional_amount(html, "subtotal"),
        (totals.discount != Money::ZERO).then(|| totals.subtotal_ht.as_centimes()),
        "the subtotal row"
    );
    // "TTC" names the tax, so the row is réel only, and under the IFU it
    // would repeat the subtotal anyway.
    assert_eq!(
        optional_amount(html, "total-ttc"),
        (doc.regime == Regime::Reel).then(|| totals.total_ttc.as_centimes()),
        "the total TTC row"
    );
    // Nothing due prints no row: a stamp of 0,00 on a credit facture would
    // say the tax was charged and came to nothing.
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

    // Each recap row carries the base its tax was taken on, which the
    // ticket has no room for and a facture must show (décret 05-468 art. 3).
    let tva = amounts(html, "tva");
    let bases = amounts(html, "tva-base");
    assert_eq!(
        tva.len(),
        totals.tva_by_rate.len(),
        "one TVA row per rate the document carries"
    );
    assert_eq!(bases.len(), tva.len(), "one base per TVA row");
    for ((printed, base), row) in tva.iter().zip(&bases).zip(&totals.tva_by_rate) {
        assert_eq!(centimes(printed), row.amount.as_centimes(), "a TVA row");
        assert_eq!(centimes(base), row.base.as_centimes(), "a TVA base");
    }

    // Each recap row names the rate its base and its tax belong to. Read
    // back beside the amounts, so a label that slid onto another row (19 %
    // over the 9 % base) is red even though every amount is still right.
    let recap_rates = rates(html, "tva");
    assert_eq!(
        recap_rates.len(),
        totals.tva_by_rate.len(),
        "one rate per recap row"
    );
    for (printed, row) in recap_rates.iter().zip(&totals.tva_by_rate) {
        assert_eq!(bps(printed), row.rate.as_u32(), "a recap row's rate");
    }

    // The rate on a line is the rate the line was sold at, réel only:
    // under the IFU there is no column at all, not a column of zeroes.
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

    let lines = amounts(html, "line");
    let unit_prices = amounts(html, "unit-price");
    assert_eq!(lines.len(), doc.lines.len(), "one total per sold line");
    assert_eq!(
        unit_prices.len(),
        doc.lines.len(),
        "one unit price per line"
    );
    // The quantity beside them: an avoir takes back a part of what the
    // facture sold, and a page showing the facture's quantity over the
    // avoir's amounts is a document whose own arithmetic does not hold.
    let quantities = qtys(html);
    assert_eq!(quantities.len(), doc.lines.len(), "one quantity per line");
    for (printed, line) in quantities.iter().zip(&doc.lines) {
        assert_eq!(milli(printed), line.qty_milli, "a quantity");
    }

    for ((printed, unit_price), line) in lines.iter().zip(&unit_prices).zip(&doc.lines) {
        assert_eq!(centimes(printed), line.line_total.as_centimes(), "a line");
        assert_eq!(
            centimes(unit_price),
            line.unit_price.as_centimes(),
            "a unit price"
        );
    }

    // A line discount is an amount on the paper like any other, so it is
    // read back like any other. A line that carries none prints an empty
    // cell in a column the facture only opens when a line uses it.
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

    // The balance triple, three amounts or none, exactly as the document
    // stores them: a reprint shows the debt the customer signed for.
    assert_eq!(
        optional_amount(html, "old-balance"),
        balance.map(|b| b.old_balance.as_centimes()),
        "the old balance row"
    );
    assert_eq!(
        optional_amount(html, "this-document"),
        balance.map(|b| b.remaining_debt.as_centimes()),
        "this document's row"
    );
    assert_eq!(
        optional_amount(html, "total-debt"),
        balance.map(|b| b.total_debt.as_centimes()),
        "the total debt row"
    );
    if let Some(balance) = balance {
        assert_eq!(
            balance.old_balance.checked_add(balance.remaining_debt),
            Ok(balance.total_debt),
            "the fixture's own triple does not add up"
        );
    }

    // The words are the net to pay written out, the amount this document
    // asks for, and they are compared to the generator and not to a string
    // typed into the test.
    assert_eq!(
        in_words(html),
        amount_in_words(totals.net_to_pay, lang).unwrap(),
        "the words line"
    );
}

/// One case, one layout, against its three goldens: the render is the file,
/// and the file says what the document stores.
///
/// The two suites call this with a different layout and nothing else. Written
/// twice they would drift, and the drift would be invisible: each would still
/// match its own goldens.
pub(crate) fn each_language_of_a_layout(case: Case, layout: FactureLayout) {
    let fixture = Fixture::of(case);
    let dir = match layout {
        FactureLayout::Standard => goldens_dir(),
        FactureLayout::Compact => compact_goldens_dir(),
        FactureLayout::HalfSheet => half_sheet_goldens_dir(),
        FactureLayout::Roll80 => roll_goldens_dir(),
    };
    let mut updated = Vec::new();
    for lang in Lang::ALL {
        let rendered = fixture.render_in(lang, Paper::A4, layout);
        let expected = golden_in(dir.clone(), lang, case, &rendered, &mut updated);
        assert_eq!(
            rendered,
            expected,
            "{layout:?} {} is not what the template renders",
            golden_name(lang, case)
        );
        the_golden_says_what_the_document_stores(
            &expected,
            &fixture.doc,
            fixture.printed_balance(),
            lang,
        );
    }
    refuse_a_silent_regeneration(&updated);
}

/// The CTCA 2026 art. 64 rule read off a printed page: an IFU document names
/// the tax nowhere. Not the word, not its abbreviations in any of the three
/// languages, not "HT", not "TTC", not the per-cent sign of a rate column,
/// and no rate cell or TVA row left behind.
///
/// It lives here rather than in one suite because the rule belongs to the
/// document and not to a layout. Every layout that can print an IFU facture
/// runs it, so a template that won a line by leaving a rate column in place
/// fails whichever sheet it was drawn on.
pub(crate) fn names_no_tax(html: &str, lang: Lang) {
    for forbidden in [
        "TVA",
        "VAT",
        "ت.ق.م",
        "HT",
        "TTC",
        "excl. tax",
        "incl. tax",
        "hors taxe",
        "خارج الرسم",
    ] {
        assert!(
            !html.contains(forbidden),
            "{forbidden} is on the {lang:?} IFU facture"
        );
    }
    // The per-cent sign is read off the printed page and not the stylesheet
    // above it: a width written as a percentage is a length and names no tax,
    // and a ban that fired on one would push the next reader into a worse
    // layout than into a legal fix.
    let printed = html.split_once("</style>").expect("the page has a head").1;
    assert!(
        !printed.contains('%'),
        "a rate is printed on the {lang:?} IFU facture"
    );
    assert!(
        rates(html, "line").is_empty() && rates(html, "tva").is_empty(),
        "the {lang:?} IFU facture keeps a rate cell"
    );
    for absent in ["tva", "tva-base", "total-ttc"] {
        assert!(
            amounts(html, absent).is_empty(),
            "the {lang:?} IFU facture keeps a {absent} row"
        );
    }
}

/// The `@page` rule, as the file carries it: the one line `Paper` sets.
///
/// Read as the rule and not as text anywhere on the page, because a test
/// asking whether a sheet reached the stylesheet has to look where the sheet
/// is written. A whole-file search for "size: A5" would pass on a page that
/// happened to echo the value somewhere else and had lost it here.
pub(crate) fn page_rule(html: &str) -> String {
    let opening = "@page {";
    let rest = html
        .split_once(opening)
        .unwrap_or_else(|| panic!("the page carries no @page rule"))
        .1;
    let end = rest.find('}').expect("the @page rule never closes");
    rest[..end].to_owned()
}

/// Every word the page prints, with the markup taken away.
///
/// For comparing two layouts whose markup is not the same. A layout may lay
/// the same facts out differently and still has to carry the same wording:
/// the labels, the titles, the legal mentions and the amounts as they are
/// written. What this cannot see is where a word sits, which is the part a
/// layout is allowed to change.
pub(crate) fn words_of(html: &str) -> std::collections::BTreeSet<String> {
    let body = html.split_once("</style>").map_or(html, |(_, rest)| rest);
    let mut out = std::collections::BTreeSet::new();
    let mut text = String::new();
    let mut inside_tag = false;
    for ch in body.chars() {
        match ch {
            '<' => inside_tag = true,
            // A tag is a word boundary. Two cells of a table row touch in
            // the source with nothing between them, so dropping the markup
            // without leaving a space glues a rate onto the amount beside
            // it and invents a word neither page prints.
            '>' => {
                inside_tag = false;
                text.push(' ');
            }
            _ if !inside_tag => text.push(ch),
            _ => {}
        }
    }
    for word in text.split_whitespace() {
        out.insert(word.to_owned());
    }
    out
}

/// A sheet render with the lines table's head cut out.
///
/// The captions above the columns are the one thing on the page that exists
/// only because there is a table. Décret 05-468 art. 3 asks for the
/// designation, the quantity and the unit price; it does not ask for the
/// words above them, and a roll printing `2 × 150,00` has said both figures
/// and has nowhere to put "Qté".
///
/// Cutting the block out, rather than collecting the caption words and
/// letting those through, is the difference between a claim and a hole. A
/// caption and a row label can be the same word: `rate_label` and the TVA
/// recap row are both `Key::Tva`, and `line_discount_label` and the global
/// discount row are both `Key::Discount`. An allowance by word therefore
/// exempts those two rows on every page under the réel, and a roll that
/// dropped either label would pass. Found by the test lens, 2026-09-20,
/// which named the mutation: delete `{{ tva.label }}` from the roll.
pub(crate) fn without_the_lines_table_head(html: &str) -> String {
    let (before, rest) = html
        .split_once("<thead>")
        .expect("a sheet layout heads its lines table");
    let (_, after) = rest
        .split_once("</thead>")
        .expect("the lines table head closes");
    format!("{before}{after}")
}

/// Every `amount-<name>` a render carries, counted, and of those how many are
/// followed by the currency span.
///
/// Read off the page rather than listed in a test, because a list cannot know
/// about a field it was written before. A view that gains an amount, reaches
/// the sheet and misses the roll, passes a hand-kept loop over fourteen
/// names: the loop never asks about the fifteenth.
///
/// Counted rather than collected into a set, and counted per marker rather
/// than in total, because both weaker shapes have a mutation that survives.
/// A set misses a line printed twice. A single total of currency spans misses
/// a roll that gave one amount two and another none, which is the same number
/// of spans and a page where a figure sits with no DA beside it.
pub(crate) fn amount_markers(html: &str) -> std::collections::BTreeMap<String, (usize, usize)> {
    const OPEN: &str = "<span class=\"amount amount-";
    let mut out: std::collections::BTreeMap<String, (usize, usize)> = Default::default();
    let mut rest = html;
    while let Some((_, after)) = rest.split_once(OPEN) {
        let (name, body) = after.split_once('"').expect("the class attribute closes");
        let (_, tail) = body.split_once("</span>").expect("an amount closes");
        let entry = out.entry(name.to_owned()).or_default();
        entry.0 += 1;
        if tail.starts_with("<span class=\"cur\">") {
            entry.1 += 1;
        }
        rest = tail;
    }
    out
}
