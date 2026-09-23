//! The printed word list, in the three languages, split by S4 of
//! `a-kernel-crate-and-retail-as-the-first-module` the way the error enum
//! was: `dzpos_kernel::print::strings::Key` carries every word this file
//! used to hold that names no shop concept (`Ticket`, `Facture`,
//! `Statement`, `DebtSlip` and the rest, re-exported below so
//! `crate::print::strings::{text, Key}` keeps resolving unchanged), and
//! `ShopKey` here carries the fourteen shop words the boundary walk
//! (`crates/core/tests/services_go_through_services.rs`,
//! `the_shared_kernel_does_not_name_the_shop`) pinned on the kernel's copy
//! of this file: `avoir`, `barcode`, `customer`, `customers`, `debt`,
//! `discount`, `document`, `price`, `products`, `proforma`, `sale`,
//! `sales`, `stock`, `suppliers`.
//!
//! Two enums and not one flat one: `Key` is `Copy` and has no shop meaning,
//! so every other file in this crate that only ever prints "Total" or
//! "Tva" keeps calling `dzpos_kernel::print::strings::text` directly and
//! never has to know `ShopKey` exists. A caller that needs a word from
//! either list imports both, the same way `crates/api/src/error.rs` imports
//! both error enums rather than one wrapping the other: wrapping would have
//! meant writing `Key::Shop(ShopKey::Avoir)` at every call site this file's
//! own `git blame` already shows using `Key::Avoir` bare.
//!
//! `crates/core/tests/print_strings.rs` walks `Key::ALL × Lang::ALL` and
//! `ShopKey::ALL × Lang::ALL` both, so a key added to either without its
//! Arabic is a failing test, not a French word on an Arabic ticket.

use dzpos_kernel::lang::Lang;

pub use dzpos_kernel::print::strings::{text, Key};

/// A shop word a printed document names. One variant per printed word that
/// the kernel's own `Key` may not carry (see this file's own doc), so a
/// translator working the shop's own papers has one list to read for them
/// and the kernel's for the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShopKey {
    Discount,
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
    /// The last row of an avoir's totals. A facture closes on the net to
    /// pay, the figure the buyer owes; an avoir asks for nothing, so the
    /// same row names the amount of the avoir. The figure is the same one,
    /// under the words that fit the paper it is on.
    AvoirAmount,
    /// The unit price column of a réel facture: "hors taxes" is one of the
    /// mentions décret 05-468 art. 3 asks for by name.
    UnitPriceHt,
    /// The same column under the IFU, where naming the tax is forbidden
    /// (CTCA 2026 art. 64).
    UnitPrice,
    AvoirInWords,
    ProformaInWords,
    ThisDocument,
    TotalDebt,
    /// The column carrying the number of the document a movement cites.
    Document,
    KindSale,
    KindAvoir,
    /// Added to the words line when the closing balance is below zero: the
    /// shop is holding money for the customer, and the words themselves carry
    /// no sign.
    InFavourOfCustomer,

    // The 80 mm debt slip (features.md §2 and §4). What a credit customer is
    // handed at the counter when they ask what they owe: the balance, the
    // movements behind it, and nothing a comptable would file. The heading
    // of its movement block and the two lines that keep it out of a
    // comptable's file carry no shop word of their own and stay on
    // `dzpos_kernel::print::strings::Key` (`LastMovements`, `NoFiscalValue`).
    /// The heading. Sentence case like the ticket's and not the facture's
    /// capitals: it is a counter paper, not a document with a series.
    DebtSlip,
    /// The words line of a debt slip. Not the statement's, which says "le
    /// présent relevé" about a page covering a period this one has none of.
    DebtInWords,

    // The Excel workbooks (features.md §1, Backup). Only the sheet name is
    // translated: a workbook's header row and its unit and status cells are
    // the stable key names, so a products export edited in a spreadsheet
    // reads straight back through the import and no column has to be found
    // again in three languages. The tab is what a person sees first, and it
    // is the one place a word costs nothing to translate. The other tab of
    // the import template, and the two headings on it, carry no shop word
    // and stay on the kernel's own `Key` (`SheetAllowedValues`,
    // `TemplateUnits`, `TemplateRates`).
    /// The tab of the products workbook, and of the import template.
    SheetProducts,
    SheetSales,
    SheetCustomers,
    SheetSuppliers,
    /// What the example row is there to say: a code the shop already sells
    /// under updates that product rather than opening a second one, and a
    /// blank code is numbered by the till.
    TemplateBarcodeNote,
    /// What the `stock` column does, which is nothing on a product the shop
    /// already has: quantities belong to the stock ledger and an import is
    /// not a movement.
    TemplateStockNote,
}

impl ShopKey {
    /// Every shop key, in the order the dictionary test walks them.
    pub const ALL: [ShopKey; 24] = [
        ShopKey::Discount,
        ShopKey::Avoir,
        ShopKey::Proforma,
        ShopKey::ProformaNotice,
        ShopKey::AvoirOnFacture,
        ShopKey::AvoirAmount,
        ShopKey::UnitPriceHt,
        ShopKey::UnitPrice,
        ShopKey::AvoirInWords,
        ShopKey::ProformaInWords,
        ShopKey::ThisDocument,
        ShopKey::TotalDebt,
        ShopKey::Document,
        ShopKey::KindSale,
        ShopKey::KindAvoir,
        ShopKey::InFavourOfCustomer,
        ShopKey::DebtSlip,
        ShopKey::DebtInWords,
        ShopKey::SheetProducts,
        ShopKey::SheetSales,
        ShopKey::SheetCustomers,
        ShopKey::SheetSuppliers,
        ShopKey::TemplateBarcodeNote,
        ShopKey::TemplateStockNote,
    ];
}

/// What `key` says in `lang`. One `match`, no map and no lookup that can
/// miss: a key with no arm does not compile. The kernel's own
/// `dzpos_kernel::print::strings::text` is the sibling of this function for
/// every word that is not a shop word.
pub const fn shop_text(key: ShopKey, lang: Lang) -> &'static str {
    match (key, lang) {
        (ShopKey::Discount, Lang::Fr) => "Remise",
        (ShopKey::Discount, Lang::En) => "Discount",
        (ShopKey::Discount, Lang::Ar) => "تخفيض",

        (ShopKey::Avoir, Lang::Fr) => "AVOIR",
        (ShopKey::Avoir, Lang::En) => "CREDIT NOTE",
        (ShopKey::Avoir, Lang::Ar) => "إشعار دائن",

        (ShopKey::Proforma, Lang::Fr) => "PROFORMA",
        (ShopKey::Proforma, Lang::En) => "PRO FORMA INVOICE",
        (ShopKey::Proforma, Lang::Ar) => "فاتورة أولية",

        (ShopKey::ProformaNotice, Lang::Fr) => {
            "Proforma, sans valeur comptable\u{202f}: ce document n\u{2019}est pas une facture et ne crée aucune dette."
        }
        (ShopKey::ProformaNotice, Lang::En) => {
            "Pro forma, of no accounting value: this document is not an invoice and creates no debt."
        }
        (ShopKey::ProformaNotice, Lang::Ar) => {
            "فاتورة أولية، بدون قيمة محاسبية: هذه الوثيقة ليست فاتورة ولا تنشئ أي دين."
        }

        (ShopKey::AvoirOnFacture, Lang::Fr) => "Avoir sur facture",
        (ShopKey::AvoirOnFacture, Lang::En) => "Credit note against invoice",
        (ShopKey::AvoirOnFacture, Lang::Ar) => "إشعار دائن على الفاتورة",

        // Unreviewed wording, like this dictionary's Arabic: no comptable
        // has read the French of it yet. The apostrophe is the typographic
        // one, which is the French a printed document uses and also the one
        // that reaches the page as itself: a typewriter apostrophe would
        // sit in the golden as `&#39;`.
        (ShopKey::AvoirAmount, Lang::Fr) => "Montant de l\u{2019}avoir",
        (ShopKey::AvoirAmount, Lang::En) => "Credit note amount",
        (ShopKey::AvoirAmount, Lang::Ar) => "مبلغ الإشعار الدائن",

        (ShopKey::UnitPriceHt, Lang::Fr) => "Prix unitaire HT",
        (ShopKey::UnitPriceHt, Lang::En) => "Unit price excl. tax",
        (ShopKey::UnitPriceHt, Lang::Ar) => "سعر الوحدة خارج الرسم",

        (ShopKey::UnitPrice, Lang::Fr) => "Prix unitaire",
        (ShopKey::UnitPrice, Lang::En) => "Unit price",
        (ShopKey::UnitPrice, Lang::Ar) => "سعر الوحدة",

        // The same sentence about the paper it is actually written on. An
        // avoir that said "la présente facture" would name another
        // document on the line a comptable reads first.
        (ShopKey::AvoirInWords, Lang::Fr) => "Arrêté le présent avoir à la somme de",
        (ShopKey::AvoirInWords, Lang::En) => "This credit note is closed at the sum of",
        (ShopKey::AvoirInWords, Lang::Ar) => "أوقف هذا الإشعار الدائن بمبلغ",

        (ShopKey::ProformaInWords, Lang::Fr) => "Arrêtée la présente proforma à la somme de",
        (ShopKey::ProformaInWords, Lang::En) => "This pro forma invoice is closed at the sum of",
        (ShopKey::ProformaInWords, Lang::Ar) => "أوقفت هذه الفاتورة الأولية بمبلغ",

        (ShopKey::ThisDocument, Lang::Fr) => "Ce document",
        (ShopKey::ThisDocument, Lang::En) => "This document",
        (ShopKey::ThisDocument, Lang::Ar) => "هذه الوثيقة",

        (ShopKey::TotalDebt, Lang::Fr) => "Solde total",
        (ShopKey::TotalDebt, Lang::En) => "Total owed",
        (ShopKey::TotalDebt, Lang::Ar) => "الرصيد الإجمالي",

        (ShopKey::Document, Lang::Fr) => "Document",
        (ShopKey::Document, Lang::En) => "Document",
        (ShopKey::Document, Lang::Ar) => "الوثيقة",

        (ShopKey::KindSale, Lang::Fr) => "Vente",
        (ShopKey::KindSale, Lang::En) => "Sale",
        (ShopKey::KindSale, Lang::Ar) => "بيع",

        (ShopKey::KindAvoir, Lang::Fr) => "Avoir",
        (ShopKey::KindAvoir, Lang::En) => "Credit note",
        (ShopKey::KindAvoir, Lang::Ar) => "إشعار دائن",

        (ShopKey::InFavourOfCustomer, Lang::Fr) => "en faveur du client",
        (ShopKey::InFavourOfCustomer, Lang::En) => "in the customer\u{2019}s favour",
        (ShopKey::InFavourOfCustomer, Lang::Ar) => "لصالح الزبون",

        (ShopKey::DebtSlip, Lang::Fr) => "Situation de compte",
        (ShopKey::DebtSlip, Lang::En) => "Account balance slip",
        (ShopKey::DebtSlip, Lang::Ar) => "وضعية الحساب",

        (ShopKey::DebtInWords, Lang::Fr) => "Arrêtée la présente situation à la somme de",
        (ShopKey::DebtInWords, Lang::En) => "This slip is closed at the sum of",
        (ShopKey::DebtInWords, Lang::Ar) => "أوقفت هذه الوضعية بمبلغ",

        (ShopKey::SheetProducts, Lang::Fr) => "Produits",
        (ShopKey::SheetProducts, Lang::En) => "Products",
        (ShopKey::SheetProducts, Lang::Ar) => "المنتجات",

        (ShopKey::SheetSales, Lang::Fr) => "Ventes",
        (ShopKey::SheetSales, Lang::En) => "Sales",
        (ShopKey::SheetSales, Lang::Ar) => "المبيعات",

        (ShopKey::SheetCustomers, Lang::Fr) => "Clients",
        (ShopKey::SheetCustomers, Lang::En) => "Customers",
        (ShopKey::SheetCustomers, Lang::Ar) => "الزبائن",

        (ShopKey::SheetSuppliers, Lang::Fr) => "Fournisseurs",
        (ShopKey::SheetSuppliers, Lang::En) => "Suppliers",
        (ShopKey::SheetSuppliers, Lang::Ar) => "الموردون",

        (ShopKey::TemplateStockNote, Lang::Fr) => "La colonne stock ne sert qu\u{2019}\u{e0} l\u{2019}ouverture d\u{2019}un nouveau produit. Sur un produit que la boutique a d\u{e9}j\u{e0}, elle est ignor\u{e9}e : un import ne fait jamais bouger le stock, seuls un achat, une vente et un recomptage le font.",
        (ShopKey::TemplateStockNote, Lang::En) => "The stock column opens a new product with that quantity. On a product the shop already has it is ignored: an import never moves stock, only a purchase, a sale and a recount do.",
        (ShopKey::TemplateStockNote, Lang::Ar) => "\u{639}\u{645}\u{648}\u{62f} \u{627}\u{644}\u{645}\u{62e}\u{632}\u{648}\u{646} \u{64a}\u{641}\u{62a}\u{62d} \u{645}\u{646}\u{62a}\u{648}\u{62c}\u{64b}\u{627} \u{62c}\u{62f}\u{64a}\u{62f}\u{64b}\u{627} \u{628}\u{647}\u{630}\u{647} \u{627}\u{644}\u{643}\u{645}\u{64a}\u{629}\u{60c} \u{648}\u{64a}\u{64f}\u{647}\u{645}\u{644} \u{639}\u{644}\u{649} \u{645}\u{646}\u{62a}\u{648}\u{62c} \u{645}\u{648}\u{62c}\u{648}\u{62f} \u{623}\u{635}\u{644}\u{64b}\u{627}: \u{627}\u{644}\u{627}\u{633}\u{62a}\u{64a}\u{631}\u{627}\u{62f} \u{644}\u{627} \u{64a}\u{62d}\u{631}\u{651}\u{643} \u{627}\u{644}\u{645}\u{62e}\u{632}\u{648}\u{646} \u{623}\u{628}\u{62f}\u{64b}\u{627}.",
        (ShopKey::TemplateBarcodeNote, Lang::Fr) => concat!(
            "Un code-barres que la boutique utilise déjà met à jour ce produit ",
            "(nom, prix d\u{2019}achat, prix de vente) au lieu d\u{2019}en créer un second. ",
            "Laissez la colonne vide et la caisse attribue un code."
        ),
        (ShopKey::TemplateBarcodeNote, Lang::En) => concat!(
            "A barcode the shop already sells under updates that product ",
            "(name, cost, price) instead of opening a second one. ",
            "Leave the column empty and the till assigns a code."
        ),
        (ShopKey::TemplateBarcodeNote, Lang::Ar) => concat!(
            "\u{0627}\u{0644}\u{0631}\u{0645}\u{0632} \u{0627}\u{0644}\u{0634}\u{0631}\u{064a}\u{0637}\u{064a} ",
            "\u{0627}\u{0644}\u{0645}\u{0633}\u{062a}\u{0639}\u{0645}\u{0644} \u{0645}\u{0646} \u{0642}\u{0628}\u{0644} ",
            "\u{064a}\u{062d}\u{062f}\u{0651}\u{062b} \u{0627}\u{0644}\u{0645}\u{0646}\u{062a}\u{0648}\u{062c} ",
            "\u{0628}\u{062f}\u{0644} \u{0625}\u{0646}\u{0634}\u{0627}\u{0621} \u{0645}\u{0646}\u{062a}\u{0648}\u{062c} \u{062b}\u{0627}\u{0646}\u{064d}. ",
            "\u{0627}\u{062a}\u{0631}\u{0643} \u{0627}\u{0644}\u{062e}\u{0627}\u{0646}\u{0629} \u{0641}\u{0627}\u{0631}\u{063a}\u{0629} ",
            "\u{0644}\u{062a}\u{0639}\u{064a}\u{0651}\u{0646} \u{0627}\u{0644}\u{0635}\u{0646}\u{062f}\u{0648}\u{0642} \u{0631}\u{0645}\u{0632}\u{064b}\u{0627}."
        ),
    }
}
