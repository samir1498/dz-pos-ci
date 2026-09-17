//! The A4 (or A5) facture, rendered from a stored document.
//!
//! Same contract as the ticket: everything the paper says is decided here
//! and handed to `templates/facture_a4.html` already made, so a template
//! change is a reviewed golden diff and never a rule hiding in markup. The
//! document is the only source; nothing is recomputed on the way to paper.
//!
//! Three kinds share this template, titled by kind: `facture`, `avoir` and
//! `proforma`. They carry the same blocks, the same lines and the same
//! totals (décret 05-468 art. 3), and splitting them into three files would
//! have been three goldens of the same layout drifting apart.
//!
//! The avoir has no file of its own on purpose: what it does not share with
//! a facture is a title, a line naming the facture it is written against, a
//! words line that says avoir, and the stamp row it never carries. Four
//! conditionals against a second copy of the parties, the lines, the totals
//! and the signature block.

use askama::Template;
use chrono::NaiveDateTime;

use crate::error::CoreError;
use crate::lang::Lang;
use crate::models::document::{
    BalanceTriple, Document, DocumentKind, DocumentLine, DocumentStatus, PartyBlock, PartyKind,
    SellerBlock,
};
use crate::money::format::{format_centimes, format_qty};
use crate::money::words::amount_in_words;
use crate::money::{Money, Regime};
use crate::print::layout::{FactureLayout, Page, Paper};
use crate::print::refusals::refuse_what_cannot_be_printed;
use crate::print::strings::{text, Key};
use crate::print::{number, payment_mode_key, percent, some_amount};

/// A facture carries the date and not the minute: the day of the sale is
/// what décret 05-468 art. 3 asks for and what a comptable files it under.
/// The ticket prints the time as well because a till is reconciled by shift.
const DATE_FORMAT: &str = "%d/%m/%Y";

/// The day the shop cancelled a facture and why, as the caller read them
/// beside the document. They are not on `Document`: `cancelled_at` and
/// `cancel_reason` are columns on the row, and a caller that has read them
/// hands them over here. Without them a cancelled facture still prints as
/// cancelled, because the status is on the row; it just cannot say when or
/// why.
#[derive(Debug, Clone, Copy)]
pub struct Cancellation<'a> {
    pub at: NaiveDateTime,
    pub reason: &'a str,
}

/// What the page needs that the document's own row does not carry.
///
/// Two things, and both for the same reason: a stored row holds ids and
/// columns another task owns, and paper holds sentences. The facture an
/// avoir names is an id on the row and a number and a day on the paper, so
/// the caller that can read that document hands it over. Everything else on
/// the page comes off the document, which stays the only source of an
/// amount.
#[derive(Debug, Clone, Copy, Default)]
pub struct FactureInput<'a> {
    pub referenced: Option<&'a Document>,
    pub cancellation: Option<Cancellation<'a>>,
}

/// One identifier row of a party block: `RC`, `NIF`, `NIS`, `AI`. The
/// labels are the abbreviations every Algerian document prints and are not
/// translated, so they stay literals here rather than dictionary keys.
struct IdRow {
    label: &'static str,
    value: String,
}

struct PartyView {
    title: &'static str,
    name: String,
    address: Option<String>,
    phone: Option<String>,
    ids: Vec<IdRow>,
}

struct LineView {
    name: String,
    qty: String,
    unit_price: String,
    /// The line's TVA rate, under the réel only. Under the IFU there is no
    /// rate column at all, not a column of zeroes
    /// (`an_ifu_facture_names_no_tax_in_any_language`).
    rate: Option<String>,
    discount: Option<String>,
    total: String,
}

struct TvaRow {
    label: &'static str,
    /// The group's rate, on its own so the golden can be read for it: an
    /// amount parser cannot tell a recap row whose label slid onto the
    /// wrong base from one that did not.
    rate: String,
    base_label: &'static str,
    base: String,
    amount: String,
}

/// The line naming the facture an avoir is written against: a sentence and
/// not a column, "Avoir sur facture FA-2026-000042 du 09/09/2026". The
/// number, its year and the day are the referenced facture's, which is why
/// they arrive from the caller and not from the avoir's own row.
struct ReferenceView {
    label: &'static str,
    number: String,
    on_label: &'static str,
    issued_at: String,
}

/// The line under the number of a cancelled facture: the day the shop
/// cancelled it and the reason typed with it. The reason is free text a
/// person wrote, so it reaches the template as a value and is escaped there
/// like a product name.
struct CancellationView {
    on_label: &'static str,
    at: String,
    reason_label: &'static str,
    reason: String,
}

/// How the document was paid, on the two faces of this template where the
/// question means anything. An avoir hands money back and a proforma asks
/// for nothing, so neither prints the block at all.
struct PaymentModeView {
    title: &'static str,
    mode: &'static str,
}

/// The three amounts of the debt as they stood when the document was
/// issued. Printed together or not at all: an old balance without the
/// closing one is a figure the reader cannot check.
struct BalanceView {
    title: &'static str,
    old_label: &'static str,
    old: String,
    this_label: &'static str,
    this: String,
    total_label: &'static str,
    total: String,
}

#[derive(Template)]
#[template(path = "facture_a4.html")]
struct FactureView {
    lang_tag: &'static str,
    dir: &'static str,
    arabic_unreviewed: bool,
    paper_size: &'static str,
    title: &'static str,
    number: String,
    issued_at: String,
    reference: Option<ReferenceView>,
    /// The word across the page of a cancelled reprint. It follows the
    /// document's status, which is on the row, so a caller with no
    /// cancellation columns to hand still prints a void document as void.
    cancelled_mark: Option<&'static str>,
    /// The day and the reason under the number. These follow what the caller
    /// read beside the document, which is why they are a second option and
    /// not a field of the first.
    cancellation: Option<CancellationView>,
    /// What a proforma says about itself. Every other face of this template
    /// is a document that counts, and says nothing.
    notice: Option<&'static str>,
    seller: PartyView,
    buyer: PartyView,
    designation_label: &'static str,
    qty_label: &'static str,
    unit_price_label: &'static str,
    rate_label: &'static str,
    line_discount_label: &'static str,
    line_total_label: &'static str,
    show_rate: bool,
    show_line_discount: bool,
    lines: Vec<LineView>,
    total_label: &'static str,
    total_amount: String,
    discount_label: &'static str,
    discount: Option<String>,
    subtotal_label: &'static str,
    subtotal: Option<String>,
    tva_rows: Vec<TvaRow>,
    total_ttc_label: &'static str,
    total_ttc: Option<String>,
    stamp_label: &'static str,
    stamp: Option<String>,
    net_to_pay_label: &'static str,
    net_to_pay: String,
    in_words_label: &'static str,
    in_words: String,
    balance: Option<BalanceView>,
    /// Absent on an avoir and on a proforma: neither was paid, and a mode
    /// of payment on either is a sentence contradicting the rest of the
    /// page.
    payment_mode: Option<PaymentModeView>,
    /// Whether the row of blocks is printed at all. A proforma has neither
    /// a balance nor a mode of payment, and an empty framed row is a block
    /// the reader looks into for something that was left out.
    show_blocks: bool,
    cachet_label: &'static str,
    seller_label: &'static str,
    buyer_label: &'static str,
    currency: &'static str,
}

/// The facture for `doc`, in `lang`, on `paper`, as one standalone HTML
/// page. An avoir that names the facture it is written against needs that
/// facture's number, which is not on the avoir's own row, so it goes
/// through `render_facture_with_reference`.
pub fn render_facture(doc: &Document, lang: Lang, page: Page) -> Result<String, CoreError> {
    render_facture_with(doc, &FactureInput::default(), lang, page)
}

/// The same page, given the document `doc.ref_document_id` points at.
///
/// The reference is a printed number (`FA-2026-000042`) and the row carries an
/// internal id, and the two are not the same figure: ids are handed out by
/// the table and numbers by the series. So the caller that can read the
/// referenced document hands it over, and an avoir that names one it cannot
/// name is refused rather than printed with the id in the number's place.
pub fn render_facture_with_reference(
    doc: &Document,
    referenced: Option<&Document>,
    lang: Lang,
    page: Page,
) -> Result<String, CoreError> {
    render_facture_with(
        doc,
        &FactureInput {
            referenced,
            cancellation: None,
        },
        lang,
        page,
    )
}

/// The same page, given everything the document's row does not carry. Every
/// face of this template goes through here; the two wrappers above are the
/// callers that have nothing to add.
pub fn render_facture_with(
    doc: &Document,
    input: &FactureInput<'_>,
    lang: Lang,
    page: Page,
) -> Result<String, CoreError> {
    // This template titles itself by kind, and the three kinds it has a
    // title for are the three it prints. A ticket has its own 80 mm paper;
    // a bon de livraison and a bon de réception are not written yet
    // (features.md §4, both parked), and printing one under the word
    // FACTURE would be a document claiming to be another.
    if !matches!(
        doc.kind,
        DocumentKind::Facture | DocumentKind::Avoir | DocumentKind::Proforma
    ) {
        return Err(CoreError::render(
            "this template prints a facture, an avoir or a proforma and nothing else",
        ));
    }
    let buyer = refuse_what_cannot_be_printed(doc, input)?;
    let reference = reference(doc, input.referenced, lang)?;
    let view = view(doc, buyer, reference, input.cancellation, lang, page.paper)?;
    render_in(page.layout, view)
}

/// The page, in the layout that was asked for.
///
/// One arm per layout. Every layout is handed the same `FactureView`, which
/// is the whole reason this is cheap: the decisions are made once above and a
/// layout chooses how to draw them, never what they are. A new layout is one
/// file under `templates/`, one wrapper struct beside `FactureView`, and one
/// arm here.
fn render_in(layout: FactureLayout, view: FactureView) -> Result<String, CoreError> {
    match layout {
        FactureLayout::Standard => view.render().map_err(CoreError::from),
        FactureLayout::Compact => CompactFactureView { facture: view }
            .render()
            .map_err(CoreError::from),
    }
}

/// The compact layout. It borrows the standard layout's view whole rather
/// than copying its fields, so a field added for one page reaches the other
/// without being listed twice and the two cannot drift apart.
#[derive(Template)]
#[template(path = "facture_compact_a4.html")]
struct CompactFactureView {
    facture: FactureView,
}

/// The printed number of the facture an avoir is written against, when the
/// document names one. A document that points at a facture the caller did
/// not hand over is refused: the reference is a legal mention of the avoir
/// (décret 05-468 art. 3), so printing the page without it would drop a
/// field and say nothing about it.
fn reference(
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

fn view(
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
const fn title(doc: &Document, lang: Lang) -> &'static str {
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
const fn in_words_key(kind: DocumentKind) -> Key {
    match kind {
        DocumentKind::Avoir => Key::AvoirInWords,
        DocumentKind::Proforma => Key::ProformaInWords,
        _ => Key::InWords,
    }
}

fn ids(rows: [(&'static str, Option<&String>); 4]) -> Vec<IdRow> {
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
fn seller_view(seller: &SellerBlock, lang: Lang) -> PartyView {
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
fn buyer_view(buyer: &PartyBlock, lang: Lang) -> PartyView {
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

fn line_view(line: &DocumentLine, reel: bool) -> LineView {
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
fn balance_view(balance: BalanceTriple, lang: Lang) -> BalanceView {
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
