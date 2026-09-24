//! The order money lowering a customer's debt settles things in
//! (features.md §2, Payments; Samir's ruling of 2026-09-24 on T33/T34): the
//! opening debt first, because the notebook the shop kept before the app is
//! the oldest thing the customer owes, then the issued documents oldest
//! first, and what is left after both is debt no paper carries, a correction
//! upwards, which the money lowers without placing anywhere.
//!
//! Pure arithmetic, no connection: `services::debt` reads the figures inside
//! the movement's transaction and writes what this decides. Kept apart so the
//! order is one function a property test can hold to, rather than a loop
//! buried in the code that writes rows.

use crate::error::CoreError;
use crate::money::Money;

/// What one movement lowering the debt settles, in the order it settled it.
/// `opening + Σ documents + beyond == amount`, always: every centime of the
/// movement is in exactly one of the three.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// Placed on the opening debt.
    pub opening: Money,
    /// Placed on each document, in the order they were offered, oldest first.
    /// A document the money did not reach is absent rather than zero.
    pub documents: Vec<(i32, Money)>,
    /// What neither the opening debt nor a document could take: a correction
    /// upwards being paid down, or credit the shop is now holding.
    pub beyond: Money,
}

/// How much of the opening debt is still owed, read off figures the ledger
/// and the documents already hold, so no column records it and nothing can
/// drift from the ledger.
///
/// - `balance`: what the customer owes, the movement being settled left out.
/// - `on_paper`: Σ of what the customer's standing documents still ask for.
/// - `opening`: Σ of the opening rows' debits.
/// - `raised`: Σ of the corrections upwards.
///
/// What the balance owes beyond the papers is debt no document carries: the
/// opening debt and the corrections upwards. Money reaches a correction
/// upwards only once the opening debt and every paper are paid, so of that
/// undocumented part, whatever passes the corrections upwards is still owed
/// on the opening. Clamped to `[0, opening]`: a customer holding credit owes
/// nothing on it, and nothing is owed on the opening beyond what it was.
///
/// A file written before the ruling settled the papers before the notebook;
/// read this way it still answers what the notebook is owed today, because
/// the figure is what is left rather than a replay of which payment went
/// where.
pub fn opening_outstanding(
    balance: Money,
    on_paper: Money,
    opening: Money,
    raised: Money,
) -> Result<Money, CoreError> {
    let undocumented = balance.checked_sub(on_paper)?.checked_sub(raised)?;
    if undocumented.is_negative() {
        return Ok(Money::ZERO);
    }
    Ok(undocumented.min(opening))
}

/// Spreads `amount` over the opening debt, then over `documents` in the order
/// given, each taken to what it still asks for and no further, and says what
/// neither could take.
///
/// `documents` carries `(document id, what it still asks for)`. A negative
/// amount, or a negative figure anywhere, is refused: nothing lowering a debt
/// is a negative, and a negative remaining would turn a take into a gift.
pub fn plan(
    amount: Money,
    opening_left: Money,
    documents: &[(i32, Money)],
) -> Result<Plan, CoreError> {
    if amount.is_negative()
        || opening_left.is_negative()
        || documents.iter().any(|(_, left)| left.is_negative())
    {
        return Err(CoreError::validation(
            "amount_centimes",
            "a settlement is planned over amounts that are none of them negative",
        ));
    }
    let opening = amount.min(opening_left);
    let mut left = amount.checked_sub(opening)?;
    let mut placed = Vec::new();
    for (document_id, asks) in documents {
        if left == Money::ZERO {
            break;
        }
        let take = left.min(*asks);
        // An allocation of nothing settles nothing and would sit against the
        // document forever.
        if take == Money::ZERO {
            continue;
        }
        placed.push((*document_id, take));
        left = left.checked_sub(take)?;
    }
    Ok(Plan {
        opening,
        documents: placed,
        beyond: left,
    })
}
