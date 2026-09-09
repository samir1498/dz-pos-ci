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
//! have been three goldens of the same layout drifting apart. T9 splits
//! them if the avoir ever needs more than its title and its reference.

use askama::Template;

use crate::error::CoreError;
use crate::lang::Lang;
use crate::models::document::{
    BalanceTriple, Document, DocumentKind, DocumentLine, DocumentStatus, PartyBlock, PartyKind,
    SellerBlock,
};
use crate::money::format::{format_centimes, format_qty};
use crate::money::words::amount_in_words;
use crate::money::{Money, Regime};
use crate::print::strings::{text, Key};
use crate::print::{number, payment_mode_key, percent, some_amount};

/// A facture carries the date and not the minute: the day of the sale is
/// what décret 05-468 art. 3 asks for and what a comptable files it under.
/// The ticket prints the time as well because a till is reconciled by shift.
const DATE_FORMAT: &str = "%d/%m/%Y";

/// The sheet the OS print dialog is given. It changes one line of the page,
/// the `@page size`, and nothing else: an A5 facture is the same facture on
/// a smaller sheet, not a second layout to keep in step (features.md §4,
/// "A4/A5 through the OS dialog").
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Paper {
    A4,
    A5,
}

impl Paper {
    /// The value of the CSS `size` descriptor. The two named page sizes are
    /// CSS's own, so the browser and the print dialog agree on the sheet
    /// without the template naming millimetres.
    const fn css_size(self) -> &'static str {
        match self {
            Paper::A4 => "A4",
            Paper::A5 => "A5",
        }
    }
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
    /// (`regime_ifu_prints_no_tva`).
    rate: Option<String>,
    discount: Option<String>,
    total: String,
}

struct TvaRow {
    label: String,
    base_label: &'static str,
    base: String,
    amount: String,
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
    reference_label: &'static str,
    reference: Option<String>,
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
    payment_mode_label: &'static str,
    payment_mode: &'static str,
    cachet_label: &'static str,
    seller_label: &'static str,
    buyer_label: &'static str,
    currency: &'static str,
}

/// The facture for `doc`, in `lang`, on `paper`, as one standalone HTML
/// page. An avoir that names the facture it is written against needs that
/// facture's number, which is not on the avoir's own row, so it goes
/// through `render_facture_with_reference`.
pub fn render_facture(doc: &Document, lang: Lang, paper: Paper) -> Result<String, CoreError> {
    render_facture_with_reference(doc, None, lang, paper)
}

/// The same page, given the document `doc.ref_document_id` points at.
///
/// The reference is a printed number (`FA-000042`) and the row carries an
/// internal id, and the two are not the same figure: ids are handed out by
/// the table and numbers by the series. So the caller that can read the
/// referenced document hands it over, and an avoir that names one it cannot
/// name is refused rather than printed with the id in the number's place.
pub fn render_facture_with_reference(
    doc: &Document,
    referenced: Option<&Document>,
    lang: Lang,
    paper: Paper,
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
    // The same refusal the ticket makes, for the same reason: a stored IFU
    // document carrying a TVA recap contradicts the régime it was issued
    // under, printing the recap names a tax the document must not name
    // (CTCA 2026 art. 64), and dropping it quietly hands the buyer a total
    // whose parts do not add up.
    if doc.regime == Regime::Ifu && !doc.totals.tva_by_rate.is_empty() {
        return Err(CoreError::render(
            "an IFU document carries a TVA recap and has no printable form",
        ));
    }
    // Décret 05-468 art. 3 puts the buyer on the paper: the identifiers of
    // a company, the name and address of a consumer. A facture with no
    // buyer block is not a facture, and the honest answer is to refuse it
    // rather than print a document with an empty half.
    let Some(buyer) = doc.buyer.as_ref() else {
        return Err(CoreError::render(
            "the document has no buyer block and no facture can be printed without one",
        ));
    };
    // Only a facture has a cancelled wording ("facture annulée",
    // features.md, Numbering row). Nothing cancels an avoir or a proforma
    // in M2, and inventing the French for it here would be a fiscal wording
    // nobody reviewed.
    if doc.status == DocumentStatus::Cancelled && doc.kind != DocumentKind::Facture {
        return Err(CoreError::render(
            "only a facture has a printed cancelled wording",
        ));
    }
    let reference = reference(doc, referenced)?;
    view(doc, buyer, reference, lang, paper)?
        .render()
        .map_err(CoreError::from)
}

/// The printed number of the facture an avoir is written against, when the
/// document names one. A document that points at a facture the caller did
/// not hand over is refused: the reference is a legal mention of the avoir
/// (décret 05-468 art. 3), so printing the page without it would drop a
/// field and say nothing about it.
fn reference(doc: &Document, referenced: Option<&Document>) -> Result<Option<String>, CoreError> {
    match (doc.ref_document_id, referenced) {
        (None, _) => Ok(None),
        (Some(id), Some(referenced))
            if referenced.id == id && referenced.shop_id == doc.shop_id =>
        {
            Ok(Some(number(referenced)))
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
    reference: Option<String>,
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
        reference_label: text(Key::ReferencedDocument, lang),
        reference,
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
                label: format!("{} {}", text(Key::Tva, lang), percent(row.rate)),
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
        net_to_pay_label: text(Key::NetToPay, lang),
        net_to_pay: format_centimes(totals.net_to_pay),
        in_words_label: text(Key::InWords, lang),
        in_words,
        balance: doc.balance.map(|b| balance_view(b, lang)),
        payment_mode_label: text(Key::PaymentMode, lang),
        payment_mode: text(payment_mode_key(doc.payment_mode), lang),
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
/// (`facture_requires_party_ids`). A company prints its identifiers; a
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
    BalanceView {
        title: text(Key::Balance, lang),
        old_label: text(Key::OldBalance, lang),
        old: format_centimes(balance.old_balance),
        this_label: text(Key::ThisDocument, lang),
        this: format_centimes(balance.remaining_debt),
        total_label: text(Key::TotalDebt, lang),
        total: format_centimes(balance.total_debt),
    }
}
