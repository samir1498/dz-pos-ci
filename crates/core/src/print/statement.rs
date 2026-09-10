//! The A4 statement of account: what a customer owed at the start of a
//! period, every movement in it, and what they owe at the end.
//!
//! Same contract as the facture: everything the paper says is decided here
//! and handed to `templates/statement_a4.html` already made. Nothing on this
//! page is computed here either. The opening balance, every running balance
//! and the closing balance are read off `services::debt`, which is the one
//! place that works out what a customer owes; a page that added a column up
//! would be a second answer to the question the page exists to answer, and
//! the two would part company the day a kind of movement is added.

use askama::Template;

use crate::error::CoreError;
use crate::lang::Lang;
use crate::models::customer::Customer;
use crate::models::customer::PartyKind;
use crate::money::format::format_centimes;
use crate::money::words::amount_in_words;
use crate::money::Money;
use crate::print::strings::{text, Key};
use crate::print::{number_of, Paper};
use crate::services::debt::{DebtKind, RangedStatement, StatementEntry};

/// A statement carries days and not minutes, like the facture: the day a
/// movement landed is what a comptable reconciles against, and two payments
/// in one afternoon are two rows either way.
const DATE_FORMAT: &str = "%d/%m/%Y";

/// One identifier row of the customer block: `RC`, `NIF`, `NIS`, `AI`. The
/// labels are the abbreviations every Algerian document prints and are not
/// translated, so they stay literals here as they do on the facture.
struct IdRow {
    label: &'static str,
    value: String,
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
    /// one. An opening balance, a payment and a correction cite none: a
    /// payment settles several documents through its allocations and naming
    /// one of them here would be the page picking a favourite.
    document: Option<String>,
    debit: Option<String>,
    credit: Option<String>,
    balance: String,
}

#[derive(Template)]
#[template(path = "statement_a4.html")]
struct StatementView {
    lang_tag: &'static str,
    dir: &'static str,
    arabic_unreviewed: bool,
    paper_size: &'static str,
    title: &'static str,
    period_label: &'static str,
    period: String,
    /// The two edge rows are dated by the day they speak for: the opening
    /// balance is where the first day started and the closing one where the
    /// last day ended.
    from: String,
    to: String,
    customer: CustomerView,
    date_label: &'static str,
    movement_label: &'static str,
    document_label: &'static str,
    debit_label: &'static str,
    credit_label: &'static str,
    balance_label: &'static str,
    opening_label: &'static str,
    opening: String,
    movements: Vec<MovementView>,
    no_movement: &'static str,
    closing_label: &'static str,
    closing: String,
    in_words_label: &'static str,
    in_words: String,
    /// Said after the words when the shop is holding money for the customer.
    /// The words themselves carry no sign, so without this a page reading
    /// "two hundred dinars" would say the opposite of what it means.
    in_favour: Option<&'static str>,
    currency: &'static str,
}

/// The statement for `customer` over the range `statement` was read for, in
/// `lang`, on `paper`, as one standalone HTML page.
///
/// `statement` comes from `services::debt::statement_between`, which is what
/// makes the page a rendering rather than a second calculation: the entries
/// arrive oldest first, each already carrying the balance as of itself.
pub fn render_statement(
    customer: &Customer,
    statement: &RangedStatement,
    lang: Lang,
    paper: Paper,
) -> Result<String, CoreError> {
    view(customer, statement, lang, paper)?
        .render()
        .map_err(CoreError::from)
}

fn view(
    customer: &Customer,
    statement: &RangedStatement,
    lang: Lang,
    paper: Paper,
) -> Result<StatementView, CoreError> {
    // A closing balance below zero is the shop holding money for the
    // customer. `amount_in_words` refuses a negative, and rightly: a written
    // amount has no sign in it. So the words are the amount itself and the
    // direction is a phrase beside them.
    let owed = statement.closing.is_negative();
    let absolute = if owed {
        Money::ZERO.checked_sub(statement.closing)?
    } else {
        statement.closing
    };
    let in_words = amount_in_words(absolute, lang).map_err(|_| {
        CoreError::render("the closing balance has no written form in the print language")
    })?;

    Ok(StatementView {
        lang_tag: lang.tag(),
        dir: lang.dir(),
        arabic_unreviewed: lang == Lang::Ar,
        paper_size: paper.css_size(),
        title: text(Key::Statement, lang),
        period_label: text(Key::Period, lang),
        period: format!(
            "{} - {}",
            statement.from.format(DATE_FORMAT),
            statement.to.format(DATE_FORMAT)
        ),
        from: statement.from.format(DATE_FORMAT).to_string(),
        to: statement.to.format(DATE_FORMAT).to_string(),
        customer: customer_view(customer, lang),
        date_label: text(Key::Date, lang),
        movement_label: text(Key::Movement, lang),
        document_label: text(Key::Document, lang),
        debit_label: text(Key::Debit, lang),
        credit_label: text(Key::CreditColumn, lang),
        balance_label: text(Key::Balance, lang),
        opening_label: text(Key::OpeningBalance, lang),
        opening: format_centimes(statement.opening),
        movements: statement
            .entries
            .iter()
            .map(|entry| movement_view(entry, lang))
            .collect(),
        no_movement: text(Key::NoMovement, lang),
        closing_label: text(Key::ClosingBalance, lang),
        closing: format_centimes(statement.closing),
        in_words_label: text(Key::StatementInWords, lang),
        in_words,
        in_favour: owed.then(|| text(Key::InFavourOfCustomer, lang)),
        currency: text(Key::Currency, lang),
    })
}

/// The customer as the statement names them: the identifiers of a company,
/// the name and address of a consumer, the same rule the facture's buyer
/// block follows (décret 05-468 art. 3-2, last alinéa). A consumer fiche that
/// once carried an RC is not turned into a company by this page either.
fn customer_view(customer: &Customer, lang: Lang) -> CustomerView {
    let company = customer.party_kind == PartyKind::Company;
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
        ids: if company {
            ids.into_iter()
                .filter_map(|(label, value)| {
                    value.map(|value| IdRow {
                        label,
                        value: value.clone(),
                    })
                })
                .collect()
        } else {
            Vec::new()
        },
    }
}

/// One row. A movement raises the debt or lowers it and never both, so the
/// column it does not use is empty rather than carrying a zero: a statement
/// full of 0,00 in the credit column is a page a shop has to read twice.
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

/// The word for why the debt moved. Sentence case, because it is read in a
/// table cell; `Key::Avoir` is the heading of a document and would shout.
const fn kind_key(kind: DebtKind) -> Key {
    match kind {
        DebtKind::Opening => Key::KindOpening,
        DebtKind::Sale => Key::KindSale,
        DebtKind::Payment => Key::KindPayment,
        DebtKind::Avoir => Key::KindAvoir,
        DebtKind::Adjustment => Key::KindAdjustment,
    }
}
