//! The 80 mm debt slip: what a credit customer is handed at the counter when
//! they ask what they owe.
//!
//! Same contract as the ticket and the statement. Everything the paper says is
//! decided here and handed to `templates/debt_slip_80mm.html` already made,
//! and nothing on it is computed here either: the balance and every running
//! balance are read off `services::debt`, which is the one place that works
//! out what a customer owes.
//!
//! The slip is a customer-facing paper without fiscal value. It carries no
//! number from a series, no stamp and no TVA, and it says so on its own last
//! line: the papers that prove something are the facture and the statement,
//! and a slip a comptable could mistake for either would be worse than no
//! slip at all.

use askama::Template;
use chrono::NaiveDateTime;

use crate::error::CoreError;
use crate::lang::Lang;
use crate::models::customer::{Customer, PartyKind};
use crate::models::document::SellerBlock;
use crate::money::format::format_centimes;
use crate::money::words::amount_in_words;
use crate::money::Money;
use crate::print::number_of;
use crate::print::strings::{text, Key};
use crate::services::debt::{DebtKind, RecentStatement, StatementEntry};

/// The day and the minute the slip was printed. A balance is a figure as of a
/// moment, and a counter paper that did not say which moment would be quoted
/// back at the shop a month later.
const STAMP_FORMAT: &str = "%d/%m/%Y %H:%M";

/// The day a movement landed, as the statement dates its own rows: the day is
/// what a customer reconciles against and 72 mm has no room for the minute.
const DATE_FORMAT: &str = "%d/%m/%Y";

/// How many movements the slip carries. The newest ten, which is roughly what
/// fits on a hand's width of paper and enough for a customer to recognise the
/// last few times they came in. The cap is here rather than at the caller so
/// that one paper is one length, whatever a route asks for.
pub const MOVEMENTS: usize = 10;

/// One identifier row, `RC`, `NIF`, `NIS` or `AI`. The labels are the
/// abbreviations every Algerian document prints and are not translated, the
/// same as on the ticket and the statement.
struct IdRow {
    label: &'static str,
    value: String,
}

struct SellerView {
    name: String,
    address: Option<String>,
    phone: Option<String>,
    ids: Vec<IdRow>,
}

struct CustomerView {
    title: &'static str,
    name: String,
    address: Option<String>,
    phone: Option<String>,
    ids: Vec<IdRow>,
}

struct MovementView {
    date: String,
    kind: &'static str,
    /// The printed number of the document the movement cites, when it cites
    /// one. An opening balance, a payment and a correction cite none.
    document: Option<String>,
    debit: Option<String>,
    credit: Option<String>,
    balance: String,
}

#[derive(Template)]
#[template(path = "debt_slip_80mm.html")]
struct DebtSlipView {
    lang_tag: &'static str,
    dir: &'static str,
    arabic_unreviewed: bool,
    title: &'static str,
    printed_at: String,
    seller: SellerView,
    customer: CustomerView,
    balance_label: &'static str,
    balance: String,
    in_words_label: &'static str,
    in_words: String,
    /// Said after the words when the shop is holding money for the customer.
    /// The words themselves carry no sign, so without this a slip reading
    /// "two hundred dinars" would say the opposite of what it means.
    in_favour: Option<&'static str>,
    movements_label: &'static str,
    debit_label: &'static str,
    credit_label: &'static str,
    movements: Vec<MovementView>,
    no_movement: &'static str,
    no_fiscal_value: &'static str,
    currency: &'static str,
}

/// The 80 mm debt slip for `customer`, in `lang`, as one standalone HTML page.
///
/// `slip` comes from `services::debt::recent`, which is what makes the page a
/// rendering rather than a second calculation: the balance is the whole
/// ledger's and each movement arrives carrying the balance as of itself.
/// `seller` is the shop as `services::shops` holds it today, not a snapshot: a
/// slip is a paper about the account and not a document with a number, so
/// there is nothing here for it to have been snapshotted against.
///
/// Pinned byte for byte by `fixtures/print/debt_slip_80mm/`, one file per
/// language.
pub fn render_debt_slip(
    seller: &SellerBlock,
    customer: &Customer,
    slip: &RecentStatement,
    at: NaiveDateTime,
    lang: Lang,
) -> Result<String, CoreError> {
    view(seller, customer, slip, at, lang)?
        .render()
        .map_err(CoreError::from)
}

fn view(
    seller: &SellerBlock,
    customer: &Customer,
    slip: &RecentStatement,
    at: NaiveDateTime,
    lang: Lang,
) -> Result<DebtSlipView, CoreError> {
    // A balance below zero is the shop holding money for the customer.
    // `amount_in_words` refuses a negative, and rightly: a written amount has
    // no sign in it. So the words are the amount itself and the direction is a
    // phrase beside them, the way the statement says it.
    let held = slip.balance.is_negative();
    let absolute = if held {
        Money::ZERO.checked_sub(slip.balance)?
    } else {
        slip.balance
    };
    let in_words = amount_in_words(absolute, lang)
        .map_err(|_| CoreError::render("the balance has no written form in the print language"))?;

    Ok(DebtSlipView {
        lang_tag: lang.tag(),
        dir: lang.dir(),
        arabic_unreviewed: lang == Lang::Ar,
        title: text(Key::DebtSlip, lang),
        printed_at: at.format(STAMP_FORMAT).to_string(),
        seller: seller_view(seller),
        customer: customer_view(customer, lang),
        balance_label: text(Key::Balance, lang),
        balance: format_centimes(slip.balance),
        in_words_label: text(Key::DebtInWords, lang),
        in_words,
        in_favour: held.then(|| text(Key::InFavourOfCustomer, lang)),
        movements_label: text(Key::LastMovements, lang),
        debit_label: text(Key::Debit, lang),
        credit_label: text(Key::CreditColumn, lang),
        // The newest ten and no more. A customer who has bought fifty times
        // gets the last ten of them and a balance that counts all fifty.
        movements: slip
            .entries
            .iter()
            .take(MOVEMENTS)
            .map(|entry| movement_view(entry, lang))
            .collect(),
        no_movement: text(Key::NoMovement, lang),
        no_fiscal_value: text(Key::NoFiscalValue, lang),
        currency: text(Key::Currency, lang),
    })
}

fn seller_view(seller: &SellerBlock) -> SellerView {
    let ids = [
        ("NIF", seller.nif.as_ref()),
        ("RC", seller.rc.as_ref()),
        ("NIS", seller.nis.as_ref()),
        ("AI", seller.ai.as_ref()),
    ];
    SellerView {
        name: seller.name.clone(),
        address: seller.address.clone(),
        phone: seller.phone.clone(),
        ids: id_rows(ids),
    }
}

/// The customer as the slip names them: the identifiers of a company, the name
/// and address of a consumer, the same rule the statement's block follows
/// (décret 05-468 art. 3-2, last alinéa). A consumer fiche that once carried an
/// RC is not turned into a company by this page either.
fn customer_view(customer: &Customer, lang: Lang) -> CustomerView {
    let ids = [
        ("RC", customer.rc.as_ref()),
        ("NIF", customer.nif.as_ref()),
        ("NIS", customer.nis.as_ref()),
        ("AI", customer.ai.as_ref()),
    ];
    CustomerView {
        title: text(Key::Buyer, lang),
        name: customer.name.clone(),
        address: customer.address.clone(),
        phone: customer.phone.clone(),
        ids: if customer.party_kind == PartyKind::Company {
            id_rows(ids)
        } else {
            Vec::new()
        },
    }
}

/// The identifiers a block carries, in the order it prints them, and no empty
/// rows: a paper 72 mm wide has no space for a label with nothing after it.
fn id_rows(ids: [(&'static str, Option<&String>); 4]) -> Vec<IdRow> {
    ids.into_iter()
        .filter_map(|(label, value)| {
            value.map(|value| IdRow {
                label,
                value: value.clone(),
            })
        })
        .collect()
}

/// One movement. It raises the debt or lowers it and never both, so the column
/// it does not use is left off rather than carrying a zero.
fn movement_view(line: &StatementEntry, lang: Lang) -> MovementView {
    MovementView {
        date: line.entry.created_at.format(DATE_FORMAT).to_string(),
        kind: text(kind_key(line.entry.kind), lang),
        document: line
            .document
            .map(|doc| number_of(doc.kind, doc.year, doc.number)),
        debit: (line.entry.debit != Money::ZERO).then(|| format_centimes(line.entry.debit)),
        credit: (line.entry.credit != Money::ZERO).then(|| format_centimes(line.entry.credit)),
        balance: format_centimes(line.balance_after),
    }
}

/// The word for why the debt moved, the statement's words rather than a second
/// set: a customer holding both papers must not find two spellings of the same
/// movement.
const fn kind_key(kind: DebtKind) -> Key {
    match kind {
        DebtKind::Opening => Key::KindOpening,
        DebtKind::Sale => Key::KindSale,
        DebtKind::Payment => Key::KindPayment,
        DebtKind::Avoir => Key::KindAvoir,
        DebtKind::Adjustment => Key::KindAdjustment,
    }
}
