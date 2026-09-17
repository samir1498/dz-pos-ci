// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The compact A4 facture against its own goldens, and against the standard
//! layout it has to agree with.
//!
//! Regenerate with `UPDATE_GOLDENS=1 cargo test -p dzpos-core --test
//! print_facture_compact`. The run that rewrites them fails on purpose.
//!
//! The goldens alone are not enough here and that is the point of the last
//! test in this file. A golden only says a file has not changed since
//! somebody looked at it, so a compact layout that won a line by dropping the
//! buyer's identifiers would pass its own goldens the moment they were
//! regenerated. Décret 05-468 art. 3 has no compact version, so every amount,
//! every identifier and the words line are compared between the two renders.

mod common;

use common::facture::*;
use dzpos_core::lang::Lang;
use dzpos_core::print::{FactureLayout, Paper};

/// The compact layout against its own goldens, case by case and language by
/// language, the same way the standard one is checked.
///
/// The same assertions run over both files, `the_golden_says_what_the_document
/// _stores` included, so a layout that drew a total the document does not hold
/// fails here and not in a shop. That check is the whole reason a second
/// layout is cheap: the two pages are the same facts drawn twice, and the
/// facts are read back out of each file rather than trusted.
fn each_language_of_the_compact_layout(case: Case) {
    each_language_of_a_layout(case, FactureLayout::Compact);
}

#[test]
fn the_compact_credit_facture_is_its_golden_in_every_language() {
    each_language_of_the_compact_layout(Case::Credit);
}

#[test]
fn the_compact_cash_facture_is_its_golden_in_every_language() {
    each_language_of_the_compact_layout(Case::Cash);
}

#[test]
fn the_compact_ifu_facture_is_its_golden_in_every_language() {
    each_language_of_the_compact_layout(Case::Ifu);
}

#[test]
fn the_compact_avoir_is_its_golden_in_every_language() {
    each_language_of_the_compact_layout(Case::Avoir);
}

#[test]
fn the_compact_proforma_is_its_golden_in_every_language() {
    each_language_of_the_compact_layout(Case::Proforma);
}

#[test]
fn the_compact_cancelled_facture_is_its_golden_in_every_language() {
    each_language_of_the_compact_layout(Case::Cancelled);
}

/// The two layouts say the same thing in different type. Every amount on the
/// page, every identifier and address of both parties, and the words line are
/// compared between them, case by case and language by language.
///
/// This is the check the compact layout exists under. A layout is allowed to
/// be smaller, tighter and differently spaced; it is not allowed to be a
/// different document, and décret 05-468 art. 3 does not have a compact
/// version. Without this, a line dropped to win space would pass its own
/// golden happily, because a golden only says the file has not changed since
/// somebody looked at it.
#[test]
fn the_compact_layout_says_everything_the_standard_one_says() {
    for case in Case::ALL {
        let fixture = Fixture::of(case);
        for lang in Lang::ALL {
            let standard = fixture.render(lang, Paper::A4);
            let compact = fixture.render_in(lang, Paper::A4, FactureLayout::Compact);
            // Every marker the standard template carries. A layout that
            // dropped a row would come back with a shorter list here, and a
            // layout that invented one would come back with a longer.
            for marker in [
                "discount",
                "line",
                "line-discount",
                "net-to-pay",
                "old-balance",
                "stamp",
                "subtotal",
                "this-document",
                "total",
                "total-debt",
                "total-ttc",
                "tva",
                "tva-base",
                "unit-price",
            ] {
                assert_eq!(
                    amounts(&standard, marker),
                    amounts(&compact, marker),
                    "{case:?} {lang:?} amount-{marker}"
                );
            }
            assert_eq!(
                in_words(&standard),
                in_words(&compact),
                "{case:?} {lang:?} the words line"
            );
            // Both parties, read off the fixture rather than typed here, so
            // a fixture that gains an identifier gains it in this test too.
            // The count and not merely the presence: a layout that printed
            // the NIF twice and the RC never would pass a presence check.
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
                    compact.matches(&id).count(),
                    "{case:?} {lang:?} the party identifier {id}"
                );
            }
        }
    }
}

/// The IFU rule holds on the compact sheet too.
///
/// The cross-layout test above compares the amounts both pages carry, so a
/// compact layout that invented a TVA row would already fail there. What it
/// cannot see is a rate column or the word itself written into this template
/// as a literal, because the standard one has nothing to compare it against.
/// CTCA 2026 art. 64 does not care which sheet the document was drawn on.
#[test]
fn the_compact_ifu_facture_names_no_tax_in_any_language() {
    let fixture = Fixture::of(Case::Ifu);
    for lang in Lang::ALL {
        names_no_tax(
            &fixture.render_in(lang, Paper::A4, FactureLayout::Compact),
            lang,
        );
    }
}

/// The compact layout is a stylesheet, not a second document.
///
/// Everything below `<body>` is byte for byte what the standard layout
/// renders; only the rules above it differ. That is the whole claim of this
/// template, and asserting it directly is stronger than any list of fields a
/// test could name, because it catches a row dropped, a row invented, a row
/// moved and a label rewritten, in one comparison and without maintaining the
/// list.
///
/// A layout that has to move things about the page cannot say this and will
/// be held to the field-by-field comparison above instead. The compact A4 can
/// say it, so it is held to the stricter thing.
#[test]
fn the_compact_layout_changes_the_stylesheet_and_nothing_below_it() {
    for case in Case::ALL {
        let fixture = Fixture::of(case);
        for lang in Lang::ALL {
            let standard = fixture.render(lang, Paper::A4);
            let compact = fixture.render_in(lang, Paper::A4, FactureLayout::Compact);
            let body = |html: &str| {
                html.split_once("<body>")
                    .expect("the page has a body")
                    .1
                    .to_owned()
            };
            assert_eq!(body(&standard), body(&compact), "{case:?} {lang:?}");
            assert_ne!(
                standard, compact,
                "{case:?} {lang:?} the two layouts render the same file"
            );
        }
    }
}
