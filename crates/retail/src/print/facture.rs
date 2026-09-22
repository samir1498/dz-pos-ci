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
use crate::models::document::{Document, DocumentKind};
use crate::print::facture_view::{reference, view};
use crate::print::layout::{FactureLayout, Page, Paper};
use crate::print::refusals::refuse_what_cannot_be_printed;

/// A facture carries the date and not the minute: the day of the sale is
/// what décret 05-468 art. 3 asks for and what a comptable files it under.
/// The ticket prints the time as well because a till is reconciled by shift.
pub(super) const DATE_FORMAT: &str = "%d/%m/%Y";

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
pub(super) struct IdRow {
    pub(super) label: &'static str,
    pub(super) value: String,
}

pub(super) struct PartyView {
    pub(super) title: &'static str,
    pub(super) name: String,
    pub(super) address: Option<String>,
    pub(super) phone: Option<String>,
    pub(super) ids: Vec<IdRow>,
}

pub(super) struct LineView {
    pub(super) name: String,
    pub(super) qty: String,
    pub(super) unit_price: String,
    /// The line's TVA rate, under the réel only. Under the IFU there is no
    /// rate column at all, not a column of zeroes
    /// (`an_ifu_facture_names_no_tax_in_any_language`).
    pub(super) rate: Option<String>,
    pub(super) discount: Option<String>,
    pub(super) total: String,
}

pub(super) struct TvaRow {
    pub(super) label: &'static str,
    /// The group's rate, on its own so the golden can be read for it: an
    /// amount parser cannot tell a recap row whose label slid onto the
    /// wrong base from one that did not.
    pub(super) rate: String,
    pub(super) base_label: &'static str,
    pub(super) base: String,
    pub(super) amount: String,
}

/// The line naming the facture an avoir is written against: a sentence and
/// not a column, "Avoir sur facture FA-2026-000042 du 09/09/2026". The
/// number, its year and the day are the referenced facture's, which is why
/// they arrive from the caller and not from the avoir's own row.
pub(super) struct ReferenceView {
    pub(super) label: &'static str,
    pub(super) number: String,
    pub(super) on_label: &'static str,
    pub(super) issued_at: String,
}

/// The line under the number of a cancelled facture: the day the shop
/// cancelled it and the reason typed with it. The reason is free text a
/// person wrote, so it reaches the template as a value and is escaped there
/// like a product name.
pub(super) struct CancellationView {
    pub(super) on_label: &'static str,
    pub(super) at: String,
    pub(super) reason_label: &'static str,
    pub(super) reason: String,
}

/// How the document was paid, on the two faces of this template where the
/// question means anything. An avoir hands money back and a proforma asks
/// for nothing, so neither prints the block at all.
pub(super) struct PaymentModeView {
    pub(super) title: &'static str,
    pub(super) mode: &'static str,
}

/// The three amounts of the debt as they stood when the document was
/// issued. Printed together or not at all: an old balance without the
/// closing one is a figure the reader cannot check.
pub(super) struct BalanceView {
    pub(super) title: &'static str,
    pub(super) old_label: &'static str,
    pub(super) old: String,
    pub(super) this_label: &'static str,
    pub(super) this: String,
    pub(super) total_label: &'static str,
    pub(super) total: String,
}

#[derive(Template)]
#[template(path = "facture_a4.html")]
pub(super) struct FactureView {
    pub(super) lang_tag: &'static str,
    pub(super) dir: &'static str,
    pub(super) arabic_unreviewed: bool,
    pub(super) paper_size: &'static str,
    pub(super) title: &'static str,
    pub(super) number: String,
    pub(super) issued_at: String,
    pub(super) reference: Option<ReferenceView>,
    /// The word across the page of a cancelled reprint. It follows the
    /// document's status, which is on the row, so a caller with no
    /// cancellation columns to hand still prints a void document as void.
    pub(super) cancelled_mark: Option<&'static str>,
    /// The day and the reason under the number. These follow what the caller
    /// read beside the document, which is why they are a second option and
    /// not a field of the first.
    pub(super) cancellation: Option<CancellationView>,
    /// What a proforma says about itself. Every other face of this template
    /// is a document that counts, and says nothing.
    pub(super) notice: Option<&'static str>,
    pub(super) seller: PartyView,
    pub(super) buyer: PartyView,
    pub(super) designation_label: &'static str,
    pub(super) qty_label: &'static str,
    pub(super) unit_price_label: &'static str,
    pub(super) rate_label: &'static str,
    pub(super) line_discount_label: &'static str,
    pub(super) line_total_label: &'static str,
    pub(super) show_rate: bool,
    pub(super) show_line_discount: bool,
    pub(super) lines: Vec<LineView>,
    pub(super) total_label: &'static str,
    pub(super) total_amount: String,
    pub(super) discount_label: &'static str,
    pub(super) discount: Option<String>,
    pub(super) subtotal_label: &'static str,
    pub(super) subtotal: Option<String>,
    pub(super) tva_rows: Vec<TvaRow>,
    pub(super) total_ttc_label: &'static str,
    pub(super) total_ttc: Option<String>,
    pub(super) stamp_label: &'static str,
    pub(super) stamp: Option<String>,
    pub(super) net_to_pay_label: &'static str,
    pub(super) net_to_pay: String,
    pub(super) in_words_label: &'static str,
    pub(super) in_words: String,
    pub(super) balance: Option<BalanceView>,
    /// Absent on an avoir and on a proforma: neither was paid, and a mode
    /// of payment on either is a sentence contradicting the rest of the
    /// page.
    pub(super) payment_mode: Option<PaymentModeView>,
    /// Whether the row of blocks is printed at all. A proforma has neither
    /// a balance nor a mode of payment, and an empty framed row is a block
    /// the reader looks into for something that was left out.
    pub(super) show_blocks: bool,
    pub(super) cachet_label: &'static str,
    pub(super) seller_label: &'static str,
    pub(super) buyer_label: &'static str,
    pub(super) currency: &'static str,
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
    render_in(page.layout, built_view(doc, input, lang, page.paper())?)
}

/// The document read, refused, or turned into the one view every layout
/// draws. Split out of `render_facture_with` when the ESC/POS roll landed:
/// `print::facture_roll` sends the same fields to a thermal head rather
/// than to a template, and the list of what a facture refuses to print has
/// to be the same list on paper and on a roll of dots. A page drawn tight
/// refuses exactly what a page drawn wide refuses (`print::refusals`).
pub(super) fn built_view(
    doc: &Document,
    input: &FactureInput<'_>,
    lang: Lang,
    paper: Paper,
) -> Result<FactureView, CoreError> {
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
    view(doc, buyer, reference, input.cancellation, lang, paper)
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
        FactureLayout::HalfSheet => HalfSheetFactureView { facture: view }
            .render()
            .map_err(CoreError::from),
        FactureLayout::Roll80 => Roll80FactureView { facture: view }
            .render()
            .map_err(CoreError::from),
    }
}

/// The compact layout. It borrows the standard layout's view whole rather
/// than copying its fields, so a field added for one page reaches the other
/// without being listed twice and the two cannot drift apart.
#[derive(Template)]
#[template(path = "facture_compact_a4.html")]
pub(super) struct CompactFactureView {
    pub(super) facture: FactureView,
}

/// The half sheet, borrowing the same view for the same reason.
#[derive(Template)]
#[template(path = "facture_half_sheet.html")]
pub(super) struct HalfSheetFactureView {
    pub(super) facture: FactureView,
}

/// The 80 mm roll. Same view again, and this is the layout that shows why
/// the view is shared rather than copied: its markup is its own, so the only
/// thing keeping it saying what the other three say is that it is filled
/// from the same fields.
#[derive(Template)]
#[template(path = "facture_roll_80mm.html")]
pub(super) struct Roll80FactureView {
    pub(super) facture: FactureView,
}
