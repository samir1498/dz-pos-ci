// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The facture on an 80 mm roll, against its own goldens and against the
//! standard page it has to agree with.
//!
//! Regenerate with `UPDATE_GOLDENS=1 cargo test -p dzpos-core --test
//! print_facture_roll`. The run that rewrites them fails on purpose.
//!
//! This is the layout the other two cannot be checked like. The compact A4
//! and the half sheet are the standard page's own markup under a different
//! stylesheet, so their suites assert that everything below `<body>` is byte
//! for byte the standard render and are done. 72 mm has no room for that:
//! the party blocks stack, the six column table becomes two rows per line,
//! and the signature panels sit one above the other. The markup is its own,
//! so what holds this layout is a field by field comparison against the
//! standard page, plus the same amount read-back every layout's goldens get.
//!
//! A reduced set of cases, like the half sheet's: a credit facture, a cash
//! one, and an avoir against the first. The IFU régime is not part of what
//! was reduced and is checked below.

mod common;

use common::facture::*;
use dzpos_retail::lang::Lang;
use dzpos_retail::print::{FactureLayout, Page, Paper};

/// The roll against its own goldens, case by case and language by language,
/// with the same amount read-back the other layouts get: every amount in the
/// file is parsed back to centimes and compared with what the document
/// stores, so a roll that drew a total the document does not hold fails here
/// and not at a counter.
fn each_language_of_the_roll(case: Case) {
    each_language_of_a_layout(case, FactureLayout::Roll80);
}

#[test]
fn the_roll_credit_facture_is_its_golden_in_every_language() {
    each_language_of_the_roll(Case::Credit);
}

#[test]
fn the_roll_cash_facture_is_its_golden_in_every_language() {
    each_language_of_the_roll(Case::Cash);
}

#[test]
fn the_roll_avoir_is_its_golden_in_every_language() {
    each_language_of_the_roll(Case::Avoir);
}

/// The roll says everything the standard page says.
///
/// This is the check the roll exists under, and it is the whole reason the
/// four layouts share one `FactureView`: this template's markup is its own,
/// so nothing but the fields it is filled from keeps it a facture. Décret
/// 05-468 art. 3 does not have a narrow version, and a roll that won its
/// width by dropping the buyer's NIF would be a ticket with a facture's
/// title on it.
///
/// Every case, not the three the goldens cover. It renders twice and reads
/// no file, so the proforma's notice, the IFU page and the cancelled
/// reprint's mark are free to include, and they are the three the reduced
/// golden set never reaches.
#[test]
fn the_roll_says_everything_the_standard_page_says() {
    for case in Case::ALL {
        let fixture = Fixture::of(case);
        for lang in Lang::ALL {
            let standard = fixture.render(lang, Paper::A4);
            let roll = fixture.render_in(lang, Paper::A4, FactureLayout::Roll80);

            // Every amount marker, read off the standard page rather than
            // listed here, with how many of each carry the currency span
            // beside them. A layout that dropped a row comes back short, one
            // that invented a row comes back long, and one that printed a
            // figure with no DA beside it comes back with the same count and
            // a smaller second number.
            let markers = amount_markers(&standard);
            assert_eq!(
                markers,
                amount_markers(&roll),
                "{case:?} {lang:?} the amount markers and their currency"
            );
            for marker in markers.keys() {
                assert_eq!(
                    amounts(&standard, marker),
                    amounts(&roll, marker),
                    "{case:?} {lang:?} amount-{marker}"
                );
            }
            for marker in ["line", "tva"] {
                assert_eq!(
                    rates(&standard, marker),
                    rates(&roll, marker),
                    "{case:?} {lang:?} rate-{marker}"
                );
            }
            assert_eq!(
                qtys(&standard),
                qtys(&roll),
                "{case:?} {lang:?} the quantities"
            );
            assert_eq!(
                in_words(&standard),
                in_words(&roll),
                "{case:?} {lang:?} the words line"
            );
            assert_eq!(
                heading(&standard),
                heading(&roll),
                "{case:?} {lang:?} the heading"
            );
            // The word and not the markup around it. The sheet draws the
            // mark as a rotated span across the page; a roll is 72 mm wide,
            // so a diagonal word would sit on top of the amounts instead of
            // behind them and this one is a banner. Both say ANNULÉE, which
            // is the part a customer holding the page reads.
            let mark_word = |html: &str| {
                mark_block(html).map(|block| {
                    block
                        .replace("<span>", "")
                        .replace("</span>", "")
                        .trim()
                        .to_owned()
                })
            };
            assert_eq!(
                mark_word(&standard),
                mark_word(&roll),
                "{case:?} {lang:?} the cancelled mark"
            );

            // Every word the standard page prints, the roll prints too.
            // The roll may say more (it separates a quantity from a price
            // with a × the sheet does not need), but it may not say less
            // and it may not say it differently: a label written into this
            // template as a literal rather than taken from the view would
            // pass every other assertion here, and the layout that reworded
            // "Net à payer" is the layout a customer argues with.
            let said = words_of(&roll);
            for word in words_of(&without_the_lines_table_head(&standard)) {
                assert!(
                    said.contains(&word),
                    "{case:?} {lang:?} the roll does not say {word}"
                );
            }

            // Both parties, read off the fixture rather than typed here, so
            // a fixture that gains an identifier gains it in this test too.
            // The count and not merely the presence: a roll that printed the
            // NIF twice and the RC never would pass a presence check.
            let seller = seller();
            let buyer = fixture.doc.buyer.as_ref().expect("a facture has a buyer");
            let printed = [
                seller.rc,
                seller.nif,
                seller.nis,
                seller.ai,
                seller.address,
                Some(seller.name),
            ]
            .into_iter()
            .chain([
                buyer.rc.clone(),
                buyer.nif.clone(),
                buyer.nis.clone(),
                buyer.ai.clone(),
                buyer.address.clone(),
                Some(buyer.name.clone()),
            ])
            .flatten();
            for id in printed {
                assert_eq!(
                    standard.matches(&id).count(),
                    roll.matches(&id).count(),
                    "{case:?} {lang:?} the party identifier {id}"
                );
            }
        }
    }
}

/// The roll is not the standard page under another stylesheet, and saying so
/// is the point.
///
/// Its two siblings assert the opposite: their bodies are byte for byte the
/// standard render. If this layout ever became that too, the cheap check
/// would be available and the expensive one above would be the wrong shape
/// for it. This fails the day someone makes that change without noticing
/// that the comparison guarding the page then needs to change with it.
#[test]
fn the_roll_draws_its_own_markup() {
    let fixture = Fixture::of(Case::Credit);
    let standard = fixture.render(Lang::Fr, Paper::A4);
    let roll = fixture.render_in(Lang::Fr, Paper::A4, FactureLayout::Roll80);
    let body = |html: &str| {
        html.split_once("<body>")
            .expect("the page has a body")
            .1
            .to_owned()
    };
    assert_ne!(
        body(&standard),
        body(&roll),
        "the roll now renders the standard body: the field by field \
         comparison above can be replaced with the byte comparison its \
         siblings use"
    );
    // And it is a roll rather than a narrow sheet: the table the standard
    // page lays its lines out in does not fit and is not there.
    assert!(
        standard.contains("<table>"),
        "the standard page lost its table"
    );
    assert!(!roll.contains("<table>"), "the roll kept the sheet's table");
}

/// The IFU rule holds on the roll too.
///
/// Its goldens are the reduced set and all three are under the réel, so no
/// file here has ever been rendered under the IFU régime. The comparison
/// above covers what the two pages share, but CTCA 2026 art. 64 is a claim
/// about the whole page and this template's own header makes it.
///
/// It is also the layout where the claim is easiest to break: the rate sits
/// inline beside the quantity here rather than in a column of its own, so a
/// template that forgot the `if let Some(rate)` would print an empty
/// separator on a page that must not mention the tax at all.
#[test]
fn the_roll_ifu_facture_names_no_tax_in_any_language() {
    let fixture = Fixture::of(Case::Ifu);
    for lang in Lang::ALL {
        names_no_tax(
            &fixture.render_in(lang, Paper::Roll80, FactureLayout::Roll80),
            lang,
        );
    }
}

/// The roll names its own paper, whatever the caller asked for.
///
/// A till naming A4 is saying which tray to use, and a shop having chosen
/// the roll is a standing decision about where its factures come out. 72 mm
/// of content on an A4 sheet is a column down the left of an empty page, and
/// nobody asking for A4 means that.
///
/// Read off the `@page` rule, which is the one line `Paper` sets, so this
/// fails if the override is dropped anywhere between the query string and
/// the stylesheet.
#[test]
fn the_roll_prints_on_the_roll_even_when_a_sheet_was_asked_for() {
    let fixture = Fixture::of(Case::Credit);
    for lang in Lang::ALL {
        for asked_for in [Paper::A4, Paper::A5, Paper::Roll80] {
            let rule = page_rule(&fixture.render_in(lang, asked_for, FactureLayout::Roll80));
            assert!(
                rule.contains("size: 80mm auto"),
                "{lang:?} the roll asked for {asked_for:?} did not print on the roll: {rule}"
            );
        }
    }

    assert_eq!(
        Page {
            paper: Paper::A4,
            layout: FactureLayout::Roll80
        }
        .paper(),
        Paper::Roll80
    );
    // A sheet layout asked for the roll still takes it, because that is the
    // caller's business: this override belongs to the roll layout and is not
    // a rule about which papers exist.
    assert_eq!(
        Page {
            paper: Paper::Roll80,
            layout: FactureLayout::Standard
        }
        .paper(),
        Paper::Roll80
    );
}
