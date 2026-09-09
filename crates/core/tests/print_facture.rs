// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The A4 facture against its golden files, one per language and one per
//! case (features.md §4: "Golden-file test for every template × language
//! against fixed fixtures. A template change is a reviewed golden diff").
//!
//! Regenerate with `UPDATE_GOLDENS=1 cargo test -p dzpos-core --test
//! print_facture`. The run that rewrites them fails on purpose: a golden
//! nobody looked at must never be green in the same run that wrote it.
//!
//! Every amount in each golden is parsed back out of the file and compared
//! to the document's stored totals, its stored lines and its stored balance
//! triple, and the words line is compared to `amount_in_words` of the
//! stored net. Those checks never call the renderer, so a golden that
//! drifts from the money cannot be accepted by regenerating it.
//!
//! The little parser below is the ticket test's, copied rather than shared:
//! two golden suites that check each other's files through one helper can
//! both be made green by editing the helper once. T9 folds them together if
//! the avoir and proforma suites want the same shape.

use std::path::PathBuf;

use chrono::NaiveDate;
use dzpos_core::lang::Lang;
use dzpos_core::money::words::amount_in_words;
use dzpos_core::money::{
    compute_totals, Bps, Line, Money, PaymentMode, Regime, TotalsOptions, TvaLine,
};
use dzpos_core::print::strings::{text, Key};
use dzpos_core::print::{render_facture, render_facture_with_reference, Paper};
use dzpos_core::services::documents::{
    BalanceTriple, Document, DocumentKind, DocumentLine, DocumentStatus, PartyBlock, PartyKind,
    SellerBlock,
};

const SHOP: i32 = 1;
const OWNER: i32 = 1;
const CUSTOMER: i32 = 7;

/// The three lines of the fixed sale, the ticket fixture's amounts: 2 ×
/// 150,00 at 19 %, 1,5 kg × 320,00 at 9 %, and 1 × 80,00 at 0 % with a
/// 10,00 line discount.
///
/// The first name is not the ticket's. A product name is typed by the shop
/// and lands in the page as it was typed, so one fixture name carries an
/// ampersand and a pair of angle brackets: the goldens then pin askama's
/// escaping, and a template that ever rendered a name raw would show it as
/// a golden diff rather than as a broken facture at a customer's desk.
const LINES: [(&str, i64, i64, i64, u32); 3] = [
    ("Huile <Elio> & Co 5 L", 2_000, 15_000, 0, 1900),
    ("Farine", 1_500, 32_000, 0, 900),
    ("Pain", 1_000, 8_000, 1_000, 0),
];

/// 20,00 DA off the whole basket.
const GLOBAL_DISCOUNT: i64 = 2_000;
/// What the customer owed before this facture: 1 500,00 DA.
const OLD_BALANCE: i64 = 150_000;

/// The same basket, invoiced three ways. The régime decides the TVA half of
/// the paper, the payment mode decides the money half and the party kind
/// decides the buyer block; the IFU case is the credit one with the régime
/// flipped, so its golden diff is the tax and nothing else.
#[derive(Debug, Clone, Copy)]
enum Case {
    /// Réel, on credit, to a company: the balance triple, no droit de
    /// timbre (it is cash only, Code du timbre 2026 art. 100-I).
    Credit,
    /// Réel, cash, to a consumer: the stamp, the buyer's name and address
    /// only (décret 05-468 art. 3-2, last alinéa), no balance block.
    Cash,
    /// IFU, on credit, to a company: no rate column, no recap, no "HT" and
    /// no "TTC" anywhere on the page.
    Ifu,
}

impl Case {
    const ALL: [Case; 3] = [Case::Credit, Case::Cash, Case::Ifu];

    const fn regime(self) -> Regime {
        match self {
            Case::Credit | Case::Cash => Regime::Reel,
            Case::Ifu => Regime::Ifu,
        }
    }

    const fn payment_mode(self) -> PaymentMode {
        match self {
            Case::Credit | Case::Ifu => PaymentMode::Credit,
            Case::Cash => PaymentMode::Cash,
        }
    }

    const fn party_kind(self) -> PartyKind {
        match self {
            Case::Credit | Case::Ifu => PartyKind::Company,
            Case::Cash => PartyKind::Consumer,
        }
    }

    /// What the golden's name carries after the language.
    const fn suffix(self) -> &'static str {
        match self {
            Case::Credit => "",
            Case::Cash => "-cash",
            Case::Ifu => "-ifu",
        }
    }
}

fn goldens_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/print/facture_a4")
}

/// The seller as a facture prints them: the four identifiers décret 05-468
/// art. 3 asks of the issuer, unlike the ticket fixture which carries two.
fn seller() -> SellerBlock {
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

fn company_buyer() -> PartyBlock {
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

fn consumer_buyer() -> PartyBlock {
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
fn fixed_facture(case: Case) -> Document {
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
            // the credit facture has to be the rule doing it, not a
            // setting.
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
        })
        .collect();

    // 9 September 2026, already on the shop's calendar (services::clock
    // writes it there, UTC+1 all year), so printing it needs no conversion.
    // A facture prints the day and not the minute.
    let issued_at = NaiveDate::from_ymd_opt(2026, 9, 9)
        .unwrap()
        .and_hms_opt(14, 5, 0)
        .unwrap();

    // A credit facture carries the ledger as it stood when it was issued:
    // what was owed before, what this document adds, what is left. A cash
    // one adds nothing to any ledger and carries no block at all.
    let credit = case.payment_mode() == PaymentMode::Credit;
    let old_balance = Money::centimes(OLD_BALANCE);
    let balance = credit.then(|| BalanceTriple {
        old_balance,
        remaining_debt: totals.net_to_pay,
        total_debt: old_balance.checked_add(totals.net_to_pay).unwrap(),
    });

    Document {
        id: 1,
        shop_id: SHOP,
        kind: DocumentKind::Facture,
        series: DocumentKind::Facture.series().to_owned(),
        number: 42,
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
        ref_document_id: None,
        balance,
        totals,
        // A facture states what is due and how it is settled; the note
        // handed over and the coins given back are the till's business and
        // the ticket's.
        tendered: None,
        change: None,
        status: DocumentStatus::Issued,
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

/// The text of every `<span class="amount amount-{marker}">` in the file.
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

/// A row a facture carries only sometimes: the stamp on a cash sale, the
/// TTC total under the réel, the three balance amounts. Absent is an
/// answer, so it comes back as `None` rather than as a zero nobody printed.
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
/// rate cells, which carry a figure no amount parser would catch.
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
/// them, so a recap label that slid onto the wrong row is caught by the
/// rate and not only by the amount beside it.
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

/// The words line, as the file carries it.
fn in_words(html: &str) -> String {
    let opening = "<strong class=\"in-words\">";
    let rest = html
        .split_once(opening)
        .unwrap_or_else(|| panic!("the page carries no words line"))
        .1;
    let end = rest.find("</strong>").expect("the words line never closes");
    rest[..end].to_owned()
}

/// Every amount in the golden, against what the document stores.
fn the_golden_says_what_the_document_stores(html: &str, doc: &Document, lang: Lang) {
    let totals = &doc.totals;
    assert_eq!(
        centimes(&one_amount(html, "total")),
        totals.total_ht.as_centimes(),
        "the total HT row"
    );
    // The discount is stored as what it takes off and printed as what it
    // does to the column, so the paper carries the sign the document does
    // not: -20,00 under a total of 850,00.
    assert_eq!(
        centimes(&one_amount(html, "discount")),
        -totals.discount.as_centimes(),
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
        doc.balance.map(|b| b.old_balance.as_centimes()),
        "the old balance row"
    );
    assert_eq!(
        optional_amount(html, "this-document"),
        doc.balance.map(|b| b.remaining_debt.as_centimes()),
        "this document's row"
    );
    assert_eq!(
        optional_amount(html, "total-debt"),
        doc.balance.map(|b| b.total_debt.as_centimes()),
        "the total debt row"
    );
    if let Some(balance) = doc.balance {
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

/// One case against its three goldens: the render is the file, and the file
/// says what the document stores.
fn each_language_of(case: Case) {
    let doc = fixed_facture(case);
    let mut updated = Vec::new();
    for lang in Lang::ALL {
        let rendered = render_facture(&doc, lang, Paper::A4).unwrap();
        let expected = golden(lang, case, &rendered, &mut updated);
        assert_eq!(
            rendered,
            expected,
            "{} is not what the template renders",
            golden_name(lang, case)
        );
        the_golden_says_what_the_document_stores(&expected, &doc, lang);
    }
    refuse_a_silent_regeneration(&updated);
}

#[test]
fn the_credit_facture_is_its_golden_in_every_language() {
    each_language_of(Case::Credit);
}

#[test]
fn the_cash_facture_is_its_golden_in_every_language() {
    each_language_of(Case::Cash);
}

#[test]
fn the_ifu_facture_is_its_golden_in_every_language() {
    each_language_of(Case::Ifu);
}

/// A5 is the same facture on a smaller sheet. One line of the page changes,
/// the one the OS print dialog reads, and the words, the amounts and the
/// blocks are the same bytes: two layouts kept in step by hand would drift
/// the first time one of them was edited.
#[test]
fn the_a5_facture_differs_from_the_a4_in_the_page_size_line_only() {
    for case in Case::ALL {
        let doc = fixed_facture(case);
        for lang in Lang::ALL {
            let a4 = render_facture(&doc, lang, Paper::A4).unwrap();
            let a5 = render_facture(&doc, lang, Paper::A5).unwrap();
            let a4_lines: Vec<&str> = a4.lines().collect();
            let a5_lines: Vec<&str> = a5.lines().collect();
            assert_eq!(a4_lines.len(), a5_lines.len(), "{lang:?} {case:?}");
            let differing: Vec<(&str, &str)> = a4_lines
                .iter()
                .zip(&a5_lines)
                .filter(|(a, b)| a != b)
                .map(|(a, b)| (*a, *b))
                .collect();
            assert_eq!(differing.len(), 1, "{lang:?} {case:?}: {differing:?}");
            let (a4_line, a5_line) = differing[0];
            assert!(a4_line.contains("@page"), "{a4_line}");
            assert!(a4_line.contains("size: A4"), "{a4_line}");
            assert!(a5_line.contains("size: A5"), "{a5_line}");
        }
    }
}

/// `regime_ifu_prints_no_tva`, read off the paper: an IFU document must not
/// mention the tax at all (CTCA 2026 art. 64), so neither the word, nor its
/// abbreviations, nor "HT", nor "TTC", nor the per-cent sign of a rate
/// column is anywhere in the file. The réel facture is checked in the same
/// test, so a template that dropped them for everyone could not pass this.
#[test]
fn an_ifu_facture_names_no_tax_in_any_language() {
    let ifu = fixed_facture(Case::Ifu);
    let reel = fixed_facture(Case::Credit);
    for lang in Lang::ALL {
        let html = render_facture(&ifu, lang, Paper::A4).unwrap();
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
        // The per-cent sign is read off the printed page and not the
        // stylesheet above it: a width written as a percentage is a length
        // and names no tax, and a ban that fired on one would push the next
        // reader into a worse layout than into a legal fix.
        let printed = html.split_once("</style>").expect("the page has a head").1;
        assert!(
            !printed.contains('%'),
            "a rate is printed on the {lang:?} IFU facture"
        );
        assert!(
            rates(&html, "line").is_empty() && rates(&html, "tva").is_empty(),
            "the {lang:?} IFU facture keeps a rate cell"
        );
        for absent in ["tva", "tva-base", "total-ttc"] {
            assert!(
                amounts(&html, absent).is_empty(),
                "the {lang:?} IFU facture keeps a {absent} row"
            );
        }
        assert!(html.contains(text(Key::Total, lang)), "{lang:?}");
        assert!(html.contains(text(Key::UnitPrice, lang)), "{lang:?}");

        // The same basket under the réel says all of it.
        let reel_html = render_facture(&reel, lang, Paper::A4).unwrap();
        assert_eq!(amounts(&reel_html, "tva").len(), 3, "0 %, 9 % and 19 %");
        assert_eq!(amounts(&reel_html, "tva-base").len(), 3, "{lang:?}");
        assert_eq!(amounts(&reel_html, "total-ttc").len(), 1, "{lang:?}");
        assert_eq!(rates(&reel_html, "tva").len(), 3, "{lang:?}");
        assert_eq!(rates(&reel_html, "line").len(), 3, "{lang:?}");
        assert!(reel_html.contains(text(Key::TotalHt, lang)), "{lang:?}");
        assert!(reel_html.contains(text(Key::UnitPriceHt, lang)), "{lang:?}");
        assert!(reel_html.contains(text(Key::Tva, lang)), "{lang:?}");
    }
    assert!(
        ifu.totals.tva_by_rate.is_empty(),
        "an IFU document stores no TVA recap"
    );
}

/// A consumer's facture carries « ses nom, prénom(s) et adresse » and
/// nothing more (décret 05-468 art. 3-2, last alinéa); a company's carries
/// the four identifiers. The kind of party is the field on the fiche, not a
/// guess from which boxes are filled in, so a consumer row that still holds
/// an RC from an earlier life prints no RC.
#[test]
fn the_buyer_block_follows_the_party_kind_and_not_the_fields_it_holds() {
    let mut doc = fixed_facture(Case::Cash);
    let identifiers = ["16/00-7654321 B 20", "000216007654321"];

    let company = render_facture(&fixed_facture(Case::Credit), Lang::Fr, Paper::A4).unwrap();
    for identifier in identifiers {
        assert!(company.contains(identifier), "{identifier}");
    }

    let consumer = render_facture(&doc, Lang::Fr, Paper::A4).unwrap();
    assert!(consumer.contains("Yacine Meziane"));
    assert!(consumer.contains("14 rue des Frères Bouadou, Bir Mourad Raïs"));
    for identifier in identifiers {
        assert!(!consumer.contains(identifier), "{identifier}");
    }

    doc.buyer = Some(PartyBlock {
        rc: Some("16/00-7654321 B 20".to_owned()),
        nif: Some("000216007654321".to_owned()),
        ..consumer_buyer()
    });
    let still_a_consumer = render_facture(&doc, Lang::Fr, Paper::A4).unwrap();
    for identifier in identifiers {
        assert!(
            !still_a_consumer.contains(identifier),
            "a consumer's facture printed {identifier}"
        );
    }
    // The seller's own identifiers are on every facture, whoever buys.
    for identifier in ["16/00-1234567 B 25", "000216001234567 00", "16001234567"] {
        assert!(still_a_consumer.contains(identifier), "{identifier}");
    }
}

/// A facture with no global discount prints neither the discount row nor
/// the subtotal, which would repeat the total above it. The lines keep
/// their own discounts, and the column that carries them opens only when a
/// line uses it.
#[test]
fn a_facture_without_a_discount_prints_neither_the_discount_nor_the_subtotal() {
    let mut doc = fixed_facture(Case::Credit);
    doc.totals = compute_totals(
        &LINES
            .iter()
            .map(|(_, qty_milli, unit, line_discount, rate)| Line {
                qty_milli: *qty_milli,
                unit_price: Money::centimes(*unit),
                line_discount: Money::centimes(*line_discount),
                rate: Bps::new(*rate).unwrap(),
            })
            .collect::<Vec<_>>(),
        &TotalsOptions {
            global_discount: Money::ZERO,
            payment_mode: PaymentMode::Credit,
            stamp_enabled: true,
            regime: Regime::Reel,
        },
    )
    .unwrap();
    doc.balance = None;

    let html = render_facture(&doc, Lang::Fr, Paper::A4).unwrap();
    assert!(amounts(&html, "discount").is_empty(), "a discount row");
    assert!(amounts(&html, "subtotal").is_empty(), "a subtotal row");
    assert_eq!(amounts(&html, "line-discount").len(), 1, "the Pain line");
    assert_eq!(
        centimes(&one_amount(&html, "total")),
        doc.totals.total_ht.as_centimes()
    );

    // With no discounted line at all the column itself is gone.
    for line in &mut doc.lines {
        line.line_discount = Money::ZERO;
    }
    let html = render_facture(&doc, Lang::Fr, Paper::A4).unwrap();
    assert!(amounts(&html, "line-discount").is_empty());
    assert_eq!(
        html.matches(text(Key::Discount, Lang::Fr)).count(),
        0,
        "the discount column header is still there"
    );
}

/// The three amounts are printed as stored, sign included: a customer who
/// overpaid is owed money and the facture says so rather than hiding a
/// negative behind a zero.
#[test]
fn a_negative_old_balance_is_printed_as_a_negative() {
    let mut doc = fixed_facture(Case::Credit);
    let old_balance = Money::centimes(-25_000);
    doc.balance = Some(BalanceTriple {
        old_balance,
        remaining_debt: doc.totals.net_to_pay,
        total_debt: old_balance.checked_add(doc.totals.net_to_pay).unwrap(),
    });
    let html = render_facture(&doc, Lang::Fr, Paper::A4).unwrap();
    assert_eq!(optional_amount(&html, "old-balance"), Some(-25_000));
    the_golden_says_what_the_document_stores(&html, &doc, Lang::Fr);
}

/// Only three kinds have a title on this paper. A ticket has its own 80 mm
/// template and the parked kinds have none at all, so a document that is
/// neither is refused rather than printed under a heading that lies about
/// what it is.
#[test]
fn a_kind_this_template_has_no_title_for_is_refused() {
    let mut doc = fixed_facture(Case::Credit);
    for kind in [
        DocumentKind::Ticket,
        DocumentKind::BonDeLivraison,
        DocumentKind::BonDeReception,
        DocumentKind::Quittance,
    ] {
        doc.kind = kind;
        let err = render_facture(&doc, Lang::Fr, Paper::A4).unwrap_err();
        assert_eq!(err.code(), "print", "{kind:?}: {err:?}");
    }
    for kind in [
        DocumentKind::Facture,
        DocumentKind::Avoir,
        DocumentKind::Proforma,
    ] {
        doc.kind = kind;
        let html = render_facture(&doc, Lang::Fr, Paper::A4).unwrap();
        let title = match kind {
            DocumentKind::Avoir => Key::Avoir,
            DocumentKind::Proforma => Key::Proforma,
            _ => Key::Facture,
        };
        assert!(html.contains(text(title, Lang::Fr)), "{kind:?}");
        assert!(
            html.contains(&format!("{}-000042", kind.number_prefix())),
            "{kind:?} lost its printed number"
        );
    }
}

/// A cancelled facture keeps its number and says on its face that it was
/// cancelled (features.md, Numbering row), so a reprint of it can never
/// pass for the live document.
#[test]
fn a_cancelled_facture_is_printed_under_the_cancelled_heading() {
    let mut doc = fixed_facture(Case::Credit);
    doc.status = DocumentStatus::Cancelled;
    for lang in Lang::ALL {
        let html = render_facture(&doc, lang, Paper::A4).unwrap();
        assert!(html.contains(text(Key::FactureCancelled, lang)), "{lang:?}");
        assert!(html.contains("FA-000042"), "{lang:?}");
    }
    // Nothing cancels an avoir or a proforma in M2 and the wording for it
    // is not written, so the printer refuses rather than invent one.
    for kind in [DocumentKind::Avoir, DocumentKind::Proforma] {
        doc.kind = kind;
        let err = render_facture(&doc, Lang::Fr, Paper::A4).unwrap_err();
        assert_eq!(err.code(), "print", "{kind:?}: {err:?}");
    }
}

/// The buyer is a legal field of a facture (décret 05-468 art. 3). A
/// document that has no buyer block cannot be printed as one, in any of the
/// three kinds this template titles.
#[test]
fn a_document_without_a_buyer_block_has_no_printable_facture() {
    let mut doc = fixed_facture(Case::Credit);
    doc.buyer = None;
    for kind in [
        DocumentKind::Facture,
        DocumentKind::Avoir,
        DocumentKind::Proforma,
    ] {
        doc.kind = kind;
        for lang in Lang::ALL {
            let err = render_facture(&doc, lang, Paper::A4).unwrap_err();
            assert_eq!(err.code(), "print", "{kind:?} {lang:?}: {err:?}");
        }
    }
}

/// The same refusal the ticket makes: a stored IFU document carrying a TVA
/// recap contradicts the régime it was issued under, and there is no honest
/// facture for it. Printing the recap names a tax the document must not
/// name (CTCA 2026 art. 64); dropping it quietly hands the buyer a total
/// whose parts do not add up.
#[test]
fn an_ifu_document_carrying_a_tva_recap_is_refused_not_quietly_stripped() {
    let mut doc = fixed_facture(Case::Ifu);
    doc.totals.tva_by_rate.push(TvaLine {
        rate: Bps::new(1900).unwrap(),
        base: Money::centimes(29_295),
        amount: Money::centimes(5_566),
    });
    for lang in Lang::ALL {
        let err = render_facture(&doc, lang, Paper::A4).unwrap_err();
        assert_eq!(err.code(), "print", "{lang:?}: {err:?}");
    }
}

/// An avoir names the facture it is written against, and it names it by its
/// printed number. The row carries an internal id, so the caller hands the
/// referenced document over; an avoir whose reference was not read with it,
/// or read from another shop, is refused rather than printed with the id
/// where the number belongs.
#[test]
fn an_avoir_prints_the_number_of_the_facture_it_references() {
    let facture = fixed_facture(Case::Credit);
    let mut avoir = fixed_facture(Case::Credit);
    avoir.id = 2;
    avoir.kind = DocumentKind::Avoir;
    avoir.number = 3;
    avoir.ref_document_id = Some(facture.id);

    for lang in Lang::ALL {
        let html = render_facture_with_reference(&avoir, Some(&facture), lang, Paper::A4).unwrap();
        assert!(html.contains("AV-000003"), "{lang:?}");
        assert!(html.contains("FA-000042"), "{lang:?}");
        assert!(
            html.contains(text(Key::ReferencedDocument, lang)),
            "{lang:?}"
        );
    }

    // No reference handed over, a reference that is another document, and a
    // reference from another shop: each is a facture number this avoir
    // cannot print, and each is refused.
    assert_eq!(
        render_facture(&avoir, Lang::Fr, Paper::A4)
            .unwrap_err()
            .code(),
        "print"
    );
    let mut other = fixed_facture(Case::Credit);
    other.id = 9;
    assert_eq!(
        render_facture_with_reference(&avoir, Some(&other), Lang::Fr, Paper::A4)
            .unwrap_err()
            .code(),
        "print"
    );
    let mut another_shop = fixed_facture(Case::Credit);
    another_shop.shop_id = SHOP + 1;
    assert_eq!(
        render_facture_with_reference(&avoir, Some(&another_shop), Lang::Fr, Paper::A4)
            .unwrap_err()
            .code(),
        "print"
    );

    // A facture that references nothing prints with no reference row.
    let plain = render_facture(&facture, Lang::Fr, Paper::A4).unwrap();
    assert!(!plain.contains(text(Key::ReferencedDocument, Lang::Fr)));
}

/// A name the shop typed is printed and never run: the ampersand and the
/// angle brackets of a product name come out as entities, so a name can
/// neither close a tag nor open one. The golden carries the escaped form,
/// and this test says which form that is.
#[test]
fn a_product_name_with_markup_in_it_is_escaped_and_not_rendered() {
    for case in Case::ALL {
        let doc = fixed_facture(case);
        for lang in Lang::ALL {
            let html = render_facture(&doc, lang, Paper::A4).unwrap();
            assert!(
                html.contains("Huile &#60;Elio&#62; &#38; Co 5 L"),
                "{lang:?} {case:?} does not carry the escaped name"
            );
            assert!(
                !html.contains("<Elio>"),
                "{lang:?} {case:?} rendered a product name as markup"
            );
        }
    }
}

#[test]
fn the_arabic_facture_reads_right_to_left_and_says_it_is_unreviewed() {
    let doc = fixed_facture(Case::Credit);
    let ar = render_facture(&doc, Lang::Ar, Paper::A4).unwrap();
    assert!(ar.contains("dir=\"rtl\""), "the Arabic facture is not RTL");
    assert!(ar.contains("lang=\"ar\""));
    assert!(
        ar.contains("unreviewed by a native speaker, like words_ar"),
        "the Arabic facture does not disclose that its wording is unreviewed"
    );
    for other in [Lang::Fr, Lang::En] {
        let html = render_facture(&doc, other, Paper::A4).unwrap();
        assert!(html.contains("dir=\"ltr\""), "{other:?}");
        assert!(
            !html.contains("unreviewed by a native speaker"),
            "{other:?}"
        );
    }
}

/// Western digits in every language, the shop's number format: an Arabic
/// facture a comptable reads carries the same figures as the French one,
/// and the words line is the only part of the page that changes alphabet.
#[test]
fn the_digits_are_western_in_every_language() {
    for case in Case::ALL {
        let doc = fixed_facture(case);
        for lang in Lang::ALL {
            let html = render_facture(&doc, lang, Paper::A4).unwrap();
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

/// The words line prints the net to pay and not the total TTC: the buyer
/// owes the stamp too, and on the cash facture the two figures differ by
/// exactly it. The credit facture, where they are equal, could not prove
/// this on its own.
#[test]
fn the_words_are_the_net_to_pay_and_not_the_total_ttc() {
    let cash = fixed_facture(Case::Cash);
    assert_ne!(
        cash.totals.stamp,
        Money::ZERO,
        "the cash fixture carries no stamp and proves nothing"
    );
    assert_eq!(
        cash.totals
            .total_ttc
            .checked_add(cash.totals.stamp)
            .unwrap(),
        cash.totals.net_to_pay
    );
    for lang in Lang::ALL {
        let html = render_facture(&cash, lang, Paper::A4).unwrap();
        assert_eq!(
            in_words(&html),
            amount_in_words(cash.totals.net_to_pay, lang).unwrap(),
            "{lang:?}"
        );
        assert_ne!(
            in_words(&html),
            amount_in_words(cash.totals.total_ttc, lang).unwrap(),
            "{lang:?}"
        );
    }
}
