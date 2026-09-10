//! The avoir, the credit note a shop writes against a facture it has already
//! issued (features.md §3).
//!
//! A facture is never edited and never deleted: it carries a number out of an
//! uninterrupted series (décret 05-468 art. 10), and the customer is holding a
//! copy of it. What a shop does instead is write a second numbered document
//! that carries the money back, out of its own series, naming the facture it
//! is written against.
//!
//! Three rules decide what one is worth. It never carries the droit de timbre:
//! the stamp is paid on money that changed hands (Code du timbre 2026
//! art. 100-I) and is not refunded with the goods. Its TVA is per rate on its
//! own lines, rounded once on its own base the way a sale's is, so a partial
//! avoir is taxed as the part it credits rather than as a share of the
//! facture's tax. And the running total of avoirs on one facture never passes
//! what that facture asked for, which is the safety net under every other
//! rule here: a credit note for more than the paper it credits is money the
//! shop never took. That last rule is read per rate as well as on the total,
//! because a slice that rounds its tax up at one rate leaves the credit note
//! that closes the facture holding the centime it took too much.
//!
//! Every one of those rules is the same subtraction: the facture less the
//! avoirs already written against it. It is done once, into `Remaining`, and
//! every rule below reads that. A partial avoir is `min(the slice, what is
//! left)` per line and per rate; the one that closes the facture is what is
//! left, whole, field by field and line by line.
//!
//! What it does to the account is the ledger's half. The unpaid part of the
//! facture is reversed first, then whatever is left of the avoir goes over the
//! customer's other unpaid documents oldest first, and only what no paper can
//! take becomes credit the shop is holding. That last step is the only way a
//! customer's balance goes below zero (features.md §2).

use chrono::NaiveDateTime;
use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::debt::{DebtKind, NewDebtEntry};
use crate::models::stock::{Movement, MovementKind};
use crate::money::{Bps, Money, MoneyError, Regime, Totals, TvaLine};
use crate::repos::documents as repo;
use crate::services::documents::{
    BalanceTriple, Document, DocumentKind, DocumentLine, DocumentStatus, NewDocument,
    NewDocumentLine,
};
use crate::services::{audit, clock, debt, documents, optional_field, products, stock};

/// One line of a facture and how much of it is coming back.
///
/// The line is named by id and never by the product on it: a facture carries
/// one product on two lines as soon as a line discount or a second price is
/// involved, and what is left to credit is a fact about a line rather than
/// about a product.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AvoirLine {
    pub document_line_id: i32,
    /// Thousandths of the unit, the way the sold quantity is.
    pub qty_milli: i64,
}

/// Writes the avoir: its number, its document with the lines and the TVA
/// recap, the stock coming back, the ledger movement and what it settled, and
/// the audit entry. One transaction, so a credit note that is refused anywhere
/// leaves no number burned and no goods back on the shelf.
///
/// `lines` of `None` is the whole of what is left on the facture, not the whole
/// of what it was: a facture already credited in part is finished off by one
/// whole avoir rather than refused for asking twice.
pub fn issue(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    facture_id: i32,
    lines: Option<Vec<AvoirLine>>,
    reason: Option<String>,
    at: Option<NaiveDateTime>,
) -> Result<Document, CoreError> {
    let reason = optional_field("reason", reason.as_deref())?;
    conn.transaction(|conn| {
        // Only a facture, and only one that still stands. `get_of_kind`
        // answers "no such facture" for a ticket, an avoir or another shop's
        // document alike (rule 3), which is the honest answer in all three.
        let facture = documents::get_of_kind(conn, shop_id, facture_id, DocumentKind::Facture)?;
        if facture.status == DocumentStatus::Cancelled {
            return Err(CoreError::validation(
                "document_id",
                "this facture is annulée, and a document that asks for nothing is credited by nothing",
            ));
        }

        // The one subtraction, done once. Everything below reads it.
        let remaining = Remaining::of(conn, shop_id, &facture)?;
        let coming_back = chosen(&facture, &remaining, lines)?;

        // The one that takes the last quantity off the facture is not computed
        // from its own slice at all: it is what is left, field by field.
        // Rounding a slice's tax is what makes three credit notes of 60
        // against a facture of 179, and the third of them is where the centime
        // has to come back (`avoir_closing_carries_the_remainder`).
        let (totals, document_lines) = if remaining.closed_by(&coming_back) {
            let (totals, lines) = remaining.whole(&facture)?;
            // The closing avoir is a subtraction, so it cannot overrun the
            // facture by arithmetic and the running-total check below would
            // never fire on it. What it can meet is a file that already
            // disagrees with itself: avoirs coming to the whole of the facture
            // while its lines still hold goods, which is what a row written
            // straight into the table looks like. The subtraction then lands
            // on a field below zero, or on a total its own lines do not add up
            // to, and either one is that overrun read on the paper that would
            // have to carry it.
            //
            // A subtraction that lands on nothing at all is not an overrun:
            // the last milligrams of a line can be worth no centime, and the
            // credit note that takes them back is written for zero and puts
            // the goods on the shelf.
            let summed = lines
                .iter()
                .try_fold(Money::ZERO, |acc, l| acc.checked_add(l.line_total))?;
            let a_line_below_zero = lines.iter().any(|l| l.line_total.is_negative());
            if below_zero(&totals) || a_line_below_zero || summed != totals.total_ht {
                return Err(CoreError::validation(
                    "lines",
                    "the avoirs on this facture would come to more than it asked for",
                ));
            }
            (totals, lines)
        } else {
            // Every line is priced as it was sold, so a reprint of the two
            // papers side by side shows the same unit price and the same rate,
            // and never for more than the facture line still holds. Half a
            // unit at 0,01 costs a centime on its own paper, so two halves
            // credit two centimes of a line worth one, and the credit note
            // that closes the facture is left owing the difference.
            let mut slice: Vec<SliceLine> = Vec::new();
            let mut document_lines = Vec::new();
            for (line, qty_milli, line_discount) in &coming_back {
                let asked = line
                    .unit_price
                    .checked_mul_milli(*qty_milli)?
                    .checked_sub(*line_discount)?;
                let line_total = asked.min(remaining.money_on_line(line.id));
                slice.push(SliceLine {
                    rate: line.rate_bps,
                    ht: line_total,
                });
                document_lines.push(NewDocumentLine {
                    product_id: line.product_id,
                    name: line.name.clone(),
                    barcode: line.barcode.clone(),
                    qty_milli: *qty_milli,
                    unit_price: line.unit_price,
                    line_discount: *line_discount,
                    rate_bps: line.rate_bps,
                    line_total,
                    ref_line_id: Some(line.id),
                });
            }
            let totals = slice_totals(&facture, &remaining, &slice)?;

            // The safety net under the quantities: a line's remaining
            // quantity is checked above, and this is checked against the one
            // figure the whole facture asked for, which `Remaining::totals`
            // says why. A forged avoir carrying no line at all moves no
            // quantity and would slip past everything else.
            if totals.net_to_pay > remaining.totals.total_ttc {
                return Err(CoreError::validation(
                    "lines",
                    "the avoirs on this facture would come to more than it asked for",
                ));
            }

            (totals, document_lines)
        };

        let customer_id = facture.customer_id;
        let old_balance = match customer_id {
            Some(customer_id) => debt::balance(conn, shop_id, customer_id)?,
            None => Money::ZERO,
        };
        // The triple on an avoir is its whole effect on the account: what was
        // owed before it, the negative of its own net, and what is owed after.
        // The middle figure is never decremented by a later settlement, unlike
        // a facture's: the paper states what this credit note was worth on the
        // day it was written, and a payment against some other facture does
        // not change that.
        let effect = Money::ZERO.checked_sub(totals.net_to_pay)?;
        let balance = match customer_id {
            Some(_) => Some(BalanceTriple {
                old_balance,
                remaining_debt: effect,
                total_debt: old_balance.checked_add(effect)?,
            }),
            None => None,
        };

        let avoir = documents::issue(
            conn,
            shop_id,
            NewDocument {
                kind: DocumentKind::Avoir,
                issued_at: at.unwrap_or_else(clock::now),
                user_id,
                regime: facture.regime,
                payment_mode: facture.payment_mode,
                // Both party blocks are the facture's, copied rather than read
                // live: an avoir is read beside the facture it credits, and
                // the two have to name the same seller and the same buyer even
                // after the shop's settings or the fiche have been edited.
                seller: facture.seller.clone(),
                customer_id,
                buyer: facture.buyer.clone(),
                ref_document_id: Some(facture_id),
                balance,
                totals: totals.clone(),
                tendered: None,
                change: None,
                lines: document_lines,
            },
        )?;

        // The goods come back after the document exists, so every movement
        // names the avoir that brought them back. A line whose product has
        // gone moves nothing: there is no count left to move.
        for (line, qty_milli, _) in &coming_back {
            let Some(product_id) = line.product_id else {
                continue;
            };
            // The cost the product carries now, which is what the goods are
            // worth back on the shelf. Read before the movement so the two
            // borrows do not overlap.
            let unit_cost = products::get(conn, shop_id, product_id)?.cost;
            stock::record(
                conn,
                shop_id,
                &Movement {
                    product_id,
                    kind: MovementKind::Return,
                    qty_milli: *qty_milli,
                    unit_cost,
                    document_id: Some(avoir.id),
                    user_id,
                },
            )?;
        }

        let Some(customer_id) = customer_id else {
            return Ok(avoir);
        };

        // An avoir for no money moves no debt. The closing one comes to nothing
        // when what is left of the facture is a quantity worth no centime: the
        // goods come back on it and the ledger is left alone, and it is
        // audited like any other because a numbered document was written.
        if totals.net_to_pay != Money::ZERO {
            // One credit movement for the whole of it, naming the avoir. What
            // it settles is said by the allocations beside it, not by this
            // row: an avoir can reach the facture it credits and then a
            // second one.
            let entry = debt::append_at(
                conn,
                shop_id,
                NewDebtEntry {
                    customer_id,
                    document_id: Some(avoir.id),
                    kind: DebtKind::Avoir,
                    debit: Money::ZERO,
                    credit: totals.net_to_pay,
                    user_id,
                    note: reason.clone(),
                },
                Some(avoir.issued_at),
            )?;

            // The facture this avoir was written against comes first, whatever
            // its age: the money is going back on that paper and nowhere else.
            // Only what it cannot take spreads over the customer's other
            // unpaid documents, oldest first, and what no paper can take at
            // all is left on the ledger as credit the shop is holding.
            let on_the_facture = totals.net_to_pay.min(unpaid_on(&facture));
            debt::settle_document(conn, shop_id, entry.id, facture_id, on_the_facture)?;
            debt::settle_oldest_first(
                conn,
                shop_id,
                customer_id,
                entry.id,
                totals.net_to_pay.checked_sub(on_the_facture)?,
            )?;
        }

        let after = debt::balance(conn, shop_id, customer_id)?;
        let left_on_facture = documents::get(conn, shop_id, facture_id)?
            .balance
            .map_or(Money::ZERO, |b| b.remaining_debt);
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: audit::ACTION_AVOIR,
                // The facture is the entity that changed: it is the paper a
                // reader is holding when they ask why it stopped asking for
                // its amount. The avoir is named in `after` as what the
                // decision produced.
                entity: "document",
                entity_id: Some(facture_id),
                before: Some(
                    serde_json::json!({
                        "remaining_debt_centimes": unpaid_on(&facture).as_centimes(),
                        "balance_centimes": old_balance.as_centimes(),
                    })
                    .to_string(),
                ),
                after: Some(
                    serde_json::json!({
                        "avoir_document_id": avoir.id,
                        "avoir_number": avoir.number,
                        "amount_centimes": totals.net_to_pay.as_centimes(),
                        "remaining_debt_centimes": left_on_facture.as_centimes(),
                        "balance_centimes": after.as_centimes(),
                        "reason": reason,
                    })
                    .to_string(),
                ),
            },
        )?;

        Ok(avoir)
    })
}

/// Every avoir written against one facture, oldest first. What the screen
/// showing a facture lists under it, and what the running total is summed
/// from.
pub fn list_for(
    conn: &mut SqliteConnection,
    shop_id: i32,
    facture_id: i32,
) -> Result<Vec<Document>, CoreError> {
    // Through the service, so a ticket or another shop's document is answered
    // as no such facture rather than as an empty list (rule 3).
    documents::get_of_kind(conn, shop_id, facture_id, DocumentKind::Facture)?;
    repo::avoirs_of(conn, shop_id, facture_id)
}

/// Whether any line of the facture still has something on it to credit.
///
/// What a cancellation asks before writing an avoir: a facture whose goods
/// have all come back already is annulled without a second credit note, and
/// calling `issue` to find that out would mean reading a refusal as an answer.
pub fn anything_left(
    conn: &mut SqliteConnection,
    shop_id: i32,
    facture: &Document,
) -> Result<bool, CoreError> {
    let remaining = Remaining::of(conn, shop_id, facture)?;
    Ok(remaining.lines.iter().any(|line| line.qty_milli > 0))
}

/// What a whole avoir on this facture would come to, without writing one.
///
/// The figure a cancellation's confirm shows, and it has to be this
/// subtraction rather than the totals of the lines still on the facture: those
/// are what the closing avoir refuses to be computed from, so a confirm
/// re-deriving them would promise 60 where the avoir will be written for 59.
pub fn what_is_left(
    conn: &mut SqliteConnection,
    shop_id: i32,
    facture: &Document,
) -> Result<Money, CoreError> {
    Ok(Remaining::of(conn, shop_id, facture)?
        .totals
        .total_ttc
        .max(Money::ZERO))
}

/// What is still unpaid on a facture. A facture with no customer carries no
/// triple at all, and nothing was ever owed on it.
fn unpaid_on(facture: &Document) -> Money {
    facture.balance.map_or(Money::ZERO, |b| b.remaining_debt)
}

/// The facture less every avoir already written against it, read three ways:
/// line by line, rate by rate, and on the totals. Built once at the top of
/// `issue`, because every rule an avoir obeys is this one subtraction asked a
/// different question.
///
/// Nothing here is clamped. A file that disagrees with itself, which is what a
/// row written straight into the table looks like, leaves a quantity or an
/// amount below zero, and each caller decides what that means: a partial avoir
/// reads a line that owes nothing as nothing left to give (the goods still
/// come back, for no money), and the avoir that closes the facture refuses,
/// because the subtraction it is written from is the file's own arithmetic.
struct Remaining {
    lines: Vec<RemainingLine>,
    rates: Vec<RemainingRate>,
    /// The facture's totals less every avoir's, the stamp taken off: what the
    /// closing avoir is written for, what the remise a partial gives back is
    /// measured against under the IFU, and, on `total_ttc`, what the running
    /// total of every avoir is capped at.
    ///
    /// The cap is read on `total_ttc` and not on the net, because the droit de
    /// timbre is the one part of a facture no avoir gives back: capping at the
    /// net would leave the stamp's worth of room for the partials to eat, and
    /// the closing avoir would then have to be written for a negative amount.
    /// Subtracting each avoir's own `total_ttc` says the same thing as
    /// subtracting its net, because an avoir never carries a stamp
    /// (`an_avoir_prints_no_stamp_and_one_that_carries_a_stamp_is_refused`).
    totals: Totals,
    /// An earlier avoir carries a rate in its recap that the facture's recap
    /// does not. Kept rather than raised: only the closing avoir is written
    /// from the rate rows, so only it has to refuse them.
    foreign_rate: bool,
}

/// What one facture line still has on it: the quantity, the money and the
/// share of the line discount that has not come back yet.
struct RemainingLine {
    id: i32,
    qty_milli: i64,
    line_total: Money,
    line_discount: Money,
}

/// What one rate of the facture still has on it: the HT of its lines, the
/// taxable base the facture charged there and the tax on it. The remise still
/// to give back at the rate is the first less the second.
struct RemainingRate {
    rate: Bps,
    ht: Money,
    base: Money,
    amount: Money,
    /// Whether the facture's TVA recap names this rate. Under the IFU a
    /// document shows no TVA at all, so every rate its lines are at is a rate
    /// with no row, and the closing avoir writes no recap either.
    on_the_recap: bool,
}

impl Remaining {
    fn of(
        conn: &mut SqliteConnection,
        shop_id: i32,
        facture: &Document,
    ) -> Result<Self, CoreError> {
        let earlier = repo::avoirs_of(conn, shop_id, facture.id)?;

        let mut lines: Vec<RemainingLine> = facture
            .lines
            .iter()
            .map(|line| RemainingLine {
                id: line.id,
                qty_milli: line.qty_milli,
                line_total: line.line_total,
                line_discount: line.line_discount,
            })
            .collect();

        // One row per rate the facture's recap names, in the recap's order,
        // because the closing avoir's recap is this one less the avoirs' and a
        // comptable reads the two side by side. Then one per rate the lines
        // are at that the recap does not name, which under the IFU is all of
        // them: no tax was charged there, so the whole of the HT is base and
        // there is no remise at that rate to give back.
        let mut rates: Vec<RemainingRate> = Vec::new();
        for row in &facture.totals.tva_by_rate {
            rates.push(RemainingRate {
                rate: row.rate,
                ht: ht_at(&facture.lines, row.rate)?,
                base: row.base,
                amount: row.amount,
                on_the_recap: true,
            });
        }
        for line in &facture.lines {
            if rates.iter().any(|r| r.rate == line.rate_bps) {
                continue;
            }
            let ht = ht_at(&facture.lines, line.rate_bps)?;
            rates.push(RemainingRate {
                rate: line.rate_bps,
                ht,
                base: ht,
                amount: Money::ZERO,
                on_the_recap: false,
            });
        }

        // The droit de timbre is the one thing not in the subtraction. It is
        // paid on money that changed hands (Code du timbre 2026 art. 100-I)
        // and never given back, so the figure being reproduced is the
        // facture's `total_ttc` and the avoir's own stamp stays at zero.
        let mut totals = Totals {
            total_ht: facture.totals.total_ht,
            discount: facture.totals.discount,
            subtotal_ht: facture.totals.subtotal_ht,
            tva_by_rate: Vec::new(),
            tva: facture.totals.tva,
            total_ttc: facture.totals.total_ttc,
            stamp: Money::ZERO,
            net_to_pay: facture.totals.total_ttc,
        };
        let mut foreign_rate = false;

        for avoir in &earlier {
            totals.total_ht = totals.total_ht.checked_sub(avoir.totals.total_ht)?;
            totals.discount = totals.discount.checked_sub(avoir.totals.discount)?;
            totals.subtotal_ht = totals.subtotal_ht.checked_sub(avoir.totals.subtotal_ht)?;
            totals.tva = totals.tva.checked_sub(avoir.totals.tva)?;
            totals.total_ttc = totals.total_ttc.checked_sub(avoir.totals.total_ttc)?;

            for taken in &avoir.lines {
                if let Some(mine) = lines
                    .iter_mut()
                    .find(|line| taken.ref_line_id == Some(line.id))
                {
                    // Saturating rather than checked, because a quantity below
                    // zero is not an error here: it is an avoir crediting more
                    // of a line than the line ever held, which the closing
                    // avoir refuses and a partial reads as a line with nothing
                    // left on it.
                    mine.qty_milli = mine.qty_milli.saturating_sub(taken.qty_milli);
                    mine.line_total = mine.line_total.checked_sub(taken.line_total)?;
                    mine.line_discount = mine.line_discount.checked_sub(taken.line_discount)?;
                }
                if let Some(mine) = rates.iter_mut().find(|r| r.rate == taken.rate_bps) {
                    mine.ht = mine.ht.checked_sub(taken.line_total)?;
                    // A rate the recap does not name was taxed on nothing and
                    // carries no remise, so its base follows its HT down and
                    // the two stay equal. Left behind, the base would sit
                    // above the HT as soon as one avoir took part of the
                    // group, and the remise still to give back there would
                    // read as less than nothing.
                    if !mine.on_the_recap {
                        mine.base = mine.base.checked_sub(taken.line_total)?;
                    }
                }
            }
            for row in &avoir.totals.tva_by_rate {
                // A rate an earlier avoir's recap carries is a rate the
                // facture's recap carries: an avoir line is a facture line and
                // takes its rate from it.
                match rates
                    .iter_mut()
                    .find(|r| r.rate == row.rate && r.on_the_recap)
                {
                    Some(mine) => {
                        mine.base = mine.base.checked_sub(row.base)?;
                        mine.amount = mine.amount.checked_sub(row.amount)?;
                    }
                    None => foreign_rate = true,
                }
            }
        }
        totals.net_to_pay = totals.total_ttc;

        Ok(Self {
            lines,
            rates,
            totals,
            foreign_rate,
        })
    }

    /// What one facture line has left, and what one rate has left. A line or a
    /// rate the facture does not carry has nothing left on it, which is what
    /// `None` says.
    fn on_line(&self, line_id: i32) -> Option<&RemainingLine> {
        self.lines.iter().find(|line| line.id == line_id)
    }

    fn at(&self, rate: Bps) -> Option<&RemainingRate> {
        self.rates.iter().find(|r| r.rate == rate)
    }

    /// The quantity still on one facture line, and the money still on it,
    /// neither below zero: a line credited past what it held, or to the
    /// centime, has nothing left to give, and the quantity still on such a
    /// line comes back for no money.
    fn quantity_on_line(&self, line_id: i32) -> i64 {
        self.on_line(line_id)
            .map_or(0, |line| line.qty_milli.max(0))
    }

    fn money_on_line(&self, line_id: i32) -> Money {
        self.on_line(line_id)
            .map_or(Money::ZERO, |line| line.line_total.max(Money::ZERO))
    }

    /// Whether this avoir takes the last quantity off the facture, which is
    /// what decides how its money is computed.
    ///
    /// Every line of the facture has to end at nothing: a line still holding
    /// one unit is a facture that can be credited again, and the avoir being
    /// written is one more partial.
    fn closed_by(&self, coming: &Coming) -> bool {
        self.lines.iter().all(|line| {
            let now = coming
                .iter()
                .find(|(l, _, _)| l.id == line.id)
                .map_or(0, |(_, qty, _)| *qty);
            line.qty_milli.max(0).saturating_sub(now) <= 0
        })
    }

    /// The whole of what is left: the totals field by field and the lines line
    /// by line, which is what the closing avoir is written for.
    ///
    /// It exists because a slice of a facture is not a fraction of it. The tax
    /// on each slice is rounded once, on that slice's own base, so three
    /// slices of a 179 facture come to 180 and three slices of a 286 one come
    /// to 285. Neither is wrong on its own paper and both are wrong added up,
    /// and what a shop and a comptable read is the sum: the avoirs on a
    /// facture have to reproduce it. So the last one is the difference, and it
    /// carries whatever the rounding left over.
    ///
    /// A line whose goods have all come back is dropped, and any centime still
    /// on it moves to a line of the same rate that has goods: that centime is
    /// the whole reason this exists, and a document line with no quantity is
    /// not a line a paper can print.
    fn whole(&self, facture: &Document) -> Result<(Totals, Vec<NewDocumentLine>), CoreError> {
        if self.foreign_rate {
            return Err(CoreError::validation(
                "lines",
                "an avoir on this facture carries a rate the facture does not",
            ));
        }
        let mut totals = self.totals.clone();
        totals.tva_by_rate = self
            .rates
            .iter()
            .filter(|r| r.on_the_recap)
            .map(|r| TvaLine {
                rate: r.rate,
                base: r.base,
                amount: r.amount,
            })
            .filter(|r: &TvaLine| r.base != Money::ZERO || r.amount != Money::ZERO)
            .collect();

        let mut lines = Vec::new();
        let mut stray: Vec<(Bps, Money)> = Vec::new();
        // Zipped rather than looked up: `of` built one `RemainingLine` per
        // facture line, in the facture's own order, so the two run together
        // and the closing avoir prints its lines in the order the facture
        // printed them.
        for (line, left) in facture.lines.iter().zip(&self.lines) {
            // A quantity below zero is an avoir crediting more of a line than
            // the line ever held, which is the file disagreeing with itself
            // and not a remainder to be clamped quietly.
            if left.qty_milli < 0 {
                return Err(CoreError::validation(
                    "lines",
                    "an avoir on this facture credits more of a line than it holds",
                ));
            }
            // A line with no goods left on it is not written at all: every
            // line of a document carries a quantity above zero, and a line of
            // nothing is not a line. Money still on such a line moves to a
            // line of the same rate that does have goods, so the avoir still
            // adds up to its own total and rate by rate. Its remaining line
            // discount goes with the line: that column describes what was
            // taken off goods, and the goods have all come back.
            if left.qty_milli == 0 {
                if left.line_total != Money::ZERO {
                    stray.push((line.rate_bps, left.line_total));
                }
                continue;
            }
            lines.push(NewDocumentLine {
                product_id: line.product_id,
                name: line.name.clone(),
                barcode: line.barcode.clone(),
                qty_milli: left.qty_milli,
                unit_price: line.unit_price,
                line_discount: left.line_discount,
                rate_bps: line.rate_bps,
                line_total: left.line_total,
                ref_line_id: Some(line.id),
            });
        }
        for (rate, amount) in stray {
            let Some(host) = lines.iter_mut().find(|l| l.rate_bps == rate) else {
                return Err(CoreError::validation(
                    "lines",
                    "what is left of this facture is money on a line that has no goods left",
                ));
            };
            host.line_total = host.line_total.checked_add(amount)?;
        }
        Ok((totals, lines))
    }
}

/// Whether the subtraction landed below zero anywhere. Every column of a
/// document and every row of its recap is an amount at or above zero, on the
/// paper and in the table alike.
fn below_zero(totals: &Totals) -> bool {
    totals.total_ht.is_negative()
        || totals.discount.is_negative()
        || totals.subtotal_ht.is_negative()
        || totals.tva.is_negative()
        || totals.total_ttc.is_negative()
        || totals
            .tva_by_rate
            .iter()
            .any(|r| r.base.is_negative() || r.amount.is_negative())
}

/// The facture lines that are coming back, each with the quantity and the
/// share of its line discount that comes with it.
///
/// `None` is every line that still has something left on it, which is what
/// finishes off a facture already credited in part. A named list is checked
/// line by line: a line of some other facture is not found, a quantity at or
/// below zero credits nothing, and a quantity past what the line has left is
/// goods that were never sold.
type Coming<'a> = Vec<(&'a DocumentLine, i64, Money)>;

fn chosen<'a>(
    facture: &'a Document,
    remaining: &Remaining,
    asked: Option<Vec<AvoirLine>>,
) -> Result<Coming<'a>, CoreError> {
    let mut coming: Coming<'a> = Vec::new();
    match asked {
        None => {
            for line in &facture.lines {
                let qty = remaining.quantity_on_line(line.id);
                if qty > 0 {
                    coming.push((line, qty, prorated_discount(line, qty)?));
                }
            }
        }
        Some(asked) => {
            for want in asked {
                let Some(line) = facture.lines.iter().find(|l| l.id == want.document_line_id)
                else {
                    return Err(CoreError::NotFound {
                        entity: "document_line",
                        id: want.document_line_id,
                    });
                };
                if coming.iter().any(|(l, _, _)| l.id == line.id) {
                    return Err(CoreError::validation(
                        "document_line_id",
                        "the same line is credited twice on one avoir",
                    ));
                }
                if want.qty_milli <= 0 {
                    return Err(CoreError::validation(
                        "qty_milli",
                        "a credited quantity is above zero",
                    ));
                }
                if want.qty_milli > remaining.quantity_on_line(line.id) {
                    return Err(CoreError::validation(
                        "qty_milli",
                        "more of this line is being credited than was sold and not yet credited",
                    ));
                }
                coming.push((
                    line,
                    want.qty_milli,
                    prorated_discount(line, want.qty_milli)?,
                ));
            }
        }
    }
    if coming.is_empty() {
        return Err(CoreError::validation(
            "lines",
            "there is nothing left to credit on this facture",
        ));
    }
    Ok(coming)
}

/// The share of a line's discount that comes back with part of that line
/// (`avoir_partial_prorates_discounts_floor`).
///
/// A line discount belongs to the whole line, so crediting a third of the line
/// credits a third of the discount. Rounded down, because the discount is what
/// the customer was not charged: rounding it up would take a centime off the
/// credit note that the customer never paid in the first place. A whole line
/// comes back with the whole of its discount and no rounding at all.
///
/// i128 keeps the product exact; two i64 factors can overflow i64.
fn prorated_discount(line: &DocumentLine, qty_milli: i64) -> Result<Money, CoreError> {
    if qty_milli >= line.qty_milli || line.line_discount == Money::ZERO {
        return Ok(line.line_discount);
    }
    let product = i128::from(line.line_discount.as_centimes())
        .checked_mul(i128::from(qty_milli))
        .ok_or(MoneyError::Overflow)?;
    let share = product
        .checked_div(i128::from(line.qty_milli))
        .ok_or(MoneyError::Overflow)?;
    Ok(i64::try_from(share)
        .map(Money::centimes)
        .map_err(|_| MoneyError::Overflow)?)
}

/// The totals of a partial avoir: its own lines, taxed as the part they are,
/// and held at every rate to what the facture has left there.
///
/// The slice's own arithmetic is the rule (features.md §3): a partial avoir is
/// taxed as the goods it credits and not as a share of the facture's tax, so
/// its base is its own HT and its tax is rounded once on that base. The cap is
/// the other rule, the one the running total on `total_ttc` already states,
/// read one column further in: no avoir gives back more base, more TVA or more
/// remise at a rate than the facture still has at that rate.
///
/// Without the cap the closing avoir, which is what is left of the facture,
/// goes negative. Two partials of a facture at two rates can round their tax
/// to a centime more than the facture charged at one of those rates, or take a
/// rate group's whole HT while the centime of remise the facture put on that
/// group stays behind, and the credit note that closes the facture is then
/// asked for a base or a tax below zero at a rate whose goods have all come
/// back. The cap spends the difference on the partial that caused it, where it
/// is one centime of rounding on a paper that is already rounding, rather than
/// leaving it for a document that cannot carry it at all (`avoir_prop`).
///
/// The droit de timbre is never on an avoir, so the stamp is zero and the net
/// is the TTC.
fn slice_totals(
    facture: &Document,
    remaining: &Remaining,
    lines: &[SliceLine],
) -> Result<Totals, CoreError> {
    let (groups, total_ht) = grouped(lines)?;

    let mut tva_by_rate = Vec::new();
    let mut discount = Money::ZERO;
    let mut tva = Money::ZERO;
    // Under the IFU a document shows no TVA at all, so there is no recap to
    // read a rate's remise off and nothing to hold at a rate either. The
    // remise is taken on the whole slice instead, the way it is spread by
    // `compute_totals`.
    if facture.regime != Regime::Reel {
        let share = global_discount(facture, remaining, total_ht)?.min(total_ht);
        let subtotal_ht = total_ht.checked_sub(share)?;
        return Ok(Totals {
            total_ht,
            discount: share,
            subtotal_ht,
            tva_by_rate,
            tva,
            total_ttc: subtotal_ht,
            stamp: Money::ZERO,
            net_to_pay: subtotal_ht,
        });
    }
    for (rate, ht) in &groups {
        // Every rate of the slice is a rate one of the facture's lines is at,
        // and `Remaining` carries a row for each of those, so a rate with no
        // row is a rate the facture never charged and has nothing left at.
        let (left_ht, left_share, left_tva) = match remaining.at(*rate) {
            Some(left) => (left.ht, left.ht.checked_sub(left.base)?, left.amount),
            None => (Money::ZERO, Money::ZERO, Money::ZERO),
        };
        // What this slice gives back of the facture's remise at this rate: its
        // proportional share rounded up, so a slice never leaves the rest of
        // the facture holding a remise it cannot place, and never more than
        // what is left.
        let share = if *ht >= left_ht {
            left_share.min(*ht)
        } else {
            up(left_share, *ht, left_ht)?.min(left_share)
        };
        let base = ht.checked_sub(share)?;
        let amount = base.pct(*rate)?.min(left_tva);
        discount = discount.checked_add(share)?;
        tva = tva.checked_add(amount)?;
        tva_by_rate.push(TvaLine {
            rate: *rate,
            base,
            amount,
        });
    }

    let subtotal_ht = total_ht.checked_sub(discount)?;
    let total_ttc = subtotal_ht.checked_add(tva)?;
    Ok(Totals {
        total_ht,
        discount,
        subtotal_ht,
        tva_by_rate,
        tva,
        total_ttc,
        stamp: Money::ZERO,
        net_to_pay: total_ttc,
    })
}

/// The share of the facture's global remise that comes back with these lines,
/// for a document with no TVA recap to spread it over: the credited HT against
/// what the facture has left, rounded up, and never more than the remise the
/// earlier avoirs have not taken.
///
/// Against what is left rather than against the whole, so the shares of the
/// successive avoirs come to the remise exactly and the last one takes
/// whatever the divisions left. Rounded up so that a slice never leaves the
/// rest of the facture holding a remise it has no HT left to put it on.
fn global_discount(
    facture: &Document,
    remaining: &Remaining,
    credited_ht: Money,
) -> Result<Money, CoreError> {
    if facture.totals.discount == Money::ZERO || facture.totals.total_ht == Money::ZERO {
        return Ok(Money::ZERO);
    }
    let left_discount = remaining.totals.discount;
    let left_ht = remaining.totals.total_ht;
    if left_discount <= Money::ZERO || left_ht <= Money::ZERO {
        return Ok(Money::ZERO);
    }
    if credited_ht >= left_ht {
        return Ok(left_discount);
    }
    Ok(up(left_discount, credited_ht, left_ht)?.min(left_discount))
}

/// One line of an avoir as the totals read it: the rate it is at and what it
/// comes to once the cap on its facture line has been applied.
pub struct SliceLine {
    rate: Bps,
    ht: Money,
}

/// HT per rate group of the lines coming back, by rising rate, and their sum:
/// the grouping the money functions do, on the slice's own lines.
fn grouped(lines: &[SliceLine]) -> Result<(Vec<(Bps, Money)>, Money), CoreError> {
    let mut groups: Vec<(Bps, Money)> = Vec::new();
    let mut total_ht = Money::ZERO;
    for line in lines {
        total_ht = total_ht.checked_add(line.ht)?;
        match groups.iter_mut().find(|(rate, _)| *rate == line.rate) {
            Some((_, ht)) => *ht = ht.checked_add(line.ht)?,
            None => groups.push((line.rate, line.ht)),
        }
    }
    groups.sort_by_key(|(rate, _)| *rate);
    Ok((groups, total_ht))
}

/// The HT the lines of one document carry at one rate.
fn ht_at(lines: &[DocumentLine], rate: Bps) -> Result<Money, CoreError> {
    let mut sum = Money::ZERO;
    for line in lines.iter().filter(|l| l.rate_bps == rate) {
        sum = sum.checked_add(line.line_total)?;
    }
    Ok(sum)
}

/// `part` of `whole` of an amount, rounded up. Every figure is at or above
/// zero here, so adding the divisor less one before dividing is the ceiling.
/// i128 keeps the product exact; two i64 factors can overflow i64.
fn up(amount: Money, part: Money, whole: Money) -> Result<Money, CoreError> {
    if amount == Money::ZERO || whole <= Money::ZERO {
        return Ok(Money::ZERO);
    }
    let divisor = i128::from(whole.as_centimes());
    let product = i128::from(amount.as_centimes())
        .checked_mul(i128::from(part.as_centimes()))
        .ok_or(MoneyError::Overflow)?
        .checked_add(divisor.checked_sub(1).ok_or(MoneyError::Overflow)?)
        .ok_or(MoneyError::Overflow)?;
    let share = product.checked_div(divisor).ok_or(MoneyError::Overflow)?;
    Ok(i64::try_from(share)
        .map(Money::centimes)
        .map_err(|_| MoneyError::Overflow)?)
}
