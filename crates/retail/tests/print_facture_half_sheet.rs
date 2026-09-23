// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The half sheet facture against its own goldens, and against the standard
//! layout it has to agree with.
//!
//! Regenerate with `UPDATE_GOLDENS=1 cargo test -p dzpos-core --test
//! print_facture_half_sheet`. The run that rewrites them fails on purpose.
//!
//! A reduced set of cases, unlike the compact layout's. The compact A4
//! replaces today's A4 in every situation a shop might print one, so it owes
//! all six; the half sheet is chosen for the counter, where a facture is
//! handed over with the goods, so it owes the three that reach one: a credit
//! facture, a cash one, and an avoir against the first.

mod common;

use common::facture::*;
use dzpos_retail::lang::Lang;
use dzpos_retail::print::{FactureLayout, Page, Paper};

/// The half sheet against its own goldens, case by case and language by
/// language, with the same amount read-back the other layouts get.
fn each_language_of_the_half_sheet(case: Case) {
    each_language_of_a_layout(case, FactureLayout::HalfSheet);
}

#[test]
fn the_half_sheet_credit_facture_is_its_golden_in_every_language() {
    each_language_of_the_half_sheet(Case::Credit);
}

#[test]
fn the_half_sheet_cash_facture_is_its_golden_in_every_language() {
    each_language_of_the_half_sheet(Case::Cash);
}

#[test]
fn the_half_sheet_avoir_is_its_golden_in_every_language() {
    each_language_of_the_half_sheet(Case::Avoir);
}

/// The half sheet is a stylesheet, not a second document.
///
/// Everything below `<body>` is byte for byte what the standard layout
/// renders; only the rules above it differ. The same claim the compact
/// layout makes, and it is asserted for the same reason: it catches a row
/// dropped, a row invented, a row moved and a label rewritten at once,
/// without a list of fields to keep current.
///
/// Décret 05-468 art. 3 has no half-sheet version, and this is what says so.
///
/// Every case, not the three the goldens cover. This test compares two live
/// renders and reads no file, so the other three are free, and they are the
/// ones carrying a conditional the reduced set never reaches: the proforma's
/// notice, the IFU page with no rate column, and the cancelled reprint's
/// mark. A shop printing a proforma on the half sheet is not a case anyone
/// planned for, which is the reason to cover it rather than not to.
#[test]
fn the_half_sheet_changes_the_stylesheet_and_nothing_below_it() {
    for case in Case::ALL {
        let fixture = Fixture::of(case);
        for lang in Lang::ALL {
            let standard = fixture.render(lang, Paper::A5);
            let half_sheet = fixture.render_in(lang, Paper::A5, FactureLayout::HalfSheet);
            let body = |html: &str| {
                html.split_once("<body>")
                    .expect("the page has a body")
                    .1
                    .to_owned()
            };
            assert_eq!(body(&standard), body(&half_sheet), "{case:?} {lang:?}");
            assert_ne!(
                standard, half_sheet,
                "{case:?} {lang:?} the two layouts render the same file"
            );
        }
    }
}

/// The IFU rule holds on the half sheet too.
///
/// This layout's goldens are the reduced set, all three under the réel, so
/// no file here has ever been rendered under the IFU régime. The body
/// comparison above covers the markup, but CTCA 2026 art. 64 is a claim
/// about the whole page, stylesheet included, and this template's own
/// header makes that claim. Nothing else on this branch proved it.
///
/// The régime is not what the reduced case set reduced: it left out document
/// kinds, and an IFU shop choosing the half sheet for its counter is exactly
/// the shop this layout was written for.
#[test]
fn the_half_sheet_ifu_facture_names_no_tax_in_any_language() {
    let fixture = Fixture::of(Case::Ifu);
    for lang in Lang::ALL {
        names_no_tax(
            &fixture.render_in(lang, Paper::A5, FactureLayout::HalfSheet),
            lang,
        );
    }
}

/// The layout names its own sheet, whatever the caller asked for.
///
/// A till naming A4 is saying which tray to use; a shop having chosen the
/// half sheet is a standing decision about what its factures look like. The
/// type sizes and the column widths here are measured for 148 mm, so the A4
/// the caller asked for would leave a small facture in the corner of a large
/// page, and nobody asking for A4 means that.
///
/// Read off the `@page` rule, which is the one line `Paper` sets, so this
/// fails if the override is dropped anywhere between the query string and
/// the stylesheet.
#[test]
fn the_half_sheet_prints_on_a5_even_when_a4_was_asked_for() {
    let fixture = Fixture::of(Case::Credit);
    for lang in Lang::ALL {
        for asked_for in [Paper::A4, Paper::A5] {
            let html = fixture.render_in(lang, asked_for, FactureLayout::HalfSheet);
            let rule = page_rule(&html);
            assert!(
                rule.contains("size: A5"),
                "{lang:?} the half sheet asked for {asked_for:?} did not print on A5: {rule}"
            );
            assert!(
                !rule.contains("size: A4"),
                "{lang:?} the half sheet asked for {asked_for:?} printed on A4: {rule}"
            );
        }
    }

    // The layouts that have no fixed sheet still take the one they are given,
    // so the override above is the half sheet's and not everybody's.
    let a4 = fixture.render(Lang::Fr, Paper::A4);
    assert!(
        page_rule(&a4).contains("size: A4"),
        "the standard layout lost its A4"
    );
    assert_eq!(
        Page {
            paper: Paper::A4,
            layout: FactureLayout::Standard
        }
        .paper(),
        Paper::A4
    );
    assert_eq!(
        Page {
            paper: Paper::A4,
            layout: FactureLayout::HalfSheet
        }
        .paper(),
        Paper::A5
    );
}
