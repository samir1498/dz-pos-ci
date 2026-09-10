//! Storage-side types. `money/mod.rs` stays free of diesel, so the
//! centimes conversion for `Money` lives in `product.rs` next to the row it
//! maps, and `Regime` and `PaymentMode` convert in `document.rs`.
//!
//! Each enum here is one column with a CHECK behind it. The macro writes the
//! same four impls every time; hand-written, they drifted from the CHECK the
//! moment a variant was added to one and not the other.

use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Text;
use diesel::sqlite::Sqlite;

/// An enum stored as the exact text its column's CHECK allows.
macro_rules! text_enum {
    (
        $(#[$meta:meta])*
        $name:ident { $($variant:ident => $stored:literal),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            serde::Serialize,
            serde::Deserialize,
            AsExpression,
            FromSqlRow,
        )]
        #[diesel(sql_type = Text)]
        #[serde(rename_all = "snake_case")]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub const fn as_str(self) -> &'static str {
                match self {
                    $($name::$variant => $stored),+
                }
            }

            pub fn parse(s: &str) -> Option<Self> {
                match s {
                    $($stored => Some($name::$variant),)+
                    _ => None,
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl FromSql<Text, Sqlite> for $name {
            fn from_sql(
                value: <Sqlite as diesel::backend::Backend>::RawValue<'_>,
            ) -> deserialize::Result<Self> {
                let raw = <String as FromSql<Text, Sqlite>>::from_sql(value)?;
                $name::parse(&raw)
                    .ok_or_else(|| format!(concat!("unknown ", stringify!($name), ": {}"), raw).into())
            }
        }

        impl ToSql<Text, Sqlite> for $name {
            fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
                out.set_value(self.as_str().to_string());
                Ok(serialize::IsNull::No)
            }
        }
    };
}

text_enum! {
    /// Unit of measure. The products CHECK constraint allows exactly these.
    Unit {
        Piece => "piece",
        Kg => "kg",
        Litre => "litre",
        Box => "box",
    }
}

text_enum! {
    /// Why stock moved (features.md §1, Stock movements). The ledger is the
    /// truth; the product's quantity is a cache of it.
    MovementKind {
        Opening => "opening",
        Purchase => "purchase",
        Sale => "sale",
        Adjustment => "adjustment",
        Return => "return",
    }
}

text_enum! {
    /// The document kinds of features.md §3. The till issues `Ticket` and
    /// `Facture`; the others exist so a later milestone adds a screen, not a
    /// migration.
    /// `Quittance` is the stamped receipt for a payment against a debt, and
    /// nothing issues one: the comptable has not said whether a payment on
    /// account needs its own numbered document (R8), and a kind added once
    /// there are documents means rebuilding the table again.
    DocumentKind {
        Ticket => "ticket",
        Facture => "facture",
        Proforma => "proforma",
        BonDeLivraison => "bon_de_livraison",
        Avoir => "avoir",
        BonDeReception => "bon_de_reception",
        Quittance => "quittance",
    }
}

text_enum! {
    /// Whether a party is a company or a private consumer. A field on the
    /// fiche, never inferred from whether an RC was typed in: loi 04-02
    /// art. 10 decides ticket against facture by who the buyer is, and
    /// `a_company_buyer_without_a_nis_refuses_the_facture_and_burns_no_number` and `a_facture_to_a_consumer_asks_for_a_name_and_an_address_and_nothing_else` ask a different set of fields of each,
    /// so an inference would flip the rule the moment a field is cleared.
    PartyKind {
        Company => "company",
        Consumer => "consumer",
    }
}

text_enum! {
    /// Why the debt moved (features.md §2). `opening` is the balance the shop
    /// was carrying before it had this app; `adjustment` is how a mistake is
    /// corrected, because the ledger is append-only and a row is never edited.
    DebtKind {
        Opening => "opening",
        Sale => "sale",
        Payment => "payment",
        Avoir => "avoir",
        Adjustment => "adjustment",
    }
}

text_enum! {
    /// How a payment against a debt reached the till (features.md §2). Two
    /// ways and not three: settling a credit with more credit is not a
    /// payment, so `PaymentMode::Credit` has no counterpart here.
    ///
    /// It is informational on the movement. No stamp is computed from it: the
    /// droit de timbre is a question about the receipt a later settlement is
    /// handed, and the comptable has not answered it (R8).
    PaymentMethod {
        Cash => "cash",
        Card => "card",
    }
}

text_enum! {
    /// A cancelled document keeps its number and its row, so the series never
    /// gaps (features.md, Numbering row).
    DocumentStatus {
        Issued => "issued",
        Cancelled => "cancelled",
    }
}

text_enum! {
    /// Why what the shop owes a supplier moved. The mirror of `DebtKind`, and
    /// it differs by exactly what the two sides do differently: `Purchase`
    /// where a customer has a sale, and `Return` where a customer has an
    /// avoir. A `Sale` here would be a supplier buying at the till.
    ///
    /// `Purchase` is written when goods arrive and not when the order is
    /// saved (plan lens, 2026-09-10), so a purchase closed short owes nothing
    /// for what never came.
    SupplierDebtKind {
        Opening => "opening",
        Purchase => "purchase",
        Payment => "payment",
        Return => "return",
        Adjustment => "adjustment",
    }
}

text_enum! {
    /// Where a purchase has got to. `Ordered` until something arrives,
    /// `PartiallyReceived` and `Received` as it does, `Cancelled` when
    /// nothing ever did, `ClosedShort` when the rest never will and the shop
    /// has stopped waiting for it.
    PurchaseStatus {
        Ordered => "ordered",
        PartiallyReceived => "partially_received",
        Received => "received",
        Cancelled => "cancelled",
        ClosedShort => "closed_short",
    }
}

impl DocumentKind {
    /// The counter series this kind takes its numbers from. One series per
    /// kind, uninterrupted (décret 05-468 art. 10).
    pub const fn series(self) -> &'static str {
        match self {
            DocumentKind::Ticket => "doc_ticket",
            DocumentKind::Facture => "doc_facture",
            DocumentKind::Proforma => "doc_proforma",
            DocumentKind::BonDeLivraison => "doc_bon_de_livraison",
            DocumentKind::Avoir => "doc_avoir",
            DocumentKind::BonDeReception => "doc_bon_de_reception",
            DocumentKind::Quittance => "doc_quittance",
        }
    }

    /// The printed form of the same series. `series` above is the counter's
    /// name and a column value; this is what a customer reads back over the
    /// phone, and a printed number is `{prefix}-{number:06}`: `TK-000123`.
    /// The two live side by side so a kind can never have one without the
    /// other.
    pub const fn number_prefix(self) -> &'static str {
        match self {
            DocumentKind::Ticket => "TK",
            DocumentKind::Facture => "FA",
            DocumentKind::Proforma => "PF",
            DocumentKind::BonDeLivraison => "BL",
            DocumentKind::Avoir => "AV",
            DocumentKind::BonDeReception => "BR",
            DocumentKind::Quittance => "QT",
        }
    }
}
