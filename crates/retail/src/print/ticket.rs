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

use crate::error::RetailError;
use crate::lang::Lang;
use crate::models::document::{BalanceTriple, Document, DocumentLine, SellerBlock};
use crate::money::format::{format_centimes, format_qty};
use crate::money::{PaymentMode, Regime};
use crate::print::strings::{shop_text, text, Key, ShopKey};
use crate::print::{number, payment_mode_key, percent, some_amount};

/// The date and time as the shop reads them. `issued_at` is already on the
/// shop's calendar (`services::clock` writes it there, UTC+1 all year), so
/// printing it is a format and never a conversion.
const STAMP_FORMAT: &str = "%d/%m/%Y %H:%M";

/// Columns on an 80 mm head. The text path counts one byte per column and
/// the raster path draws 42 glyphs of a monospaced face across the head's
/// dots (`print::raster`), so both papers have the same budget and a line
/// padded to fit one fits the other.
pub(crate) const WIDTH: usize = 42;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Align {
    Left,
    Center,
}

/// One thing the ticket does, in the order it does it.
///
/// This is the whole ticket, and it is the only place its words are
/// decided. `escpos::encode` turns this list into bytes a text-mode head
/// prints; `raster::draw` turns the same list into dots for a head with no
/// Arabic table. Neither of them formats an amount: by the time an `Item`
/// exists every number on the paper is already a string, so the two papers
/// cannot disagree about a total
/// (`the_raster_draws_the_lines_the_text_path_prints`).
pub(crate) enum Item {
    Align(Align),
    Bold(bool),
    Line(String),
    Feed(u8),
    Cut,
}

/// The seller's identifiers, in the order a ticket prints them. Only the
/// ones the document snapshotted appear: a ticket carries the seller
/// identity and no empty rows (features.md, party identifiers row; a
/// facture carries the fuller block of both parties).
pub(crate) struct SellerId {
    pub(crate) label: &'static str,
    pub(crate) value: String,
}

pub(crate) struct SellerView {
    pub(crate) name: String,
    pub(crate) address: Option<String>,
    pub(crate) phone: Option<String>,
    pub(crate) ids: Vec<SellerId>,
}

pub(crate) struct LineView {
    pub(crate) name: String,
    pub(crate) qty: String,
    pub(crate) unit_price: String,
    /// The line's TVA rate, under the réel only. Under the IFU there is no
    /// rate column at all, not a column of zeroes
    /// (`an_ifu_ticket_names_no_tax_in_any_language`).
    pub(crate) rate: Option<String>,
    pub(crate) discount: Option<String>,
    pub(crate) total: String,
}

/// One line of the TVA recap. The word and the rate are two fields rather
/// than one built string, so the golden carries the rate in a span of its own
/// and the suite can read it back the way the facture's does: a recap row
/// that slid onto the wrong rate is caught by the rate and not only by the
/// amount beside it.
pub(crate) struct TvaRow {
    pub(crate) label: &'static str,
    pub(crate) rate: String,
    pub(crate) amount: String,
}

/// The three amounts of the debt as they stood when the ticket was issued.
/// Printed together or not at all, the way the facture prints them: an old
/// balance without the closing one is a figure the reader cannot check. The
/// two papers say the same thing in the same words, so a customer holding
/// both does not find two spellings of what they owe.
pub(crate) struct BalanceView {
    pub(crate) title: &'static str,
    pub(crate) old_label: &'static str,
    pub(crate) old: String,
    pub(crate) this_label: &'static str,
    pub(crate) this: String,
    pub(crate) total_label: &'static str,
    pub(crate) total: String,
}

#[derive(Template)]
#[template(path = "ticket_80mm.html")]
pub(crate) struct TicketView {
    pub(crate) lang_tag: &'static str,
    pub(crate) dir: &'static str,
    pub(crate) arabic_unreviewed: bool,
    pub(crate) title: &'static str,
    pub(crate) seller: SellerView,
    pub(crate) number: String,
    pub(crate) issued_at: String,
    pub(crate) lines: Vec<LineView>,
    pub(crate) total_label: &'static str,
    pub(crate) total_amount: String,
    pub(crate) discount_label: &'static str,
    pub(crate) discount: Option<String>,
    pub(crate) tva_rows: Vec<TvaRow>,
    pub(crate) stamp_label: &'static str,
    pub(crate) stamp: Option<String>,
    pub(crate) net_to_pay_label: &'static str,
    pub(crate) net_to_pay: String,
    pub(crate) payment_mode_label: &'static str,
    pub(crate) payment_mode: &'static str,
    pub(crate) tendered_label: &'static str,
    pub(crate) tendered: Option<String>,
    pub(crate) change_label: &'static str,
    pub(crate) change: Option<String>,
    pub(crate) balance: Option<BalanceView>,
    pub(crate) currency: &'static str,
    pub(crate) thank_you: &'static str,
}

/// The 80 mm ticket for `doc`, in `lang`, as one standalone HTML page.
///
/// `show_fiscal_ids` is the shop's own choice
/// (`preferences::ticket_fiscal_ids`), read by the caller at print time the
/// same look-at-the-file `lang` already is (`print_lang_for`): off by
/// default, a ticket prints the seller's name, address and phone and stops
/// there, the four identifiers staying for the facture alone. Not baked
/// into the document at issue time, because nothing about a sale changes
/// when a shop flips this later — the same totals, the same seller — so a
/// reprint honestly reads today's choice rather than the one in force the
/// day it was rung up.
///
/// Pinned byte for byte by `fixtures/print/ticket_80mm/`, nine files: three
/// languages under the réel paid in cash, the same three under the IFU, and
/// the same three under the réel paid by card. Every one of them has the
/// four identifiers on (the historical default this flag now makes
/// optional).
pub fn render_ticket(
    doc: &Document,
    lang: Lang,
    show_fiscal_ids: bool,
) -> Result<String, RetailError> {
    // `an_ifu_ticket_names_no_tax_in_any_language` is a rule about the document, not a layout
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
        return Err(RetailError::render(
            "an IFU document carries a TVA recap and has no printable form",
        ));
    }
    view(doc, lang, show_fiscal_ids)
        .render()
        .map_err(RetailError::from)
}

pub(crate) fn view(doc: &Document, lang: Lang, show_fiscal_ids: bool) -> TicketView {
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
        seller: seller(&doc.seller, show_fiscal_ids),
        number: number(doc),
        issued_at: doc.issued_at.format(STAMP_FORMAT).to_string(),
        lines: doc.lines.iter().map(|l| line(l, reel)).collect(),
        // Under the IFU a price is one price: "HT" would name a tax the
        // document must not mention (CTCA 2026 art. 64), so the row keeps
        // the amount and changes the word.
        total_label: text(if reel { Key::TotalHt } else { Key::Total }, lang),
        total_amount: format_centimes(totals.total_ht),
        discount_label: shop_text(ShopKey::Discount, lang),
        discount: some_amount(totals.discount),
        // Empty under the IFU because the document stores no recap there,
        // not because the template hides one.
        tva_rows: totals
            .tva_by_rate
            .iter()
            .map(|row| TvaRow {
                label: text(Key::Tva, lang),
                rate: percent(row.rate),
                amount: format_centimes(row.amount),
            })
            .collect(),
        stamp_label: text(Key::Stamp, lang),
        stamp: some_amount(totals.stamp),
        net_to_pay_label: text(Key::NetToPay, lang),
        net_to_pay: format_centimes(totals.net_to_pay),
        payment_mode_label: text(Key::PaymentMode, lang),
        payment_mode: text(payment_mode_key(doc.payment_mode), lang),
        tendered_label: text(Key::Tendered, lang),
        tendered: cash.then(|| doc.tendered.map(format_centimes)).flatten(),
        change_label: text(Key::Change, lang),
        change: cash.then(|| doc.change.map(format_centimes)).flatten(),
        // Whenever the document stored one, which is whenever it names a
        // customer. Not a rule about the payment mode: a ticket paid in cash
        // by a customer who still owes for last week says so, and it says it
        // in the same three rows the facture uses.
        balance: doc.balance.map(|b| balance(b, lang)),
        currency: text(Key::Currency, lang),
        thank_you: text(Key::ThankYou, lang),
    }
}

/// The debt block, read off the document and never recomputed: a reprint
/// shows the balance the customer was handed, not a sum of today's ledger.
///
/// The closing row is named by its sign, exactly as the facture names it
/// (`facture::balance_view`): below zero is money the shop is holding for
/// the customer, and calling that a debt on the 80 mm paper while the A4
/// paper calls it a credit gives one account two names. The figure keeps the
/// sign the document stores either way; only the label moves.
fn balance(balance: BalanceTriple, lang: Lang) -> BalanceView {
    let in_credit = balance.total_debt.is_negative();
    BalanceView {
        title: text(Key::Balance, lang),
        old_label: text(Key::OldBalance, lang),
        old: format_centimes(balance.old_balance),
        this_label: shop_text(ShopKey::ThisDocument, lang),
        this: format_centimes(balance.remaining_debt),
        total_label: if in_credit {
            text(Key::TotalCredit, lang)
        } else {
            shop_text(ShopKey::TotalDebt, lang)
        },
        total: format_centimes(balance.total_debt),
    }
}

fn seller(seller: &SellerBlock, show_fiscal_ids: bool) -> SellerView {
    // `[]` rather than skipping the field, so the template keeps its one
    // `for` and the ticket answers "no identifiers on this copy" the same
    // way it already answers "no identifiers stored" (an empty shop
    // block): the row simply does not print, never a stray heading over
    // nothing.
    let ids = if show_fiscal_ids {
        [
            ("NIF", seller.nif.as_ref()),
            ("RC", seller.rc.as_ref()),
            ("NIS", seller.nis.as_ref()),
            ("AI", seller.ai.as_ref()),
        ]
        .into_iter()
        .filter_map(|(label, value)| {
            value.map(|value| SellerId {
                label,
                value: value.clone(),
            })
        })
        .collect()
    } else {
        Vec::new()
    };
    SellerView {
        name: seller.name.clone(),
        address: seller.address.clone(),
        phone: seller.phone.clone(),
        ids,
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

/// The ticket as a list of things to put on paper, from the same view the
/// HTML template renders. Moved here from the ESC/POS encoder when the
/// raster path landed: a line model with two readers has to sit above both
/// of them, or the second reader starts building its own strings.
pub(crate) fn items(view: &TicketView) -> Vec<Item> {
    let mut out = Items::new();
    out.align(Align::Center);
    out.bold(true);
    out.line(view.title);
    out.bold(false);
    out.line(&view.seller.name);
    if let Some(address) = &view.seller.address {
        out.line(address);
    }
    if let Some(phone) = &view.seller.phone {
        out.line(phone);
    }
    for id in &view.seller.ids {
        out.line(&format!("{} {}", id.label, id.value));
    }
    out.line(&view.number);
    out.line(&view.issued_at);
    out.rule();
    out.align(Align::Left);
    for line in &view.lines {
        out.line(&line.name);
        let mut qty = format!("{} × {}", line.qty, line.unit_price);
        if let Some(rate) = &line.rate {
            qty = format!("{qty}  {rate}");
        }
        out.pair(&qty, &line.total);
        if let Some(discount) = &line.discount {
            out.pair(view.discount_label, &format!("-{discount}"));
        }
    }
    out.rule();
    out.pair(view.total_label, &view.total_amount);
    if let Some(discount) = &view.discount {
        out.pair(view.discount_label, &format!("-{discount}"));
    }
    for row in &view.tva_rows {
        out.pair(&format!("{} {}", row.label, row.rate), &row.amount);
    }
    if let Some(stamp) = &view.stamp {
        out.pair(view.stamp_label, stamp);
    }
    out.bold(true);
    out.pair(view.net_to_pay_label, &view.net_to_pay);
    out.bold(false);
    out.pair(view.payment_mode_label, view.payment_mode);
    if let Some(tendered) = &view.tendered {
        out.pair(view.tendered_label, tendered);
    }
    if let Some(change) = &view.change {
        out.pair(view.change_label, change);
    }
    if let Some(balance) = &view.balance {
        out.rule();
        out.line(balance.title);
        out.pair(balance.old_label, &balance.old);
        out.pair(balance.this_label, &balance.this);
        out.pair(balance.total_label, &balance.total);
    }
    out.align(Align::Center);
    out.feed(1);
    out.line(view.thank_you);
    out.line(view.currency);
    out.feed(2);
    out.cut();
    out.into_items()
}

/// The padding rules, in one place. A label and an amount on the same row
/// are spaced to the column budget here and nowhere else, so the raster
/// draws the spacing the text path counted rather than a second guess at it.
///
/// Shared with `print::facture_roll`, which builds the same kind of list
/// for the 80 mm facture. The two papers say different things and both owe
/// the head lines of at most `WIDTH` columns, so the padding and the
/// wrapping belong to one builder rather than to each paper.
pub(crate) struct Items(Vec<Item>);

impl Items {
    pub(crate) const fn new() -> Self {
        Self(Vec::new())
    }

    pub(crate) fn into_items(self) -> Vec<Item> {
        self.0
    }

    pub(crate) fn align(&mut self, align: Align) {
        self.0.push(Item::Align(align));
    }

    pub(crate) fn bold(&mut self, on: bool) {
        self.0.push(Item::Bold(on));
    }

    pub(crate) fn feed(&mut self, lines: u8) {
        self.0.push(Item::Feed(lines));
    }

    pub(crate) fn cut(&mut self) {
        self.0.push(Item::Cut);
    }

    pub(crate) fn line(&mut self, s: &str) {
        self.0.push(Item::Line(s.to_owned()));
    }

    pub(crate) fn rule(&mut self) {
        self.line(&"-".repeat(WIDTH));
    }

    pub(crate) fn pair(&mut self, label: &str, amount: &str) {
        let gap = WIDTH
            .saturating_sub(label.chars().count())
            .saturating_sub(amount.chars().count());
        let pad = if gap == 0 { 1 } else { gap };
        let mut line = String::new();
        line.push_str(label);
        for _ in 0..pad {
            line.push(' ');
        }
        line.push_str(amount);
        self.line(&line);
    }

    /// A label and an amount on one row where they fit the budget, and the
    /// label wrapped above a right-aligned amount where they do not.
    ///
    /// The ticket never needs this: its labels are single words chosen to
    /// sit beside a figure. A facture's are sentences the law names
    /// ("Montant de l'avoir", "TVA 19 % base 12 345,67"), and `pair` would
    /// run them past the edge of the roll, where `raster::draw` refuses the
    /// whole document rather than trim a total. Pushing the amount onto its
    /// own row keeps the figure whole, which is the part of the row a
    /// reader cannot reconstruct.
    pub(crate) fn row(&mut self, label: &str, amount: &str) {
        let amount_cols = amount.chars().count();
        if label.chars().count().saturating_add(amount_cols) < WIDTH {
            self.pair(label, amount);
            return;
        }
        self.wrapped(label);
        self.pair("", amount);
    }

    /// A sentence broken across as many rows as it needs, on the spaces
    /// between its words.
    ///
    /// Only on U+0020. The narrow no-break space U+202F groups the
    /// thousands of every amount on the paper (features.md §4), and
    /// breaking a line there would leave "12" at the end of one row and
    /// "345,67" at the start of the next: two numbers where the document
    /// stores one. A single word longer than the budget — a product
    /// reference, an email — is cut by characters, because the alternative
    /// is a line the head refuses and a facture that does not print at all.
    pub(crate) fn wrapped(&mut self, text: &str) {
        let mut row = String::new();
        let mut cols = 0usize;
        for word in text.split(' ').filter(|w| !w.is_empty()) {
            let word_cols = word.chars().count();
            if cols > 0 && cols.saturating_add(1).saturating_add(word_cols) > WIDTH {
                self.line(&row);
                row = String::new();
                cols = 0;
            }
            if word_cols > WIDTH {
                if cols > 0 {
                    self.line(&row);
                    row = String::new();
                    cols = 0;
                }
                for chunk in chunks(word) {
                    self.line(&chunk);
                }
                continue;
            }
            if cols > 0 {
                row.push(' ');
                cols = cols.saturating_add(1);
            }
            row.push_str(word);
            cols = cols.saturating_add(word_cols);
        }
        if cols > 0 {
            self.line(&row);
        }
    }
}

/// One unbreakable word, cut into rows of at most `WIDTH` characters.
fn chunks(word: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut row = String::new();
    let mut cols = 0usize;
    for ch in word.chars() {
        row.push(ch);
        cols = cols.saturating_add(1);
        if cols == WIDTH {
            out.push(std::mem::take(&mut row));
            cols = 0;
        }
    }
    if cols > 0 {
        out.push(row);
    }
    out
}

#[cfg(test)]
#[path = "../../tests/unit/print_ticket.rs"]
mod tests;
