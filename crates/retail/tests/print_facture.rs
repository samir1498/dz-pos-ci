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
//! The fixtures and the parser are `common::facture`, shared with
//! `print_facture_compact.rs` and with nothing else. The ticket suite keeps
//! its own copy of the same parser: two golden suites over two templates
//! that check each other's files through one helper can both be made green
//! by editing the helper once.

mod common;

use chrono::NaiveDate;
use common::facture::*;
use dzpos_retail::lang::Lang;
use dzpos_retail::models::document::{BalanceTriple, PartyBlock};
use dzpos_retail::money::words::amount_in_words;
use dzpos_retail::money::{
    compute_totals, Bps, Line, Money, PaymentMode, Regime, TotalsOptions, TvaLine,
};
use dzpos_retail::print::strings::{shop_text, text, Key, ShopKey};
use dzpos_retail::print::{
    render_facture, render_facture_with, render_facture_with_reference, Cancellation, FactureInput,
    FactureLayout, Page, Paper,
};
use dzpos_retail::services::documents::{DocumentKind, DocumentStatus};

fn each_language_of(case: Case) {
    each_language_of_a_layout(case, FactureLayout::Standard);
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

#[test]
fn the_avoir_is_its_golden_in_every_language() {
    each_language_of(Case::Avoir);
}

#[test]
fn the_proforma_is_its_golden_in_every_language() {
    each_language_of(Case::Proforma);
}

#[test]
fn the_cancelled_facture_is_its_golden_in_every_language() {
    each_language_of(Case::Cancelled);
}

/// A5 is the same facture on a smaller sheet. One line of the page changes,
/// the one the OS print dialog reads, and the words, the amounts and the
/// blocks are the same bytes: two layouts kept in step by hand would drift
/// the first time one of them was edited.
#[test]
fn the_a5_facture_differs_from_the_a4_in_the_page_size_line_only() {
    for case in Case::ALL {
        let fixture = Fixture::of(case);
        for lang in Lang::ALL {
            let a4 = fixture.render(lang, Paper::A4);
            let a5 = fixture.render(lang, Paper::A5);
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

/// The IFU rule read off the paper: an IFU document must not
/// mention the tax at all (CTCA 2026 art. 64), so neither the word, nor its
/// abbreviations, nor "HT", nor "TTC", nor the per-cent sign of a rate
/// column is anywhere in the file. The réel facture is checked in the same
/// test, so a template that dropped them for everyone could not pass this.
#[test]
fn an_ifu_facture_names_no_tax_in_any_language() {
    let ifu = fixed_facture(Case::Ifu);
    let reel = fixed_facture(Case::Credit);
    for lang in Lang::ALL {
        let html = render_facture(&ifu, lang, Page::A4).unwrap();
        names_no_tax(&html, lang);
        assert!(html.contains(text(Key::Total, lang)), "{lang:?}");
        assert!(
            html.contains(shop_text(ShopKey::UnitPrice, lang)),
            "{lang:?}"
        );

        // The same basket under the réel says all of it.
        let reel_html = render_facture(&reel, lang, Page::A4).unwrap();
        assert_eq!(amounts(&reel_html, "tva").len(), 3, "0 %, 9 % and 19 %");
        assert_eq!(amounts(&reel_html, "tva-base").len(), 3, "{lang:?}");
        assert_eq!(amounts(&reel_html, "total-ttc").len(), 1, "{lang:?}");
        assert_eq!(rates(&reel_html, "tva").len(), 3, "{lang:?}");
        assert_eq!(rates(&reel_html, "line").len(), 3, "{lang:?}");
        assert!(reel_html.contains(text(Key::TotalHt, lang)), "{lang:?}");
        assert!(
            reel_html.contains(shop_text(ShopKey::UnitPriceHt, lang)),
            "{lang:?}"
        );
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

    let company = render_facture(&fixed_facture(Case::Credit), Lang::Fr, Page::A4).unwrap();
    for identifier in identifiers {
        assert!(company.contains(identifier), "{identifier}");
    }

    let consumer = render_facture(&doc, Lang::Fr, Page::A4).unwrap();
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
    let still_a_consumer = render_facture(&doc, Lang::Fr, Page::A4).unwrap();
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

    let html = render_facture(&doc, Lang::Fr, Page::A4).unwrap();
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
    let html = render_facture(&doc, Lang::Fr, Page::A4).unwrap();
    assert!(amounts(&html, "line-discount").is_empty());
    assert_eq!(
        html.matches(shop_text(ShopKey::Discount, Lang::Fr)).count(),
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
    let html = render_facture(&doc, Lang::Fr, Page::A4).unwrap();
    assert_eq!(optional_amount(&html, "old-balance"), Some(-25_000));
    the_golden_says_what_the_document_stores(&html, &doc, doc.balance, Lang::Fr);
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
        let err = render_facture(&doc, Lang::Fr, Page::A4).unwrap_err();
        assert_eq!(err.code(), "print", "{kind:?}: {err:?}");
    }
    for kind in [
        DocumentKind::Facture,
        DocumentKind::Avoir,
        DocumentKind::Proforma,
    ] {
        let mut doc = fixed_facture(Case::Credit);
        doc.kind = kind;
        // A proforma creates no debt and this template refuses one that
        // carries a triple; the title is what is under test here.
        if kind == DocumentKind::Proforma {
            doc.balance = None;
        }
        let html = render_facture(&doc, Lang::Fr, Page::A4).unwrap();
        let title = match kind {
            DocumentKind::Avoir => shop_text(ShopKey::Avoir, Lang::Fr),
            DocumentKind::Proforma => shop_text(ShopKey::Proforma, Lang::Fr),
            _ => text(Key::Facture, Lang::Fr),
        };
        assert!(html.contains(title), "{kind:?}");
        assert!(
            html.contains(&format!("{}-2026-000042", kind.number_prefix())),
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
        let html = render_facture(&doc, lang, Page::A4).unwrap();
        assert!(html.contains(text(Key::FactureCancelled, lang)), "{lang:?}");
        assert!(html.contains("FA-2026-000042"), "{lang:?}");
    }
    // Nothing cancels an avoir or a proforma and the wording for it
    // is not written, so the printer refuses rather than invent one.
    for kind in [DocumentKind::Avoir, DocumentKind::Proforma] {
        doc.kind = kind;
        let err = render_facture(&doc, Lang::Fr, Page::A4).unwrap_err();
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
            let err = render_facture(&doc, lang, Page::A4).unwrap_err();
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
        let err = render_facture(&doc, lang, Page::A4).unwrap_err();
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
    let avoir = fixed_facture(Case::Avoir);

    for lang in Lang::ALL {
        let html = render_facture_with_reference(&avoir, Some(&facture), lang, Page::A4).unwrap();
        assert!(html.contains("AV-2026-000003"), "{lang:?}");
        assert!(html.contains("FA-2026-000042"), "{lang:?}");
        assert!(
            html.contains(shop_text(ShopKey::AvoirOnFacture, lang)),
            "{lang:?}"
        );
    }

    // No reference handed over, a reference that is another document, and a
    // reference from another shop: each is a facture number this avoir
    // cannot print, and each is refused.
    assert_eq!(
        render_facture(&avoir, Lang::Fr, Page::A4)
            .unwrap_err()
            .code(),
        "print"
    );
    let mut other = fixed_facture(Case::Credit);
    other.id = 9;
    assert_eq!(
        render_facture_with_reference(&avoir, Some(&other), Lang::Fr, Page::A4)
            .unwrap_err()
            .code(),
        "print"
    );
    let mut another_shop = fixed_facture(Case::Credit);
    another_shop.shop_id = SHOP + 1;
    assert_eq!(
        render_facture_with_reference(&avoir, Some(&another_shop), Lang::Fr, Page::A4)
            .unwrap_err()
            .code(),
        "print"
    );

    // The line says "avoir sur facture", so the paper it names has to be one.
    // A reference that is a ticket, an avoir or a proforma is refused rather
    // than named as a facture on a page a comptable reads.
    for kind in [
        DocumentKind::Ticket,
        DocumentKind::Avoir,
        DocumentKind::Proforma,
    ] {
        let mut not_a_facture = fixed_facture(Case::Credit);
        not_a_facture.kind = kind;
        assert_eq!(
            render_facture_with_reference(&avoir, Some(&not_a_facture), Lang::Fr, Page::A4)
                .unwrap_err()
                .code(),
            "print",
            "{kind:?}"
        );
    }

    // A facture that references nothing prints with no reference row.
    let plain = render_facture(&facture, Lang::Fr, Page::A4).unwrap();
    assert!(!plain.contains(shop_text(ShopKey::AvoirOnFacture, Lang::Fr)));
}

/// A name the shop typed is printed and never run: the ampersand and the
/// angle brackets of a product name come out as entities, so a name can
/// neither close a tag nor open one. The golden carries the escaped form,
/// and this test says which form that is.
#[test]
fn a_product_name_with_markup_in_it_is_escaped_and_not_rendered() {
    for case in Case::ALL {
        let fixture = Fixture::of(case);
        for lang in Lang::ALL {
            let html = fixture.render(lang, Paper::A4);
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
    let ar = render_facture(&doc, Lang::Ar, Page::A4).unwrap();
    assert!(ar.contains("dir=\"rtl\""), "the Arabic facture is not RTL");
    assert!(ar.contains("lang=\"ar\""));
    assert!(
        ar.contains("unreviewed by a native speaker, like words_ar"),
        "the Arabic facture does not disclose that its wording is unreviewed"
    );
    for other in [Lang::Fr, Lang::En] {
        let html = render_facture(&doc, other, Page::A4).unwrap();
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
        let fixture = Fixture::of(case);
        for lang in Lang::ALL {
            let html = fixture.render(lang, Paper::A4);
            assert!(
                !html.chars().any(|c| ('\u{0660}'..='\u{0669}').contains(&c)),
                "{lang:?} {case:?} carries Arabic-Indic digits"
            );
            assert_eq!(
                centimes(&one_amount(&html, "net-to-pay")),
                fixture.doc.totals.net_to_pay.as_centimes(),
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
        let html = render_facture(&cash, lang, Page::A4).unwrap();
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

/// The avoir says which facture it is written against and when that facture
/// was issued: "Avoir sur facture FA-2026-000042 du 09/09/2026". The day is the
/// referenced facture's and not the avoir's own, which is three days later
/// in the fixture, so a line built from the wrong document is red rather
/// than plausible.
#[test]
fn the_avoir_names_the_facture_it_is_written_against_and_the_day_of_it() {
    let fixture = Fixture::of(Case::Avoir);
    for lang in Lang::ALL {
        let html = fixture.render(lang, Paper::A4);
        let line = reference_line(&html);
        assert!(
            line.contains(shop_text(ShopKey::AvoirOnFacture, lang)),
            "{lang:?}: {line}"
        );
        assert!(line.contains("FA-2026-000042"), "{lang:?}: {line}");
        assert!(line.contains(text(Key::IssuedOn, lang)), "{lang:?}: {line}");
        assert!(line.contains("09/09/2026"), "{lang:?}: {line}");
        assert!(
            !line.contains("12/09/2026"),
            "{lang:?} dated the reference by the avoir: {line}"
        );
        // The avoir's own date is on the page all the same, under its own
        // number, where every document carries it.
        assert!(html.contains("12/09/2026"), "{lang:?}");
        assert!(html.contains("AV-2026-000003"), "{lang:?}");
    }
}

/// A shop that credits a December facture in January prints two years on one
/// page: the avoir's own on its number and the facture's on the reference
/// line. The number a customer quotes belongs to the paper it names
/// (features.md §4, Numbering), so a reference spelled with the year the
/// reprint is happening in would send them looking for a facture that does
/// not exist.
#[test]
fn an_avoir_prints_the_year_of_the_facture_it_credits_and_not_its_own() {
    let mut facture = fixed_facture(Case::Credit);
    facture.series_year = 2025;
    facture.series = DocumentKind::Facture.series_of_year(2025);
    facture.issued_at = NaiveDate::from_ymd_opt(2025, 12, 31)
        .unwrap()
        .and_hms_opt(23, 30, 0)
        .unwrap();
    let avoir = fixed_facture(Case::Avoir);
    assert_eq!(
        avoir.series_year, 2026,
        "the avoir is the one of the new year"
    );

    for lang in Lang::ALL {
        let html = render_facture_with_reference(&avoir, Some(&facture), lang, Page::A4).unwrap();
        let line = reference_line(&html);
        assert!(
            line.contains("FA-2025-000042"),
            "{lang:?} credited a facture of the wrong year: {line}"
        );
        assert!(!line.contains("FA-2026-000042"), "{lang:?}: {line}");
        assert!(html.contains("AV-2026-000003"), "{lang:?}");
    }
}

/// An avoir hands the lines back and asks for nothing, so it carries no
/// droit de timbre (the stamp is zero on an avoir) and its totals block
/// has no stamp row. A stored avoir that carries one contradicts the rule
/// that wrote it, and the honest answer is the one the IFU recap gets:
/// refuse, rather than drop a row and hand over a total whose parts do not
/// add up.
#[test]
fn an_avoir_prints_no_stamp_and_one_that_carries_a_stamp_is_refused() {
    let fixture = Fixture::of(Case::Avoir);
    assert_eq!(
        fixture.doc.totals.stamp,
        Money::ZERO,
        "the avoir fixture was built with a stamp"
    );
    for lang in Lang::ALL {
        let html = fixture.render(lang, Paper::A4);
        assert!(
            amounts(&html, "stamp").is_empty(),
            "{lang:?} avoir carries a stamp row"
        );
        assert!(!html.contains(text(Key::Stamp, lang)), "{lang:?}");
    }

    let mut stamped = fixed_facture(Case::Avoir);
    stamped.totals.stamp = Money::centimes(100);
    let facture = fixed_facture(Case::Credit);
    let err =
        render_facture_with_reference(&stamped, Some(&facture), Lang::Fr, Page::A4).unwrap_err();
    assert_eq!(err.code(), "print", "{err:?}");

    // The same stamp on the cash facture is printed, so the refusal is the
    // kind doing it and not the template having lost the row.
    let cash = fixed_facture(Case::Cash);
    assert_ne!(cash.totals.stamp, Money::ZERO);
    let html = render_facture(&cash, Lang::Fr, Page::A4).unwrap();
    assert_eq!(
        optional_amount(&html, "stamp"),
        Some(cash.totals.stamp.as_centimes())
    );
}

/// A customer handed back more than they owed is owed money, and the block
/// says credit where it would otherwise say what is due. The label follows
/// the sign of the closing figure and not the kind of the document: an avoir
/// that only cuts a debt down leaves a debt, and calling that a credit would
/// tell the customer they were owed money they are not.
#[test]
fn the_balance_block_says_credit_when_the_avoir_closes_below_zero() {
    let fixture = Fixture::of(Case::Avoir);
    let triple = fixture.doc.balance.expect("the avoir carries a triple");
    assert!(
        triple.total_debt.as_centimes() < 0,
        "the avoir fixture does not close below zero: {triple:?}"
    );
    for lang in Lang::ALL {
        let html = fixture.render(lang, Paper::A4);
        assert!(html.contains(text(Key::TotalCredit, lang)), "{lang:?}");
        assert!(
            !html.contains(shop_text(ShopKey::TotalDebt, lang)),
            "{lang:?}"
        );
        // The amount keeps the sign the document stores, the way the
        // statement's closing balance does: the label says which way it
        // points and the figure is read back against the row.
        assert_eq!(
            optional_amount(&html, "total-debt"),
            Some(triple.total_debt.as_centimes()),
            "{lang:?}"
        );
    }

    // The same avoir against a larger old balance still leaves a debt, and
    // the block says so.
    let mut smaller = fixed_facture(Case::Avoir);
    let old_balance = Money::centimes(500_000);
    let this = Money::ZERO.checked_sub(smaller.totals.net_to_pay).unwrap();
    smaller.balance = Some(BalanceTriple {
        old_balance,
        remaining_debt: this,
        total_debt: old_balance.checked_add(this).unwrap(),
    });
    let facture = fixed_facture(Case::Credit);
    for lang in Lang::ALL {
        let html = render_facture_with_reference(&smaller, Some(&facture), lang, Page::A4).unwrap();
        assert!(
            html.contains(shop_text(ShopKey::TotalDebt, lang)),
            "{lang:?}"
        );
        assert!(!html.contains(text(Key::TotalCredit, lang)), "{lang:?}");
    }
}

/// The words line names the document it closes. An avoir that said "la
/// présente facture" would be a wording a comptable reads as being about
/// another paper, so each kind closes itself, the way the statement does.
#[test]
fn each_kind_closes_itself_in_its_own_words() {
    let avoir = Fixture::of(Case::Avoir);
    for lang in Lang::ALL {
        let html = avoir.render(lang, Paper::A4);
        assert!(
            html.contains(shop_text(ShopKey::AvoirInWords, lang)),
            "{lang:?}"
        );
        assert!(!html.contains(text(Key::InWords, lang)), "{lang:?}");
        assert_eq!(
            in_words(&html),
            amount_in_words(avoir.doc.totals.net_to_pay, lang).unwrap(),
            "{lang:?}"
        );
    }
    let facture = Fixture::of(Case::Credit);
    for lang in Lang::ALL {
        let html = facture.render(lang, Paper::A4);
        assert!(html.contains(text(Key::InWords, lang)), "{lang:?}");
        assert!(
            !html.contains(shop_text(ShopKey::AvoirInWords, lang)),
            "{lang:?}"
        );
    }
}

/// The avoir's own lines and its own totals, never the facture's. The
/// fixture takes back two lines of three and one of them by half, so a page
/// that read the referenced facture for its amounts would print figures this
/// test names as wrong.
#[test]
fn the_avoir_prints_its_own_lines_and_not_the_ones_it_references() {
    let fixture = Fixture::of(Case::Avoir);
    let facture = fixed_facture(Case::Credit);
    assert_ne!(
        fixture.doc.totals.net_to_pay, facture.totals.net_to_pay,
        "the avoir fixture asks for the same amount as its facture"
    );
    for lang in Lang::ALL {
        let html = fixture.render(lang, Paper::A4);
        assert_eq!(amounts(&html, "line").len(), 2, "{lang:?}");
        assert_eq!(
            centimes(&one_amount(&html, "net-to-pay")),
            fixture.doc.totals.net_to_pay.as_centimes(),
            "{lang:?}"
        );
        assert_ne!(
            centimes(&one_amount(&html, "net-to-pay")),
            facture.totals.net_to_pay.as_centimes(),
            "{lang:?}"
        );
    }
}

/// Only an avoir is written against another document. A facture or a
/// proforma carrying a reference would print "avoir sur facture" over a page
/// that is not one, so the printer refuses it rather than label it.
#[test]
fn a_document_that_is_not_an_avoir_may_not_reference_a_facture() {
    let facture = fixed_facture(Case::Credit);
    for kind in [DocumentKind::Facture, DocumentKind::Proforma] {
        let mut doc = fixed_facture(Case::Credit);
        doc.kind = kind;
        doc.id = 2;
        doc.ref_document_id = Some(facture.id);
        if kind == DocumentKind::Proforma {
            doc.balance = None;
        }
        let err =
            render_facture_with_reference(&doc, Some(&facture), Lang::Fr, Page::A4).unwrap_err();
        assert_eq!(err.code(), "print", "{kind:?}: {err:?}");
    }
}

/// A proforma is a quote on facture paper. It burns its own number, moves
/// no stock and creates no debt, and the page has to say so: a
/// customer handed one must not file it as a facture, and a comptable
/// reading it must not book it. So it carries a wording of its own, and no
/// balance block at all.
#[test]
fn a_proforma_says_it_is_not_a_facture_and_carries_no_balance_block() {
    let fixture = Fixture::of(Case::Proforma);
    // The document stores a triple, all three of it zero, which is what the
    // proforma rule writes. The page dropping the block is the rule doing
    // it and not the fixture having nothing to print.
    let triple = fixture
        .doc
        .balance
        .expect("the proforma fixture stores no triple");
    assert_eq!(triple.old_balance, Money::ZERO);
    assert_eq!(triple.remaining_debt, Money::ZERO);
    assert_eq!(triple.total_debt, Money::ZERO);

    for lang in Lang::ALL {
        let html = fixture.render(lang, Paper::A4);
        assert!(
            html.contains(shop_text(ShopKey::ProformaNotice, lang)),
            "{lang:?}"
        );
        assert_eq!(
            heading(&html),
            shop_text(ShopKey::Proforma, lang),
            "{lang:?}"
        );
        assert!(html.contains("PF-2026-000005"), "{lang:?}");
        for absent in ["old-balance", "this-document", "total-debt"] {
            assert!(
                amounts(&html, absent).is_empty(),
                "the {lang:?} proforma carries a {absent} row"
            );
        }
        assert!(!html.contains(text(Key::Balance, lang)), "{lang:?}");
        assert!(
            !html.contains(shop_text(ShopKey::TotalDebt, lang)),
            "{lang:?}"
        );

        // The credit facture, the same basket, prints all three: the block
        // is gone for the proforma and not gone for everyone.
        let facture = Fixture::of(Case::Credit).render(lang, Paper::A4);
        assert_eq!(amounts(&facture, "total-debt").len(), 1, "{lang:?}");
        assert!(
            !facture.contains(shop_text(ShopKey::ProformaNotice, lang)),
            "{lang:?}"
        );
    }
}

/// A mode of payment is how a document was paid, and neither of these two
/// is. An avoir hands money back, so "Crédit" under a heading saying
/// "payment mode" tells the customer they owe what the page is giving them
/// (and the Arabic word is "debt" outright); a proforma is a quote whose own
/// notice says it settles nothing, so a mode of payment on it contradicts
/// the line above. Both drop the block; the facture keeps it.
#[test]
fn neither_an_avoir_nor_a_proforma_prints_how_it_was_paid() {
    for case in [Case::Avoir, Case::Proforma] {
        let fixture = Fixture::of(case);
        for lang in Lang::ALL {
            let html = fixture.render(lang, Paper::A4);
            assert!(
                !html.contains(text(Key::PaymentMode, lang)),
                "{lang:?} {case:?}: the page says how it was paid"
            );
        }
    }

    // Counted rather than read, so a block that lost its heading and kept
    // its word is caught too: a facture prints the mode and the balance, an
    // avoir the balance alone, a proforma neither.
    let blocks = |html: &str| html.matches("<div class=\"block\">").count();
    for lang in Lang::ALL {
        assert_eq!(
            blocks(&Fixture::of(Case::Credit).render(lang, Paper::A4)),
            2
        );
        assert_eq!(blocks(&Fixture::of(Case::Avoir).render(lang, Paper::A4)), 1);
        assert_eq!(
            blocks(&Fixture::of(Case::Proforma).render(lang, Paper::A4)),
            0
        );
    }
}

/// The last row of the totals says what the figure is for. On a facture it
/// is the net to pay, the amount the buyer owes; an avoir asks for nothing,
/// so the same row names the amount of the avoir instead. The figure does
/// not move, only the words beside it.
#[test]
fn the_avoir_names_its_last_row_as_its_own_amount_and_not_as_a_net_to_pay() {
    let avoir = Fixture::of(Case::Avoir);
    let facture = Fixture::of(Case::Credit);
    let proforma = Fixture::of(Case::Proforma);
    for lang in Lang::ALL {
        let html = avoir.render(lang, Paper::A4);
        assert!(
            html.contains(shop_text(ShopKey::AvoirAmount, lang)),
            "{lang:?}"
        );
        assert!(
            !html.contains(text(Key::NetToPay, lang)),
            "{lang:?}: the avoir asks the buyer for a net to pay"
        );
        // The amount itself is untouched: the row is relabelled and not
        // recomputed.
        assert_eq!(
            centimes(&one_amount(&html, "net-to-pay")),
            avoir.doc.totals.net_to_pay.as_centimes(),
            "{lang:?}"
        );

        // The two documents that do ask for money keep the words for it.
        for other in [&facture, &proforma] {
            let html = other.render(lang, Paper::A4);
            assert!(html.contains(text(Key::NetToPay, lang)), "{lang:?}");
            assert!(
                !html.contains(shop_text(ShopKey::AvoirAmount, lang)),
                "{lang:?}"
            );
        }
    }
}

/// A proforma creates no debt, so a stored one carrying a triple that is not
/// three zeroes contradicts the rule that wrote it. Dropping the block would
/// hide the contradiction and printing it would say a quote moved a debt, so
/// the page is refused the way an IFU document carrying a TVA recap is.
#[test]
fn a_proforma_carrying_a_debt_is_refused_not_quietly_stripped() {
    let zero = BalanceTriple {
        old_balance: Money::ZERO,
        remaining_debt: Money::ZERO,
        total_debt: Money::ZERO,
    };
    // Three zeroes is the triple a proforma stores, and it prints.
    let mut doc = fixed_facture(Case::Proforma);
    doc.balance = Some(zero);
    assert!(render_facture(&doc, Lang::Fr, Page::A4).is_ok());

    // One field at a time, so a check that read the closing balance alone
    // would let a proforma carrying an old balance through, and one that
    // read the document's own row would let the two others through. Each
    // triple below is a ledger a proforma cannot have touched.
    let debt = Money::centimes(150_000);
    let sub_cases = [
        BalanceTriple {
            old_balance: debt,
            ..zero
        },
        BalanceTriple {
            remaining_debt: debt,
            ..zero
        },
        BalanceTriple {
            total_debt: debt,
            ..zero
        },
    ];
    for triple in sub_cases {
        let mut doc = fixed_facture(Case::Proforma);
        doc.balance = Some(triple);
        for lang in Lang::ALL {
            let err = render_facture(&doc, lang, Page::A4)
                .err()
                .unwrap_or_else(|| panic!("{lang:?} {triple:?} printed a proforma with a debt"));
            assert_eq!(err.code(), "print", "{lang:?} {triple:?}: {err:?}");
        }
    }
}

/// The words line names the paper it closes here too: a proforma that said
/// "la présente facture" would be the one wording on the page contradicting
/// the notice above it.
#[test]
fn a_proforma_closes_itself_in_its_own_words() {
    let fixture = Fixture::of(Case::Proforma);
    for lang in Lang::ALL {
        let html = fixture.render(lang, Paper::A4);
        assert!(
            html.contains(shop_text(ShopKey::ProformaInWords, lang)),
            "{lang:?}"
        );
        assert!(!html.contains(text(Key::InWords, lang)), "{lang:?}");
        assert!(
            !html.contains(shop_text(ShopKey::AvoirInWords, lang)),
            "{lang:?}"
        );
        assert_eq!(
            in_words(&html),
            amount_in_words(fixture.doc.totals.net_to_pay, lang).unwrap(),
            "{lang:?}"
        );
    }
}

/// The reprint of a cancelled facture is the same document. It keeps its
/// number and its money to the centime (features.md, Numbering row) and says
/// on its face that it was cancelled, so what separates it from the live
/// page is the heading, the mark and the line naming the day and the reason,
/// and nothing else. A reprint that also moved an amount would be a second
/// version of a document the shop already handed over.
#[test]
fn a_cancelled_reprint_differs_from_the_live_facture_in_the_cancellation_only() {
    let cancelled = Fixture::of(Case::Cancelled);
    let live = Fixture::of(Case::Credit);
    for lang in Lang::ALL {
        let after = cancelled.render(lang, Paper::A4);
        let before = live.render(lang, Paper::A4);

        // Everything from the lines table to the words line, which is the
        // whole of the money on this page, byte for byte.
        assert_eq!(
            money_block(&after),
            money_block(&before),
            "{lang:?}: the cancelled reprint moved an amount"
        );

        let after_lines: Vec<&str> = after.lines().collect();
        let before_lines: Vec<&str> = before.lines().collect();
        // What the cancelled page adds: the mark, the line under the number
        // and the heading that says the document is cancelled.
        let added: Vec<&str> = after_lines
            .iter()
            .filter(|line| !before_lines.contains(line))
            .copied()
            .collect();
        assert!(!added.is_empty(), "{lang:?}: nothing says it was cancelled");
        // One of them is the mark. The word alone is not enough: it is a
        // substring of the cancelled heading in all three languages, so a
        // page that had lost the mark and kept the heading would satisfy
        // every other check in this test.
        assert!(
            added.iter().any(|line| line.contains("class=\"annulee\"")),
            "{lang:?}: the reprint carries no mark across its face"
        );
        for line in &added {
            assert!(
                line.contains("class=\"annulee\"")
                    || line.contains("class=\"cancelled\"")
                    || line.contains(text(Key::FactureCancelled, lang)),
                "{lang:?}: {line} is neither the heading nor the cancellation"
            );
        }
        // And what it drops, which has to be that same heading and nothing
        // else: a reprint that lost the payment mode or the signature block
        // would pass a check that only read the lines it added.
        let dropped: Vec<&str> = before_lines
            .iter()
            .filter(|line| !after_lines.contains(line))
            .copied()
            .collect();
        for line in &dropped {
            assert!(
                line.contains(text(Key::Facture, lang)),
                "{lang:?}: the cancelled reprint dropped {line}"
            );
        }
        // The mark, the day, the reason, and the number the document keeps.
        assert!(after.contains(text(Key::CancelledMark, lang)), "{lang:?}");
        assert!(after.contains(text(Key::CancelledOn, lang)), "{lang:?}");
        assert!(after.contains("12/09/2026"), "{lang:?}");
        assert!(after.contains(text(Key::CancelReason, lang)), "{lang:?}");
        assert!(after.contains("FA-2026-000042"), "{lang:?}");
        // A reason typed by the shop is printed and never run, like a
        // product name.
        assert!(
            after.contains("Erreur de saisie &#60;quantité&#62; &#38; prix"),
            "{lang:?} did not escape the reason"
        );
        assert!(!after.contains("<quantité>"), "{lang:?}");

        // The live facture carries none of it, so the mark is the status
        // doing it and not the template printing it for everyone.
        assert!(!before.contains(text(Key::CancelledMark, lang)), "{lang:?}");
        assert!(!before.contains(text(Key::CancelledOn, lang)), "{lang:?}");
    }
}

/// The mark follows the document's status and the line under the number
/// follows what the caller read with it. A cancelled facture printed by a
/// caller that has no cancellation columns to hand still says it is
/// cancelled: the alternative is a void document that looks live.
#[test]
fn a_cancelled_facture_carries_the_mark_even_with_no_cancellation_read_with_it() {
    let doc = fixed_facture(Case::Cancelled);
    for lang in Lang::ALL {
        let html = render_facture(&doc, lang, Page::A4).unwrap();
        // In the mark's own block, not merely somewhere on a page whose
        // heading is "FACTURE ANNULÉE" and carries the word already.
        let mark = mark_block(&html).unwrap_or_else(|| panic!("{lang:?}: no mark block"));
        assert!(mark.contains(text(Key::CancelledMark, lang)), "{lang:?}");
        // And drawn across the page rather than printed in a corner: the
        // rotation is what a reader sees from the other side of a counter.
        assert!(mark_rule(&html).contains("rotate("), "{lang:?}");
        assert!(html.contains(text(Key::FactureCancelled, lang)), "{lang:?}");
        assert!(!html.contains(text(Key::CancelledOn, lang)), "{lang:?}");
        assert!(!html.contains(text(Key::CancelReason, lang)), "{lang:?}");
    }
}

/// A day and a reason belong to a document the shop cancelled. Printing them
/// over a live facture would hand a customer a page saying it is void while
/// the ledger still counts it, so the caller that hands them over for a live
/// document is refused rather than obeyed.
#[test]
fn a_cancellation_printed_over_a_live_facture_is_refused() {
    let live = fixed_facture(Case::Credit);
    let input = FactureInput {
        referenced: None,
        cancellation: Some(Cancellation {
            at: at(CANCELLED_AT),
            reason: CANCEL_REASON,
        }),
    };
    for lang in Lang::ALL {
        let err = render_facture_with(&live, &input, lang, Page::A4).unwrap_err();
        assert_eq!(err.code(), "print", "{lang:?}: {err:?}");
    }
    // The same input over the same document once it is cancelled is the page
    // this test is about, and it renders.
    let cancelled = fixed_facture(Case::Cancelled);
    assert!(render_facture_with(&cancelled, &input, Lang::Fr, Page::A4).is_ok());
}
