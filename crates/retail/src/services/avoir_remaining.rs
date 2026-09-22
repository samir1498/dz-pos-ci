//! The one subtraction every rule of an avoir is: the facture less every
//! avoir already written against it, read line by line, rate by rate and on
//! the totals (features.md §3).
//!
//! Lifted out of `services::avoir` on 2026-09-21 with no rule and no
//! arithmetic edited, because that file sits on a size pin and the cash
//! refund of ruling 5 had to go somewhere. One edge is drawn by the move,
//! `avoir -> avoir_remaining`, and none comes back: nothing here knows what
//! an avoir does with the answer.
//!
//! Nothing here is clamped, and that is the point. A file that disagrees with
//! itself, which is what a row written straight into the table looks like,
//! leaves a quantity or an amount below zero, and each caller decides what
//! that means: a partial avoir reads a line that owes nothing as nothing left
//! to give (the goods still come back, for no money), and the avoir that
//! closes the facture refuses, because the subtraction it is written from is
//! the file's own arithmetic.

use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::money::{Bps, Money, Totals, TvaLine};
use crate::services::documents;
use crate::services::documents::{Document, DocumentLine, NewDocumentLine};

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
pub(crate) struct Remaining {
    pub(crate) lines: Vec<RemainingLine>,
    pub(crate) rates: Vec<RemainingRate>,
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
    pub(crate) totals: Totals,
    /// An earlier avoir carries a rate in its recap that the facture's recap
    /// does not. Kept rather than raised: only the closing avoir is written
    /// from the rate rows, so only it has to refuse them.
    pub(crate) foreign_rate: bool,
}

/// What one facture line still has on it: the quantity, the money and the
/// share of the line discount that has not come back yet.
pub(crate) struct RemainingLine {
    pub(crate) id: i32,
    pub(crate) qty_milli: i64,
    pub(crate) line_total: Money,
    pub(crate) line_discount: Money,
}

/// What one rate of the facture still has on it: the HT of its lines, the
/// taxable base the facture charged there and the tax on it. The remise still
/// to give back at the rate is the first less the second.
pub(crate) struct RemainingRate {
    pub(crate) rate: Bps,
    pub(crate) ht: Money,
    pub(crate) base: Money,
    pub(crate) amount: Money,
    /// Whether the facture's TVA recap names this rate. Under the IFU a
    /// document shows no TVA at all, so every rate its lines are at is a rate
    /// with no row, and the closing avoir writes no recap either.
    pub(crate) on_the_recap: bool,
}

impl Remaining {
    pub(crate) fn of(
        conn: &mut SqliteConnection,
        shop_id: i32,
        facture: &Document,
    ) -> Result<Self, CoreError> {
        let earlier = documents::avoirs_of(conn, shop_id, facture.id)?;

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
    pub(crate) fn on_line(&self, line_id: i32) -> Option<&RemainingLine> {
        self.lines.iter().find(|line| line.id == line_id)
    }

    pub(crate) fn at(&self, rate: Bps) -> Option<&RemainingRate> {
        self.rates.iter().find(|r| r.rate == rate)
    }

    /// The quantity still on one facture line, and the money still on it,
    /// neither below zero: a line credited past what it held, or to the
    /// centime, has nothing left to give, and the quantity still on such a
    /// line comes back for no money.
    pub(crate) fn quantity_on_line(&self, line_id: i32) -> i64 {
        self.on_line(line_id)
            .map_or(0, |line| line.qty_milli.max(0))
    }

    pub(crate) fn money_on_line(&self, line_id: i32) -> Money {
        self.on_line(line_id)
            .map_or(Money::ZERO, |line| line.line_total.max(Money::ZERO))
    }

    /// Whether this avoir takes the last quantity off the facture, which is
    /// what decides how its money is computed.
    ///
    /// Every line of the facture has to end at nothing: a line still holding
    /// one unit is a facture that can be credited again, and the avoir being
    /// written is one more partial.
    pub(crate) fn closed_by(&self, coming: &Coming) -> bool {
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
    pub(crate) fn whole(
        &self,
        facture: &Document,
    ) -> Result<(Totals, Vec<NewDocumentLine>), CoreError> {
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
pub(crate) fn below_zero(totals: &Totals) -> bool {
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
pub(crate) type Coming<'a> = Vec<(&'a DocumentLine, i64, Money)>;

/// One line of an avoir as the totals read it: the rate it is at and what it
/// comes to once the cap on its facture line has been applied.
pub(crate) struct SliceLine {
    pub(crate) rate: Bps,
    pub(crate) ht: Money,
}

/// The HT the lines of one document carry at one rate.
pub(crate) fn ht_at(lines: &[DocumentLine], rate: Bps) -> Result<Money, CoreError> {
    let mut sum = Money::ZERO;
    for line in lines.iter().filter(|l| l.rate_bps == rate) {
        sum = sum.checked_add(line.line_total)?;
    }
    Ok(sum)
}
