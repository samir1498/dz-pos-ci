//! The till's one write: a sale becomes a fiscal document, its stock leaves
//! the ledger, and both commit together (features.md §1, Sale).
//!
//! A sale on credit names a customer. It snapshots the buyer block onto the
//! document, writes one `sale` movement on that customer's ledger and stores
//! the balance triple the paper prints, all inside the same transaction as
//! the document and the stock.
//!
//! The caller also chooses the paper (features.md §3). Loi 04-02 art. 10, as
//! rewritten by loi 10-06 art. 3, decides ticket against facture by who the
//! buyer is and never by an amount or by how the sale is paid, so the
//! operator names the kind on the request and a facture is refused unless
//! both party blocks carry what décret 05-468 art. 3 asks of them.

use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::error::{CoreError, RetailError};
use crate::models::debt::{DebtKind, NewDebtEntry};
use crate::models::document::{
    payment_mode_stored, BalanceTriple, Document, NewDocument, NewDocumentLine, SellerBlock,
};
use crate::models::stock::{Movement, MovementKind};
use crate::money::{compute_totals, Line, Money, PaymentMode, TotalsOptions};
use crate::repos::sale_idempotency as idempotency;
use crate::services::pricing::{
    buyer_block, money_lines, price_lines, sum_line_totals, too_large, STAMP_ENABLED,
};
use crate::services::{
    customers, debt, discount_threshold, documents, party_ids, proforma, shifts, stock,
};
use crate::{audit_actions, models::customer::ProvedCustomer};
use dzpos_kernel::services::permissions::{self, Permission};
use dzpos_kernel::services::{audit, clock, role_of, settings, shops};

/// The basket a till sends, and the paper it asks for. They live in
/// `services::pricing`, under both this module and `proforma`, and are named
/// here because `services::sales::NewSale` is what every caller writes.
pub use crate::services::pricing::{NewSale, NewSaleLine, SaleKind};

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
    /// True when no new ring happened: the key was seen before and the
    /// stored paper is answered again. A replay answers the paper, not the
    /// chatter — the warning travels on the first ring only.
    pub replayed: bool,
}

/// Client retry keys live at most this long on the wire. Longer is not more
/// unique, only more room for a pasted secret to travel in.
pub const MAX_IDEMPOTENCY_KEY_LEN: usize = 128;

/// The edge calls this for a presented key: present, non-empty, short.
pub fn validate_idempotency_key(key: &str) -> Result<(), CoreError> {
    if key.is_empty() {
        return Err(CoreError::validation(
            "idempotency_key",
            "a retry key is the client's promise this ring is one sale; an empty one promises nothing",
        ));
    }
    if key.len() > MAX_IDEMPOTENCY_KEY_LEN {
        return Err(CoreError::validation(
            "idempotency_key",
            "a retry key is at most 128 characters",
        ));
    }
    Ok(())
}

/// The ring a key was first seen with, fingerprinted. Same key plus same
/// fingerprint replays the stored paper; same key plus a different one is a
/// client reusing a key across sales and is refused. Only the request is
/// hashed, never the stamped answer: `issued_at` unset on both tries is the
/// ordinary retry and must match.
fn fingerprint(new: &NewSale) -> String {
    use sha2::{Digest, Sha256};
    let mut parts = String::new();
    for line in &new.lines {
        parts.push_str(&format!(
            "{}:{}:{}:{};",
            line.product_id,
            line.qty_milli,
            line.unit_price.map(Money::as_centimes).unwrap_or(-1),
            line.line_discount.as_centimes()
        ));
    }
    let mode = payment_mode_stored(new.payment_mode);
    let kind = match new.kind {
        SaleKind::Ticket => "ticket",
        SaleKind::Facture => "facture",
        SaleKind::Proforma => "proforma",
    };
    parts.push_str(&format!(
        "{}|{}|{}|{}|{}|{}|{:?}",
        new.global_discount.as_centimes(),
        mode,
        new.tendered.map(Money::as_centimes).unwrap_or(-1),
        new.customer_id.unwrap_or(-1),
        new.override_credit,
        kind,
        new.issued_at
    ));
    let mut hex = String::with_capacity(64);
    for b in Sha256::digest(parts.as_bytes()) {
        use std::fmt::Write;
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

/// Issues the ticket: totals, numbering, the document with its lines and TVA
/// recap, one stock movement per line, and on credit the customer's ledger
/// movement, all in one transaction.
/// Rings one basket, no promise attached: today's desktop behavior. A call
/// that got no answer and carries a retry key goes through
/// [`issue_idempotent`] instead, which is this with the dedup around it.
pub fn issue(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    new: NewSale,
) -> Result<Sale, RetailError> {
    issue_inner(conn, shop_id, user_id, new, None)
}

/// Rings one basket, promising it is one sale (M7 T4): a retry after a lost
/// answer carries the same key and gets the original sale back instead of
/// ringing twice. A key on a quotation is refused — it moves no money, so
/// there is nothing to dedupe, and dropping the promise in silence would be
/// worse.
pub fn issue_idempotent(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    new: NewSale,
    idempotency_key: String,
) -> Result<Sale, RetailError> {
    validate_idempotency_key(&idempotency_key)?;
    issue_inner(conn, shop_id, user_id, new, Some(idempotency_key))
}

fn issue_inner(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    new: NewSale,
    idempotency_key: Option<String>,
) -> Result<Sale, RetailError> {
    // A quotation is not a sale: it writes the document and stops. Handed
    // over before any of the rules below, because none of them is about it,
    // and it never warns about a credit limit it does not move.
    if new.kind == SaleKind::Proforma {
        if idempotency_key.is_some() {
            return Err(RetailError::validation(
                "idempotency_key",
                "a quotation takes no retry key; only ticket and facture rings do",
            ));
        }
        return proforma::issue(conn, shop_id, user_id, new).map(|document| Sale {
            document,
            warning: None,
            replayed: false,
        });
    }
    if new.lines.is_empty() {
        return Err(RetailError::validation("lines", "a sale needs a line"));
    }
    if new.payment_mode == PaymentMode::Credit && new.customer_id.is_none() {
        // Money owed by nobody. The ledger is per customer (features.md §2),
        // so a credit sale with no fiche has nowhere to be owed from.
        return Err(RetailError::validation(
            "customer_id",
            "a credit sale is owed by a customer, so one has to be named",
        ));
    }
    if new.kind == SaleKind::Facture && new.customer_id.is_none() {
        // Décret 05-468 art. 3 puts the buyer on the paper, and the fiche is
        // where the buyer block comes from. Refused on the field the caller
        // sent rather than as `party_ids`: nothing is missing from a block
        // here, there is no block at all.
        return Err(RetailError::validation(
            "customer_id",
            "a facture is made out to a customer, so one has to be named",
        ));
    }
    if new.global_discount.is_negative() {
        return Err(RetailError::validation(
            "global_discount",
            "a discount cannot be negative",
        ));
    }

    // Read off `new` before the closure takes it: the blocked row below is
    // written after the transaction has gone, and it names the customer the
    // limit belongs to.
    let customer_id = new.customer_id;

    // What the credit limit refused, if it refused. It lives out here, in
    // memory, precisely because memory does not roll back: the transaction
    // below unwinds every row it wrote, and the facts of the refusal have to
    // outlive it to be written down at all.
    let mut refused: Option<Refused> = None;

    // The same thing for the two ways money comes off a price. Same reason,
    // same lifetime: out here so the rollback below cannot take it.
    let mut price_refused: Option<PriceRefused> = None;

    let issued = conn.transaction(|conn| {
        let issued_at = new.issued_at.unwrap_or_else(clock::now);
        // A retry after a lost answer carries the key of a ring already
        // stored: answer the stored paper instead of ringing twice. Same key
        // on a different ring is a client reusing keys and is refused —
        // handing back a neighbor's sale in silence is the bug this table
        // exists to prevent. First, so a replay burns no number and writes
        // no row of any kind.
        if let Some(key) = &idempotency_key {
            let hash = fingerprint(&new);
            if let Some(hit) = idempotency::find(conn, shop_id, key)? {
                if hit.request_hash != hash {
                    return Err(RetailError::conflict(
                        "idempotency_key",
                        "this retry key already rang a different sale",
                    ));
                }
                let document = documents::get(conn, shop_id, hit.sale_id)?;
                return Ok(Sale {
                    document,
                    warning: None,
                    replayed: true,
                });
            }
        }
        let regime = settings::regime_as_of(conn, shop_id, issued_at)?;
        let seller = SellerBlock::from(shops::get(conn, shop_id)?);

        let priced = price_lines(conn, shop_id, regime, &new.lines)?;
        let money_lines = money_lines(&priced);
        let total_ht = sum_line_totals(&money_lines)?;
        if new.global_discount > total_ht {
            return Err(RetailError::validation(
                "global_discount",
                "a discount above the basket would make the sale negative",
            ));
        }

        // The discount rule of features.md §5, on the basket and not on a
        // line: the threshold is a percentage of what the basket was worth
        // before anything came off it, and every line discount is counted
        // with the global one, so a discount split across the lines cannot
        // duck a threshold the same discount would meet in one place.
        // Checked here, before `documents::issue`, so a refusal burns no
        // number; the row it writes is further down, where the document it
        // produced has an id.
        // A price typed over the product's own is the other way the same
        // money comes off, so it is gated beside the discount rather than
        // after it: a cashier refused a 10 % discount and allowed to type
        // the discounted price is not refused at all (M1 carry-in,
        // 2026-09-09). Checked before `documents::issue`, so a refusal burns
        // no number; the row it writes is further down, where the document
        // has an id.
        let negotiated: Vec<serde_json::Value> = priced
            .iter()
            .filter(|line| line.unit_price != line.stored_price)
            .map(|line| {
                serde_json::json!({
                    "product_id": line.product_id,
                    "name": line.name,
                    "card_price_centimes": line.stored_price.as_centimes(),
                    "charged_centimes": line.unit_price.as_centimes(),
                    "qty_milli": line.qty_milli,
                })
            })
            .collect();
        if !negotiated.is_empty() {
            let role = role_of(conn, shop_id, user_id)?;
            if !permissions::can(role, Permission::ChangePriceAtTheTill) {
                price_refused = Some(PriceRefused {
                    permission: Permission::ChangePriceAtTheTill,
                    detail: serde_json::json!({ "lines": negotiated.clone() }),
                });
            }
            permissions::require(role, Permission::ChangePriceAtTheTill)?;
        }

        let basket = sum_line_gross(&money_lines)?;
        let discount = sum_discounts(&money_lines, new.global_discount)?;
        let threshold = discount_threshold::discount_threshold_as_of(conn, shop_id, issued_at)?;
        let discounted_past_threshold =
            permissions::discount_needs_permission(basket, discount, threshold)?;
        if discounted_past_threshold {
            let role = role_of(conn, shop_id, user_id)?;
            if !permissions::can(role, Permission::DiscountAboveThreshold) {
                price_refused = Some(PriceRefused {
                    permission: Permission::DiscountAboveThreshold,
                    detail: serde_json::json!({
                        "basket_centimes": basket.as_centimes(),
                        "discount_centimes": discount.as_centimes(),
                        "threshold_bps": threshold.as_u32(),
                    }),
                });
            }
            permissions::require(role, Permission::DiscountAboveThreshold)?;
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
                user_id,
                customer_id,
                CreditAsk {
                    payment_mode: new.payment_mode,
                    net_to_pay: totals.net_to_pay,
                    override_credit: new.override_credit,
                },
                &mut refused,
            )?),
        };

        // A named customer is snapshotted onto the document the way the
        // seller is, on a ticket as much as on a facture: the fiche is
        // edited in place, and a reprint months later has to hand back the
        // block the buyer was given.
        let buyer = credit.as_ref().map(|c| buyer_block(c.customer.fiche()));
        // Still before `documents::issue`, so a facture the identifiers
        // refuse burns no number of either series (features.md, Numbering).
        if new.kind == SaleKind::Facture {
            party_ids::check(&seller, buyer.as_ref())?;
        }

        let document = documents::issue(
            conn,
            shop_id,
            NewDocument {
                kind: new.kind.document_kind(),
                issued_at,
                user_id,
                regime,
                payment_mode: new.payment_mode,
                seller,
                customer: credit.as_ref().map(|c| c.customer.clone()),
                buyer,
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
                        // A sold line credits nothing; only an avoir line
                        // names the line it is written against.
                        ref_line_id: None,
                    })
                    .collect(),
            },
        )?;

        // The key lands in the same transaction as the sale: a crash between
        // the two is a sale with no key, which a retry would ring again.
        // Lost the race with a same-key ring (UNIQUE) means the winner
        // committed first — answer 409 and let the client read that sale
        // with the same key rather than inventing a second one here.
        if let Some(key) = &idempotency_key {
            let hash = fingerprint(&new);
            if !idempotency::record(conn, shop_id, key, document.id, &hash, issued_at)? {
                return Err(RetailError::conflict(
                    "idempotency_key",
                    "this key just rang on another call; send it again to read that sale",
                ));
            }
        }

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

        // A discount past the threshold is a decision somebody took past a
        // rule, the same as an override of a credit block, so it is logged
        // the same way: `before` is what the rule allowed, `after` is what
        // was given. Written above the credit block below because a cash
        // sale returns there and a discount is not a thing only a credit
        // sale can carry.
        // What the product's card says, against what the customer was
        // actually charged, line by line. One row per document rather than
        // one per line: the decision was taken once, at the till, over a
        // basket.
        if !negotiated.is_empty() {
            audit::record(
                conn,
                shop_id,
                user_id,
                audit::Change {
                    action: audit_actions::ACTION_PRICE_OVERRIDE,
                    entity: "document",
                    entity_id: Some(document.id),
                    before: Some(serde_json::json!({ "lines": negotiated.len() }).to_string()),
                    after: Some(
                        serde_json::json!({
                            "document_id": document.id,
                            "lines": negotiated,
                        })
                        .to_string(),
                    ),
                },
            )?;
        }

        if discounted_past_threshold {
            audit::record(
                conn,
                shop_id,
                user_id,
                audit::Change {
                    action: audit_actions::ACTION_DISCOUNT_OVERRIDE,
                    entity: "document",
                    entity_id: Some(document.id),
                    // The threshold in force on the day travels beside the
                    // amount it allowed: a later change to the setting must
                    // not make this row unreadable, and a reader asking why
                    // 5 % was refused should not have to walk the dated
                    // history to find out it was 2 % that day.
                    before: Some(
                        serde_json::json!({
                            "basket_centimes": basket.as_centimes(),
                            "threshold_bps": threshold.as_u32(),
                            "allowed_discount_centimes": basket.pct(threshold)?.as_centimes(),
                        })
                        .to_string(),
                    ),
                    // Split as well as summed: the threshold is tested on the
                    // whole, and a reader still wants to see whether it was
                    // one line or the basket that carried it.
                    after: Some(
                        serde_json::json!({
                            "document_id": document.id,
                            "discount_centimes": discount.as_centimes(),
                            "global_discount_centimes": new.global_discount.as_centimes(),
                            "line_discount_centimes":
                                discount.checked_sub(new.global_discount)?.as_centimes(),
                        })
                        .to_string(),
                    ),
                },
            )?;
        }

        // Ruling 2, and above the cash arm's early return below or a ticket
        // never reaches it; `shifts::tag_if_outside_a_shift` says the rest.
        shifts::tag_if_outside_a_shift(conn, shop_id, user_id, document.id, issued_at)?;

        let Some(credit) = credit else {
            return Ok(Sale {
                document,
                warning: None,
                replayed: false,
            });
        };

        // The ledger movement, after the document exists so it can name it.
        // The whole of what the sale put on the account is written, not what
        // the document is left asking for: credit the customer was holding is
        // already a movement here, and writing only the unsettled part would
        // count that credit a second time and leave the balance short by it.
        //
        // A credit sale of a basket a discount took to nothing puts nothing on
        // the account, and a movement of zero would sit in every statement the
        // customer is ever handed (the same rule `customers::create` applies to
        // an opening debt).
        //
        // Stamped with the document's own `issued_at` and not with the clock
        // at the moment of the write: the paper and the movement are one
        // event, and a sale rung up in the last second of a day would
        // otherwise put its facture on one day and its debt on the next, so
        // the statement of the day the customer was handed the paper would
        // close without the movement that paper made.
        if credit.added != Money::ZERO {
            debt::append_at(
                conn,
                shop_id,
                NewDebtEntry {
                    customer_id: credit.customer.id(),
                    document_id: Some(document.id),
                    kind: DebtKind::Sale,
                    debit: credit.added,
                    credit: Money::ZERO,
                    user_id,
                    note: None,
                },
                Some(issued_at),
            )?;
        }

        // And the credit the customer was already holding is placed on the
        // document it just settled, so the ledger says which paper it went to
        // rather than leaving the two to be netted out by whoever reads them.
        debt::settle_from_credit(
            conn,
            shop_id,
            credit.customer.id(),
            document.id,
            credit.consumed,
        )?;

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
                    action: audit_actions::ACTION_CREDIT_OVERRIDE,
                    // The row is about the document the decision produced,
                    // which is what `entity_id` names, so it says `document`
                    // like every other row about one. A reader after the
                    // history of a facture asks for one entity, not three.
                    entity: "document",
                    entity_id: Some(document.id),
                    // `before` is the state the decision was taken against
                    // and nothing else: what the customer owed and what
                    // they were allowed to owe. The balance the sale left
                    // behind is not a before, it is what the override
                    // caused, so it sits in `after` beside the rest of it.
                    before: Some(
                        serde_json::json!({
                            "customer_id": credit.customer.id(),
                            "balance_centimes": credit.balance.old_balance.as_centimes(),
                            "credit_limit_centimes":
                                credit.customer.fiche().credit_limit.map(Money::as_centimes),
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

        // A sale that crossed the customer's warn threshold and was not an
        // override. An override already carries the warning in its own row,
        // and the log is read one row per sale (M2 carry-in, 2026-09-09).
        // Inside the transaction, unlike the blocked row below, because a
        // sale that warns is a sale that happened.
        if credit.warning.is_some() && !credit.overridden {
            audit::record(
                conn,
                shop_id,
                user_id,
                audit::Change {
                    action: audit_actions::ACTION_CREDIT_WARNED,
                    entity: "document",
                    entity_id: Some(document.id),
                    before: Some(
                        serde_json::json!({
                            "customer_id": credit.customer.id(),
                            "balance_centimes": credit.balance.old_balance.as_centimes(),
                            "warn_threshold_centimes":
                                credit.customer.fiche().warn_threshold.map(Money::as_centimes),
                            "credit_limit_centimes":
                                credit.customer.fiche().credit_limit.map(Money::as_centimes),
                        })
                        .to_string(),
                    ),
                    after: Some(
                        serde_json::json!({
                            "document_id": document.id,
                            "balance_after_centimes": credit.balance.total_debt.as_centimes(),
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
            replayed: false,
        })
    });

    // The refusal the credit limit raised unwound everything the closure
    // wrote, a row about the refusal included, which is why one was never
    // written there. A shop that cannot see a cashier trying a customer's
    // limit over and over has no control at all (M2 carry-in, 2026-09-09),
    // so the row goes down here, on the same connection, after the rollback.
    //
    // The refusal wins if this write fails. A lost row is a lost row; a till
    // told "the database is unwell" when what actually happened is that the
    // customer is over their limit sends the cashier to the wrong person.
    if let (Err(_), Some(refusal)) = (&issued, &refused) {
        let _ = audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: audit_actions::ACTION_CREDIT_BLOCKED,
                // The customer, not a document: the refusal produced none.
                entity: "customer",
                entity_id: customer_id,
                before: Some(
                    serde_json::json!({
                        "customer_id": customer_id,
                        "credit_limit_centimes": refusal.credit_limit.as_centimes(),
                        "balance_centimes": refusal.balance_before.as_centimes(),
                    })
                    .to_string(),
                ),
                // `asked_to_override` is the difference between a cashier
                // who rang a sale up and was stopped, and a cashier who sent
                // the override flag and was stopped because they are not
                // allowed to send it. A shop reading its log wants to tell
                // those two apart.
                after: Some(
                    serde_json::json!({
                        "balance_would_be_centimes": refusal.balance_after.as_centimes(),
                        "asked_to_override": refusal.asked_to_override,
                    })
                    .to_string(),
                ),
            },
        );
    }

    // The same shape for the two ways money comes off a price, and the same
    // reason: both checks run inside the transaction above, so a row written
    // where they refuse would unwind with the sale. Written only when the
    // sale actually failed, so a refusal captured and then allowed on a later
    // pass cannot leave a row saying somebody was stopped.
    if let (Err(_), Some(refusal)) = (&issued, &price_refused) {
        let _ = audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: audit_actions::ACTION_PRICE_CUT_BLOCKED,
                // The customer if the sale named one, and the shop itself if
                // it did not: a cash sale at the counter has no fiche behind
                // it, and the refusal is about the person at the till rather
                // than about whoever is buying. `user_id` on the row is who
                // tried; that is the column an owner filters on.
                entity: "sale",
                entity_id: Some(customer_id.unwrap_or(shop_id)),
                before: Some(
                    serde_json::json!({
                        "permission": refusal.permission.as_str(),
                    })
                    .to_string(),
                ),
                after: Some(refusal.detail.to_string()),
            },
        );
    }

    issued
}

/// What the sale is asking the customer's standing for: how it is being
/// paid, what it comes to, and whether the till sent the flag that says
/// somebody means to pass the limit on purpose. Three fields rather than
/// three arguments because the check already takes the shop, the person and
/// the customer, and a list that long stops being readable.
struct CreditAsk {
    payment_mode: PaymentMode,
    net_to_pay: Money,
    override_credit: bool,
}

/// What a credit limit refused, kept where a rollback cannot reach it.
///
/// The sale that was refused unwound, taking any row written inside it with
/// it, which is the whole reason the M2 review found a cashier could probe a
/// customer's limit and leave nothing behind. These three facts are computed
/// inside the transaction and read after it, so the row that records the
/// refusal is written on ground the refusal did not wash away.
/// A refusal of one of the two ways money comes off a price, carried out of
/// the transaction the same way `Refused` is and for the same reason. Both
/// checks sit inside `conn.transaction`, so a row written where they refuse
/// unwinds with everything else and the log sees nothing. A cashier who
/// learns that could try a discount on every basket of the day and leave no
/// trace of a single attempt, which is the hole the credit refusal was
/// written to close and this is the same hole on the other side of the
/// total (M4 closing review, 2026-09-11).
struct PriceRefused {
    /// Which of the two doors was tried. The two are held by the same roles
    /// today (`permissions_service.rs` pins that), so the log carries which
    /// one anyway: an owner reading it wants to know whether somebody typed
    /// a price over a card or took a percentage off the basket.
    permission: Permission,
    /// What the till asked for, already shaped the way the row will show it.
    detail: serde_json::Value,
}

struct Refused {
    /// What the customer owed before the sale was attempted. The row carries
    /// it beside `balance_after` because the difference between the two is
    /// the basket, and without it an owner reading the log cannot tell one
    /// large attempt from twenty small ones against the same limit, which is
    /// the thing the row exists to show. The override row and the warned row
    /// have always carried it.
    balance_before: Money,
    /// What the customer would have owed had the sale landed.
    balance_after: Money,
    /// What they are allowed to owe.
    credit_limit: Money,
    /// Whether the till sent the override flag. False is a cashier who rang
    /// a sale up and was stopped by the limit. True is a cashier who tried
    /// to pass it and was stopped because they may not, which is the case a
    /// shop most wants to see in its log.
    asked_to_override: bool,
}

/// What the customer's standing decided: the fiche, the triple the document
/// stores, whether a limit was passed on purpose, and what the till should
/// say.
struct CreditCheck {
    customer: ProvedCustomer,
    balance: BalanceTriple,
    /// What this sale puts on the ledger: the whole net on credit, nothing on
    /// cash or card. Not the same figure as `balance.remaining_debt` once the
    /// customer was already holding credit, and it is this one the movement is
    /// written for.
    added: Money,
    /// How much of `added` was covered at issue out of credit the customer
    /// already held, and so is placed on this document rather than owed on it.
    consumed: Money,
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
    user_id: i32,
    customer_id: i32,
    ask: CreditAsk,
    refused: &mut Option<Refused>,
) -> Result<CreditCheck, RetailError> {
    let CreditAsk {
        payment_mode,
        net_to_pay,
        override_credit,
    } = ask;
    // Reads through the service, so another shop's fiche is a NotFound here
    // rather than a buyer block printed on this shop's paper (rule 3).
    let customer = customers::prove(conn, shop_id, customer_id)?;
    if !customer.fiche().active {
        return Err(RetailError::validation(
            "customer_id",
            "this customer's fiche is closed",
        ));
    }
    let old_balance = debt::balance(conn, shop_id, customer_id)?;
    // Cash and card leave the ledger where it was: the customer owes nothing
    // new, so this document puts nothing on it.
    let added = match payment_mode {
        PaymentMode::Credit => net_to_pay,
        PaymentMode::Cash | PaymentMode::Card => Money::ZERO,
    };
    // What the customer owes once the sale has landed: the whole of the sale
    // against the balance, whichever side of zero that balance was on. Credit
    // the customer is holding is already a movement on the ledger, so it is
    // netted out here by the addition itself and never subtracted twice.
    let total_debt = old_balance.checked_add(added)?;
    // A customer holding credit has this document settled out of it at issue
    // (features.md §3): the paper is what they pay against, so it must not ask
    // for money the shop already has. What the credit does not cover is what
    // the document is left asking for.
    let consumed = debt::credit_held(old_balance)?.min(added);
    let remaining_debt = added.checked_sub(consumed)?;

    let mut overridden = false;
    if payment_mode == PaymentMode::Credit {
        if let Some(credit_limit) = customer.fiche().credit_limit {
            if total_debt > credit_limit {
                // Written down before either refusal below, because both of
                // them are refusals and the log has to see both. The first
                // version of this only caught the one on the left, so a
                // cashier who learned to always send the override flag
                // probed a customer's limit for ever without leaving a row:
                // the flag turned the refusal into a forbidden, and only the
                // limit refusal was being logged. The flag is a deliberate
                // attempt to pass the limit, so it is the case the log most
                // needs, which is why it is carried in the row.
                *refused = Some(Refused {
                    balance_before: old_balance,
                    balance_after: total_debt,
                    credit_limit,
                    asked_to_override: override_credit,
                });
                if !override_credit {
                    return Err(RetailError::CreditLimit {
                        balance_after: total_debt,
                        credit_limit,
                    });
                }
                // The refusal comes first and the permission second, so a
                // cashier who sends no flag still hears that the limit is
                // what stopped them, with the two amounts the till shows,
                // rather than a forbidden that names a permission they were
                // not asking for.
                permissions::require(
                    role_of(conn, shop_id, user_id)?,
                    Permission::OverrideCreditBlock,
                )?;
                // Allowed. This is an override somebody was entitled to
                // take, not a refusal, and the override's own row says so
                // further up.
                *refused = None;
                overridden = true;
            }
        }
    }

    // At the threshold, not past it: a shop that sets one at 4 000,00 wants
    // to hear about the sale that reaches it.
    let warn_at = customer.fiche().warn_threshold;
    let warning = (payment_mode == PaymentMode::Credit
        && warn_at.is_some_and(|threshold| total_debt >= threshold))
    .then_some(Warning::NearLimit);

    Ok(CreditCheck {
        customer,
        balance: BalanceTriple {
            old_balance,
            remaining_debt,
            total_debt,
        },
        added,
        consumed,
        overridden,
        warning,
    })
}

/// The basket's HT, read back to compare the global discount against it.
/// A sum that does not fit is a basket the caller sent, so it is named as
/// one rather than raised as a money fault.
/// What the basket was worth before anything came off it: the base the
/// discount threshold is a percentage of.
fn sum_line_gross(lines: &[Line]) -> Result<Money, CoreError> {
    let mut total = Money::ZERO;
    for line in lines {
        let gross = line
            .unit_price
            .checked_mul_milli(line.qty_milli)
            .map_err(too_large("qty_milli"))?;
        total = total.checked_add(gross).map_err(too_large("lines"))?;
    }
    Ok(total)
}

/// Everything the customer is not being asked to pay: every line discount
/// and the one on the basket.
fn sum_discounts(lines: &[Line], global_discount: Money) -> Result<Money, CoreError> {
    let mut total = global_discount;
    for line in lines {
        total = total
            .checked_add(line.line_discount)
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
