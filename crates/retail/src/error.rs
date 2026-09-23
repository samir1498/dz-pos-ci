//! The retail crate's own error variants: the six that name a shop concept
//! (`DuplicateBarcode`, `PaymentAboveDebt`, `CreditLimit`, `Unstamped`,
//! `UnpricedReversal`, `PartyIds`) plus `Render` and `Workbook`, which name
//! no shop word but wrap `askama::Error` and `rust_xlsxwriter::XlsxError`,
//! and every caller of both is retail's own print and export code (S3
//! already moved the print engine here whole); a kernel enum that still
//! wrapped them would keep `askama` and `rust_xlsxwriter` on
//! `crates/kernel`'s dependency list for two variants nothing in the kernel
//! ever raises. S4 of `a-kernel-crate-and-retail-as-the-first-module`
//! moved all eight out of `dzpos_kernel::error::CoreError` for that reason.
//!
//! Every other failure a retail function can raise (`NotFound`,
//! `Validation`, a storage fault, a permission refused, …) is still a
//! `CoreError`, so `RetailError` wraps it whole rather than repeating it:
//! `Kernel(#[from] CoreError)` is `#[error(transparent)]`, so its `Display`
//! string is the kernel error's own, unprefixed, and `?` on a call that
//! returns `Result<_, CoreError>` converts into a `Result<_, RetailError>`
//! without a caller writing `.into()`.

use dzpos_kernel::money::Money;

// Re-exported so `crate::error::CoreError` keeps resolving in this crate:
// `lib.rs` used to re-export the kernel's `error` module whole before this
// crate had one of its own, and every retail file's `use crate::error::
// CoreError` was written against that.
pub use dzpos_kernel::error::CoreError;

/// Which half of a facture a `PartyIds` refusal is about. The two blocks are
/// filled in from two different screens, so the side is what tells the till
/// whether to send the cashier to the settings or to the customer's fiche.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartySide {
    Seller,
    Buyer,
}

impl PartySide {
    /// The stable key the UI translates, the way an error code is.
    pub const fn as_str(self) -> &'static str {
        match self {
            PartySide::Seller => "seller",
            PartySide::Buyer => "buyer",
        }
    }
}

impl std::fmt::Display for PartySide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RetailError {
    /// Every failure this crate does not itself name: `NotFound`,
    /// `Validation`, a storage fault, a permission refused, and the rest of
    /// `CoreError`'s variants unchanged from before the split.
    #[error(transparent)]
    Kernel(#[from] CoreError),
    #[error("barcode {0} is already used in this shop")]
    DuplicateBarcode(String),
    /// A payment for more than is owed, on either ledger. Its own variant
    /// rather than a `Validation`, because the only useful thing to say back
    /// is a figure the caller never sent: what is outstanding right now. The
    /// code stays `validation`, so a screen that already translates it says
    /// the same sentence and reads the amount out of the payload.
    ///
    /// The party is not named, because both sides raise it: money over what a
    /// customer owes is an avoir's business and never a credit balance a
    /// payment quietly opened, and money over what the shop owes a supplier
    /// is an advance somebody writes on purpose.
    #[error("a payment is never more than what is owed")]
    PaymentAboveDebt { outstanding_centimes: i64 },
    /// A credit sale the customer's limit will not carry (features.md §1).
    /// Not a validation failure: every field the caller sent is well formed,
    /// and what refuses the sale is what the customer already owes. The two
    /// amounts travel with the code because the till has to say by how much
    /// and against what, and a screen may not re-derive either.
    #[error(
        "this sale would leave {} centimes owed against a credit limit of {} centimes",
        balance_after.as_centimes(),
        credit_limit.as_centimes()
    )]
    CreditLimit {
        balance_after: Money,
        credit_limit: Money,
    },
    /// A facture whose party blocks do not carry what décret 05-468 art. 3
    /// asks of them (`a_company_buyer_without_a_nis_refuses_the_facture_and_burns_no_number`). Not a validation failure
    /// on a field the caller sent: the basket is well formed and what
    /// refuses the paper sits on the settings page or on the customer's
    /// fiche. The side and the identifiers travel with the code because the
    /// till has to say where to go and what is missing, and a screen may not
    /// work either out from the rule (architecture.md rule 2).
    #[error("the {side} block of a facture is missing {}", missing.join(", "))]
    PartyIds {
        side: PartySide,
        missing: Vec<&'static str>,
    },
    /// A ledger row handed to a repo with no moment on it. The column's
    /// default is SQLite's CURRENT_TIMESTAMP, which is UTC, while every
    /// period this app answers for is a stretch of days on the shop's
    /// calendar (UTC+1): a payment taken at 00:30 in Algiers would be stored
    /// on the day before and fall out of the day the shop counted its
    /// drawer. The caller stamps it from `dzpos_kernel::services::clock`.
    ///
    /// Its own variant and not a `Validation`: no field a caller sent is
    /// wrong, and nobody using the app can correct it. It is a mistake in
    /// this crate, so the API answers 500 and the code is the storage one.
    #[error("a row of {entity} is stamped from the shop clock, never left to the file's default")]
    Unstamped { entity: &'static str },
    /// A reversal has no cost to write back, because the stock ledger cannot
    /// say what the goods cost when they left. Two shapes, both of them a
    /// file that disagrees with itself: a sold line with no movement at all,
    /// and one product's sale movements on one document carrying two
    /// different costs.
    ///
    /// Neither can arise from anything a caller sent: `unit_cost_centimes`
    /// is NOT NULL from the first documents migration, a sale is the only
    /// writer of a `sale` movement, and it reads the fiche once for the whole
    /// basket. So it is this crate's bug or a row somebody wrote by hand, and
    /// it is refused rather than papered over with the fiche's cost today:
    /// guessing here is how a month's margin moves with a purchase, which is
    /// the whole thing the ledger cost exists to prevent.
    ///
    /// Its own variant and not a `Validation`, for the reason `Unstamped` is:
    /// the API answers 500 and the code is the storage one.
    #[error("the stock ledger cannot price the reversal of product {product_id} on document {document_id}: {reason}")]
    UnpricedReversal {
        document_id: i32,
        product_id: i32,
        reason: &'static str,
    },
    /// A printed template failed to render. The template and the data it is
    /// given are both the app's own, so this is a bug in the app and never
    /// something a caller can correct; the API answers 500 and the message
    /// stays fixed, like the file one.
    #[error("the document could not be rendered for printing")]
    Render(#[from] askama::Error),
    /// A workbook this app was writing could not be finished. Like `Render`,
    /// the data and the layout are both the app's own, so this is a bug here
    /// and never something a caller can correct; the message is fixed and the
    /// writer's own text stays on the source chain for the server's log.
    ///
    /// Reading a workbook is not this: a file a shop uploaded is input, and a
    /// file that is not a workbook at all comes back as a `Validation` the
    /// import screen can put under the file picker.
    #[error("the workbook could not be written")]
    Workbook(#[from] rust_xlsxwriter::XlsxError),
}

impl RetailError {
    /// Stable key the UI translates. Never the message. `Kernel` delegates to
    /// `CoreError::code`, unchanged, so a route matching on the code sees the
    /// same string it always has for every failure this crate does not name
    /// itself.
    pub const fn code(&self) -> &'static str {
        match self {
            RetailError::Kernel(e) => e.code(),
            RetailError::DuplicateBarcode(_) => "duplicate_barcode",
            RetailError::PaymentAboveDebt { .. } => "validation",
            RetailError::CreditLimit { .. } => "credit_limit",
            RetailError::PartyIds { .. } => "party_ids",
            RetailError::Unstamped { .. } | RetailError::UnpricedReversal { .. } => "storage",
            RetailError::Render(_) => "print",
            RetailError::Workbook(_) => "workbook",
        }
    }

    /// A document the printer refuses. `reason` says which rule the stored
    /// row breaks; it stays on the server, on the error's source chain,
    /// because the wire message for a render failure is fixed.
    pub fn render(reason: &'static str) -> Self {
        RetailError::Render(askama::Error::custom(reason))
    }

    /// `CoreError::validation`, already wrapped, so a retail function that
    /// returns `RetailError` refuses a field in one expression, the way it did
    /// before the split. Same code, same field, same message.
    pub fn validation(field: &str, message: &str) -> Self {
        RetailError::Kernel(CoreError::validation(field, message))
    }

    /// `CoreError::conflict`, already wrapped, for the same reason.
    pub fn conflict(field: &str, message: &str) -> Self {
        RetailError::Kernel(CoreError::conflict(field, message))
    }
}

/// A file or a socket that failed, straight into `Kernel(Io)`: the ESC/POS
/// writer talks to a printer through both.
impl From<std::io::Error> for RetailError {
    fn from(e: std::io::Error) -> Self {
        RetailError::Kernel(CoreError::from(e))
    }
}

/// Money arithmetic that overflows, straight into `Kernel(Money)`, so `?` on
/// a checked sum works in a function that returns `RetailError` without a
/// `map_err` hop through `CoreError` at every call site.
impl From<dzpos_kernel::money::MoneyError> for RetailError {
    fn from(e: dzpos_kernel::money::MoneyError) -> Self {
        RetailError::Kernel(CoreError::from(e))
    }
}

/// A direct hop from a diesel error to `RetailError`, past `Kernel`, so
/// `diesel::connection::Connection::transaction`'s own bound
/// (`E: From<diesel::result::Error>`) is satisfied for a closure that
/// returns `Result<_, RetailError>`: that bound wants one `From` impl, and a
/// caller cannot spell two hops through `CoreError` for it. `?` on an
/// ordinary query still goes through `CoreError::Query` first and reaches
/// the same variant, so a query's `code` and message are identical whichever
/// way it arrived.
impl From<diesel::result::Error> for RetailError {
    fn from(e: diesel::result::Error) -> Self {
        RetailError::Kernel(CoreError::from(e))
    }
}
