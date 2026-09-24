//! What a facture must carry before it may take a number
//! (`a_shop_whose_settings_carry_no_nis_cannot_issue_a_facture_at_all`,
//! décret 05-468 art. 3 and 4). Split out of `services::sales` (T37): the
//! rule reads two independent blocks and used to stop at the first short
//! one, which is the wrong shape for a check with two sides to report.

use crate::error::RetailError;
use crate::models::document::{PartyBlock, PartyKind, SellerBlock};

/// The seller answers with RC and NIS. NIF and AI print when the settings
/// hold them and refuse nothing: they are on every facture in circulation,
/// but a shop that has not been handed one yet still has a sale to ring up,
/// and refusing it would stop the till over a number nobody typed in.
///
/// The buyer answers by the kind of party they are on the day. A company
/// gives the same two identifiers; a consumer gives « ses nom, prénom(s) et
/// adresse » and nothing more (art. 3-2, last alinéa), which is why the
/// party kind is snapshotted onto the block rather than inferred from
/// whether an RC happens to be filled in.
///
/// Both blocks are read before either refuses (T37): a shop whose own
/// settings are short and a customer fiche that is also short used to be
/// two refusals, one per attempt, because the seller was checked first and
/// the buyer was never reached until it passed. A cashier who fixed the
/// settings and tried again then met the fiche for the first time. Now both
/// travel together, so fixing either one at the till shows whatever is
/// still short of the other on the very next try.
pub(crate) fn check(seller: &SellerBlock, buyer: Option<&PartyBlock>) -> Result<(), RetailError> {
    let seller_missing = missing_seller_ids(seller);

    // `issue` refuses a facture with no customer before any of this, so a
    // block missing here is a fiche the document service could not read;
    // the same refusal is the honest answer either way, and it stands on
    // its own rather than folding into `PartyIds`: naming no customer at
    // all is not a fiche that is short of a field, it is no fiche.
    let Some(buyer) = buyer else {
        if !seller_missing.is_empty() {
            return Err(RetailError::PartyIds {
                seller_missing,
                buyer_missing: Vec::new(),
            });
        }
        return Err(RetailError::validation(
            "customer_id",
            "a facture is made out to a customer, so one has to be named",
        ));
    };
    let buyer_missing = missing_buyer_ids(buyer);

    if !seller_missing.is_empty() || !buyer_missing.is_empty() {
        return Err(RetailError::PartyIds {
            seller_missing,
            buyer_missing,
        });
    }
    Ok(())
}

fn missing_seller_ids(seller: &SellerBlock) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if unset(seller.rc.as_deref()) {
        missing.push("rc");
    }
    if unset(seller.nis.as_deref()) {
        missing.push("nis");
    }
    missing
}

fn missing_buyer_ids(buyer: &PartyBlock) -> Vec<&'static str> {
    let mut missing = Vec::new();
    match buyer.party_kind {
        PartyKind::Company => {
            if unset(buyer.rc.as_deref()) {
                missing.push("rc");
            }
            if unset(buyer.nis.as_deref()) {
                missing.push("nis");
            }
        }
        PartyKind::Consumer => {
            if buyer.name.trim().is_empty() {
                missing.push("name");
            }
            if unset(buyer.address.as_deref()) {
                missing.push("address");
            }
        }
    }
    missing
}

/// An identifier a block does not really carry. A field of spaces is not an
/// RC: the services store what was typed after a trim, and a row written
/// before that rule existed could still hold one.
fn unset(value: Option<&str>) -> bool {
    !value.is_some_and(|v| !v.trim().is_empty())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "../../tests/unit/services_party_ids.rs"]
mod tests;
