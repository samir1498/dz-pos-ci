// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The 80 mm facture down the ESC/POS wire, against its own goldens and
//! against the standard page it has to agree with.
//!
//! Regenerate with `UPDATE_GOLDENS=1 cargo test -p dzpos-core --test
//! print_facture_roll_escpos`. The run that rewrites them fails on purpose.
//!
//! Three kinds of check, and the order matters:
//!
//! 1. **The amounts.** `the_escpos_roll_prints_the_amounts_the_document_stores`
//!    reads every figure back out of the line model with a parser that
//!    shares no code with the formatter and compares the multiset against
//!    what the document holds in centimes. It runs on the line model and
//!    not on the picture, which is the last moment a wrong figure is still
//!    a string somebody can read. This is the rule the HTML goldens obey,
//!    applied a layer earlier because there are no `amount-*` spans on a
//!    roll of dots to read it out of.
//! 2. **The fields.** `the_escpos_roll_says_everything_the_standard_page_says`
//!    walks every case, including the three the reduced golden set never
//!    reaches, and asserts each field of the view reaches the paper. Décret
//!    05-468 art. 3 has no thermal version: a roll that won its width by
//!    dropping the buyer's NIF would be a ticket with a facture's title.
//! 3. **The dots.** Nothing is clipped, no character falls back to the box
//!    glyph, and the Arabic pictures are goldens a reviewer opens.
//!
//! Most of the amounts are not written out by hand here, for a reason worth
//! saying: what this suite tests is the printing, and the arithmetic that
//! turned three lines into a TVA recap is pinned by name in `fixtures/money/`
//! and by `money_fixtures.rs`. Those fixtures stay the rule, and nothing here
//! redefines it. The exception is
//! `a_hand_checked_facture_prints_the_centimes_the_rows_add_up_to`, which
//! writes the three line totals and the closing figures of the cash case out
//! as constants derived from the fixture's own rows, with the derivation in
//! the comment above them: one place where a reader with a pen can check the
//! whole chain — apportionment, rounding, stamp — against the paper, without
//! running anything. It reads each of them off the label it is printed under,
//! so a page carrying the right figures on the wrong rows fails there.

mod common;

use common::facture::*;
use dzpos_retail::lang::Lang;
use dzpos_retail::money::{Money, Regime};
use dzpos_retail::print::strings::{shop_text, text, Key, ShopKey};
use dzpos_retail::print::{
    draw_facture_raster, dump_ticket_escpos, dump_ticket_escpos_png, facture_roll_lines, number,
    render_facture_escpos, render_facture_escpos_raster, render_facture_escpos_text, Paper,
    ThermalMode, HEAD_WIDTH_DOTS,
};
use dzpos_retail::services::documents::{DocumentKind, PartyKind};

/// The reduced golden set, the same three the HTML roll carries: a credit
/// facture to a company, a cash one to a consumer with the droit de timbre,
/// and a partial avoir against the first. The IFU, the proforma and the
/// cancelled reprint are not part of what was reduced — they are checked
/// below, where the test renders twice and reads no file.
const GOLDEN_CASES: [Case; 3] = [Case::Credit, Case::Cash, Case::Avoir];

/// The dump's own name. Spelled here rather than taken from
/// `common::facture`, whose `golden_name` says `.html`: this suite pins a
/// byte dump and not a page.
fn dump_name(lang: Lang, case: Case) -> String {
    format!("{}{}.txt", lang.tag(), case.suffix())
}

fn dump_golden(lang: Lang, case: Case, rendered: &str, updated: &mut Vec<String>) -> String {
    let dir = escpos_roll_goldens_dir();
    let name = dump_name(lang, case);
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

/// The narrow no-break space the money formatter groups thousands with is a
/// plain space on the text wire (`escpos::Buf::text`), so the golden carries
/// a plain space where the line model carries U+202F. Both start from the
/// same character; only the comparison needs telling.
fn on_the_wire(line: &str) -> String {
    line.replace('\u{202f}', " ")
}

/// The `.txt` golden is the text encoding in all three languages, the same
/// as the ticket's `ar.txt`. No shop is routed down this wire in Arabic —
/// `render_facture_escpos` sends `ar` to the raster under either preference
/// — but the golden has to carry the words for the line-equality test below
/// to have anything to compare the drawing against.
fn dump_of(fixture: &Fixture, lang: Lang) -> String {
    dump_ticket_escpos(&render_facture_escpos_text(&fixture.doc, &fixture.input(), lang).unwrap())
}

fn lines_of(fixture: &Fixture, lang: Lang) -> Vec<String> {
    facture_roll_lines(&fixture.doc, &fixture.input(), lang).unwrap()
}

/// The whole paper as one string, with the row breaks turned back into the
/// spaces they were.
///
/// A sentence the law asks for — the words line, the proforma's notice, the
/// line naming the facture an avoir corrects — is one sentence on an A4
/// sheet and three rows on 72 mm of roll. Asking whether it is *on the
/// paper* is therefore a question about the paper and not about a row.
/// `Items::wrapped` breaks on single spaces and rejoins on single spaces,
/// so a sentence survives this round trip exactly.
fn flat(fixture: &Fixture, lang: Lang) -> String {
    on_the_wire(&lines_of(fixture, lang).join(" "))
}

fn each_language_of(case: Case) {
    let fixture = Fixture::of(case);
    let mut updated = Vec::new();
    for lang in Lang::ALL {
        let rendered = dump_of(&fixture, lang);
        let expected = dump_golden(lang, case, &rendered, &mut updated);
        assert_eq!(
            rendered,
            expected,
            "{} is not what the encoder dumps",
            dump_name(lang, case)
        );
    }
    refuse_a_silent_regeneration(&updated);
}

#[test]
fn the_escpos_roll_credit_facture_is_its_dump_in_every_language() {
    each_language_of(Case::Credit);
}

#[test]
fn the_escpos_roll_cash_facture_is_its_dump_in_every_language() {
    each_language_of(Case::Cash);
}

#[test]
fn the_escpos_roll_avoir_is_its_dump_in_every_language() {
    each_language_of(Case::Avoir);
}

/// "1 234,56" back to 123456 centimes, by a reader that shares no code with
/// the formatter that wrote it. Either kind of grouping space, because the
/// line model holds U+202F and the wire holds U+0020.
fn centimes_of(printed: &str) -> i64 {
    let digits: String = printed
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ',' || *c == '-')
        .collect();
    let (whole, rest) = digits
        .split_once(',')
        .unwrap_or_else(|| panic!("{printed} is not an amount"));
    assert_eq!(rest.len(), 2, "{printed} does not carry two centime digits");
    let sign: i64 = if whole.starts_with('-') { -1 } else { 1 };
    let whole: i64 = whole.trim_start_matches('-').parse().unwrap();
    let rest: i64 = rest.parse().unwrap();
    sign * (whole * 100 + rest)
}

/// Every amount in one printed line, in the order it appears.
///
/// An amount is the shape the formatter writes and nothing else: optional
/// minus, groups of digits separated by the grouping space, a comma, and
/// exactly two digits. A quantity has at most one decimal
/// (`format_qty`), a rate is followed by a percent sign, and a date, a
/// phone and an RC carry no comma at all, so none of them is caught here.
/// The percent check is explicit rather than trusted to the two-digit rule,
/// because `percent` can write `0,01 %`.
fn amounts_in(line: &str) -> Vec<i64> {
    let chars: Vec<char> = line.chars().collect();
    let mut found = Vec::new();
    let mut at = 0usize;
    while at < chars.len() {
        if !chars[at].is_ascii_digit() {
            at = at.saturating_add(1);
            continue;
        }
        let start = if at > 0 && chars[at.saturating_sub(1)] == '-' {
            at.saturating_sub(1)
        } else {
            at
        };
        let mut end = at;
        while end < chars.len()
            && (chars[end].is_ascii_digit() || chars[end] == '\u{202f}' || chars[end] == ' ')
        {
            // A grouping space is only part of the number when three digits
            // follow it; the space before a percent sign or a word is not.
            if chars[end] == '\u{202f}' || chars[end] == ' ' {
                let after = chars.get(end.saturating_add(1)..end.saturating_add(4));
                if !after.is_some_and(|d| d.len() == 3 && d.iter().all(char::is_ascii_digit)) {
                    break;
                }
            }
            end = end.saturating_add(1);
        }
        let decimals = chars.get(end..end.saturating_add(3));
        let is_amount = decimals
            .is_some_and(|d| d.first() == Some(&',') && d[1..].iter().all(char::is_ascii_digit));
        if !is_amount {
            at = end.max(at.saturating_add(1));
            continue;
        }
        let stop = end.saturating_add(3);
        let followed_by_percent = chars.get(stop..).and_then(|rest| {
            rest.iter()
                .find(|c| !c.is_whitespace() && **c != '\u{202f}')
        }) == Some(&'%');
        if !followed_by_percent {
            let token: String = chars.get(start..stop).unwrap_or(&[]).iter().collect();
            found.push(centimes_of(&token));
        }
        at = stop;
    }
    found
}

/// The figures on the one row whose text opens with `prefix`, in the order
/// they are printed.
///
/// A closing figure read off its own label and not out of the multiset: two
/// amounts swapped under each other's labels leave the multiset untouched,
/// and the paper a customer pays from is the row and not the set.
fn amounts_on_the_row(lines: &[String], prefix: &str) -> Vec<i64> {
    let row = lines
        .iter()
        .find(|line| line.starts_with(prefix))
        .unwrap_or_else(|| panic!("no row on the roll opens with {prefix:?}: {lines:?}"));
    amounts_in(row)
}

/// The single figure on the row the key's own label opens.
fn figure_on(lines: &[String], key: Key, lang: Lang) -> i64 {
    let label = text(key, lang);
    let found = amounts_on_the_row(lines, label);
    assert_eq!(
        found.len(),
        1,
        "{label:?} does not hold one figure: {found:?}"
    );
    found[0]
}

/// Every figure the document holds that its facture prints, in centimes.
/// Built off the stored document and never off the render, so the two sides
/// of the comparison below share nothing.
fn stored_amounts(fixture: &Fixture) -> Vec<i64> {
    let doc = &fixture.doc;
    let totals = &doc.totals;
    let mut out: Vec<i64> = Vec::new();
    for line in &doc.lines {
        out.push(line.unit_price.as_centimes());
        if line.line_discount != Money::ZERO {
            // Stored as what it takes off and printed as what it does to
            // the column, the same sign the A4 goldens carry: -10,00 under
            // a line of 80,00.
            out.push(-line.line_discount.as_centimes());
        }
        out.push(line.line_total.as_centimes());
    }
    out.push(totals.total_ht.as_centimes());
    if totals.discount != Money::ZERO {
        out.push(-totals.discount.as_centimes());
        out.push(totals.subtotal_ht.as_centimes());
    }
    for row in &totals.tva_by_rate {
        out.push(row.base.as_centimes());
        out.push(row.amount.as_centimes());
    }
    // "TTC" names the tax, so the row is réel only; under the IFU it would
    // repeat the subtotal anyway.
    if doc.regime == Regime::Reel {
        out.push(totals.total_ttc.as_centimes());
    }
    if totals.stamp != Money::ZERO {
        out.push(totals.stamp.as_centimes());
    }
    out.push(totals.net_to_pay.as_centimes());
    if let Some(balance) = fixture.printed_balance() {
        out.push(balance.old_balance.as_centimes());
        out.push(balance.remaining_debt.as_centimes());
        out.push(balance.total_debt.as_centimes());
    }
    out.sort_unstable();
    out
}

/// **The rule this task is money work under.** Every figure the roll prints
/// is a figure the document stores, and every figure the document stores
/// reaches the roll: one multiset against the other, in centimes, read out
/// of the line model by a parser that shares no code with the formatter.
///
/// A row dropped to win the width comes back short. A row invented, or one
/// amount printed twice where the document holds it once, comes back long.
/// A total recomputed in the print path comes back with a figure the
/// document does not hold. Every case and every language, because a figure
/// that only goes wrong in Arabic is exactly the failure the raster path
/// exists to prevent.
#[test]
fn the_escpos_roll_prints_the_amounts_the_document_stores() {
    for case in Case::ALL {
        let fixture = Fixture::of(case);
        let stored = stored_amounts(&fixture);
        for lang in Lang::ALL {
            let mut printed: Vec<i64> = lines_of(&fixture, lang)
                .iter()
                .flat_map(|line| amounts_in(line))
                .collect();
            printed.sort_unstable();
            assert_eq!(
                printed, stored,
                "{case:?} {lang:?}: the roll's figures are not the document's"
            );
        }
    }
}

/// The three line totals and the total before tax, written out by hand from
/// the fixture's own rows rather than read off any code: 2 × 150,00 is
/// 300,00, 1,5 × 320,00 is 480,00, 1 × 80,00 less 10,00 is 70,00, and the
/// three make 850,00. One case and one language is enough — the multiset
/// check above carries the rest — and what this one adds is a figure a
/// reader can check against the fixture without running anything.
#[test]
fn a_hand_checked_facture_prints_the_centimes_the_rows_add_up_to() {
    let fixture = Fixture::of(Case::Cash);
    let printed: Vec<i64> = lines_of(&fixture, Lang::Fr)
        .iter()
        .flat_map(|line| amounts_in(line))
        .collect();
    for expected in [30_000, 48_000, 7_000, 85_000] {
        assert!(
            printed.contains(&expected),
            "the roll does not carry {expected} centimes: {printed:?}"
        );
    }
    // And the line discount, the one amount on the paper that is not a sum
    // of the others: 10,00 off the bread.
    assert!(
        printed.contains(&-1_000),
        "the line discount is not printed as what it takes off"
    );

    // The closing figures, worked out here by hand from the same three rows
    // and read back off the label each one is printed under.
    //
    // The 20,00 global discount is spread over the rate groups by their
    // share of the 850,00 HT, each share floored, and what the floors leave
    // over goes to the group holding the most HT
    // (`discount_spread_largest_remainder`, features.md §3, the fiscal rules
    // table). In centimes, the group HTs being 7 000, 48 000 and 30 000:
    //   0 %:  2 000 × 7 000 / 85 000 = 164,7 → 164
    //   9 %:  2 000 × 48 000 / 85 000 = 1 129,4 → 1 129
    //   19 %: 2 000 × 30 000 / 85 000 = 705,9 → 705
    // which allocates 1 998 of 2 000, so the two centimes left go to the
    // 9 % group, the largest of the three. The discounted bases are then
    // 6 836, 46 869 and 29 295 centimes — 68,36, 468,69 and 292,95 — and TVA
    // is rounded once per group, half away from zero
    // (`tva_rounding_once_per_rate`):
    //   9 %:  46 869 × 9 % = 4 218,21 → 42,18
    //   19 %: 29 295 × 19 % = 5 566,05 → 55,66
    // Total TTC is 830,00 + 0,00 + 42,18 + 55,66 = 927,84. The droit de
    // timbre is cash only and this case is cash: 927,84 is above the 300,00
    // floor and cut into 100,00 tranches rounded up, ten of them, at 1,00 a
    // tranche in the band up to 30 000,00 (`stamp_progressive_tranches`),
    // so 10,00. Net à payer is 927,84 + 10,00 = 937,84.
    const TVA_9: i64 = 4_218;
    const TVA_19: i64 = 5_566;
    const BASE_9: i64 = 46_869;
    const BASE_19: i64 = 29_295;
    const TOTAL_TTC: i64 = 92_784;
    const STAMP: i64 = 1_000;
    const NET_TO_PAY: i64 = 93_784;

    let lines = lines_of(&fixture, Lang::Fr);
    let tva = text(Key::Tva, Lang::Fr);
    // The narrow no-break space `percent` writes before the sign, which is
    // what the line model holds (the wire turns it into a plain space).
    let rate = |bps: &str| format!("{tva} {bps}\u{202f}%");
    // Base and tax on one row, in the order the row prints them, so a page
    // that carried the tax against the wrong base comes back wrong here.
    assert_eq!(amounts_on_the_row(&lines, &rate("9")), vec![BASE_9, TVA_9]);
    assert_eq!(
        amounts_on_the_row(&lines, &rate("19")),
        vec![BASE_19, TVA_19]
    );
    assert_eq!(figure_on(&lines, Key::TotalTtc, Lang::Fr), TOTAL_TTC);
    assert_eq!(figure_on(&lines, Key::Stamp, Lang::Fr), STAMP);
    assert_eq!(figure_on(&lines, Key::NetToPay, Lang::Fr), NET_TO_PAY);
}

/// Every closing row carries the figure its own label names.
///
/// The multiset check sorts both sides, so it cannot see two amounts
/// swapped under each other's labels: a paper reading `Total TTC 927,84`
/// and `Net à payer 917,84` holds exactly the same figures as one reading
/// them the right way round, and a customer paying the first would be
/// handing over the wrong money. This reads each figure off the row its
/// label is on instead. The cash case is the one to do it in, because the
/// droit de timbre sits between the two and makes them different numbers.
#[test]
fn each_closing_row_carries_the_figure_its_own_label_names() {
    let fixture = Fixture::of(Case::Cash);
    let totals = &fixture.doc.totals;
    let lines = lines_of(&fixture, Lang::Fr);
    let figure = |key: Key| figure_on(&lines, key, Lang::Fr);
    assert_eq!(
        figure(Key::TotalHt),
        totals.total_ht.as_centimes(),
        "the total before tax is not on its own row"
    );
    assert_eq!(
        figure(Key::TotalTtc),
        totals.total_ttc.as_centimes(),
        "the TTC row does not carry the stored TTC"
    );
    assert_eq!(
        figure(Key::Stamp),
        totals.stamp.as_centimes(),
        "the droit de timbre row does not carry the stored stamp"
    );
    assert_eq!(
        figure(Key::NetToPay),
        totals.net_to_pay.as_centimes(),
        "the row the customer pays does not carry the stored net"
    );
    // The two that could be swapped are different numbers here, which is
    // what makes the four assertions above worth writing.
    assert_ne!(
        totals.total_ttc.as_centimes(),
        totals.net_to_pay.as_centimes()
    );
}

/// The roll says everything the standard page says.
///
/// Every field of the view, not a list of amounts: the number, the day,
/// both parties with every identifier they carry, every line's name and
/// rate, and the words line read off the standard render rather than
/// restated here. Every case, because the reduced golden set never reaches
/// the proforma's notice, the IFU page or the cancelled reprint's mark.
#[test]
fn the_escpos_roll_says_everything_the_standard_page_says() {
    for case in Case::ALL {
        let fixture = Fixture::of(case);
        let doc = &fixture.doc;
        for lang in Lang::ALL {
            let paper = flat(&fixture, lang);
            let carries = |needle: &str| paper.contains(&on_the_wire(needle));
            let standard = fixture.render(lang, Paper::A4);

            assert!(
                carries(&number(doc)),
                "{case:?} {lang:?}: the roll lost the document number"
            );
            // The document's own day, never today's.
            assert!(
                carries(&doc.issued_at.format("%d/%m/%Y").to_string()),
                "{case:?} {lang:?}: the roll lost the day"
            );
            // The words line décret 05-468 art. 3 asks for, taken off the
            // A4 render so the two papers are compared and neither is
            // compared against a sentence written here.
            assert!(
                carries(&in_words(&standard)),
                "{case:?} {lang:?}: the roll lost the words line"
            );
            assert!(
                carries(&doc.seller.name),
                "{case:?} {lang:?}: the roll lost the seller"
            );
            // The identifiers are what make this a facture rather than a
            // receipt. A consumer buyer prints none, which `buyer_view`
            // decided long before the roll sees it, so only what the
            // document carries is asked for.
            for id in [
                &doc.seller.rc,
                &doc.seller.nif,
                &doc.seller.nis,
                &doc.seller.ai,
            ]
            .into_iter()
            .flatten()
            {
                assert!(carries(id), "{case:?} {lang:?}: the seller's {id} is gone");
            }
            if let Some(buyer) = doc.buyer.as_ref() {
                assert!(
                    carries(&buyer.name),
                    "{case:?} {lang:?}: the roll lost the buyer"
                );
                if buyer.party_kind == PartyKind::Company {
                    for id in [&buyer.rc, &buyer.nif, &buyer.nis, &buyer.ai]
                        .into_iter()
                        .flatten()
                    {
                        assert!(carries(id), "{case:?} {lang:?}: the buyer's {id} is gone");
                    }
                } else {
                    // A consumer's facture carries their name and address
                    // and nothing else (décret 05-468 art. 3-2, last
                    // alinéa). A row that turned up here would be an
                    // identifier the sheet refuses to print.
                    for id in [&buyer.rc, &buyer.nif, &buyer.nis, &buyer.ai]
                        .into_iter()
                        .flatten()
                    {
                        assert!(
                            !carries(id),
                            "{case:?} {lang:?}: a consumer's {id} reached the roll"
                        );
                    }
                }
            }
            for line in &doc.lines {
                assert!(
                    carries(&line.name),
                    "{case:?} {lang:?}: the line {:?} is gone",
                    line.name
                );
            }
            // A rate is a figure a reader checks a line against, so each
            // line's rate and each recap row's rate is on the paper under
            // the réel. Under the IFU there is no rate at all, which the
            // test below says in its own words.
            if doc.regime == Regime::Reel {
                for rate in doc
                    .lines
                    .iter()
                    .map(|l| l.rate_bps)
                    .chain(doc.totals.tva_by_rate.iter().map(|r| r.rate))
                {
                    // Whole percentages only: the fixture has 19, 9 and 0,
                    // and spelling the half-point form here would be a
                    // second copy of `print::percent`.
                    assert_eq!(rate.as_u32() % 100, 0, "the fixture grew a part rate");
                    let printed = rate.as_u32() / 100;
                    assert!(
                        carries(&format!("{printed} %")),
                        "{case:?} {lang:?}: the rate {printed} % is not on the roll"
                    );
                }
            }
            // How it was paid, where the question means anything: an avoir
            // hands money back and a proforma settles nothing, so neither
            // names a mode and the sheet says so too.
            // The block's own heading and not the word for the mode: an
            // avoir whose triple closes below zero prints "total credit"
            // in its balance block, and asking for the word "credit" would
            // have read that as a mode of payment on a document that names
            // none.
            let names_a_mode = !matches!(doc.kind, DocumentKind::Avoir | DocumentKind::Proforma);
            assert_eq!(
                carries(text(Key::PaymentMode, lang)),
                names_a_mode,
                "{case:?} {lang:?}: the mode of payment block"
            );
            if names_a_mode {
                assert!(
                    carries(text(
                        match doc.payment_mode {
                            dzpos_retail::money::PaymentMode::Cash => Key::Cash,
                            dzpos_retail::money::PaymentMode::Card => Key::Card,
                            dzpos_retail::money::PaymentMode::Credit => Key::Credit,
                        },
                        lang
                    )),
                    "{case:?} {lang:?}: the mode itself"
                );
            }
            // Both signature lines, on every face including the proforma,
            // the same call the HTML roll makes.
            for label in [Key::Seller, Key::Buyer, Key::Cachet] {
                assert!(
                    carries(text(label, lang)),
                    "{case:?} {lang:?}: the roll lost a signature label"
                );
            }
        }
    }
}

/// An avoir names the facture it corrects, a proforma says it settles
/// nothing, and a cancelled reprint says it is cancelled, when, and why.
/// Each is a sentence the law or the spec asks for on one face of this
/// document only, so each is asked for on the face that owes it.
#[test]
fn the_escpos_roll_carries_the_sentence_each_face_owes() {
    for lang in Lang::ALL {
        let avoir = Fixture::of(Case::Avoir);
        let paper = flat(&avoir, lang);
        // The facture it is written against, by the number that facture
        // printed and by that facture's day, neither of which is the
        // avoir's own: the avoir was written three days later.
        assert!(
            paper.contains("FA-2026-000042"),
            "{lang:?}: the avoir does not name the facture it corrects"
        );
        assert!(
            paper.contains("09/09/2026"),
            "{lang:?}: the avoir does not carry the referenced facture's day"
        );
        assert!(
            paper.contains(shop_text(ShopKey::AvoirOnFacture, lang)),
            "{lang:?}: the avoir does not say it is one"
        );

        let proforma = Fixture::of(Case::Proforma);
        let paper = flat(&proforma, lang);
        assert!(
            paper.contains(&on_the_wire(shop_text(ShopKey::ProformaNotice, lang))),
            "{lang:?}: the proforma does not say it settles nothing"
        );

        let cancelled = Fixture::of(Case::Cancelled);
        let paper = flat(&cancelled, lang);
        assert!(
            paper.contains(text(Key::CancelledMark, lang)),
            "{lang:?}: the cancelled reprint does not say it is void"
        );
        assert!(
            paper.contains("12/09/2026"),
            "{lang:?}: the cancelled reprint does not carry the day it was cancelled"
        );
        assert!(
            paper.contains(&on_the_wire(CANCEL_REASON)),
            "{lang:?}: the cancelled reprint does not carry the reason"
        );
        // And it keeps the number it burned.
        assert!(
            paper.contains("FA-2026-000042"),
            "{lang:?}: the cancelled reprint lost its number"
        );
    }
}

/// Under the IFU the paper must not mention the tax at all (CTCA 2026
/// art. 64), which on a roll means the same three words are absent and the
/// total row changes its wording rather than its figure.
#[test]
fn the_escpos_roll_ifu_facture_names_no_tax_in_any_language() {
    let fixture = Fixture::of(Case::Ifu);
    for lang in Lang::ALL {
        let paper = flat(&fixture, lang);
        for forbidden in ["TVA", "VAT", "ت.ق.م", "TTC", "%"] {
            assert!(
                !paper.contains(forbidden),
                "{lang:?}: an IFU roll names {forbidden}"
            );
        }
    }
}

/// Nothing is trimmed to fit. A line wider than the head is an error from
/// `raster::draw`, so reaching these assertions means every line fit; they
/// say by how much, and `notdef` says no character was drawn as the box
/// this whole plan is about.
///
/// Every case and every language, and the 58 mm head beside the 80 mm one,
/// because a narrower head is where a wrapped sentence is most likely not
/// to fit.
#[test]
fn no_escpos_roll_line_is_clipped_at_the_head_width() {
    for case in Case::ALL {
        let fixture = Fixture::of(case);
        for lang in Lang::ALL {
            for width in [HEAD_WIDTH_DOTS, 384] {
                let drawn =
                    draw_facture_raster(&fixture.doc, &fixture.input(), lang, width).unwrap();
                assert!(!drawn.lines.is_empty(), "{case:?} {lang:?}: nothing drawn");
                for line in &drawn.lines {
                    assert!(
                        line.advance <= width,
                        "{case:?} {lang:?} at {width}: {:?} took {} dots",
                        line.text,
                        line.advance
                    );
                    assert_eq!(
                        line.notdef, 0,
                        "{case:?} {lang:?} at {width}: {:?} has a character no vendored font carries",
                        line.text
                    );
                }
                assert_eq!(drawn.bitmap.width(), width as usize, "{case:?} {lang:?}");
                assert!(drawn.bitmap.height() > 0, "{case:?} {lang:?}");
            }
        }
    }
}

/// The raster's source lines are the text path's lines, string for string.
/// The left side is the committed `.txt` golden — a file, not a value this
/// run computed — and the right side is what `print::raster` says it drew.
/// If this goes red because an amount differs, the bug is that something
/// inside the drawing code formatted a number, which nothing in there is
/// allowed to do.
#[test]
fn the_facture_raster_draws_the_lines_the_text_path_prints() {
    for case in GOLDEN_CASES {
        let fixture = Fixture::of(case);
        for lang in Lang::ALL {
            let name = dump_name(lang, case);
            let golden = std::fs::read_to_string(escpos_roll_goldens_dir().join(&name))
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            let text_lines: Vec<String> = golden
                .lines()
                .filter(|line| !line.is_empty() && !(line.starts_with('<') && line.ends_with('>')))
                .map(str::to_owned)
                .collect();
            let drawn =
                draw_facture_raster(&fixture.doc, &fixture.input(), lang, HEAD_WIDTH_DOTS).unwrap();
            let raster: Vec<String> = drawn
                .lines
                .iter()
                .map(|line| on_the_wire(&line.text))
                .collect();
            assert_eq!(raster, text_lines, "{name}: the two paths differ");
        }
    }
}

/// Arabic is bands and nothing else: no codepage is selected and no Arabic
/// character is on the wire, because the whole point is that the head is
/// never asked to spell anything.
#[test]
fn the_arabic_escpos_roll_is_bands_of_dots() {
    for case in GOLDEN_CASES {
        let fixture = Fixture::of(case);
        let bytes =
            render_facture_escpos(&fixture.doc, &fixture.input(), Lang::Ar, ThermalMode::Text)
                .unwrap();
        let dump = dump_ticket_escpos(&bytes);
        assert!(dump.starts_with("<init>\n<align left>\n"), "{dump}");
        assert!(dump.ends_with("<cut>\n"), "{dump}");
        assert!(!dump.contains("<codepage"), "{:?}: {dump}", case.suffix());
        assert!(
            dump.lines().any(|line| line.starts_with("<raster 576x")),
            "{:?}: no band",
            case.suffix()
        );
        assert!(
            !dump.contains('ت'),
            "{:?}: Arabic text on a raster wire",
            case.suffix()
        );
    }
}

/// The pictures a reviewer opens. Decoded back out of the bytes the head is
/// sent, so what is written to disk is what the paper would carry; a
/// ligature drawn wrong is visible here and nowhere else.
#[test]
fn the_arabic_escpos_roll_writes_a_picture_a_reviewer_opens() {
    let mut updated: Vec<String> = Vec::new();
    for case in GOLDEN_CASES {
        let fixture = Fixture::of(case);
        let bytes =
            render_facture_escpos_raster(&fixture.doc, &fixture.input(), Lang::Ar, HEAD_WIDTH_DOTS)
                .unwrap();
        let png = dump_ticket_escpos_png(&bytes).expect("the Arabic roll carries bands");
        let name = format!("ar{}.png", case.suffix());
        let path = escpos_roll_goldens_dir().join(&name);
        if std::env::var_os("UPDATE_GOLDENS").is_some() {
            std::fs::create_dir_all(escpos_roll_goldens_dir()).unwrap();
            if std::fs::read(&path).unwrap_or_default() != png {
                std::fs::write(&path, &png).unwrap();
                updated.push(name);
            }
            continue;
        }
        let on_disk = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("{}: {e}; run UPDATE_GOLDENS=1", path.display()));
        assert_eq!(
            on_disk, png,
            "{name} is not the picture the bands decode to"
        );
    }
    refuse_a_silent_regeneration(&updated);
}

/// Which wire the facture goes down, the same rule the ticket keeps: Arabic
/// is drawn whatever the shop stored, French and English follow it.
#[test]
fn the_escpos_roll_takes_the_wire_the_shop_is_on_except_in_arabic() {
    let fixture = Fixture::of(Case::Credit);
    for stored in ThermalMode::ALL {
        let ar = dump_ticket_escpos(
            &render_facture_escpos(&fixture.doc, &fixture.input(), Lang::Ar, stored).unwrap(),
        );
        assert!(
            ar.contains("<raster 576x") && !ar.contains("<codepage"),
            "{stored:?}: the Arabic facture went down the text path"
        );
        for lang in [Lang::Fr, Lang::En] {
            let dump = dump_ticket_escpos(
                &render_facture_escpos(&fixture.doc, &fixture.input(), lang, stored).unwrap(),
            );
            match stored {
                ThermalMode::Text => assert!(
                    dump.contains("<codepage 19>") && !dump.contains("<raster "),
                    "{lang:?} under text is not the text wire"
                ),
                ThermalMode::Raster => assert!(
                    dump.contains("<raster 576x") && !dump.contains("<codepage"),
                    "{lang:?} under raster is not the raster wire"
                ),
            }
        }
    }
}

/// The roll refuses exactly what the sheet refuses. The list is one list in
/// `print::refusals` and this route reaches it through the same builder, so
/// a document the A4 page will not print does not quietly come out of a
/// thermal head instead.
#[test]
fn the_escpos_roll_refuses_what_the_sheet_refuses() {
    // An IFU document carrying a TVA recap: the régime it was issued under
    // says it has no tax to name, and the stored row says it does.
    let mut ifu = Fixture::of(Case::Ifu);
    ifu.doc
        .totals
        .tva_by_rate
        .push(dzpos_retail::money::TvaLine {
            rate: dzpos_retail::money::Bps::new(1900).unwrap(),
            base: Money::centimes(29_295),
            amount: Money::centimes(5_566),
        });
    // A facture with no buyer block is not a facture.
    let mut no_buyer = Fixture::of(Case::Credit);
    no_buyer.doc.buyer = None;
    // An avoir hands the lines back and asks for nothing, so it carries no
    // droit de timbre.
    let mut stamped_avoir = Fixture::of(Case::Avoir);
    stamped_avoir.doc.totals.stamp = Money::centimes(4_000);

    for (what, fixture) in [
        ("an IFU document with a TVA recap", &ifu),
        ("a facture with no buyer", &no_buyer),
        ("an avoir carrying a stamp", &stamped_avoir),
    ] {
        for lang in Lang::ALL {
            for mode in ThermalMode::ALL {
                let err =
                    render_facture_escpos(&fixture.doc, &fixture.input(), lang, mode).unwrap_err();
                assert_eq!(err.code(), "print", "{what} {lang:?} {mode:?}");
            }
        }
    }
}

/// A ticket's id is not a facture. The ESC/POS roll titles itself facture,
/// avoir or proforma and has no wording for anything else, so it refuses
/// rather than printing a till slip under the word FACTURE.
#[test]
fn a_ticket_has_no_escpos_facture() {
    let mut fixture = Fixture::of(Case::Credit);
    fixture.doc.kind = DocumentKind::Ticket;
    let err = render_facture_escpos(&fixture.doc, &fixture.input(), Lang::Fr, ThermalMode::Text)
        .unwrap_err();
    assert_eq!(err.code(), "print");
}
