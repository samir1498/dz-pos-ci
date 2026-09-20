//! How a stored document becomes the view every facture layout draws.
//!
//! The split is by job and not by size: `facture.rs` owns the shapes the
//! templates read and which template a layout picks, this file owns turning
//! a `Document` into one of those shapes. Every rule about what reaches the
//! paper lives here, so a layout added next door cannot quietly change what
//! a facture says.

use super::facture::*;
use crate::error::CoreError;
use crate::lang::Lang;
use crate::models::document::{
    BalanceTriple, Document, DocumentKind, DocumentLine, DocumentStatus, PartyBlock, PartyKind,
    SellerBlock,
};
use crate::money::format::{format_centimes, format_qty};
use crate::money::words::amount_in_words;
use crate::money::{Money, Regime};
use crate::print::layout::Paper;
use crate::print::strings::{text, Key};
use crate::print::{number, payment_mode_key, percent, some_amount};

/// The printed number of the facture an avoir is written against, when the
/// document names one. A document that points at a facture the caller did
/// not hand over is refused: the reference is a legal mention of the avoir
/// (décret 05-468 art. 3), so printing the page without it would drop a
/// field and say nothing about it.
pub(super) fn reference(
    doc: &Document,
    referenced: Option<&Document>,
    lang: Lang,
) -> Result<Option<ReferenceView>, CoreError> {
    match (doc.ref_document_id, referenced) {
        (None, _) => Ok(None),
        (Some(id), Some(referenced))
            if referenced.id == id && referenced.shop_id == doc.shop_id =>
        {
            // The sentence says "avoir sur facture", so the paper it names
            // has to be one. An avoir written against a ticket is not a
            // document this template can describe.
            if referenced.kind != DocumentKind::Facture {
                return Err(CoreError::render(
                    "an avoir is written against a facture and this reference is another kind",
                ));
            }
            Ok(Some(ReferenceView {
                label: text(Key::AvoirOnFacture, lang),
                number: number(referenced),
                on_label: text(Key::IssuedOn, lang),
                // The referenced facture's own day, which is not the
                // avoir's: an avoir is written after the sale it corrects.
                issued_at: referenced.issued_at.format(DATE_FORMAT).to_string(),
            }))
        }
        (Some(_), Some(_)) => Err(CoreError::render(
            "the document handed over as the reference is not the one this document names",
        )),
        (Some(_), None) => Err(CoreError::render(
            "the document names a facture whose number was not read with it",
        )),
    }
}

pub(super) fn view(
    doc: &Document,
    buyer: &PartyBlock,
    reference: Option<ReferenceView>,
    cancellation: Option<Cancellation<'_>>,
    lang: Lang,
    paper: Paper,
) -> Result<FactureView, CoreError> {
    let reel = doc.regime == Regime::Reel;
    let totals = &doc.totals;
    // The words line prints the net to pay, the amount this document asks
    // the buyer for, and not the total TTC: décret 05-468 art. 3 asks for
    // the total "en chiffres et en lettres", and the figure the buyer pays
    // carries the droit de timbre. The two are the same number on every
    // document with no stamp, which is why the cash golden is the one that
    // proves it.
    let in_words = amount_in_words(totals.net_to_pay, lang).map_err(|_| {
        CoreError::render("the net to pay has no written form in the print language")
    })?;
    // A row that repeats the row above it is noise: with no global
    // discount the subtotal HT is the total HT, printed twice.
    let discounted = totals.discount != Money::ZERO;

    Ok(FactureView {
        lang_tag: lang.tag(),
        dir: lang.dir(),
        arabic_unreviewed: lang == Lang::Ar,
        paper_size: paper.css_size(),
        title: title(doc, lang),
        number: number(doc),
        issued_at: doc.issued_at.format(DATE_FORMAT).to_string(),
        reference,
        cancelled_mark: (doc.status == DocumentStatus::Cancelled)
            .then(|| text(Key::CancelledMark, lang)),
        cancellation: cancellation.map(|c| CancellationView {
            on_label: text(Key::CancelledOn, lang),
            at: c.at.format(DATE_FORMAT).to_string(),
            reason_label: text(Key::CancelReason, lang),
            reason: c.reason.to_owned(),
        }),
        notice: (doc.kind == DocumentKind::Proforma).then(|| text(Key::ProformaNotice, lang)),
        seller: seller_view(&doc.seller, lang),
        buyer: buyer_view(buyer, lang),
        designation_label: text(Key::Designation, lang),
        qty_label: text(Key::Qty, lang),
        // Under the IFU a price is one price: "HT" would name a tax the
        // document must not mention (CTCA 2026 art. 64), so the column
        // keeps the amounts and changes the word.
        unit_price_label: text(
            if reel {
                Key::UnitPriceHt
            } else {
                Key::UnitPrice
            },
            lang,
        ),
        rate_label: text(Key::Tva, lang),
        line_discount_label: text(Key::Discount, lang),
        line_total_label: text(
            if reel {
                Key::LineTotalHt
            } else {
                Key::LineTotal
            },
            lang,
        ),
        show_rate: reel,
        // The column is there when a line uses it. A facture with no line
        // discount anywhere prints five columns, not six with a blank one.
        show_line_discount: doc.lines.iter().any(|l| l.line_discount != Money::ZERO),
        lines: doc.lines.iter().map(|l| line_view(l, reel)).collect(),
        total_label: text(if reel { Key::TotalHt } else { Key::Total }, lang),
        total_amount: format_centimes(totals.total_ht),
        discount_label: text(Key::Discount, lang),
        discount: some_amount(totals.discount),
        subtotal_label: text(if reel { Key::SubtotalHt } else { Key::Subtotal }, lang),
        subtotal: discounted.then(|| format_centimes(totals.subtotal_ht)),
        // Empty under the IFU because the document stores no recap there,
        // not because the template hides one.
        tva_rows: totals
            .tva_by_rate
            .iter()
            .map(|row| TvaRow {
                label: text(Key::Tva, lang),
                rate: percent(row.rate),
                base_label: text(Key::TvaBase, lang),
                base: format_centimes(row.base),
                amount: format_centimes(row.amount),
            })
            .collect(),
        total_ttc_label: text(Key::TotalTtc, lang),
        // "TTC" names the tax as loudly as "TVA" does, and under the IFU
        // the row would repeat the subtotal anyway: there is no tax between
        // the two.
        total_ttc: reel.then(|| format_centimes(totals.total_ttc)),
        stamp_label: text(Key::Stamp, lang),
        stamp: some_amount(totals.stamp),
        // The row a facture closes on is the net to pay. An avoir asks for
        // nothing, so the same row names the amount it hands back: the
        // figure is untouched and only the words change.
        net_to_pay_label: text(
            match doc.kind {
                DocumentKind::Avoir => Key::AvoirAmount,
                _ => Key::NetToPay,
            },
            lang,
        ),
        net_to_pay: format_centimes(totals.net_to_pay),
        in_words_label: text(in_words_key(doc.kind), lang),
        in_words,
        // A proforma settles nothing and shows no ledger: it stores a triple
        // of zeroes and printing it would be a debt of nothing said
        // three times under a heading that says "solde".
        balance: match doc.kind {
            DocumentKind::Proforma => None,
            _ => doc.balance.map(|b| balance_view(b, lang)),
        },
        // How it was paid, where the question means anything. On an avoir
        // the stored mode is the one the facture was sold under, and
        // printing "crédit" over a document handing money back tells the
        // buyer they owe it ("دين", debt, in the Arabic). A proforma is a
        // quote whose own notice says it settles nothing, and a mode of
        // payment on it contradicts the line above.
        payment_mode: match doc.kind {
            DocumentKind::Avoir | DocumentKind::Proforma => None,
            _ => Some(PaymentModeView {
                title: text(Key::PaymentMode, lang),
                mode: text(payment_mode_key(doc.payment_mode), lang),
            }),
        },
        show_blocks: !matches!(doc.kind, DocumentKind::Proforma),
        cachet_label: text(Key::Cachet, lang),
        seller_label: text(Key::Seller, lang),
        buyer_label: text(Key::Buyer, lang),
        currency: text(Key::Currency, lang),
    })
}

/// The heading. A cancelled facture keeps its number and says on its face
/// that it was cancelled (features.md, Numbering row), so the reprint of a
/// cancelled document can never be mistaken for the live one.
pub(super) const fn title(doc: &Document, lang: Lang) -> &'static str {
    let key = match (doc.kind, doc.status) {
        (DocumentKind::Facture, DocumentStatus::Cancelled) => Key::FactureCancelled,
        (DocumentKind::Avoir, _) => Key::Avoir,
        (DocumentKind::Proforma, _) => Key::Proforma,
        _ => Key::Facture,
    };
    text(key, lang)
}

/// Whether a stored triple says anything at all. Three zeroes is what a
/// document that touched no ledger carries, and it is not a debt of nothing:
/// it is the ledger saying it was never asked.
pub(crate) const fn carries_a_debt(balance: BalanceTriple) -> bool {
    !(balance.old_balance.as_centimes() == 0
        && balance.remaining_debt.as_centimes() == 0
        && balance.total_debt.as_centimes() == 0)
}

/// The opening of the words line, which names the paper it closes. Décret
/// 05-468 art. 3 asks the total to be written out; the sentence around it
/// says which document was closed at that sum, and an avoir saying "la
/// présente facture" would name the wrong one. The statement already has
/// its own for the same reason.
pub(super) const fn in_words_key(kind: DocumentKind) -> Key {
    match kind {
        DocumentKind::Avoir => Key::AvoirInWords,
        DocumentKind::Proforma => Key::ProformaInWords,
        _ => Key::InWords,
    }
}

pub(super) fn ids(rows: [(&'static str, Option<&String>); 4]) -> Vec<IdRow> {
    rows.into_iter()
        .filter_map(|(label, value)| {
            value.map(|value| IdRow {
                label,
                value: value.clone(),
            })
        })
        .collect()
}

/// The seller as décret 05-468 art. 3 asks for them: the name, the
/// identifiers the shop has, the address and the phone. An identifier the
/// document did not snapshot prints no empty row.
pub(super) fn seller_view(seller: &SellerBlock, lang: Lang) -> PartyView {
    PartyView {
        title: text(Key::Seller, lang),
        name: seller.name.clone(),
        address: seller.address.clone(),
        phone: seller.phone.clone(),
        ids: ids([
            ("RC", seller.rc.as_ref()),
            ("NIF", seller.nif.as_ref()),
            ("NIS", seller.nis.as_ref()),
            ("AI", seller.ai.as_ref()),
        ]),
    }
}

/// The buyer, by the kind of party they were on the day
/// (`a_facture_to_a_consumer_asks_for_a_name_and_an_address_and_nothing_else`). A company prints its identifiers; a
/// consumer prints « ses nom, prénom(s) et adresse » and nothing else
/// (décret 05-468 art. 3-2, last alinéa), whatever the row happens to hold:
/// a consumer fiche that once carried an RC is not turned into a company by
/// this template.
pub(super) fn buyer_view(buyer: &PartyBlock, lang: Lang) -> PartyView {
    let company = buyer.party_kind == PartyKind::Company;
    PartyView {
        title: text(Key::Buyer, lang),
        name: buyer.name.clone(),
        address: buyer.address.clone(),
        phone: None,
        ids: if company {
            ids([
                ("RC", buyer.rc.as_ref()),
                ("NIF", buyer.nif.as_ref()),
                ("NIS", buyer.nis.as_ref()),
                ("AI", buyer.ai.as_ref()),
            ])
        } else {
            Vec::new()
        },
    }
}

pub(super) fn line_view(line: &DocumentLine, reel: bool) -> LineView {
    LineView {
        name: line.name.clone(),
        qty: format_qty(line.qty_milli),
        unit_price: format_centimes(line.unit_price),
        rate: reel.then(|| percent(line.rate_bps)),
        discount: some_amount(line.line_discount),
        total: format_centimes(line.line_total),
    }
}

/// The debt as the ledger had it when the document was issued, read off the
/// document and never recomputed: a reprint shows the balance the customer
/// signed for. Every one of the three can be negative (a customer who
/// overpaid is owed money), so none of them is dropped for being zero.
pub(super) fn balance_view(balance: BalanceTriple, lang: Lang) -> BalanceView {
    // A balance that closes below zero is the shop holding money for the
    // customer, which is where an avoir leaves one who owed less than it
    // gives back: the excess becomes customer credit. The label is what
    // changes and not the figure: the amount is printed with the sign the
    // document stores, the way the statement prints its closing balance.
    //
    // The three rows are the triple features.md §3 defines and not a sum a
    // reader can check on the page: `remaining_debt` is this document's own
    // unpaid part, so a credit sale settled out of credit the customer was
    // already holding prints an old balance and a closing one that differ by
    // more than the middle row. What the closing row states is what the
    // customer owes after the document, which is the figure that matters at
    // a counter.
    //
    // The sign decides it and not the kind: an avoir that only cuts a debt
    // down leaves a debt, and calling that a credit would tell the customer
    // they are owed money they are not.
    let in_credit = balance.total_debt.as_centimes() < 0;
    BalanceView {
        title: text(Key::Balance, lang),
        old_label: text(Key::OldBalance, lang),
        old: format_centimes(balance.old_balance),
        this_label: text(Key::ThisDocument, lang),
        this: format_centimes(balance.remaining_debt),
        total_label: text(
            if in_credit {
                Key::TotalCredit
            } else {
                Key::TotalDebt
            },
            lang,
        ),
        total: format_centimes(balance.total_debt),
    }
}
