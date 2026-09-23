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
//! Arabic ticket.
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

    // The A4 facture (features.md §3 and §4). The three kinds the template
    // titles itself with, then the fields décret 05-468 art. 3 asks a
    // facture to carry.
    /// The heading of a facture, in the printed case: a title is read
    /// across a desk, so it is stored as it prints and not uppercased by a
    /// stylesheet a print driver may drop.
    Facture,
    /// A facture that was cancelled keeps its number and says so
    /// (features.md, Numbering row).
    FactureCancelled,
    /// The one word the mark across a cancelled reprint carries. Not the
    /// heading, which is a sentence read at the top of the page: this is
    /// read at arm's length, across the sheet.
    CancelledMark,
    /// The day the shop cancelled it, and why. Both are stored beside the
    /// document, in `cancelled_at` and `cancel_reason`.
    CancelledOn,
    CancelReason,
    /// What joins a document to its date in that sentence.
    IssuedOn,
    Seller,
    Buyer,
    Designation,
    Qty,
    LineTotalHt,
    LineTotal,
    SubtotalHt,
    Subtotal,
    /// The HT the tax of a recap row was taken on.
    TvaBase,
    TotalTtc,
    /// The words line: décret 05-468 art. 3 asks the total to be written
    /// "en chiffres et en lettres". This one names a facture, so an avoir
    /// and a proforma close themselves in their own words below rather than
    /// call themselves a facture on their last line.
    InWords,
    Balance,
    OldBalance,
    /// The same row when the customer's balance closes below zero: the shop
    /// is holding money for them, and "solde total" over a negative reads as
    /// a debt with a typo in it.
    TotalCredit,
    /// What the two parties put on the paper at the bottom of a facture
    /// (décret 05-468 art. 4).
    Cachet,

    // The statement of account (features.md §2 and §4). What a customer is
    // handed when they ask what they owe and how it got there.
    /// The heading, in the printed case like the facture's.
    Statement,
    /// The two days the page covers.
    Period,
    /// What was owed on the morning of the first day.
    OpeningBalance,
    /// What was owed on the evening of the last.
    ClosingBalance,
    Date,
    /// The column naming why the debt moved.
    Movement,
    Debit,
    /// The credit column of a statement. Not `Key::Credit`, which is the word
    /// for a sale that was not paid for on the day: one is a column and the
    /// other is a payment mode, and a translator reading one list must not
    /// have to work out which of the two a shared key meant.
    CreditColumn,
    /// Why the balance moved, one per kind of ledger row a trade may write
    /// (features.md §2). Read in a table cell, so they are the sentence case
    /// a column takes and not a heading's own capitals. The kinds a shop's
    /// own trade adds sit in `dzpos_retail::print::strings::ShopKey`
    /// (`KindSale`, `KindAvoir`) beside this one, since S4 of
    /// `a-kernel-crate-and-retail-as-the-first-module` took the shop words
    /// out of this file; a trade with no such kind simply carries fewer
    /// rows in the column.
    KindOpening,
    KindPayment,
    KindAdjustment,
    /// The words line of a statement. Not the facture's, which says "la
    /// présente facture" in words a comptable reads as being about one
    /// document.
    StatementInWords,
    /// A range with nothing in it. An empty table is a page that looks broken;
    /// a sentence saying nothing moved is an answer.
    NoMovement,
    /// The heading of the movement block. The slip carries the newest ten and
    /// says so in the word it uses, because a customer counting four rows
    /// against a year of buying has to know the page is not the whole ledger.
    LastMovements,
    /// The line that keeps the slip out of a comptable's file: it carries no
    /// number, no stamp and no TVA, so it says on its face that it proves
    /// nothing. The statement and the facture are the papers that do.
    NoFiscalValue,
    // The three of the Excel workbooks' own words this file still carries:
    // no shop noun sits in any of the three, unlike the sheet tabs
    // `ShopKey::SheetProducts`, `ShopKey::SheetSales`, `ShopKey::
    // SheetCustomers` and `ShopKey::SheetSuppliers` moved to
    // `dzpos_retail::print::strings` in S4 of
    // `a-kernel-crate-and-retail-as-the-first-module`, and both are read
    // only from `dzpos_retail::services::import`, the one caller either
    // kind has. They stay here rather than following the sheet tabs across:
    // moving three words nothing on this list catches would be tidying past
    // the rule this walk actually enforces.
    /// The second tab of the import template: the units and the TVA rates a
    /// row may name, and nothing else. It is read, never written back.
    SheetAllowedValues,
    /// The heading over the list of units a `unit` cell may hold.
    TemplateUnits,
    /// The heading over the list of TVA rates a `rate_percent` cell may hold.
    TemplateRates,
}

impl Key {
    /// Every key, in the order the dictionary test walks them.
    pub const ALL: [Key; 53] = [
        Key::Ticket,
        Key::TotalHt,
        Key::Total,
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
        Key::Facture,
        Key::FactureCancelled,
        Key::CancelledMark,
        Key::CancelledOn,
        Key::CancelReason,
        Key::IssuedOn,
        Key::Seller,
        Key::Buyer,
        Key::Designation,
        Key::Qty,
        Key::LineTotalHt,
        Key::LineTotal,
        Key::SubtotalHt,
        Key::Subtotal,
        Key::TvaBase,
        Key::TotalTtc,
        Key::InWords,
        Key::Balance,
        Key::OldBalance,
        Key::TotalCredit,
        Key::Cachet,
        Key::Statement,
        Key::Period,
        Key::OpeningBalance,
        Key::ClosingBalance,
        Key::Date,
        Key::Movement,
        Key::Debit,
        Key::CreditColumn,
        Key::KindOpening,
        Key::KindPayment,
        Key::KindAdjustment,
        Key::StatementInWords,
        Key::NoMovement,
        Key::LastMovements,
        Key::NoFiscalValue,
        Key::SheetAllowedValues,
        Key::TemplateUnits,
        Key::TemplateRates,
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

        (Key::Facture, Lang::Fr) => "FACTURE",
        (Key::Facture, Lang::En) => "INVOICE",
        (Key::Facture, Lang::Ar) => "فاتورة",

        (Key::FactureCancelled, Lang::Fr) => "FACTURE ANNULÉE",
        (Key::FactureCancelled, Lang::En) => "CANCELLED INVOICE",
        (Key::FactureCancelled, Lang::Ar) => "فاتورة ملغاة",

        (Key::CancelledMark, Lang::Fr) => "ANNULÉE",
        (Key::CancelledMark, Lang::En) => "CANCELLED",
        (Key::CancelledMark, Lang::Ar) => "ملغاة",

        (Key::CancelledOn, Lang::Fr) => "Annulée le",
        (Key::CancelledOn, Lang::En) => "Cancelled on",
        (Key::CancelledOn, Lang::Ar) => "ألغيت في",

        // The colon belongs to the wording and not to the template: French
        // puts a narrow no-break space before it and English puts none, and
        // a template that punctuated the line would be deciding that.
        (Key::CancelReason, Lang::Fr) => "Motif\u{202f}:",
        (Key::CancelReason, Lang::En) => "Reason:",
        (Key::CancelReason, Lang::Ar) => "السبب:",

        (Key::IssuedOn, Lang::Fr) => "du",
        (Key::IssuedOn, Lang::En) => "dated",
        (Key::IssuedOn, Lang::Ar) => "بتاريخ",

        (Key::Seller, Lang::Fr) => "Vendeur",
        (Key::Seller, Lang::En) => "Seller",
        (Key::Seller, Lang::Ar) => "البائع",

        (Key::Buyer, Lang::Fr) => "Client",
        (Key::Buyer, Lang::En) => "Customer",
        (Key::Buyer, Lang::Ar) => "الزبون",

        (Key::Designation, Lang::Fr) => "Désignation",
        (Key::Designation, Lang::En) => "Description",
        (Key::Designation, Lang::Ar) => "التعيين",

        (Key::Qty, Lang::Fr) => "Qté",
        (Key::Qty, Lang::En) => "Qty",
        (Key::Qty, Lang::Ar) => "الكمية",

        (Key::LineTotalHt, Lang::Fr) => "Montant HT",
        (Key::LineTotalHt, Lang::En) => "Amount excl. tax",
        (Key::LineTotalHt, Lang::Ar) => "المبلغ خارج الرسم",

        (Key::LineTotal, Lang::Fr) => "Montant",
        (Key::LineTotal, Lang::En) => "Amount",
        (Key::LineTotal, Lang::Ar) => "المبلغ",

        (Key::SubtotalHt, Lang::Fr) => "Sous-total HT",
        (Key::SubtotalHt, Lang::En) => "Subtotal excl. tax",
        (Key::SubtotalHt, Lang::Ar) => "المجموع الفرعي خارج الرسم",

        (Key::Subtotal, Lang::Fr) => "Sous-total",
        (Key::Subtotal, Lang::En) => "Subtotal",
        (Key::Subtotal, Lang::Ar) => "المجموع الفرعي",

        (Key::TvaBase, Lang::Fr) => "Base",
        (Key::TvaBase, Lang::En) => "Base",
        (Key::TvaBase, Lang::Ar) => "الوعاء",

        (Key::TotalTtc, Lang::Fr) => "Total TTC",
        (Key::TotalTtc, Lang::En) => "Total incl. tax",
        (Key::TotalTtc, Lang::Ar) => "المجموع مع الرسم",

        // The décret's own wording, for the paper it is written on. The
        // sentence is split per kind rather than have an avoir call itself a
        // facture on the line a comptable reads first; the two sentences that
        // are not the décret's are unreviewed wording, like this
        // dictionary's Arabic.
        (Key::InWords, Lang::Fr) => "Arrêtée la présente facture à la somme de",
        (Key::InWords, Lang::En) => "This invoice is closed at the sum of",
        (Key::InWords, Lang::Ar) => "أوقفت هذه الفاتورة بمبلغ",

        (Key::Balance, Lang::Fr) => "Solde",
        (Key::Balance, Lang::En) => "Balance",
        (Key::Balance, Lang::Ar) => "الرصيد",

        (Key::OldBalance, Lang::Fr) => "Ancien solde",
        (Key::OldBalance, Lang::En) => "Previous balance",
        (Key::OldBalance, Lang::Ar) => "الرصيد السابق",

        (Key::TotalCredit, Lang::Fr) => "Solde créditeur",
        (Key::TotalCredit, Lang::En) => "Credit balance",
        (Key::TotalCredit, Lang::Ar) => "رصيد دائن",

        (Key::Cachet, Lang::Fr) => "Cachet et signature",
        (Key::Cachet, Lang::En) => "Stamp and signature",
        (Key::Cachet, Lang::Ar) => "الختم والتوقيع",

        (Key::Statement, Lang::Fr) => "RELEVÉ DE COMPTE",
        (Key::Statement, Lang::En) => "ACCOUNT STATEMENT",
        (Key::Statement, Lang::Ar) => "كشف الحساب",

        (Key::Period, Lang::Fr) => "Période",
        (Key::Period, Lang::En) => "Period",
        (Key::Period, Lang::Ar) => "الفترة",

        (Key::OpeningBalance, Lang::Fr) => "Solde à l\u{2019}ouverture",
        (Key::OpeningBalance, Lang::En) => "Opening balance",
        (Key::OpeningBalance, Lang::Ar) => "الرصيد الافتتاحي",

        (Key::ClosingBalance, Lang::Fr) => "Solde à la clôture",
        (Key::ClosingBalance, Lang::En) => "Closing balance",
        (Key::ClosingBalance, Lang::Ar) => "الرصيد الختامي",

        (Key::Date, Lang::Fr) => "Date",
        (Key::Date, Lang::En) => "Date",
        (Key::Date, Lang::Ar) => "التاريخ",

        (Key::Movement, Lang::Fr) => "Mouvement",
        (Key::Movement, Lang::En) => "Movement",
        (Key::Movement, Lang::Ar) => "الحركة",

        (Key::Debit, Lang::Fr) => "Débit",
        (Key::Debit, Lang::En) => "Debit",
        (Key::Debit, Lang::Ar) => "مدين",

        (Key::CreditColumn, Lang::Fr) => "Crédit",
        (Key::CreditColumn, Lang::En) => "Credit",
        (Key::CreditColumn, Lang::Ar) => "دائن",

        (Key::KindOpening, Lang::Fr) => "Solde de départ",
        (Key::KindOpening, Lang::En) => "Balance carried over",
        (Key::KindOpening, Lang::Ar) => "رصيد مُرحَّل",

        (Key::KindPayment, Lang::Fr) => "Paiement",
        (Key::KindPayment, Lang::En) => "Payment",
        (Key::KindPayment, Lang::Ar) => "دفع",

        (Key::KindAdjustment, Lang::Fr) => "Ajustement",
        (Key::KindAdjustment, Lang::En) => "Adjustment",
        (Key::KindAdjustment, Lang::Ar) => "تسوية",

        (Key::StatementInWords, Lang::Fr) => "Arrêté le présent relevé à la somme de",
        (Key::StatementInWords, Lang::En) => "This statement is closed at the sum of",
        (Key::StatementInWords, Lang::Ar) => "أوقف هذا الكشف بمبلغ",

        (Key::NoMovement, Lang::Fr) => "Aucun mouvement sur la période",
        (Key::NoMovement, Lang::En) => "No movement in this period",
        (Key::NoMovement, Lang::Ar) => "لا توجد حركة في هذه الفترة",

        (Key::LastMovements, Lang::Fr) => "Derniers mouvements",
        (Key::LastMovements, Lang::En) => "Latest movements",
        (Key::LastMovements, Lang::Ar) => "آخر الحركات",

        (Key::NoFiscalValue, Lang::Fr) => "Document sans valeur fiscale",
        (Key::NoFiscalValue, Lang::En) => "This slip has no fiscal value",
        (Key::NoFiscalValue, Lang::Ar) => "وثيقة بدون قيمة جبائية",

        (Key::SheetAllowedValues, Lang::Fr) => "Valeurs autorisées",
        (Key::SheetAllowedValues, Lang::En) => "Allowed values",
        (Key::SheetAllowedValues, Lang::Ar) => "القيم المسموح بها",

        (Key::TemplateUnits, Lang::Fr) => "Unités",
        (Key::TemplateUnits, Lang::En) => "Units",
        (Key::TemplateUnits, Lang::Ar) => "الوحدات",

        (Key::TemplateRates, Lang::Fr) => "Taux de TVA",
        (Key::TemplateRates, Lang::En) => "VAT rates",
        (Key::TemplateRates, Lang::Ar) => "نسب الرسم على القيمة المضافة",
    }
}
