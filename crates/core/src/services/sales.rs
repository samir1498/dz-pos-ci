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

use chrono::NaiveDateTime;
use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::error::{CoreError, PartySide};
use crate::models::customer::Customer;
use crate::models::debt::{DebtKind, NewDebtEntry};
use crate::models::document::{
    payment_mode_stored, BalanceTriple, Document, DocumentKind, NewDocument, NewDocumentLine,
    PartyBlock, PartyKind, SellerBlock,
};
use crate::models::stock::{Movement, MovementKind};
use crate::money::{
    compute_totals, Bps, Line, Money, MoneyError, PaymentMode, Regime, TotalsOptions,
};
use crate::services::permissions::{self, Permission, Role};
use crate::services::{
    audit, clock, customers, debt, documents, products, proforma, settings, shops, stock, users,
};

/// Whether the droit de timbre applies at all. There is no shop setting for
/// it yet; a cash payment is still what makes it due (features.md,
/// `stamp_progressive_tranches`). It becomes a setting the day a shop needs
/// to turn it off, not before.
pub(crate) const STAMP_ENABLED: bool = true;

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

/// The paper the till is ringing this basket up on (features.md §3). Three
/// values and not `DocumentKind`: an avoir and a bon de livraison are their
/// own writes with their own rules, and letting the till name one would be a
/// stock movement and a numbered document nobody asked for.
///
/// A ticket is the default because a sale to a consumer is the till's
/// ordinary case and needs nothing from the buyer (loi 04-02 art. 10 al. 1).
///
/// A proforma is on this switch because the till is where the basket is, and
/// a quotation is that same basket priced. It is the one value here that ends
/// in no sale at all: `issue` hands it straight to `services::proforma`, which
/// writes the document and moves neither stock nor debt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SaleKind {
    #[default]
    Ticket,
    Facture,
    Proforma,
}

impl SaleKind {
    /// The document kind the sale is issued as, and with it the series it
    /// numbers in (`doc_ticket`, `doc_facture`).
    pub const fn document_kind(self) -> DocumentKind {
        match self {
            SaleKind::Ticket => DocumentKind::Ticket,
            SaleKind::Facture => DocumentKind::Facture,
            SaleKind::Proforma => DocumentKind::Proforma,
        }
    }
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
    /// The decision to sell past the customer's credit limit. The sale goes
    /// through and the audit log carries who took it. Sending it is not
    /// taking it: the flag only means anything on a sale the limit would
    /// have refused, and there it asks for
    /// `Permission::OverrideCreditBlock` (features.md §1 and §5).
    pub override_credit: bool,
    /// Ticket or facture, decided at the till before the sale is saved
    /// (features.md §3). Never by a later reprint: the document is due « dès
    /// la réalisation de la vente ».
    pub kind: SaleKind,
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
    // A quotation is not a sale: it writes the document and stops. Handed
    // over before any of the rules below, because none of them is about it,
    // and it never warns about a credit limit it does not move.
    if new.kind == SaleKind::Proforma {
        return proforma::issue(conn, shop_id, user_id, new).map(|document| Sale {
            document,
            warning: None,
        });
    }
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
    if new.kind == SaleKind::Facture && new.customer_id.is_none() {
        // Décret 05-468 art. 3 puts the buyer on the paper, and the fiche is
        // where the buyer block comes from. Refused on the field the caller
        // sent rather than as `party_ids`: nothing is missing from a block
        // here, there is no block at all.
        return Err(CoreError::validation(
            "customer_id",
            "a facture is made out to a customer, so one has to be named",
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

        let priced = price_lines(conn, shop_id, regime, &new.lines)?;
        let money_lines = money_lines(&priced);
        let total_ht = sum_line_totals(&money_lines)?;
        if new.global_discount > total_ht {
            return Err(CoreError::validation(
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
            permissions::require(
                role_of(conn, shop_id, user_id)?,
                Permission::ChangePriceAtTheTill,
            )?;
        }

        let basket = sum_line_gross(&money_lines)?;
        let discount = sum_discounts(&money_lines, new.global_discount)?;
        let threshold = settings::discount_threshold_as_of(conn, shop_id, issued_at)?;
        let discounted_past_threshold =
            permissions::discount_needs_permission(basket, discount, threshold)?;
        if discounted_past_threshold {
            permissions::require(
                role_of(conn, shop_id, user_id)?,
                Permission::DiscountAboveThreshold,
            )?;
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
                new.payment_mode,
                totals.net_to_pay,
                new.override_credit,
            )?),
        };

        // A named customer is snapshotted onto the document the way the
        // seller is, on a ticket as much as on a facture: the fiche is
        // edited in place, and a reprint months later has to hand back the
        // block the buyer was given.
        let buyer = credit.as_ref().map(|c| buyer_block(&c.customer));
        // Still before `documents::issue`, so a facture the identifiers
        // refuse burns no number of either series (features.md, Numbering).
        if new.kind == SaleKind::Facture {
            check_party_ids(&seller, buyer.as_ref())?;
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
                customer_id: new.customer_id,
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
                    action: audit::ACTION_PRICE_OVERRIDE,
                    entity: "document",
                    entity_id: Some(document.id),
                    before: Some(
                        serde_json::json!({ "lines": negotiated.len() }).to_string(),
                    ),
                    after: Some(serde_json::json!({
                        "document_id": document.id,
                        "lines": negotiated,
                    })
                    .to_string()),
                },
            )?;
        }

        if discounted_past_threshold {
            audit::record(
                conn,
                shop_id,
                user_id,
                audit::Change {
                    action: audit::ACTION_DISCOUNT_OVERRIDE,
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

        let Some(credit) = credit else {
            return Ok(Sale {
                document,
                warning: None,
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
                    customer_id: credit.customer.id,
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
            credit.customer.id,
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
                    action: audit::ACTION_CREDIT_OVERRIDE,
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
/// `a_company_buyer_without_a_nis_refuses_the_facture_and_burns_no_number` and `a_facture_to_a_consumer_asks_for_a_name_and_an_address_and_nothing_else` ask a different set of fields of a company
/// than of a consumer, and a reprint may not read that from a fiche somebody
/// has since edited.
pub(crate) fn buyer_block(customer: &Customer) -> PartyBlock {
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

/// What a facture must carry before it may take a number
/// (`a_shop_whose_settings_carry_no_nis_cannot_issue_a_facture_at_all`, décret 05-468 art. 3 and 4).
///
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
/// The seller is checked first: a shop whose own settings are short has no
/// fiche the cashier could edit that would make the facture printable, so
/// hearing about the buyer first would send them to the wrong screen.
fn check_party_ids(seller: &SellerBlock, buyer: Option<&PartyBlock>) -> Result<(), CoreError> {
    let mut missing = Vec::new();
    if unset(seller.rc.as_deref()) {
        missing.push("rc");
    }
    if unset(seller.nis.as_deref()) {
        missing.push("nis");
    }
    if !missing.is_empty() {
        return Err(CoreError::PartyIds {
            side: PartySide::Seller,
            missing,
        });
    }

    // `issue` refuses a facture with no customer before any of this, so a
    // block missing here is a fiche the document service could not read;
    // the same refusal is the honest answer either way.
    let Some(buyer) = buyer else {
        return Err(CoreError::validation(
            "customer_id",
            "a facture is made out to a customer, so one has to be named",
        ));
    };
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
    if !missing.is_empty() {
        return Err(CoreError::PartyIds {
            side: PartySide::Buyer,
            missing,
        });
    }
    Ok(())
}

/// An identifier a block does not really carry. A field of spaces is not an
/// RC: the services store what was typed after a trim, and a row written
/// before that rule existed could still hold one.
fn unset(value: Option<&str>) -> bool {
    !value.is_some_and(|v| !v.trim().is_empty())
}

/// What the customer's standing decided: the fiche, the triple the document
/// stores, whether a limit was passed on purpose, and what the till should
/// say.
struct CreditCheck {
    customer: Customer,
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
        if let Some(credit_limit) = customer.credit_limit {
            if total_debt > credit_limit {
                if !override_credit {
                    return Err(CoreError::CreditLimit {
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
        added,
        consumed,
        overridden,
        warning,
    })
}

/// Turns an amount that does not fit into a validation error on the field
/// the caller sent. `CoreError::Money` is a 500, and 500 means the stored
/// file is at fault; a quantity and a price the caller chose whose product
/// is past i64 centimes is the caller's arithmetic, so it is a 422 with the
/// field named. Every other MoneyError keeps its meaning.
pub(crate) fn too_large(field: &'static str) -> impl Fn(MoneyError) -> CoreError {
    move |e| match e {
        MoneyError::Overflow => CoreError::validation(
            field,
            "this amount is past what the till can hold in centimes",
        ),
        other => CoreError::from(other),
    }
}

/// A line with its product read and its price settled.
pub(crate) struct PricedLine {
    pub(crate) product_id: i32,
    pub(crate) name: String,
    pub(crate) barcode: Option<String>,
    pub(crate) qty_milli: i64,
    pub(crate) unit_price: Money,
    pub(crate) line_discount: Money,
    pub(crate) rate_bps: crate::money::Bps,
    pub(crate) line_total: Money,
    cost: Money,
    /// The price on the product's own card, kept beside the one actually
    /// charged so the negotiated-price gate and its audit row can say what
    /// was given away without reading the product a second time.
    pub(crate) stored_price: Money,
}

/// Every line of a basket, priced. The one place a caller turns what the till
/// sent into what the document stores, so a quotation and the sale it becomes
/// price the same basket the same way.
pub(crate) fn price_lines(
    conn: &mut SqliteConnection,
    shop_id: i32,
    regime: Regime,
    lines: &[NewSaleLine],
) -> Result<Vec<PricedLine>, CoreError> {
    let mut priced = Vec::with_capacity(lines.len());
    for line in lines {
        priced.push(price(conn, shop_id, regime, line)?);
    }
    Ok(priced)
}

/// The priced lines as the money module reads them.
pub(crate) fn money_lines(priced: &[PricedLine]) -> Vec<Line> {
    priced
        .iter()
        .map(|p| Line {
            qty_milli: p.qty_milli,
            unit_price: p.unit_price,
            line_discount: p.line_discount,
            rate: p.rate_bps,
        })
        .collect()
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
        // no TVA at all (`an_ifu_line_stores_no_rate_so_a_reprint_never_needs_the_regime`). The line stores
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
        stored_price: product.selling,
    })
}

/// The basket's HT, read back to compare the global discount against it.
/// A sum that does not fit is a basket the caller sent, so it is named as
/// one rather than raised as a money fault.
/// The role the permission table is asked about, read from the user the sale
/// is being written under. Read here rather than taken as an argument so the
/// role that was checked and the user the document and the audit row name are
/// the same person: a role handed in beside a `user_id` is a second statement
/// of who is acting, and two statements can disagree.
fn role_of(conn: &mut SqliteConnection, shop_id: i32, user_id: i32) -> Result<Role, CoreError> {
    Ok(users::get(conn, shop_id, user_id)?.role)
}

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

pub(crate) fn sum_line_totals(lines: &[Line]) -> Result<Money, CoreError> {
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
