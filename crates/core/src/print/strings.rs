//! Every word a printed document says, in the three languages.
//!
//! Not the desktop's `apps/desktop/src/i18n/*.json`. Two reasons, and both
//! are permanent: the core cannot read the desktop's files (a server with
//! no UI prints the same bytes, features.md §4), and paper and screen are
//! different products. A screen says "Charge" on a button; a ticket says
//! "Net à payer" on a line a customer may take to a comptable.
//!
//! `crates/core/tests/print_strings.rs` walks `Key::ALL × Lang::ALL`, so a
//! key added without its Arabic is a failing test, not a French word on an
//! Arabic ticket. T6 renders the same keys to ESC/POS.
//!
//! The Arabic is unreviewed by a native speaker, like `words_ar`
//! (research R6). It is taken from the mockup's `design/shared/i18n.js`
//! where a key existed there.

use crate::lang::Lang;

/// A thing a printed document names. One variant per printed word, so the
/// template holds no text of its own and a translator has one list to read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// The kind of document, printed as its heading.
    Ticket,
    /// The sum of the lines before TVA. Réel only: under the IFU a price is
    /// a single price and "HT" would name a tax the document must not
    /// mention (CTCA 2026 art. 64), so the IFU ticket uses `Total`.
    TotalHt,
    /// The same row under the IFU, with no "hors taxe" in it.
    Total,
    Discount,
    Tva,
    Stamp,
    NetToPay,
    PaymentMode,
    Cash,
    Card,
    Credit,
    Tendered,
    Change,
    ThankYou,
    /// The dinar, as it is written after an amount.
    Currency,
}

impl Key {
    /// Every key, in the order the dictionary test walks them.
    pub const ALL: [Key; 15] = [
        Key::Ticket,
        Key::TotalHt,
        Key::Total,
        Key::Discount,
        Key::Tva,
        Key::Stamp,
        Key::NetToPay,
        Key::PaymentMode,
        Key::Cash,
        Key::Card,
        Key::Credit,
        Key::Tendered,
        Key::Change,
        Key::ThankYou,
        Key::Currency,
    ];
}

/// What `key` says in `lang`. One `match`, no map and no lookup that can
/// miss: a key with no arm does not compile.
pub const fn text(key: Key, lang: Lang) -> &'static str {
    match (key, lang) {
        (Key::Ticket, Lang::Fr) => "Ticket",
        (Key::Ticket, Lang::En) => "Receipt",
        (Key::Ticket, Lang::Ar) => "تذكرة",

        (Key::TotalHt, Lang::Fr) => "Total HT",
        (Key::TotalHt, Lang::En) => "Total excl. tax",
        (Key::TotalHt, Lang::Ar) => "المجموع خارج الرسم",

        (Key::Total, Lang::Fr) => "Total",
        (Key::Total, Lang::En) => "Total",
        (Key::Total, Lang::Ar) => "المجموع",

        (Key::Discount, Lang::Fr) => "Remise",
        (Key::Discount, Lang::En) => "Discount",
        (Key::Discount, Lang::Ar) => "تخفيض",

        // The abbreviation, not the four words behind it: a ticket is 80 mm
        // wide. The Arabic is the abbreviation of الرسم على القيمة المضافة
        // and is one of the three strings the IFU golden proves absent.
        (Key::Tva, Lang::Fr) => "TVA",
        (Key::Tva, Lang::En) => "VAT",
        (Key::Tva, Lang::Ar) => "ت.ق.م",

        (Key::Stamp, Lang::Fr) => "Droit de timbre",
        (Key::Stamp, Lang::En) => "Stamp duty",
        (Key::Stamp, Lang::Ar) => "حق الطابع",

        (Key::NetToPay, Lang::Fr) => "Net à payer",
        (Key::NetToPay, Lang::En) => "Net to pay",
        (Key::NetToPay, Lang::Ar) => "الصافي للدفع",

        (Key::PaymentMode, Lang::Fr) => "Mode de paiement",
        (Key::PaymentMode, Lang::En) => "Payment mode",
        (Key::PaymentMode, Lang::Ar) => "طريقة الدفع",

        (Key::Cash, Lang::Fr) => "Espèces",
        (Key::Cash, Lang::En) => "Cash",
        (Key::Cash, Lang::Ar) => "نقدًا",

        (Key::Card, Lang::Fr) => "Carte",
        (Key::Card, Lang::En) => "Card",
        (Key::Card, Lang::Ar) => "بطاقة",

        (Key::Credit, Lang::Fr) => "Crédit",
        (Key::Credit, Lang::En) => "Credit",
        (Key::Credit, Lang::Ar) => "دين",

        (Key::Tendered, Lang::Fr) => "Montant reçu",
        (Key::Tendered, Lang::En) => "Amount received",
        (Key::Tendered, Lang::Ar) => "المبلغ المستلم",

        (Key::Change, Lang::Fr) => "Monnaie à rendre",
        (Key::Change, Lang::En) => "Change",
        (Key::Change, Lang::Ar) => "الباقي",

        (Key::ThankYou, Lang::Fr) => "Merci de votre visite",
        (Key::ThankYou, Lang::En) => "Thank you for your visit",
        (Key::ThankYou, Lang::Ar) => "شكرًا لزيارتكم",

        (Key::Currency, Lang::Fr) => "DA",
        (Key::Currency, Lang::En) => "DZD",
        (Key::Currency, Lang::Ar) => "د.ج",
    }
}
