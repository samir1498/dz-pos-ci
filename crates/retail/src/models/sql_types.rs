//! Storage-side types, the shop half. Built with `dzpos_kernel::text_enum!`,
//! the same macro `Role` uses in the kernel; nine of the ten invocations
//! named a shop concept and moved here in S4 of
//! `a-kernel-crate-and-retail-as-the-first-module`, `Role` being the one
//! that stayed. `Role` is re-exported below so `crate::models::sql_types::
//! Role` keeps resolving in this crate the way it did before the split.

use dzpos_kernel::text_enum;

pub use dzpos_kernel::models::sql_types::Role;

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
    /// The counter series this kind takes its numbers from in a given year,
    /// `doc_facture:2026`. One series per kind and per year: the series
    /// restart at 1 each year (features.md §4, Numbering), and a counter key
    /// that did not carry the year would hand January the number December
    /// stopped at.
    ///
    /// The year is the shop clock's at issue, never the machine's, so the
    /// caller reads it off the document's `issued_at` and not off `Utc::now`.
    pub fn series_of_year(self, year: i32) -> String {
        format!("{}:{year}", self.series())
    }

    /// The stem of the same key, without a year. One per kind, and on its own
    /// it is not what a number is taken from: `series_of_year` is.
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
    /// phone, and a printed number is `{prefix}-{year}-{number:06}`:
    /// `TK-2026-000123`. The two live side by side so a kind can never have
    /// one without the other.
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
