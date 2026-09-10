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
    Avoir,
    Proforma,
    /// What a proforma says about itself, in a line of its own: it is a
    /// quote and not a facture, it books nothing and it owes nothing.
    /// A customer handed one must not file it as a facture.
    ProformaNotice,
    /// The opening of the line naming the facture an avoir is written
    /// against, one of the mentions décret 05-468 art. 3 asks an avoir to
    /// carry. It is the start of a sentence and not a column label:
    /// "Avoir sur facture FA-2026-000042 du 09/09/2026".
    AvoirOnFacture,
    /// What joins a document to its date in that sentence.
    IssuedOn,
    /// The last row of an avoir's totals. A facture closes on the net to
    /// pay, the figure the buyer owes; an avoir asks for nothing, so the
    /// same row names the amount of the avoir. The figure is the same one,
    /// under the words that fit the paper it is on.
    AvoirAmount,
    Seller,
    Buyer,
    Designation,
    Qty,
    /// The unit price column of a réel facture: "hors taxes" is one of the
    /// mentions décret 05-468 art. 3 asks for by name.
    UnitPriceHt,
    /// The same column under the IFU, where naming the tax is forbidden
    /// (CTCA 2026 art. 64).
    UnitPrice,
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
    AvoirInWords,
    ProformaInWords,
    Balance,
    OldBalance,
    ThisDocument,
    TotalDebt,
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
    /// The column carrying the number of the document a movement cites.
    Document,
    Debit,
    /// The credit column of a statement. Not `Key::Credit`, which is the word
    /// for a sale that was not paid for on the day: one is a column and the
    /// other is a payment mode, and a translator reading one list must not
    /// have to work out which of the two a shared key meant.
    CreditColumn,
    /// Why the debt moved, one per kind of ledger row (features.md §2). Read
    /// in a table cell, so they are the sentence case a column takes and not
    /// the heading case `Avoir` above is.
    KindOpening,
    KindSale,
    KindPayment,
    KindAvoir,
    KindAdjustment,
    /// The words line of a statement. Not the facture's, which says "la
    /// présente facture" in words a comptable reads as being about one
    /// document.
    StatementInWords,
    /// Added to the words line when the closing balance is below zero: the
    /// shop is holding money for the customer, and the words themselves carry
    /// no sign.
    InFavourOfCustomer,
    /// A range with nothing in it. An empty table is a page that looks broken;
    /// a sentence saying nothing moved is an answer.
    NoMovement,

    // The 80 mm debt slip (features.md §2 and §4). What a credit customer is
    // handed at the counter when they ask what they owe: the balance, the
    // movements behind it, and nothing a comptable would file.
    /// The heading. Sentence case like the ticket's and not the facture's
    /// capitals: it is a counter paper, not a document with a series.
    DebtSlip,
    /// The heading of the movement block. The slip carries the newest ten and
    /// says so in the word it uses, because a customer counting four rows
    /// against a year of buying has to know the page is not the whole ledger.
    LastMovements,
    /// The line that keeps the slip out of a comptable's file: it carries no
    /// number, no stamp and no TVA, so it says on its face that it proves
    /// nothing. The statement and the facture are the papers that do.
    NoFiscalValue,
    /// The words line of a debt slip. Not the statement's, which says "le
    /// présent relevé" about a page covering a period this one has none of.
    DebtInWords,

    // The Excel workbooks (features.md §1, Backup). Only the sheet name is
    // translated: a workbook's header row and its unit and status cells are
    // the stable key names, so a products export edited in a spreadsheet
    // reads straight back through the import and no column has to be found
    // again in three languages. The tab is what a person sees first, and it
    // is the one place a word costs nothing to translate.
    /// The tab of the products workbook, and of the import template.
    SheetProducts,
    SheetSales,
    SheetCustomers,
    SheetSuppliers,
    /// The second tab of the import template: the units and the TVA rates a
    /// row may name, and nothing else. It is read, never written back.
    SheetAllowedValues,
}

impl Key {
    /// Every key, in the order the dictionary test walks them.
    pub const ALL: [Key; 73] = [
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
        Key::Facture,
        Key::FactureCancelled,
        Key::CancelledMark,
        Key::CancelledOn,
        Key::CancelReason,
        Key::Avoir,
        Key::Proforma,
        Key::ProformaNotice,
        Key::AvoirOnFacture,
        Key::IssuedOn,
        Key::AvoirAmount,
        Key::Seller,
        Key::Buyer,
        Key::Designation,
        Key::Qty,
        Key::UnitPriceHt,
        Key::UnitPrice,
        Key::LineTotalHt,
        Key::LineTotal,
        Key::SubtotalHt,
        Key::Subtotal,
        Key::TvaBase,
        Key::TotalTtc,
        Key::InWords,
        Key::AvoirInWords,
        Key::ProformaInWords,
        Key::Balance,
        Key::OldBalance,
        Key::ThisDocument,
        Key::TotalDebt,
        Key::TotalCredit,
        Key::Cachet,
        Key::Statement,
        Key::Period,
        Key::OpeningBalance,
        Key::ClosingBalance,
        Key::Date,
        Key::Movement,
        Key::Document,
        Key::Debit,
        Key::CreditColumn,
        Key::KindOpening,
        Key::KindSale,
        Key::KindPayment,
        Key::KindAvoir,
        Key::KindAdjustment,
        Key::StatementInWords,
        Key::InFavourOfCustomer,
        Key::NoMovement,
        Key::DebtSlip,
        Key::LastMovements,
        Key::NoFiscalValue,
        Key::DebtInWords,
        Key::SheetProducts,
        Key::SheetSales,
        Key::SheetCustomers,
        Key::SheetSuppliers,
        Key::SheetAllowedValues,
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

        (Key::Avoir, Lang::Fr) => "AVOIR",
        (Key::Avoir, Lang::En) => "CREDIT NOTE",
        (Key::Avoir, Lang::Ar) => "إشعار دائن",

        (Key::Proforma, Lang::Fr) => "PROFORMA",
        (Key::Proforma, Lang::En) => "PRO FORMA INVOICE",
        (Key::Proforma, Lang::Ar) => "فاتورة أولية",

        (Key::ProformaNotice, Lang::Fr) => {
            "Proforma, sans valeur comptable\u{202f}: ce document n\u{2019}est pas une facture et ne crée aucune dette."
        }
        (Key::ProformaNotice, Lang::En) => {
            "Pro forma, of no accounting value: this document is not an invoice and creates no debt."
        }
        (Key::ProformaNotice, Lang::Ar) => {
            "فاتورة أولية، بدون قيمة محاسبية: هذه الوثيقة ليست فاتورة ولا تنشئ أي دين."
        }

        (Key::AvoirOnFacture, Lang::Fr) => "Avoir sur facture",
        (Key::AvoirOnFacture, Lang::En) => "Credit note against invoice",
        (Key::AvoirOnFacture, Lang::Ar) => "إشعار دائن على الفاتورة",

        (Key::IssuedOn, Lang::Fr) => "du",
        (Key::IssuedOn, Lang::En) => "dated",
        (Key::IssuedOn, Lang::Ar) => "بتاريخ",

        // Unreviewed wording, like this dictionary's Arabic: no comptable
        // has read the French of it yet. The apostrophe is the typographic
        // one, which is the French a printed document uses and also the one
        // that reaches the page as itself: a typewriter apostrophe would
        // sit in the golden as `&#39;`.
        (Key::AvoirAmount, Lang::Fr) => "Montant de l\u{2019}avoir",
        (Key::AvoirAmount, Lang::En) => "Credit note amount",
        (Key::AvoirAmount, Lang::Ar) => "مبلغ الإشعار الدائن",

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

        (Key::UnitPriceHt, Lang::Fr) => "Prix unitaire HT",
        (Key::UnitPriceHt, Lang::En) => "Unit price excl. tax",
        (Key::UnitPriceHt, Lang::Ar) => "سعر الوحدة خارج الرسم",

        (Key::UnitPrice, Lang::Fr) => "Prix unitaire",
        (Key::UnitPrice, Lang::En) => "Unit price",
        (Key::UnitPrice, Lang::Ar) => "سعر الوحدة",

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

        // The same sentence about the paper it is actually written on. An
        // avoir that said "la présente facture" would name another
        // document on the line a comptable reads first.
        (Key::AvoirInWords, Lang::Fr) => "Arrêté le présent avoir à la somme de",
        (Key::AvoirInWords, Lang::En) => "This credit note is closed at the sum of",
        (Key::AvoirInWords, Lang::Ar) => "أوقف هذا الإشعار الدائن بمبلغ",

        (Key::ProformaInWords, Lang::Fr) => "Arrêtée la présente proforma à la somme de",
        (Key::ProformaInWords, Lang::En) => "This pro forma invoice is closed at the sum of",
        (Key::ProformaInWords, Lang::Ar) => "أوقفت هذه الفاتورة الأولية بمبلغ",

        (Key::Balance, Lang::Fr) => "Solde",
        (Key::Balance, Lang::En) => "Balance",
        (Key::Balance, Lang::Ar) => "الرصيد",

        (Key::OldBalance, Lang::Fr) => "Ancien solde",
        (Key::OldBalance, Lang::En) => "Previous balance",
        (Key::OldBalance, Lang::Ar) => "الرصيد السابق",

        (Key::ThisDocument, Lang::Fr) => "Ce document",
        (Key::ThisDocument, Lang::En) => "This document",
        (Key::ThisDocument, Lang::Ar) => "هذه الوثيقة",

        (Key::TotalDebt, Lang::Fr) => "Solde total",
        (Key::TotalDebt, Lang::En) => "Total owed",
        (Key::TotalDebt, Lang::Ar) => "الرصيد الإجمالي",

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

        (Key::Document, Lang::Fr) => "Document",
        (Key::Document, Lang::En) => "Document",
        (Key::Document, Lang::Ar) => "الوثيقة",

        (Key::Debit, Lang::Fr) => "Débit",
        (Key::Debit, Lang::En) => "Debit",
        (Key::Debit, Lang::Ar) => "مدين",

        (Key::CreditColumn, Lang::Fr) => "Crédit",
        (Key::CreditColumn, Lang::En) => "Credit",
        (Key::CreditColumn, Lang::Ar) => "دائن",

        (Key::KindOpening, Lang::Fr) => "Solde de départ",
        (Key::KindOpening, Lang::En) => "Balance carried over",
        (Key::KindOpening, Lang::Ar) => "رصيد مُرحَّل",

        (Key::KindSale, Lang::Fr) => "Vente",
        (Key::KindSale, Lang::En) => "Sale",
        (Key::KindSale, Lang::Ar) => "بيع",

        (Key::KindPayment, Lang::Fr) => "Paiement",
        (Key::KindPayment, Lang::En) => "Payment",
        (Key::KindPayment, Lang::Ar) => "دفع",

        (Key::KindAvoir, Lang::Fr) => "Avoir",
        (Key::KindAvoir, Lang::En) => "Credit note",
        (Key::KindAvoir, Lang::Ar) => "إشعار دائن",

        (Key::KindAdjustment, Lang::Fr) => "Ajustement",
        (Key::KindAdjustment, Lang::En) => "Adjustment",
        (Key::KindAdjustment, Lang::Ar) => "تسوية",

        (Key::StatementInWords, Lang::Fr) => "Arrêté le présent relevé à la somme de",
        (Key::StatementInWords, Lang::En) => "This statement is closed at the sum of",
        (Key::StatementInWords, Lang::Ar) => "أوقف هذا الكشف بمبلغ",

        (Key::InFavourOfCustomer, Lang::Fr) => "en faveur du client",
        (Key::InFavourOfCustomer, Lang::En) => "in the customer\u{2019}s favour",
        (Key::InFavourOfCustomer, Lang::Ar) => "لصالح الزبون",

        (Key::NoMovement, Lang::Fr) => "Aucun mouvement sur la période",
        (Key::NoMovement, Lang::En) => "No movement in this period",
        (Key::NoMovement, Lang::Ar) => "لا توجد حركة في هذه الفترة",

        (Key::DebtSlip, Lang::Fr) => "Situation de compte",
        (Key::DebtSlip, Lang::En) => "Account balance slip",
        (Key::DebtSlip, Lang::Ar) => "وضعية الحساب",

        (Key::LastMovements, Lang::Fr) => "Derniers mouvements",
        (Key::LastMovements, Lang::En) => "Latest movements",
        (Key::LastMovements, Lang::Ar) => "آخر الحركات",

        (Key::NoFiscalValue, Lang::Fr) => "Document sans valeur fiscale",
        (Key::NoFiscalValue, Lang::En) => "This slip has no fiscal value",
        (Key::NoFiscalValue, Lang::Ar) => "وثيقة بدون قيمة جبائية",

        (Key::DebtInWords, Lang::Fr) => "Arrêtée la présente situation à la somme de",
        (Key::DebtInWords, Lang::En) => "This slip is closed at the sum of",
        (Key::DebtInWords, Lang::Ar) => "أوقفت هذه الوضعية بمبلغ",

        (Key::SheetProducts, Lang::Fr) => "Produits",
        (Key::SheetProducts, Lang::En) => "Products",
        (Key::SheetProducts, Lang::Ar) => "المنتجات",

        (Key::SheetSales, Lang::Fr) => "Ventes",
        (Key::SheetSales, Lang::En) => "Sales",
        (Key::SheetSales, Lang::Ar) => "المبيعات",

        (Key::SheetCustomers, Lang::Fr) => "Clients",
        (Key::SheetCustomers, Lang::En) => "Customers",
        (Key::SheetCustomers, Lang::Ar) => "الزبائن",

        (Key::SheetSuppliers, Lang::Fr) => "Fournisseurs",
        (Key::SheetSuppliers, Lang::En) => "Suppliers",
        (Key::SheetSuppliers, Lang::Ar) => "الموردون",

        (Key::SheetAllowedValues, Lang::Fr) => "Valeurs autorisées",
        (Key::SheetAllowedValues, Lang::En) => "Allowed values",
        (Key::SheetAllowedValues, Lang::Ar) => "القيم المسموح بها",
    }
}
