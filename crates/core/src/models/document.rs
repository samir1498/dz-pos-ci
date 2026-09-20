//! A fiscal document, its lines and its TVA recap (features.md §3). Every
//! kind shares this shape; only numbering, legal blocks and stock effect
//! differ. Amounts are `Money`, the `*_centimes` columns the `i64` behind
//! them, and `Regime` and `PaymentMode` convert to their column text here so
//! the money module stays free of diesel.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::error::CoreError;
use crate::models::shop::Shop;
use crate::money::{Bps, Money, PaymentMode, Regime, Totals, TvaLine};
use crate::schema::{document_lines, document_tva, documents};

pub use super::sql_types::{DocumentKind, DocumentStatus, PartyKind};

/// The seven seller fields as they were on the day. Snapshotted, never
/// joined: `shops` is replaced in place, so a reprint that read it live
/// would print a document the shop never issued.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SellerBlock {
    pub name: String,
    pub rc: Option<String>,
    pub nif: Option<String>,
    pub nis: Option<String>,
    pub ai: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
}

impl From<Shop> for SellerBlock {
    fn from(s: Shop) -> Self {
        SellerBlock {
            name: s.name,
            rc: s.rc,
            nif: s.nif,
            nis: s.nis,
            ai: s.ai,
            address: s.address,
            phone: s.phone,
        }
    }
}

/// The buyer as the document printed them (features.md §3). Snapshotted for
/// the same reason the seller block is: the fiche is edited in place and a
/// reprint has to show the facture the customer was handed. `party_kind`
/// travels with it because `a_company_buyer_without_a_nis_refuses_the_facture_and_burns_no_number` and `a_facture_to_a_consumer_asks_for_a_name_and_an_address_and_nothing_else` ask a different set
/// of fields of a company than of a consumer, and which one this buyer was on
/// the day is not something a later reader can work out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyBlock {
    pub name: String,
    pub party_kind: PartyKind,
    pub rc: Option<String>,
    pub nif: Option<String>,
    pub nis: Option<String>,
    pub ai: Option<String>,
    pub address: Option<String>,
}

/// What the customer owed before this document, what this document adds, and
/// what is left afterwards (features.md §3, totals table). Read from the
/// ledger at issue time and stored, so a reprint never recomputes it.
///
/// Signed: a customer who overpaid is owed money and the paper says so.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BalanceTriple {
    pub old_balance: Money,
    pub remaining_debt: Money,
    pub total_debt: Money,
}

/// What a cancellation left on the document it annulled (features.md §3).
///
/// One struct and not four fields on `Document`, because the four are one
/// fact: a document is annulled on a day, by somebody, for a reason, and
/// sometimes with an avoir behind it. `Some` is the whole of that fact, so a
/// caller printing the annulée face never has to ask whether the date is
/// there while the reason is not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cancellation {
    pub at: NaiveDateTime,
    pub by: i32,
    pub reason: String,
    /// The avoir the cancellation issued. `None` when there was nothing to
    /// carry back: a cash ticket owed nobody anything, so the stock returning
    /// is the whole of it.
    pub avoir_document_id: Option<i32>,
}

/// One sold line as it was sold. `name` and `barcode` are snapshots: the
/// product may be renamed or deleted and a reprint still shows what the
/// customer was handed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentLine {
    pub id: i32,
    pub position: i32,
    pub product_id: Option<i32>,
    pub name: String,
    pub barcode: Option<String>,
    pub qty_milli: i64,
    pub unit_price: Money,
    pub line_discount: Money,
    pub rate_bps: Bps,
    pub line_total: Money,
    /// The facture line this one credits, on an avoir line and nowhere else
    /// (features.md §3). What is left to credit on a facture line is its
    /// quantity less what earlier avoirs took off that same line, so the two
    /// are matched by id and never by which product they name.
    pub ref_line_id: Option<i32>,
}

/// A line as a caller hands it over, before it has an id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewDocumentLine {
    pub product_id: Option<i32>,
    pub name: String,
    pub barcode: Option<String>,
    pub qty_milli: i64,
    pub unit_price: Money,
    pub line_discount: Money,
    pub rate_bps: Bps,
    pub line_total: Money,
    pub ref_line_id: Option<i32>,
}

/// A document as the rest of the app sees it. `totals` carries the TVA recap
/// read back from `document_tva`, so a reprint never recomputes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub id: i32,
    pub shop_id: i32,
    pub kind: DocumentKind,
    pub series: String,
    /// The year the series counts in, off the shop clock at issue
    /// (features.md §4, Numbering). It is in `series` too, which is the
    /// counter's key; this is the number the paper prints, so a template
    /// never has to parse a key to find it.
    pub series_year: i32,
    pub number: i64,
    pub issued_at: NaiveDateTime,
    pub user_id: i32,
    pub regime: Regime,
    pub payment_mode: PaymentMode,
    pub seller: SellerBlock,
    pub customer_id: Option<i32>,
    /// What the paper says about the buyer. `None` on a ticket sold to
    /// whoever walked in; a facture always carries one, because it is made
    /// out to somebody (décret 05-468 art. 3).
    pub buyer: Option<PartyBlock>,
    /// The facture an avoir is written against. The rule about which
    /// factures may be named lives in `services::avoir`; here it is only
    /// carried.
    pub ref_document_id: Option<i32>,
    /// `None` when the document has no customer and so no balance to print.
    pub balance: Option<BalanceTriple>,
    pub totals: Totals,
    pub tendered: Option<Money>,
    pub change: Option<Money>,
    pub status: DocumentStatus,
    /// Filled exactly when `status` is `Cancelled`. The two are written
    /// together by `cancellation::cancel` and read back together below.
    pub cancellation: Option<Cancellation>,
    pub lines: Vec<DocumentLine>,
    pub created_at: NaiveDateTime,
}

/// A document as a service hands it over. The number and the series are the
/// numbering's to assign, never the caller's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewDocument {
    pub kind: DocumentKind,
    pub issued_at: NaiveDateTime,
    pub user_id: i32,
    pub regime: Regime,
    pub payment_mode: PaymentMode,
    pub seller: SellerBlock,
    pub customer_id: Option<i32>,
    pub buyer: Option<PartyBlock>,
    pub ref_document_id: Option<i32>,
    pub balance: Option<BalanceTriple>,
    pub totals: Totals,
    pub tendered: Option<Money>,
    pub change: Option<Money>,
    pub lines: Vec<NewDocumentLine>,
}

/// The column text for a régime and a payment mode. The migration carries
/// the same CHECK; this is the one place the two spellings meet.
pub(crate) const fn regime_stored(regime: Regime) -> &'static str {
    match regime {
        Regime::Reel => "reel",
        Regime::Ifu => "ifu",
    }
}

pub(crate) fn regime_parse(value: &str) -> Result<Regime, CoreError> {
    match value {
        "reel" => Ok(Regime::Reel),
        "ifu" => Ok(Regime::Ifu),
        other => Err(CoreError::validation(
            "regime",
            &format!("{other} is not a régime fiscal"),
        )),
    }
}

pub(crate) const fn payment_mode_stored(mode: PaymentMode) -> &'static str {
    match mode {
        PaymentMode::Cash => "cash",
        PaymentMode::Card => "card",
        PaymentMode::Credit => "credit",
    }
}

/// cheque and transfer are parked payment modes (features.md, Later): the
/// column admits them so adding one is not a migration, and the enum does
/// not, because whether a cheque carries the droit de timbre is undecided.
pub(crate) fn payment_mode_parse(value: &str) -> Result<PaymentMode, CoreError> {
    match value {
        "cash" => Ok(PaymentMode::Cash),
        "card" => Ok(PaymentMode::Card),
        "credit" => Ok(PaymentMode::Credit),
        other => Err(CoreError::validation(
            "payment_mode",
            &format!("{other} is not a payment mode this version issues"),
        )),
    }
}

fn bps(raw: i32) -> Result<Bps, CoreError> {
    let raw = u32::try_from(raw).map_err(|_| crate::money::MoneyError::RateOutOfRange)?;
    Ok(Bps::new(raw)?)
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = documents)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct DocumentRow {
    pub id: i32,
    pub shop_id: i32,
    pub kind: DocumentKind,
    pub series: String,
    pub number: i64,
    pub issued_at: NaiveDateTime,
    pub user_id: i32,
    pub regime: String,
    pub payment_mode: String,
    pub seller_name: String,
    pub seller_rc: Option<String>,
    pub seller_nif: Option<String>,
    pub seller_nis: Option<String>,
    pub seller_ai: Option<String>,
    pub seller_address: Option<String>,
    pub seller_phone: Option<String>,
    pub customer_id: Option<i32>,
    pub buyer_name: Option<String>,
    pub buyer_party_kind: Option<PartyKind>,
    pub buyer_rc: Option<String>,
    pub buyer_nif: Option<String>,
    pub buyer_nis: Option<String>,
    pub buyer_ai: Option<String>,
    pub buyer_address: Option<String>,
    pub ref_document_id: Option<i32>,
    pub total_ht_centimes: i64,
    pub discount_centimes: i64,
    pub subtotal_ht_centimes: i64,
    pub tva_centimes: i64,
    pub total_ttc_centimes: i64,
    pub stamp_centimes: i64,
    pub net_to_pay_centimes: i64,
    pub tendered_centimes: Option<i64>,
    pub change_centimes: Option<i64>,
    pub old_balance_centimes: Option<i64>,
    pub remaining_debt_centimes: Option<i64>,
    pub total_debt_centimes: Option<i64>,
    pub status: DocumentStatus,
    pub created_at: NaiveDateTime,
    pub cancelled_at: Option<NaiveDateTime>,
    pub cancelled_by: Option<i32>,
    pub cancel_reason: Option<String>,
    pub cancel_avoir_document_id: Option<i32>,
    pub series_year: i32,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = documents)]
pub(crate) struct DocumentRowWrite {
    pub shop_id: i32,
    pub kind: DocumentKind,
    pub series: String,
    pub series_year: i32,
    pub number: i64,
    pub issued_at: NaiveDateTime,
    pub user_id: i32,
    pub regime: &'static str,
    pub payment_mode: &'static str,
    pub seller_name: String,
    pub seller_rc: Option<String>,
    pub seller_nif: Option<String>,
    pub seller_nis: Option<String>,
    pub seller_ai: Option<String>,
    pub seller_address: Option<String>,
    pub seller_phone: Option<String>,
    pub customer_id: Option<i32>,
    pub buyer_name: Option<String>,
    pub buyer_party_kind: Option<PartyKind>,
    pub buyer_rc: Option<String>,
    pub buyer_nif: Option<String>,
    pub buyer_nis: Option<String>,
    pub buyer_ai: Option<String>,
    pub buyer_address: Option<String>,
    pub ref_document_id: Option<i32>,
    pub total_ht_centimes: i64,
    pub discount_centimes: i64,
    pub subtotal_ht_centimes: i64,
    pub tva_centimes: i64,
    pub total_ttc_centimes: i64,
    pub stamp_centimes: i64,
    pub net_to_pay_centimes: i64,
    pub tendered_centimes: Option<i64>,
    pub change_centimes: Option<i64>,
    pub old_balance_centimes: Option<i64>,
    pub remaining_debt_centimes: Option<i64>,
    pub total_debt_centimes: Option<i64>,
    pub status: DocumentStatus,
}

/// The four columns a cancellation writes, together. `AsChangeset` rather than
/// four `set` calls at the repo, so a cancellation cannot be written half way.
#[derive(Debug, AsChangeset)]
#[diesel(table_name = documents)]
pub(crate) struct CancelWrite {
    pub cancelled_at: NaiveDateTime,
    pub cancelled_by: i32,
    pub cancel_reason: String,
    pub cancel_avoir_document_id: Option<i32>,
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = document_lines)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct DocumentLineRow {
    pub id: i32,
    pub position: i32,
    pub product_id: Option<i32>,
    pub name: String,
    pub barcode: Option<String>,
    pub qty_milli: i64,
    pub unit_price_centimes: i64,
    pub line_discount_centimes: i64,
    pub rate_bps: i32,
    pub line_total_centimes: i64,
    pub ref_line_id: Option<i32>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = document_lines)]
pub(crate) struct DocumentLineRowWrite {
    pub shop_id: i32,
    pub document_id: i32,
    pub position: i32,
    pub product_id: Option<i32>,
    pub name: String,
    pub barcode: Option<String>,
    pub qty_milli: i64,
    pub unit_price_centimes: i64,
    pub line_discount_centimes: i64,
    pub rate_bps: i32,
    pub line_total_centimes: i64,
    pub ref_line_id: Option<i32>,
}

// Only the three columns a recap row prints. The document it belongs to is
// the query's filter, so selecting it back would be dead weight.
#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = document_tva)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct DocumentTvaRow {
    pub rate_bps: i32,
    pub base_centimes: i64,
    pub amount_centimes: i64,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = document_tva)]
pub(crate) struct DocumentTvaRowWrite {
    pub shop_id: i32,
    pub document_id: i32,
    pub rate_bps: i32,
    pub base_centimes: i64,
    pub amount_centimes: i64,
}

impl TryFrom<DocumentLineRow> for DocumentLine {
    type Error = CoreError;

    fn try_from(r: DocumentLineRow) -> Result<Self, CoreError> {
        Ok(DocumentLine {
            id: r.id,
            position: r.position,
            product_id: r.product_id,
            name: r.name,
            barcode: r.barcode,
            qty_milli: r.qty_milli,
            unit_price: Money::centimes(r.unit_price_centimes),
            line_discount: Money::centimes(r.line_discount_centimes),
            rate_bps: bps(r.rate_bps)?,
            line_total: Money::centimes(r.line_total_centimes),
            ref_line_id: r.ref_line_id,
        })
    }
}

impl TryFrom<DocumentTvaRow> for TvaLine {
    type Error = CoreError;

    fn try_from(r: DocumentTvaRow) -> Result<Self, CoreError> {
        Ok(TvaLine {
            rate: bps(r.rate_bps)?,
            base: Money::centimes(r.base_centimes),
            amount: Money::centimes(r.amount_centimes),
        })
    }
}

/// The stored row plus the lines and TVA rows read with it.
/// The buyer block, which is there whole or not at all. A row holding an RC
/// and no name is a write that got half way; reading it back as "no buyer"
/// would print a facture missing the identifiers `a_company_buyer_without_a_nis_refuses_the_facture_and_burns_no_number`
/// asks for and say nothing about it.
fn buyer_block(row: &DocumentRow) -> Result<Option<PartyBlock>, CoreError> {
    match (row.buyer_name.as_deref(), row.buyer_party_kind) {
        (Some(name), Some(party_kind)) => Ok(Some(PartyBlock {
            name: name.to_string(),
            party_kind,
            rc: row.buyer_rc.clone(),
            nif: row.buyer_nif.clone(),
            nis: row.buyer_nis.clone(),
            ai: row.buyer_ai.clone(),
            address: row.buyer_address.clone(),
        })),
        (None, None) => Ok(None),
        _ => Err(CoreError::validation(
            "buyer",
            "the stored buyer block has a name without a party kind, or the other way round",
        )),
    }
}

/// The same rule for the balance triple: three amounts or none. Two of three
/// would let a facture print a closing balance that its own opening balance
/// does not explain.
fn balance_triple(row: &DocumentRow) -> Result<Option<BalanceTriple>, CoreError> {
    match (
        row.old_balance_centimes,
        row.remaining_debt_centimes,
        row.total_debt_centimes,
    ) {
        (Some(old_balance), Some(remaining_debt), Some(total_debt)) => Ok(Some(BalanceTriple {
            old_balance: Money::centimes(old_balance),
            remaining_debt: Money::centimes(remaining_debt),
            total_debt: Money::centimes(total_debt),
        })),
        (None, None, None) => Ok(None),
        _ => Err(CoreError::validation(
            "balance",
            "the stored balance triple is missing one of its three amounts",
        )),
    }
}

/// The cancellation block, whole or not at all, and only on a document whose
/// `status` says it was annulled. The same rule the buyer block and the
/// balance triple follow: a row saying a facture was cancelled without saying
/// when, or saying when without saying it was cancelled, is a write that got
/// half way, and printing an annulée face off one of those halves would put a
/// claim on paper that the file cannot back up.
fn cancellation(row: &DocumentRow) -> Result<Option<Cancellation>, CoreError> {
    let block = match (
        row.cancelled_at,
        row.cancelled_by,
        row.cancel_reason.as_deref(),
    ) {
        (Some(at), Some(by), Some(reason)) => Some(Cancellation {
            at,
            by,
            reason: reason.to_string(),
            avoir_document_id: row.cancel_avoir_document_id,
        }),
        (None, None, None) => None,
        _ => {
            return Err(CoreError::validation(
                "cancellation",
                "the stored cancellation is missing its day, its author or its reason",
            ))
        }
    };
    match (row.status, block.is_some()) {
        (DocumentStatus::Cancelled, true) | (DocumentStatus::Issued, false) => Ok(block),
        (DocumentStatus::Cancelled, false) => Err(CoreError::validation(
            "cancellation",
            "the document is annulée and says nothing about when or by whom",
        )),
        (DocumentStatus::Issued, true) => Err(CoreError::validation(
            "cancellation",
            "the document carries a cancellation and still says it stands",
        )),
    }
}

pub(crate) fn assemble(
    row: DocumentRow,
    lines: Vec<DocumentLineRow>,
    tva: Vec<DocumentTvaRow>,
) -> Result<Document, CoreError> {
    let buyer = buyer_block(&row)?;
    let balance = balance_triple(&row)?;
    let cancellation = cancellation(&row)?;
    Ok(Document {
        id: row.id,
        shop_id: row.shop_id,
        kind: row.kind,
        series: row.series,
        series_year: row.series_year,
        number: row.number,
        issued_at: row.issued_at,
        user_id: row.user_id,
        regime: regime_parse(&row.regime)?,
        payment_mode: payment_mode_parse(&row.payment_mode)?,
        seller: SellerBlock {
            name: row.seller_name,
            rc: row.seller_rc,
            nif: row.seller_nif,
            nis: row.seller_nis,
            ai: row.seller_ai,
            address: row.seller_address,
            phone: row.seller_phone,
        },
        customer_id: row.customer_id,
        buyer,
        ref_document_id: row.ref_document_id,
        balance,
        totals: Totals {
            total_ht: Money::centimes(row.total_ht_centimes),
            discount: Money::centimes(row.discount_centimes),
            subtotal_ht: Money::centimes(row.subtotal_ht_centimes),
            tva_by_rate: tva
                .into_iter()
                .map(TvaLine::try_from)
                .collect::<Result<Vec<_>, _>>()?,
            tva: Money::centimes(row.tva_centimes),
            total_ttc: Money::centimes(row.total_ttc_centimes),
            stamp: Money::centimes(row.stamp_centimes),
            net_to_pay: Money::centimes(row.net_to_pay_centimes),
        },
        tendered: row.tendered_centimes.map(Money::centimes),
        change: row.change_centimes.map(Money::centimes),
        status: row.status,
        cancellation,
        lines: lines
            .into_iter()
            .map(DocumentLine::try_from)
            .collect::<Result<Vec<_>, _>>()?,
        created_at: row.created_at,
    })
}
