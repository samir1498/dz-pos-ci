//! Suppliers, their ledger, their statement and what closes an account.
//!
//! Re-exported by `super`, so every path outside this folder is unchanged.

use super::*;

/// Why the supplier debt moved (features.md §1). The whole union crosses from
/// the first version, the way the customer side's does: the receipt path
/// writes the `purchase` and `return` rows, and a screen that met an unknown kind could
/// only refuse the whole answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "SupplierDebtKindDto.ts")]
#[serde(rename_all = "lowercase")]
pub enum SupplierDebtKindDto {
    Opening,
    Purchase,
    Payment,
    Return,
    Adjustment,
}

impl From<SupplierDebtKind> for SupplierDebtKindDto {
    fn from(k: SupplierDebtKind) -> Self {
        match k {
            SupplierDebtKind::Opening => SupplierDebtKindDto::Opening,
            SupplierDebtKind::Purchase => SupplierDebtKindDto::Purchase,
            SupplierDebtKind::Payment => SupplierDebtKindDto::Payment,
            SupplierDebtKind::Return => SupplierDebtKindDto::Return,
            SupplierDebtKind::Adjustment => SupplierDebtKindDto::Adjustment,
        }
    }
}

/// The fiche as a screen reads it, with what the shop owes the supplier. The
/// balance is the ledger's sum computed in the core, never a stored column,
/// and it travels with the fiche so the list does not make a call per row.
///
/// There is no credit limit and no `party_kind`: those are what a shop grants
/// a buyer, and nothing it hands a supplier is a document it issues.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "SupplierDto.ts")]
pub struct SupplierDto {
    pub id: i32,
    pub shop_id: i32,
    pub name: String,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub rc: Option<String>,
    pub nif: Option<String>,
    pub nis: Option<String>,
    pub ai: Option<String>,
    pub notes: Option<String>,
    pub active: bool,
    /// Below zero is the supplier owing the shop after an advance or a return
    /// past what was due.
    pub balance_centimes: i64,
}

impl From<SupplierWithBalance> for SupplierDto {
    fn from(s: SupplierWithBalance) -> Self {
        let balance = s.balance.as_centimes();
        let s = s.supplier;
        SupplierDto {
            id: s.id,
            shop_id: s.shop_id,
            name: s.name,
            phone: s.phone,
            address: s.address,
            rc: s.rc,
            nif: s.nif,
            nis: s.nis,
            ai: s.ai,
            notes: s.notes,
            active: s.active,
            balance_centimes: balance,
        }
    }
}

/// The fields a supplier fiche is written with, on a create and on an update
/// alike. The whole row travels every time: a field left out is a bug at the
/// edge, and a null clears the column.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "SupplierWriteDto.ts")]
#[serde(deny_unknown_fields)]
pub struct SupplierWriteDto {
    pub name: String,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub rc: Option<String>,
    pub nif: Option<String>,
    pub nis: Option<String>,
    pub ai: Option<String>,
    pub notes: Option<String>,
    pub active: bool,
    /// Why a fiche is being closed. Asked for only when the update closes one
    /// whose account is still open, and ignored on every other update. The
    /// close route sends the same reason under its own field.
    #[serde(default)]
    pub close_reason: Option<String>,
}

/// A new fiche: the same fields, plus the debt the shop was already carrying
/// to this supplier before it had the app. The opening debt is only on the
/// create because it is a ledger movement, not a column.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "NewSupplierDto.ts")]
#[serde(deny_unknown_fields)]
pub struct NewSupplierDto {
    pub name: String,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub rc: Option<String>,
    pub nif: Option<String>,
    pub nis: Option<String>,
    pub ai: Option<String>,
    pub notes: Option<String>,
    #[serde(default = "yes")]
    pub active: bool,
    #[serde(default)]
    pub opening_debt_centimes: Option<i64>,
}

/// Why the shop has stopped buying from this supplier. Its own body rather
/// than a field of the fiche: closing is one decision and the route says so.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "CloseSupplierDto.ts")]
#[serde(deny_unknown_fields)]
pub struct CloseSupplierDto {
    #[serde(default)]
    pub reason: Option<String>,
}

impl From<SupplierWriteDto> for NewSupplier {
    fn from(d: SupplierWriteDto) -> Self {
        NewSupplier {
            name: d.name,
            phone: d.phone,
            address: d.address,
            rc: d.rc,
            nif: d.nif,
            nis: d.nis,
            ai: d.ai,
            notes: d.notes,
            active: d.active,
        }
    }
}

impl From<NewSupplierDto> for NewSupplier {
    fn from(d: NewSupplierDto) -> Self {
        NewSupplier::from(SupplierWriteDto {
            name: d.name,
            phone: d.phone,
            address: d.address,
            rc: d.rc,
            nif: d.nif,
            nis: d.nis,
            ai: d.ai,
            notes: d.notes,
            active: d.active,
            // A fiche being created closes nothing.
            close_reason: None,
        })
    }
}

/// What one payment placed on one order. A payment is one movement and the
/// orders it settled are these, oldest first.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "SupplierAllocationDto.ts")]
pub struct SupplierAllocationDto {
    pub purchase_id: i32,
    pub amount_centimes: i64,
}

impl From<SupplierAllocation> for SupplierAllocationDto {
    fn from(a: SupplierAllocation) -> Self {
        SupplierAllocationDto {
            purchase_id: a.purchase_id,
            amount_centimes: a.amount.as_centimes(),
        }
    }
}

/// One movement of the supplier ledger, with the balance it left behind and,
/// on a payment, the orders it settled. The allocations travel with the row
/// so a fiche showing a payment never asks a second time what the money went
/// to; every other kind carries an empty list.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "SupplierEntryDto.ts")]
pub struct SupplierEntryDto {
    pub id: i32,
    pub supplier_id: i32,
    /// The order the movement came from, when it came from one. An opening
    /// balance, a payment and a correction cite none.
    pub purchase_id: Option<i32>,
    pub kind: SupplierDebtKindDto,
    /// What the movement added to what the shop owes; zero on a payment or a
    /// return.
    pub debit_centimes: i64,
    /// What it took off; zero on a purchase or an opening balance.
    pub credit_centimes: i64,
    /// The balance as of this movement: every older one counted, no newer
    /// one. Computed in the core (services::supplier_debt).
    pub balance_after_centimes: i64,
    /// Null on every movement that is not a payment.
    pub payment_mode: Option<PaymentMethodDto>,
    pub user_id: i32,
    pub note: Option<String>,
    pub allocations: Vec<SupplierAllocationDto>,
    /// `YYYY-MM-DD HH:MM:SS`, the shape every stored timestamp holds.
    pub created_at: String,
}

/// A supplier's ledger: the movements newest first and the balance they sum
/// to. The balance is in the envelope so a screen showing it never adds the
/// column up itself.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "SupplierLedgerDto.ts")]
pub struct SupplierLedgerDto {
    pub supplier_id: i32,
    pub balance_centimes: i64,
    pub entries: Vec<SupplierEntryDto>,
}

/// A supplier's account over a range of days: what the shop owed on the
/// morning of `from`, every movement between the two days oldest first, and
/// what it owed on the evening of `to`. Both balances are read off the core's
/// running column, so a page printing them adds nothing up.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "SupplierStatementDto.ts")]
pub struct SupplierStatementDto {
    pub supplier_id: i32,
    /// `YYYY-MM-DD`, both ends included.
    pub from: String,
    pub to: String,
    pub opening_centimes: i64,
    pub entries: Vec<SupplierEntryDto>,
    pub closing_centimes: i64,
}
