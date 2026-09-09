//! The till's one write: a sale becomes a fiscal document, its stock leaves
//! the ledger, and both commit together (features.md §1, Sale).
//!
//! M1 issues a `ticket` and nothing else. The facture-or-ticket rule is in
//! features.md §3; the buyer block it needs arrives with customers in M2.

use chrono::NaiveDateTime;
use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::document::{Document, DocumentKind, NewDocument, NewDocumentLine, SellerBlock};
use crate::models::stock::{Movement, MovementKind};
use crate::money::{compute_totals, Line, Money, PaymentMode, TotalsOptions};
use crate::services::{clock, documents, products, settings, shops, stock};

/// Whether the droit de timbre applies at all. There is no shop setting for
/// it yet; a cash payment is still what makes it due (features.md,
/// `stamp_progressive_tranches`). It becomes a setting the day a shop needs
/// to turn it off, not before.
const STAMP_ENABLED: bool = true;

/// One line of the basket. `unit_price` unset takes the product's selling
/// price, so a till that shows the price and a till that overrides it send
/// the same shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSaleLine {
    pub product_id: i32,
    /// Thousandths of the unit: 1,5 kg is 1500.
    pub qty_milli: i64,
    pub unit_price: Option<Money>,
    pub line_discount: Money,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSale {
    pub lines: Vec<NewSaleLine>,
    pub global_discount: Money,
    pub payment_mode: PaymentMode,
    /// What the customer handed over. Cash only.
    pub tendered: Option<Money>,
    /// Unset means now on the shop's calendar (services::clock).
    pub issued_at: Option<NaiveDateTime>,
}

/// Issues the ticket: totals, numbering, the document with its lines and TVA
/// recap, and one stock movement per line, all in one transaction.
pub fn issue(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    new: NewSale,
) -> Result<Document, CoreError> {
    if new.lines.is_empty() {
        return Err(CoreError::validation("lines", "a sale needs a line"));
    }
    if new.payment_mode == PaymentMode::Credit {
        // A credit sale goes on a customer's ledger, and there are no
        // customers before M2 (features.md §2). Taken now it would be money
        // owed by nobody.
        return Err(CoreError::validation(
            "payment_mode",
            "a credit sale needs a customer, which this version does not have",
        ));
    }
    if new.global_discount.is_negative() {
        return Err(CoreError::validation(
            "global_discount",
            "a discount cannot be negative",
        ));
    }

    conn.transaction(|conn| {
        let issued_at = new.issued_at.unwrap_or_else(clock::now);
        let regime = settings::regime_as_of(conn, shop_id, issued_at)?;
        let seller = SellerBlock::from(shops::get(conn, shop_id)?);

        let mut priced = Vec::with_capacity(new.lines.len());
        for line in &new.lines {
            priced.push(price(conn, shop_id, line)?);
        }

        let money_lines: Vec<Line> = priced
            .iter()
            .map(|p| Line {
                qty_milli: p.qty_milli,
                unit_price: p.unit_price,
                line_discount: p.line_discount,
                rate: p.rate_bps,
            })
            .collect();
        let total_ht = sum_line_totals(&money_lines)?;
        if new.global_discount > total_ht {
            return Err(CoreError::validation(
                "global_discount",
                "a discount above the basket would make the sale negative",
            ));
        }
        // Every input compute_totals could refuse has been refused above with
        // the field named, so a MoneyError out of here is a real overflow and
        // the API answers 500 rather than blaming the caller.
        let totals = compute_totals(
            &money_lines,
            &TotalsOptions {
                global_discount: new.global_discount,
                payment_mode: new.payment_mode,
                stamp_enabled: STAMP_ENABLED,
                regime,
            },
        )?;

        let (tendered, change) = settle(new.payment_mode, new.tendered, totals.net_to_pay)?;

        let document = documents::issue(
            conn,
            shop_id,
            NewDocument {
                kind: DocumentKind::Ticket,
                issued_at,
                user_id,
                regime,
                payment_mode: new.payment_mode,
                seller,
                customer_id: None,
                totals,
                tendered,
                change,
                lines: priced
                    .iter()
                    .map(|p| NewDocumentLine {
                        product_id: Some(p.product_id),
                        name: p.name.clone(),
                        barcode: p.barcode.clone(),
                        qty_milli: p.qty_milli,
                        unit_price: p.unit_price,
                        line_discount: p.line_discount,
                        rate_bps: p.rate_bps,
                        line_total: p.line_total,
                    })
                    .collect(),
            },
        )?;

        // Stock leaves after the document exists, so every movement names the
        // document that moved it. The count may end up below zero: a shop's
        // count is often wrong before its first inventory, and refusing the
        // sale would stop the till over a number nobody typed in.
        for p in &priced {
            let out = p
                .qty_milli
                .checked_neg()
                .ok_or(crate::money::MoneyError::Overflow)?;
            stock::record(
                conn,
                shop_id,
                &Movement {
                    product_id: p.product_id,
                    kind: MovementKind::Sale,
                    qty_milli: out,
                    unit_cost: p.cost,
                    document_id: Some(document.id),
                    user_id,
                },
            )?;
        }

        Ok(document)
    })
}

/// A line with its product read and its price settled.
struct PricedLine {
    product_id: i32,
    name: String,
    barcode: Option<String>,
    qty_milli: i64,
    unit_price: Money,
    line_discount: Money,
    rate_bps: crate::money::Bps,
    line_total: Money,
    cost: Money,
}

fn price(
    conn: &mut SqliteConnection,
    shop_id: i32,
    line: &NewSaleLine,
) -> Result<PricedLine, CoreError> {
    let product = products::get(conn, shop_id, line.product_id)?;
    if !product.active {
        return Err(CoreError::validation(
            "product_id",
            "this product is not on sale",
        ));
    }
    if line.qty_milli <= 0 {
        return Err(CoreError::validation(
            "qty_milli",
            "a sold quantity is above zero",
        ));
    }
    let unit_price = line.unit_price.unwrap_or(product.selling);
    if unit_price.is_negative() {
        return Err(CoreError::validation(
            "unit_price",
            "a price cannot be negative",
        ));
    }
    if line.line_discount.is_negative() {
        return Err(CoreError::validation(
            "line_discount",
            "a discount cannot be negative",
        ));
    }
    // The rounded gross, the same one compute_totals works from: a discount
    // compared against the unrounded product would pass here and be refused
    // a centime later as a MoneyError, which the API reads as a 500.
    let gross = unit_price.checked_mul_milli(line.qty_milli)?;
    if line.line_discount > gross {
        return Err(CoreError::validation(
            "line_discount",
            "a line discount above its own line",
        ));
    }
    Ok(PricedLine {
        product_id: product.id,
        name: product.name,
        barcode: product.barcode,
        qty_milli: line.qty_milli,
        unit_price,
        line_discount: line.line_discount,
        rate_bps: product.rate_bps,
        line_total: gross.checked_sub(line.line_discount)?,
        cost: product.cost,
    })
}

fn sum_line_totals(lines: &[Line]) -> Result<Money, CoreError> {
    let mut total = Money::ZERO;
    for line in lines {
        let gross = line.unit_price.checked_mul_milli(line.qty_milli)?;
        total = total.checked_add(gross.checked_sub(line.line_discount)?)?;
    }
    Ok(total)
}

/// What was handed over and what goes back. Cash has to cover the net to
/// pay; a card leaves both columns empty, since the terminal takes the exact
/// amount and there is nothing to give back.
fn settle(
    mode: PaymentMode,
    tendered: Option<Money>,
    net_to_pay: Money,
) -> Result<(Option<Money>, Option<Money>), CoreError> {
    match mode {
        PaymentMode::Cash => {
            let Some(tendered) = tendered else {
                return Err(CoreError::validation(
                    "tendered",
                    "a cash sale records what the customer handed over",
                ));
            };
            if tendered < net_to_pay {
                return Err(CoreError::validation(
                    "tendered",
                    "less than the amount to pay",
                ));
            }
            Ok((Some(tendered), Some(tendered.checked_sub(net_to_pay)?)))
        }
        // Refused rather than dropped: an amount the caller sent and the
        // document does not hold is money nobody can account for later.
        _ if tendered.is_some() => Err(CoreError::validation(
            "tendered",
            "only a cash sale records an amount tendered",
        )),
        _ => Ok((None, None)),
    }
}
