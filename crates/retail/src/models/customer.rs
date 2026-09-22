//! The customer fiche (features.md §2). The party a facture is made out to,
//! and the party a debt belongs to. Amounts are `Money`, the `*_centimes`
//! columns the `i64` behind them.

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::money::Money;
use crate::schema::customers;

pub use super::sql_types::PartyKind;

/// A customer as the rest of the app sees it.
///
/// The opening debt features.md lists among the fields is not here: it is the
/// first `opening` row of the debt ledger, so the balance has one source and
/// a correction to it is a movement somebody can read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Customer {
    pub id: i32,
    pub shop_id: i32,
    pub name: String,
    pub party_kind: PartyKind,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub rc: Option<String>,
    pub nif: Option<String>,
    pub nis: Option<String>,
    pub ai: Option<String>,
    /// What the customer may owe at most. `None` is no limit at all and
    /// `Some(Money::ZERO)` is no credit at all; they are different answers
    /// and the till acts on them differently. The check itself is in
    /// `services::sales`, not here.
    pub credit_limit: Option<Money>,
    /// Where the till starts warning. `None` is no warning.
    pub warn_threshold: Option<Money>,
    pub notes: Option<String>,
    pub active: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// The fields a caller hands over, on a create and on an update alike. The
/// opening debt is not among them: it is passed to `create` on its own,
/// because an update never touches the ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewCustomer {
    pub name: String,
    pub party_kind: PartyKind,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub rc: Option<String>,
    pub nif: Option<String>,
    pub nis: Option<String>,
    pub ai: Option<String>,
    pub credit_limit: Option<Money>,
    pub warn_threshold: Option<Money>,
    pub notes: Option<String>,
    pub active: bool,
}

/// A customer that has been looked up in a shop and found there, carried as
/// a value so the lookup cannot be skipped.
///
/// `documents::issue` writes `customer_id` onto a document, and the foreign
/// key alone would take the neighbour's fiche (features.md §3, rule 3). That
/// check used to sit inside `issue`, which made `services::documents` import
/// `services::customers` and closed the ring that kept `customers` and `debt`
/// reaching past `documents` into its repo. `NewDocument` asks for one of
/// these instead of an `Option<i32>`, so the check is done by the type: a
/// caller with only an id has nothing to hand over, and the compiler says so
/// at the call.
///
/// The field is private and this module holds the only constructor, which
/// `services::customers::prove` calls on the shop-scoped read that proves it.
/// Rust cannot narrow a constructor to one non-descendant module, so the
/// constructor is `pub(crate)` and
/// `crates/core/tests/services_go_through_services.rs` asserts that
/// `customers.rs` is the only service that names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvedCustomer {
    customer: Customer,
}

impl ProvedCustomer {
    /// Made only from a fiche read under a `shop_id` filter, so the shop the
    /// proof is good for is the fiche's own column and the two can never
    /// disagree.
    pub(crate) fn proved(customer: Customer) -> Self {
        Self { customer }
    }

    pub fn id(&self) -> i32 {
        self.customer.id
    }

    /// The shop this is a proof about. `documents::issue` compares it with
    /// the shop it was called for: a fiche proved in one shop is no proof at
    /// all on another shop's document.
    pub fn shop_id(&self) -> i32 {
        self.customer.shop_id
    }

    /// The fiche the proof read. A caller that needs the name, the credit
    /// limit or the identifiers next has them here rather than reading the
    /// row a second time.
    pub fn fiche(&self) -> &Customer {
        &self.customer
    }
}

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = customers)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct CustomerRow {
    pub id: i32,
    pub shop_id: i32,
    pub name: String,
    pub party_kind: PartyKind,
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
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// `updated_at` is set by the service on every write, the way a product's is:
/// SQLite's DEFAULT only fires on the insert.
///
/// `treat_none_as_null`: an update carries the whole fiche, so a cleared RC or
/// a credit limit taken off has to reach the column. Without it diesel reads
/// `None` as "leave this one alone" and a limit could never be lifted.
#[derive(Debug, Insertable, AsChangeset)]
#[diesel(table_name = customers, treat_none_as_null = true)]
pub(crate) struct CustomerRowWrite {
    pub shop_id: i32,
    pub name: String,
    pub party_kind: PartyKind,
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
    pub updated_at: NaiveDateTime,
}

impl From<CustomerRow> for Customer {
    fn from(r: CustomerRow) -> Self {
        Customer {
            id: r.id,
            shop_id: r.shop_id,
            name: r.name,
            party_kind: r.party_kind,
            phone: r.phone,
            address: r.address,
            rc: r.rc,
            nif: r.nif,
            nis: r.nis,
            ai: r.ai,
            credit_limit: r.credit_limit_centimes.map(Money::centimes),
            warn_threshold: r.warn_threshold_centimes.map(Money::centimes),
            notes: r.notes,
            active: r.active,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
