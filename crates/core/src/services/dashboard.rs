//! The dashboard (features.md §1). One read, one day, and every figure on it
//! derived from the ledgers at the moment it is asked for: no column stores a
//! total, so there is nothing to keep in step and nothing that can be right on
//! one screen and wrong on another.
//!
//! The two columns
//! Every figure comes twice, for the day the caller named and for the month
//! that day falls in on the shop's calendar. The day is a slice of its own
//! month, so the day's figures plus the rest of the month are the month's,
//! and `dashboard_prop` is what holds that.
//!
//! The margin
//! What the papers asked for, less what the goods on them cost the shop.
//!
//! The revenue side is the lines' own HT, less the remise the paper gave off
//! the whole document: a shop that knocked 5 % off the bottom of a facture
//! did not collect that money, and a margin read before it would be higher
//! than the till was. The two halves come back beside the net as `lines_ht`
//! and `discounts`, so a screen can show where the difference went.
//!
//! The cost side reads the stock movements and never the fiche. What a unit
//! cost is what it cost when it left, written on the movement by the sale;
//! the fiche's cost price is the last delivery's and moves with every
//! purchase. A reversal writes the same figure back
//! (`an_avoir_returns_the_goods_at_the_cost_of_the_sale_it_reverses`), so a
//! credit note lowers the revenue and the cost together and the margin it
//! leaves is the margin of what the customer kept.
//!
//! What a figure is read from is one set of papers, named once in
//! `repos::dashboard`: still standing, a ticket, a facture or an avoir, and
//! not a credit note written against a paper that was annulled. A cancelled
//! sale leaves both sides of the margin at once, so it is felt exactly once.
//!
//! The cash position is not recomputed here. `cash::position` is the one
//! place that rule lives (features.md §1) and this screen calls it.

use std::collections::HashMap;

use chrono::NaiveDate;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::sql_types::DocumentKind;
use crate::money::Money;
use crate::repos::{dashboard as repo, debt as debt_repo, supplier_debt as supplier_repo};
use crate::services::cash::{self, CashPosition};
use crate::services::clock::{Month, Period};
use crate::services::expenses;

/// How many products a top list names. Ten is what fits beside the rest of
/// the screen; a shop wanting the whole catalogue exports it (T7).
const TOP: usize = 10;

/// What one stretch of days came to. Every field is derived; nothing here is
/// stored anywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Figures {
    /// What the tickets and factures of the period asked for, at
    /// `total_ttc`. Credit notes are not taken off it: this is what was sold
    /// over the counter, and what came back is the margin's business.
    pub sales_ttc: Money,
    /// How many tickets and factures that was.
    pub sales_count: i64,
    /// The HT of every line of every counted paper, an avoir's counted
    /// negative.
    pub lines_ht: Money,
    /// The remise given off the whole document, which no line carries.
    pub discounts: Money,
    /// `lines_ht` less `discounts`: the revenue the margin is read against.
    pub sales_ht: Money,
    /// What the goods on those papers cost the shop, at the cost they left
    /// on.
    pub cost_of_goods: Money,
    /// `sales_ht` less `cost_of_goods`.
    pub margin: Money,
    /// Money out that is not stock, over the same days.
    pub expenses: Money,
}

/// A product the shop is short of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowStock {
    pub product_id: i32,
    pub name: String,
    pub qty_on_hand_milli: i64,
    pub low_stock_at_milli: i64,
}

/// One product's month: how much of it went out the door, what it brought in
/// and what it left after its own cost.
///
/// The quantity is net of what came back, so a product sold and credited in
/// the same month reads the way it should: nothing moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopProduct {
    pub product_id: i32,
    pub name: String,
    pub qty_milli: i64,
    /// This product's lines, an avoir's counted negative. Named `lines_ht`
    /// and not `sales_ht` on purpose: a remise given off a whole document
    /// belongs to no line, so it is not here and the per product margins do
    /// not add up to `Figures::margin` on a period that carried one. The
    /// ranking is what this figure is for.
    pub lines_ht: Money,
    pub cost_of_goods: Money,
    /// `lines_ht` less `cost_of_goods`, before any whole document remise.
    pub margin: Money,
}

/// One side of the outstanding money: what is owed, and by or to how many.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Owed {
    /// The sum of the balances that are above zero. A party in credit is
    /// left out rather than netted off: money the shop is holding for one
    /// customer does not reduce what another one owes.
    pub total: Money,
    pub parties: i64,
}

/// Everything the dashboard shows, for one day and the month it falls in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dashboard {
    pub day: NaiveDate,
    pub month: Month,
    pub today: Figures,
    pub this_month: Figures,
    pub cash_today: CashPosition,
    pub cash_this_month: CashPosition,
    pub low_stock: Vec<LowStock>,
    /// The month's ten busiest products, most units first.
    pub top_by_quantity: Vec<TopProduct>,
    /// The month's ten most profitable products, most margin first.
    pub top_by_margin: Vec<TopProduct>,
    pub customer_debt: Owed,
    pub supplier_debt: Owed,
    pub open_purchases: i64,
}

/// The whole screen for one day on the shop's calendar.
pub fn read(
    conn: &mut SqliteConnection,
    shop_id: i32,
    day: NaiveDate,
) -> Result<Dashboard, CoreError> {
    let month = Month::of(day);
    let today = figures(conn, shop_id, Period::Day(day))?;
    let this_month = figures(conn, shop_id, Period::Month(month))?;
    let monthly = per_product(conn, shop_id, Period::Month(month))?;
    Ok(Dashboard {
        day,
        month,
        today,
        this_month,
        cash_today: cash::position(conn, shop_id, Period::Day(day))?,
        cash_this_month: cash::position(conn, shop_id, Period::Month(month))?,
        low_stock: repo::low_stock(conn, shop_id)?
            .into_iter()
            .map(|r| LowStock {
                product_id: r.product_id,
                name: r.name,
                qty_on_hand_milli: r.qty_on_hand_milli,
                low_stock_at_milli: r.low_stock_at_milli,
            })
            .collect(),
        top_by_quantity: top(&monthly, |p| i128::from(p.qty_milli)),
        top_by_margin: top(&monthly, |p| i128::from(p.margin.as_centimes())),
        customer_debt: owed(&debt_repo::balances(conn, shop_id)?)?,
        supplier_debt: owed(&supplier_repo::balances(conn, shop_id)?)?,
        open_purchases: repo::open_purchases(conn, shop_id)?,
    })
}

/// One stretch of days, summed.
fn figures(
    conn: &mut SqliteConnection,
    shop_id: i32,
    period: Period,
) -> Result<Figures, CoreError> {
    let (from, until) = period.moments()?;

    let mut sales_ttc = Money::ZERO;
    let mut sales_count = 0i64;
    let mut discounts = Money::ZERO;
    for row in repo::totals_by_kind(conn, shop_id, from, until)? {
        let ttc = Money::centimes(row.total_ttc_centimes);
        let discount = Money::centimes(row.discount_centimes);
        if row.kind == DocumentKind::Avoir {
            // A credit note's own remise comes off the credit, so it is
            // added back: the paper gave less than its lines say and the
            // reversal has to give less back too.
            discounts = discounts.checked_sub(discount)?;
            continue;
        }
        sales_ttc = sales_ttc.checked_add(ttc)?;
        sales_count = sales_count
            .checked_add(row.count)
            .ok_or(crate::money::MoneyError::Overflow)?;
        discounts = discounts.checked_add(discount)?;
    }

    let mut lines_ht = Money::ZERO;
    for line in repo::lines(conn, shop_id, from, until)? {
        lines_ht = add_signed(
            lines_ht,
            line.kind,
            Money::centimes(line.line_total_centimes),
        )?;
    }

    let mut cost_of_goods = Money::ZERO;
    for movement in repo::movements(conn, shop_id, from, until)? {
        cost_of_goods = cost_of_goods.checked_sub(cost_of(&movement)?)?;
    }

    let sales_ht = lines_ht.checked_sub(discounts)?;
    let expenses = match period {
        // The expenses table is keyed by a day and not by a moment, so the
        // day is read straight rather than through the two moments above.
        Period::Day(day) => expenses::total_on(conn, shop_id, day)?,
        Period::Month(month) => expenses::total(conn, shop_id, month)?,
    };
    Ok(Figures {
        sales_ttc,
        sales_count,
        lines_ht,
        discounts,
        sales_ht,
        cost_of_goods,
        margin: sales_ht.checked_sub(cost_of_goods)?,
        expenses,
    })
}

/// The same two sides, split by product, for the top lists.
///
/// Keyed by the product and not by the line, so a product sold twice in a
/// month is one entry. A line whose product has been deleted has no entry:
/// it is in the totals above and there is no fiche left for a list to name.
fn per_product(
    conn: &mut SqliteConnection,
    shop_id: i32,
    period: Period,
) -> Result<Vec<TopProduct>, CoreError> {
    let (from, until) = period.moments()?;
    let mut by_product: HashMap<i32, TopProduct> = HashMap::new();

    for line in repo::lines(conn, shop_id, from, until)? {
        let Some(product_id) = line.product_id else {
            continue;
        };
        let entry = by_product.entry(product_id).or_insert_with(|| TopProduct {
            product_id,
            name: line.name.clone(),
            qty_milli: 0,
            lines_ht: Money::ZERO,
            cost_of_goods: Money::ZERO,
            margin: Money::ZERO,
        });
        entry.lines_ht = add_signed(
            entry.lines_ht,
            line.kind,
            Money::centimes(line.line_total_centimes),
        )?;
    }

    for movement in repo::movements(conn, shop_id, from, until)? {
        let entry = by_product
            .entry(movement.product_id)
            .or_insert_with(|| TopProduct {
                product_id: movement.product_id,
                // A movement carries no name. Every movement of a sale has a
                // line beside it, so this is the fallback for a file whose
                // line was deleted from under it.
                name: String::new(),
                qty_milli: 0,
                lines_ht: Money::ZERO,
                cost_of_goods: Money::ZERO,
                margin: Money::ZERO,
            });
        // The ledger signs a sale negative; what went out the door is the
        // other way up, and a return takes it back off.
        entry.qty_milli = entry
            .qty_milli
            .checked_sub(movement.qty_milli)
            .ok_or(crate::money::MoneyError::Overflow)?;
        entry.cost_of_goods = entry.cost_of_goods.checked_sub(cost_of(&movement)?)?;
    }

    let mut products: Vec<TopProduct> = by_product.into_values().collect();
    for product in &mut products {
        product.margin = product.lines_ht.checked_sub(product.cost_of_goods)?;
    }
    Ok(products)
}

/// The ten highest by whatever the caller ranks them on, the product id
/// breaking a tie so two runs over one file never answer different lists.
///
/// A product whose month came to nothing at all is left out: sold and
/// credited back in the same month, it moved no units and brought in no
/// money, and a row of zeros on a list of the ten best reads as a product
/// that did something. It is still in the month's totals, where the sale and
/// its reversal cancel each other the same way.
fn top(products: &[TopProduct], by: impl Fn(&TopProduct) -> i128) -> Vec<TopProduct> {
    let mut ranked: Vec<&TopProduct> = products
        .iter()
        .filter(|p| p.qty_milli != 0 || p.lines_ht != Money::ZERO)
        .collect();
    ranked.sort_by(|a, b| by(b).cmp(&by(a)).then(a.product_id.cmp(&b.product_id)));
    ranked.into_iter().take(TOP).cloned().collect()
}

/// What one movement's goods cost, signed the way the ledger signs the
/// quantity: negative on the way out, positive on the way back.
fn cost_of(movement: &repo::MovementRow) -> Result<Money, CoreError> {
    Ok(Money::centimes(movement.unit_cost_centimes).checked_mul_milli(movement.qty_milli)?)
}

/// An avoir's line lowers the figure it is added to; a ticket's and a
/// facture's raise it.
fn add_signed(total: Money, kind: DocumentKind, amount: Money) -> Result<Money, CoreError> {
    if kind == DocumentKind::Avoir {
        return Ok(total.checked_sub(amount)?);
    }
    Ok(total.checked_add(amount)?)
}

/// The positive balances of one ledger, summed and counted. `balances` hands
/// back the debit and the credit of each party; what is owed is the first
/// less the second.
fn owed(balances: &[(i32, i64, i64)]) -> Result<Owed, CoreError> {
    let mut total = Money::ZERO;
    let mut parties = 0i64;
    for (_, debit, credit) in balances {
        let balance = Money::centimes(*debit).checked_sub(Money::centimes(*credit))?;
        if balance.as_centimes() <= 0 {
            continue;
        }
        total = total.checked_add(balance)?;
        parties = parties
            .checked_add(1)
            .ok_or(crate::money::MoneyError::Overflow)?;
    }
    Ok(Owed { total, parties })
}
