//! The till's one write: a sale becomes a fiscal document, its stock leaves
//! the ledger, and both commit together (features.md §1, Sale).
//!
//! M2 adds the sale on credit. It names a customer, it snapshots the buyer
//! block onto the document, it writes one `sale` movement on that customer's
//! ledger and it stores the balance triple the paper prints, all inside the
//! same transaction as the document and the stock. The facture-or-ticket
//! rule is in features.md §3; this issues a `ticket` whatever the payment.

use chrono::NaiveDateTime;
use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::customer::Customer;
use crate::models::debt::{DebtKind, NewDebtEntry};
use crate::models::document::{
    payment_mode_stored, BalanceTriple, Document, DocumentKind, NewDocument, NewDocumentLine,
    PartyBlock, SellerBlock,
};
use crate::models::stock::{Movement, MovementKind};
use crate::money::{
    compute_totals, Bps, Line, Money, MoneyError, PaymentMode, Regime, TotalsOptions,
};
use crate::services::{audit, clock, customers, debt, documents, products, settings, shops, stock};

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
    /// Who the sale is made out to. Required on credit, allowed on cash and
    /// on card: a named customer gets a buyer block on the document either
    /// way, and only a credit sale gets a ledger movement.
    pub customer_id: Option<i32>,
    /// The owner's decision to sell past the customer's credit limit. The
    /// sale goes through and the audit log carries who did it; until M4
    /// there are no roles, so anyone may send it (features.md §1).
    pub override_credit: bool,
    /// Unset means now on the shop's calendar (services::clock).
    pub issued_at: Option<NaiveDateTime>,
}

/// Why the till should say something while still handing over the ticket.
/// A warning never refuses a sale; the refusal is `CoreError::CreditLimit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Warning {
    /// The customer's balance after this sale reached their warn threshold.
    NearLimit,
}

impl Warning {
    /// The stable key the till translates, the way an error code is.
    pub const fn code(self) -> &'static str {
        match self {
            Warning::NearLimit => "near_limit",
        }
    }
}

/// The document that was issued, and what the till should say about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sale {
    pub document: Document,
    pub warning: Option<Warning>,
}

/// Issues the ticket: totals, numbering, the document with its lines and TVA
/// recap, one stock movement per line, and on credit the customer's ledger
/// movement, all in one transaction.
pub fn issue(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    new: NewSale,
) -> Result<Sale, CoreError> {
    if new.lines.is_empty() {
        return Err(CoreError::validation("lines", "a sale needs a line"));
    }
    if new.payment_mode == PaymentMode::Credit && new.customer_id.is_none() {
        // Money owed by nobody. The ledger is per customer (features.md §2),
        // so a credit sale with no fiche has nowhere to be owed from.
        return Err(CoreError::validation(
            "customer_id",
            "a credit sale is owed by a customer, so one has to be named",
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
            priced.push(price(conn, shop_id, regime, line)?);
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
        // the field named, so what is left is a basket whose amounts do not
        // fit. That is still the caller's arithmetic, so it is named too.
        let totals = compute_totals(
            &money_lines,
            &TotalsOptions {
                global_discount: new.global_discount,
                payment_mode: new.payment_mode,
                stamp_enabled: STAMP_ENABLED,
                regime,
            },
        )
        .map_err(too_large("lines"))?;

        let (tendered, change) = settle(new.payment_mode, new.tendered, totals.net_to_pay)?;

        // The customer, their standing, and what this sale does to what they
        // owe. All of it before `documents::issue` takes a number, so a sale
        // the credit limit refuses burns none (features.md §3, Numbering).
        let credit = match new.customer_id {
            None => None,
            Some(customer_id) => Some(credit_check(
                conn,
                shop_id,
                customer_id,
                new.payment_mode,
                totals.net_to_pay,
                new.override_credit,
            )?),
        };

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
                // A named customer is snapshotted onto the document the way
                // the seller is, on a ticket as much as on a facture: the
                // fiche is edited in place, and T4's facture switch then has
                // nothing to backfill.
                customer_id: new.customer_id,
                buyer: credit.as_ref().map(|c| buyer_block(&c.customer)),
                ref_document_id: None,
                balance: credit.as_ref().map(|c| c.balance),
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

        let Some(credit) = credit else {
            return Ok(Sale {
                document,
                warning: None,
            });
        };

        // The ledger movement, after the document exists so it can name it.
        // Only what the customer is left owing is written: a credit sale of a
        // basket a discount took to nothing owes nothing, and a movement of
        // zero would sit in every statement the customer is ever handed
        // (the same rule `customers::create` applies to an opening debt).
        if credit.balance.remaining_debt != Money::ZERO {
            debt::append(
                conn,
                shop_id,
                NewDebtEntry {
                    customer_id: credit.customer.id,
                    document_id: Some(document.id),
                    kind: DebtKind::Sale,
                    debit: credit.balance.remaining_debt,
                    credit: Money::ZERO,
                    user_id,
                    note: None,
                },
            )?;
        }

        // An override is a decision somebody took past a rule, which is
        // exactly what features.md §5 keeps a log for. Written here rather
        // than at the check because `after` is the document the decision
        // produced, and that document has no id until now.
        if credit.overridden {
            audit::record(
                conn,
                shop_id,
                user_id,
                audit::Change {
                    action: audit::ACTION_CREDIT_OVERRIDE,
                    entity: "sale",
                    entity_id: Some(document.id),
                    // `before` is the state the decision was taken against
                    // and nothing else: what the customer owed and what
                    // they were allowed to owe. The balance the sale left
                    // behind is not a before, it is what the override
                    // caused, so it sits in `after` beside the rest of it.
                    before: Some(
                        serde_json::json!({
                            "customer_id": credit.customer.id,
                            "balance_centimes": credit.balance.old_balance.as_centimes(),
                            "credit_limit_centimes":
                                credit.customer.credit_limit.map(Money::as_centimes),
                        })
                        .to_string(),
                    ),
                    // How it was paid and how much of it is owed are both
                    // here because a later avoir or a part payment changes
                    // the document without changing this row: what the
                    // decision was worth on the day stays readable.
                    after: Some(
                        serde_json::json!({
                            "document_id": document.id,
                            "balance_after_centimes": credit.balance.total_debt.as_centimes(),
                            "remaining_debt_centimes":
                                credit.balance.remaining_debt.as_centimes(),
                            "payment_mode": payment_mode_stored(document.payment_mode),
                            "warning": credit.warning.map(Warning::code),
                        })
                        .to_string(),
                    ),
                },
            )?;
        }

        Ok(Sale {
            document,
            warning: credit.warning,
        })
    })
}

/// The customer as the document will print them. Every field the buyer block
/// holds is a snapshot of the fiche on the day, `party_kind` included:
/// `facture_requires_party_ids` asks a different set of fields of a company
/// than of a consumer, and a reprint may not read that from a fiche somebody
/// has since edited.
fn buyer_block(customer: &Customer) -> PartyBlock {
    PartyBlock {
        name: customer.name.clone(),
        party_kind: customer.party_kind,
        rc: customer.rc.clone(),
        nif: customer.nif.clone(),
        nis: customer.nis.clone(),
        ai: customer.ai.clone(),
        address: customer.address.clone(),
    }
}

/// What the customer's standing decided: the fiche, the triple the document
/// stores, whether a limit was passed on purpose, and what the till should
/// say.
struct CreditCheck {
    customer: Customer,
    balance: BalanceTriple,
    overridden: bool,
    warning: Option<Warning>,
}

/// The credit rules of features.md §1, in the order they refuse.
///
/// A null credit limit is no limit at all and zero is no credit at all; a
/// null warn threshold is no warning. The test is on the balance the sale
/// would leave behind, not on the sale's own amount: a customer 100,00 under
/// their limit cannot buy 200,00 on credit however small each basket is.
fn credit_check(
    conn: &mut SqliteConnection,
    shop_id: i32,
    customer_id: i32,
    payment_mode: PaymentMode,
    net_to_pay: Money,
    override_credit: bool,
) -> Result<CreditCheck, CoreError> {
    // Reads through the service, so another shop's fiche is a NotFound here
    // rather than a buyer block printed on this shop's paper (rule 3).
    let customer = customers::get(conn, shop_id, customer_id)?;
    if !customer.active {
        return Err(CoreError::validation(
            "customer_id",
            "this customer's fiche is closed",
        ));
    }
    let old_balance = debt::balance(conn, shop_id, customer_id)?;
    // Cash and card leave the ledger where it was: the customer owes nothing
    // new, so this document's unpaid part is nothing.
    let remaining_debt = match payment_mode {
        PaymentMode::Credit => net_to_pay,
        PaymentMode::Cash | PaymentMode::Card => Money::ZERO,
    };
    let total_debt = old_balance.checked_add(remaining_debt)?;

    let mut overridden = false;
    if payment_mode == PaymentMode::Credit {
        if let Some(credit_limit) = customer.credit_limit {
            if total_debt > credit_limit {
                if !override_credit {
                    return Err(CoreError::CreditLimit {
                        balance_after: total_debt,
                        credit_limit,
                    });
                }
                overridden = true;
            }
        }
    }

    // At the threshold, not past it: a shop that sets one at 4 000,00 wants
    // to hear about the sale that reaches it.
    let warning = (payment_mode == PaymentMode::Credit
        && customer
            .warn_threshold
            .is_some_and(|threshold| total_debt >= threshold))
    .then_some(Warning::NearLimit);

    Ok(CreditCheck {
        customer,
        balance: BalanceTriple {
            old_balance,
            remaining_debt,
            total_debt,
        },
        overridden,
        warning,
    })
}

/// Turns an amount that does not fit into a validation error on the field
/// the caller sent. `CoreError::Money` is a 500, and 500 means the stored
/// file is at fault; a quantity and a price the caller chose whose product
/// is past i64 centimes is the caller's arithmetic, so it is a 422 with the
/// field named. Every other MoneyError keeps its meaning.
fn too_large(field: &'static str) -> impl Fn(MoneyError) -> CoreError {
    move |e| match e {
        MoneyError::Overflow => CoreError::validation(
            field,
            "this amount is past what the till can hold in centimes",
        ),
        other => CoreError::from(other),
    }
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
    regime: Regime,
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
    let gross = unit_price
        .checked_mul_milli(line.qty_milli)
        .map_err(too_large("qty_milli"))?;
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
        // Under the IFU the price is a single price and the document mentions
        // no TVA at all (fixture `regime_ifu_prints_no_tva`). The line stores
        // no rate either, so the stored document says so on its own and a
        // reprint never has to know the régime to hide one.
        rate_bps: match regime {
            Regime::Ifu => Bps::ZERO,
            Regime::Reel => product.rate_bps,
        },
        line_total: gross
            .checked_sub(line.line_discount)
            .map_err(too_large("line_discount"))?,
        cost: product.cost,
    })
}

/// The basket's HT, read back to compare the global discount against it.
/// A sum that does not fit is a basket the caller sent, so it is named as
/// one rather than raised as a money fault.
fn sum_line_totals(lines: &[Line]) -> Result<Money, CoreError> {
    let mut total = Money::ZERO;
    for line in lines {
        let gross = line
            .unit_price
            .checked_mul_milli(line.qty_milli)
            .map_err(too_large("qty_milli"))?;
        total = total
            .checked_add(gross.checked_sub(line.line_discount)?)
            .map_err(too_large("lines"))?;
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
