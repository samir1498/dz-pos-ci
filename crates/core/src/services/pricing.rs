//! What a basket costs, before anything decides what to do with it.
//!
//! A quotation and the sale it becomes have to price the same basket the
//! same way, so the pricing lives under both rather than inside one of them.
//! It was inside `sales` until 2026-09-20, which made `proforma` import
//! `sales` while `sales` imported `proforma`: a ring the compiler accepts
//! and nobody can read one half of.
//!
//! Nothing here writes. It reads a product's card, settles a price against
//! the shop's regime, and hands back lines the money module can total.

use chrono::NaiveDateTime;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::customer::Customer;
use crate::models::document::{DocumentKind, PartyBlock};
use crate::money::{Bps, Line, Money, MoneyError, PaymentMode, Regime};
use crate::services::products;

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
    pub(crate) cost: Money,
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
