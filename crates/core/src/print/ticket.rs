//! The 80 mm ticket, rendered from a stored document.
//!
//! Everything a ticket says is decided here and handed to
//! `templates/ticket_80mm.html` already made: the template has an `if` and
//! a `for` and no rule at all. That is what makes a template change a
//! reviewed golden diff and not a change of behaviour hiding in markup.
//!
//! The document is the only source. Nothing is recomputed on the way to
//! paper: a reprint six months later shows the totals the shop was paid,
//! not the totals today's rates would give.

use askama::Template;

use crate::error::CoreError;
use crate::lang::Lang;
use crate::models::document::{Document, DocumentLine, SellerBlock};
use crate::money::format::{format_centimes, format_qty};
use crate::money::{Bps, Money, PaymentMode, Regime};
use crate::print::strings::{text, Key};

/// `TK-000123`: the kind's short prefix, a hyphen, and the number in the
/// series padded to six digits. The stored `series` is the counter's name
/// (`doc_ticket`), which is a column and not something a customer quotes;
/// the prefix is the printed form of the same series and lives beside it on
/// `DocumentKind` so the two cannot drift.
const NUMBER_DIGITS: usize = 6;

/// The date and time as the shop reads them. `issued_at` is already on the
/// shop's calendar (`services::clock` writes it there, UTC+1 all year), so
/// printing it is a format and never a conversion.
const STAMP_FORMAT: &str = "%d/%m/%Y %H:%M";

/// The seller's identifiers, in the order a ticket prints them. Only the
/// ones the document snapshotted appear: a ticket carries the seller
/// identity and no empty rows (features.md, party identifiers row; a
/// facture's fuller block is M2's).
struct SellerId {
    label: &'static str,
    value: String,
}

struct SellerView {
    name: String,
    address: Option<String>,
    phone: Option<String>,
    ids: Vec<SellerId>,
}

struct LineView {
    name: String,
    qty: String,
    unit_price: String,
    /// The line's TVA rate, under the réel only. Under the IFU there is no
    /// rate column at all, not a column of zeroes
    /// (`regime_ifu_prints_no_tva`).
    rate: Option<String>,
    discount: Option<String>,
    total: String,
}

struct TvaRow {
    label: String,
    amount: String,
}

#[derive(Template)]
#[template(path = "ticket_80mm.html")]
struct TicketView {
    lang_tag: &'static str,
    dir: &'static str,
    arabic_unreviewed: bool,
    title: &'static str,
    seller: SellerView,
    number: String,
    issued_at: String,
    lines: Vec<LineView>,
    total_label: &'static str,
    total_amount: String,
    discount_label: &'static str,
    discount: Option<String>,
    tva_rows: Vec<TvaRow>,
    stamp_label: &'static str,
    stamp: Option<String>,
    net_to_pay_label: &'static str,
    net_to_pay: String,
    payment_mode_label: &'static str,
    payment_mode: &'static str,
    tendered_label: &'static str,
    tendered: Option<String>,
    change_label: &'static str,
    change: Option<String>,
    currency: &'static str,
    thank_you: &'static str,
}

/// The 80 mm ticket for `doc`, in `lang`, as one standalone HTML page.
///
/// Pinned byte for byte by `fixtures/print/ticket_80mm/`, nine files: three
/// languages under the réel paid in cash, the same three under the IFU, and
/// the same three under the réel paid by card.
pub fn render_ticket(doc: &Document, lang: Lang) -> Result<String, CoreError> {
    // `regime_ifu_prints_no_tva` is a rule about the document, not a layout
    // the template applies on the way past. A stored IFU document that
    // carries a TVA recap contradicts the régime it was issued under (a
    // restored file, a repaired row, an import), and there is no honest
    // ticket for it: printing the recap names a tax the document must not
    // name (CTCA 2026 art. 64), and dropping it quietly hands the customer
    // a total whose parts do not add up. The refusal is the answer.
    //
    // The test for it is the whole point of the check: nothing this app
    // writes can reach here, so only a file that came from somewhere else
    // can, and that is exactly when a wrong ticket would be believed.
    if doc.regime == Regime::Ifu && !doc.totals.tva_by_rate.is_empty() {
        return Err(CoreError::render(
            "an IFU document carries a TVA recap and has no printable form",
        ));
    }
    view(doc, lang).render().map_err(CoreError::from)
}

fn view(doc: &Document, lang: Lang) -> TicketView {
    let reel = doc.regime == Regime::Reel;
    let totals = &doc.totals;
    // Cash is the only mode that takes a note and gives coins back. A card
    // or a credit sale stores neither, and a ticket that printed "Monnaie à
    // rendre 0,00" would be answering a question nobody asked.
    let cash = doc.payment_mode == PaymentMode::Cash;

    TicketView {
        lang_tag: lang.tag(),
        dir: lang.dir(),
        arabic_unreviewed: lang == Lang::Ar,
        title: text(Key::Ticket, lang),
        seller: seller(&doc.seller),
        number: number(doc),
        issued_at: doc.issued_at.format(STAMP_FORMAT).to_string(),
        lines: doc.lines.iter().map(|l| line(l, reel)).collect(),
        // Under the IFU a price is one price: "HT" would name a tax the
        // document must not mention (CTCA 2026 art. 64), so the row keeps
        // the amount and changes the word.
        total_label: text(if reel { Key::TotalHt } else { Key::Total }, lang),
        total_amount: format_centimes(totals.total_ht),
        discount_label: text(Key::Discount, lang),
        discount: some_amount(totals.discount),
        // Empty under the IFU because the document stores no recap there,
        // not because the template hides one.
        tva_rows: totals
            .tva_by_rate
            .iter()
            .map(|row| TvaRow {
                label: format!("{} {}", text(Key::Tva, lang), percent(row.rate)),
                amount: format_centimes(row.amount),
            })
            .collect(),
        stamp_label: text(Key::Stamp, lang),
        stamp: some_amount(totals.stamp),
        net_to_pay_label: text(Key::NetToPay, lang),
        net_to_pay: format_centimes(totals.net_to_pay),
        payment_mode_label: text(Key::PaymentMode, lang),
        payment_mode: text(payment_mode(doc.payment_mode), lang),
        tendered_label: text(Key::Tendered, lang),
        tendered: cash.then(|| doc.tendered.map(format_centimes)).flatten(),
        change_label: text(Key::Change, lang),
        change: cash.then(|| doc.change.map(format_centimes)).flatten(),
        currency: text(Key::Currency, lang),
        thank_you: text(Key::ThankYou, lang),
    }
}

/// A row that is only there when there is something on it. Zero is not a
/// discount and not a stamp; a ticket says nothing about either.
fn some_amount(amount: Money) -> Option<String> {
    (amount != Money::ZERO).then(|| format_centimes(amount))
}

fn number(doc: &Document) -> String {
    format!(
        "{}-{:0width$}",
        doc.kind.number_prefix(),
        doc.number,
        width = NUMBER_DIGITS
    )
}

fn seller(seller: &SellerBlock) -> SellerView {
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
        ids: ids
            .into_iter()
            .filter_map(|(label, value)| {
                value.map(|value| SellerId {
                    label,
                    value: value.clone(),
                })
            })
            .collect(),
    }
}

fn line(line: &DocumentLine, reel: bool) -> LineView {
    LineView {
        name: line.name.clone(),
        qty: format_qty(line.qty_milli),
        unit_price: format_centimes(line.unit_price),
        rate: reel.then(|| percent(line.rate_bps)),
        discount: some_amount(line.line_discount),
        total: format_centimes(line.line_total),
    }
}

/// A rate in basis points as a person reads it: 1900 is `19 %`, 950 is
/// `9,5 %`. The space before the sign is a narrow no-break one, the same
/// character the thousands separator uses, so a rate never breaks across
/// two lines of a 72 mm column.
fn percent(rate: Bps) -> String {
    let bps = rate.as_u32();
    let whole = bps / 100;
    let rest = bps % 100;
    if rest == 0 {
        return format!("{whole}\u{202f}%");
    }
    let decimals = format!("{rest:02}");
    format!("{whole},{}\u{202f}%", decimals.trim_end_matches('0'))
}

const fn payment_mode(mode: PaymentMode) -> Key {
    match mode {
        PaymentMode::Cash => Key::Cash,
        PaymentMode::Card => Key::Card,
        PaymentMode::Credit => Key::Credit,
    }
}

#[cfg(test)]
mod tests {
    use super::percent;
    use crate::money::Bps;

    #[test]
    fn a_rate_is_a_percentage_with_the_decimals_it_needs() {
        let bps = |v: u32| Bps::new(v).unwrap_or(Bps::ZERO);
        assert_eq!(percent(bps(1900)), "19\u{202f}%");
        assert_eq!(percent(bps(900)), "9\u{202f}%");
        assert_eq!(percent(bps(0)), "0\u{202f}%");
        assert_eq!(percent(bps(950)), "9,5\u{202f}%");
        assert_eq!(percent(bps(1)), "0,01\u{202f}%");
        assert_eq!(percent(bps(10_000)), "100\u{202f}%");
    }
}
