//! What a facture refuses to print, and why each refusal exists.
//!
//! Every one of these is a stored document contradicting the rule that wrote
//! it, and in each case the two ways of carrying on are worse than stopping:
//! printing the contradiction hands a buyer a page whose parts do not add up,
//! and dropping the offending part hands them a total nobody can check.
//!
//! They live apart from `facture.rs` because they are the same list for every
//! layout. A page drawn tight refuses exactly what a page drawn wide refuses,
//! and a layout that quietly printed one of these would be the one bug the
//! golden files cannot catch, because a golden only says the file has not
//! changed since somebody looked at it.

use crate::error::CoreError;
use crate::models::document::{Document, DocumentKind, DocumentStatus, PartyBlock};
use crate::money::{Money, Regime};
use crate::print::facture::{carries_a_debt, FactureInput};

/// The buyer block, once the document has been found printable.
///
/// It returns the buyer rather than `()` because the check that there is one
/// is in this list, and handing the caller the reference it just proved is
/// there saves unwrapping the same option twice.
pub(crate) fn refuse_what_cannot_be_printed<'a>(
    doc: &'a Document,
    input: &FactureInput<'_>,
) -> Result<&'a PartyBlock, CoreError> {
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
    // features.md, Numbering row). Nothing cancels an avoir or a proforma,
    // and inventing the French for it here would be a fiscal wording nobody
    // reviewed.
    if doc.status == DocumentStatus::Cancelled && doc.kind != DocumentKind::Facture {
        return Err(CoreError::render(
            "only a facture has a printed cancelled wording",
        ));
    }
    // An avoir hands the lines back and asks for nothing, so it carries no
    // droit de timbre (the stamp is a cash sale's tax anyway, Code du
    // timbre 2026 art. 100-I). A stored avoir carrying one contradicts
    // the rule that wrote it: the recap refusal above is the honest answer
    // here too, because dropping the row hands the buyer a total whose parts
    // do not add up.
    if doc.kind == DocumentKind::Avoir && doc.totals.stamp != Money::ZERO {
        return Err(CoreError::render(
            "an avoir carries a droit de timbre and has no printable form",
        ));
    }
    // A proforma moves no stock and creates no debt, and the triple it
    // stores is three zeroes. One that carries a debt contradicts the rule
    // that wrote it: printing the block would say a quote moved a ledger and
    // dropping it would hide that the stored row says otherwise.
    if doc.kind == DocumentKind::Proforma && doc.balance.is_some_and(carries_a_debt) {
        return Err(CoreError::render(
            "a proforma carries a debt and has no printable form",
        ));
    }
    // The day and the reason belong to a document the shop cancelled.
    // Printing them over a live facture would hand a customer a page saying
    // it is void while the ledger still counts it.
    if input.cancellation.is_some() && doc.status != DocumentStatus::Cancelled {
        return Err(CoreError::render(
            "a document that was not cancelled has no cancellation to print",
        ));
    }
    // Only an avoir is written against another document. Printing "avoir sur
    // facture" over a facture or a proforma would label the page as
    // something it is not, and there is no other wording for a reference.
    if doc.ref_document_id.is_some() && doc.kind != DocumentKind::Avoir {
        return Err(CoreError::render(
            "only an avoir names the facture it is written against",
        ));
    }
    Ok(buyer)
}
