//! Customers, their ledger and the payments against it.
//!
//! Re-exported by `super`, so every path outside this folder is unchanged.

use super::*;

/// The fiche as a screen reads it, with what the customer owes. The balance
/// is the ledger's sum computed in the core, never a stored column, and it
/// travels with the fiche so the list does not make a call per row.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "CustomerDto.ts")]
pub struct CustomerDto {
    pub id: i32,
    pub shop_id: i32,
    pub name: String,
    pub party_kind: PartyKindDto,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub rc: Option<String>,
    pub nif: Option<String>,
    pub nis: Option<String>,
    pub ai: Option<String>,
    /// Null is no limit at all, zero is no credit at all: two different
    /// answers, and the till acts on them differently.
    pub credit_limit_centimes: Option<i64>,
    pub warn_threshold_centimes: Option<i64>,
    pub notes: Option<String>,
    pub active: bool,
    /// Below zero is the shop owing the customer after an overpayment.
    pub balance_centimes: i64,
}

impl From<CustomerWithBalance> for CustomerDto {
    fn from(c: CustomerWithBalance) -> Self {
        let balance = c.balance.as_centimes();
        let c = c.customer;
        CustomerDto {
            id: c.id,
            shop_id: c.shop_id,
            name: c.name,
            party_kind: c.party_kind.into(),
            phone: c.phone,
            address: c.address,
            rc: c.rc,
            nif: c.nif,
            nis: c.nis,
            ai: c.ai,
            credit_limit_centimes: c.credit_limit.map(Money::as_centimes),
            warn_threshold_centimes: c.warn_threshold.map(Money::as_centimes),
            notes: c.notes,
            active: c.active,
            balance_centimes: balance,
        }
    }
}

/// The fields a fiche is written with, on a create and on an update alike.
/// The whole row travels every time, the way the store block does: a field
/// left out is a bug at the edge, and a null clears the column.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "CustomerWriteDto.ts")]
#[serde(deny_unknown_fields)]
pub struct CustomerWriteDto {
    pub name: String,
    pub party_kind: PartyKindDto,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub rc: Option<String>,
    pub nif: Option<String>,
    pub nis: Option<String>,
    pub ai: Option<String>,
    pub credit_limit_centimes: Option<i64>,
    pub warn_threshold_centimes: Option<i64>,
    pub notes: Option<String>,
    pub active: bool,
    /// Why a fiche is being closed. Asked for only when the update closes one
    /// that still carries a balance either way or a document still asking to
    /// be paid, and ignored on every other update.
    #[serde(default)]
    pub close_reason: Option<String>,
}

/// A new fiche: the same fields, plus the debt the shop was already carrying
/// for this customer before it had the app. The opening debt is only on the
/// create because it is a ledger movement, not a column, and an update that
/// could set it would be an edit to the ledger nobody could see (features.md
/// §2).
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "NewCustomerDto.ts")]
#[serde(deny_unknown_fields)]
pub struct NewCustomerDto {
    pub name: String,
    pub party_kind: PartyKindDto,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub rc: Option<String>,
    pub nif: Option<String>,
    pub nis: Option<String>,
    pub ai: Option<String>,
    pub credit_limit_centimes: Option<i64>,
    pub warn_threshold_centimes: Option<i64>,
    pub notes: Option<String>,
    #[serde(default = "yes")]
    pub active: bool,
    #[serde(default)]
    pub opening_debt_centimes: Option<i64>,
}

impl TryFrom<CustomerWriteDto> for NewCustomer {
    type Error = ApiError;

    fn try_from(d: CustomerWriteDto) -> Result<Self, ApiError> {
        Ok(NewCustomer {
            name: d.name,
            party_kind: d.party_kind.into(),
            phone: d.phone,
            address: d.address,
            rc: d.rc,
            nif: d.nif,
            nis: d.nis,
            ai: d.ai,
            credit_limit: money_field("credit_limit_centimes", d.credit_limit_centimes)?,
            warn_threshold: money_field("warn_threshold_centimes", d.warn_threshold_centimes)?,
            notes: d.notes,
            active: d.active,
        })
    }
}

impl TryFrom<NewCustomerDto> for NewCustomer {
    type Error = ApiError;

    fn try_from(d: NewCustomerDto) -> Result<Self, ApiError> {
        NewCustomer::try_from(CustomerWriteDto {
            name: d.name,
            party_kind: d.party_kind,
            phone: d.phone,
            address: d.address,
            rc: d.rc,
            nif: d.nif,
            nis: d.nis,
            ai: d.ai,
            credit_limit_centimes: d.credit_limit_centimes,
            warn_threshold_centimes: d.warn_threshold_centimes,
            notes: d.notes,
            active: d.active,
            // A fiche being created closes nothing.
            close_reason: None,
        })
    }
}

/// One movement of the ledger, with the balance it left behind.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "DebtEntryDto.ts")]
pub struct DebtEntryDto {
    pub id: i32,
    pub customer_id: i32,
    /// The document the movement came from, when it came from one. An
    /// opening balance and an adjustment cite none.
    pub document_id: Option<i32>,
    pub kind: DebtKindDto,
    /// What the movement added to the debt; zero on a payment or an avoir.
    pub debit_centimes: i64,
    /// What it took off; zero on a sale or an opening balance.
    pub credit_centimes: i64,
    /// The balance as of this movement: every older one counted, no newer
    /// one. Computed in the core (services::debt).
    pub balance_after_centimes: i64,
    pub user_id: i32,
    pub note: Option<String>,
    /// `YYYY-MM-DD HH:MM:SS`, the shape every stored timestamp holds.
    pub created_at: String,
}

impl From<LedgerLine> for DebtEntryDto {
    fn from(l: LedgerLine) -> Self {
        DebtEntryDto {
            id: l.entry.id,
            customer_id: l.entry.customer_id,
            document_id: l.entry.document_id,
            kind: l.entry.kind.into(),
            debit_centimes: l.entry.debit.as_centimes(),
            credit_centimes: l.entry.credit.as_centimes(),
            balance_after_centimes: l.balance_after.as_centimes(),
            user_id: l.entry.user_id,
            note: l.entry.note,
            created_at: l.entry.created_at.format(DATE_TIME_FORMAT).to_string(),
        }
    }
}

/// A customer's ledger: the movements newest first and the balance they sum
/// to. The balance is in the envelope so a screen showing it never adds the
/// column up itself.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "CustomerLedgerDto.ts")]
pub struct CustomerLedgerDto {
    pub customer_id: i32,
    pub balance_centimes: i64,
    pub entries: Vec<DebtEntryDto>,
}

/// A correction to what a customer owes: signed centimes and why. Positive
/// raises the debt, negative lowers it, zero is refused. The ledger is
/// append-only, so this writes a movement rather than editing one.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "AdjustmentDto.ts")]
#[serde(deny_unknown_fields)]
pub struct AdjustmentDto {
    pub amount_centimes: i64,
    #[serde(default)]
    pub note: Option<String>,
}

impl AdjustmentDto {
    /// The amount as money the core will take. The safe-integer bound is
    /// checked here, at the edge, like every other amount on the wire.
    pub fn amount(&self) -> Result<Money, ApiError> {
        Ok(Money::centimes(within_js_safe_range(
            "amount_centimes",
            self.amount_centimes,
        )?))
    }
}

/// How a payment against a debt was taken (features.md §2). Two ways and not
/// three: settling a credit with more credit is not a payment, so this is not
/// `PaymentModeDto`, which is what a document was sold under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "PaymentMethodDto.ts")]
#[serde(rename_all = "lowercase")]
pub enum PaymentMethodDto {
    Cash,
    Card,
}

impl From<PaymentMethod> for PaymentMethodDto {
    fn from(m: PaymentMethod) -> Self {
        match m {
            PaymentMethod::Cash => PaymentMethodDto::Cash,
            PaymentMethod::Card => PaymentMethodDto::Card,
        }
    }
}

impl From<PaymentMethodDto> for PaymentMethod {
    fn from(m: PaymentMethodDto) -> Self {
        match m {
            PaymentMethodDto::Cash => PaymentMethod::Cash,
            PaymentMethodDto::Card => PaymentMethod::Card,
        }
    }
}

/// What one payment placed on one document (features.md §2). A payment is one
/// movement and the documents it settled are these, oldest first.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "PaymentAllocationDto.ts")]
pub struct PaymentAllocationDto {
    pub document_id: i32,
    /// The paper as the customer holds it, `TK-2026-000002`, built by the
    /// core beside the templates that print it.
    pub printed_number: String,
    pub amount_centimes: i64,
}

/// One payment, with what it settled and the balance it left behind. The
/// allocations travel with it so a screen showing a payment never asks a
/// second time what the money went to.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "PaymentDto.ts")]
pub struct PaymentDto {
    /// The ledger movement's id: a payment is a row of the ledger, and an
    /// allocation names it.
    pub ledger_id: i32,
    pub customer_id: i32,
    pub amount_centimes: i64,
    /// Null on a payment written before the mode was stored; nothing writes
    /// one without it now.
    pub payment_mode: Option<PaymentMethodDto>,
    pub note: Option<String>,
    /// The balance as of this payment: every older movement counted, no newer
    /// one. Computed in the core (services::debt).
    pub balance_after_centimes: i64,
    /// Oldest document first.
    pub allocations: Vec<PaymentAllocationDto>,
    /// What the payment settled that no document carries: the opening debt,
    /// which a payment settles before any paper, and past the papers a
    /// correction upwards. Computed in the core (services::debt).
    pub without_document_centimes: i64,
    /// `YYYY-MM-DD HH:MM:SS`, the shape every stored timestamp holds.
    pub created_at: String,
}

impl From<Payment> for PaymentDto {
    fn from(p: Payment) -> Self {
        PaymentDto {
            ledger_id: p.entry.id,
            customer_id: p.entry.customer_id,
            amount_centimes: p.entry.credit.as_centimes(),
            payment_mode: p.entry.payment_mode.map(Into::into),
            note: p.entry.note,
            balance_after_centimes: p.balance_after.as_centimes(),
            allocations: p
                .allocations
                .into_iter()
                .map(|a| PaymentAllocationDto {
                    // Every allocation's document is this shop's (the
                    // allocation refuses one that is not), so the core named
                    // it; the id is the fallback a screen can still show.
                    printed_number: p
                        .numbers
                        .get(&a.document_id)
                        .cloned()
                        .unwrap_or_else(|| a.document_id.to_string()),
                    document_id: a.document_id,
                    amount_centimes: a.amount.as_centimes(),
                })
                .collect(),
            without_document_centimes: p.without_document.as_centimes(),
            created_at: p.entry.created_at.format(DATE_TIME_FORMAT).to_string(),
        }
    }
}

/// A customer's payments, newest first, and the balance the whole ledger sums
/// to. The balance is in the envelope for the reason the ledger's is: a screen
/// showing it never adds a column up itself.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export_to = "CustomerPaymentsDto.ts")]
pub struct CustomerPaymentsDto {
    pub customer_id: i32,
    pub balance_centimes: i64,
    pub payments: Vec<PaymentDto>,
}

/// Money against a debt: how much, how it was taken, and why if the shop
/// wants to say. The moment is the server's, like a document's `issued_at`.
#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export_to = "NewPaymentDto.ts")]
#[serde(deny_unknown_fields)]
pub struct NewPaymentDto {
    pub amount_centimes: i64,
    pub payment_mode: PaymentMethodDto,
    #[serde(default)]
    pub note: Option<String>,
}

impl NewPaymentDto {
    /// The amount as money the core will take. The safe-integer bound is
    /// checked here, at the edge, like every other amount on the wire.
    pub fn amount(&self) -> Result<Money, ApiError> {
        Ok(Money::centimes(within_js_safe_range(
            "amount_centimes",
            self.amount_centimes,
        )?))
    }
}
