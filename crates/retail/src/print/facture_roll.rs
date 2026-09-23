//! The 80 mm facture as a list of lines, for a thermal head.
//!
//! `facture_roll_80mm.html` already prints this document on a roll through a
//! driver. This is the same document down the other wire: the one a cheap
//! head eats directly, text for French and English and dots for Arabic
//! (`context/plans/20260921-arabic-on-a-cheap-thermal-head.md`, T4).
//!
//! **Nothing here decides what the facture says, and nothing here computes
//! an amount.** The fields arrive already made from `facture_view::view`,
//! the same view all four HTML layouts are drawn from, so every figure on
//! this paper is the document's own stored total formatted once, far above
//! this file. What this file owns is where a field sits on 42 columns and
//! where a sentence breaks. If an amount were ever formatted here, the roll
//! and the A4 sheet could disagree about a total, which is what
//! `the_escpos_roll_prints_the_amounts_the_document_stores` is there to
//! catch.
//!
//! It owes every field décret 05-468 art. 3 asks of a facture, the same
//! list the A4 page carries: both parties with their identifiers, the
//! number and the day, every line with its quantity, unit price, rate and
//! total, the totals table, the amount in words, and the mentions that
//! belong to an avoir, a proforma or a cancelled reprint. A roll that won
//! its width by dropping the buyer's NIF would be a ticket with a facture's
//! title on it.
//!
//! Where 72 mm cannot take a row the sheet takes, the row becomes two: the
//! party blocks stack, a line's figures sit under its designation, and a
//! label too long to share a row with its amount is wrapped above it. What
//! never happens is a figure being shortened.

use crate::error::RetailError;
use crate::lang::Lang;
use crate::models::document::Document;
use crate::print::escpos;
use crate::print::facture::{built_view, FactureInput, FactureView};
use crate::print::layout::Paper;
use crate::print::raster;
use crate::print::thermal::ThermalMode;
use crate::print::ticket::{Align, Item, Items};

/// ESC/POS bytes for the 80 mm facture, down whichever path the shop's head
/// is on. Arabic is drawn whatever the shop chose, for the reason it is on
/// the ticket: no single-byte table a cheap head carries has Arabic in it.
///
/// Refuses exactly what the A4 page refuses, because it goes through the
/// same `built_view`: an IFU document carrying a TVA recap, a document with
/// no buyer block, an avoir carrying a droit de timbre, a proforma carrying
/// a debt, a cancellation printed over a live document, and a reference on
/// anything but an avoir.
pub fn render_facture_escpos(
    doc: &Document,
    input: &FactureInput<'_>,
    lang: Lang,
    mode: ThermalMode,
) -> Result<Vec<u8>, RetailError> {
    match mode.for_lang(lang) {
        ThermalMode::Text => render_facture_escpos_text(doc, input, lang),
        ThermalMode::Raster => {
            render_facture_escpos_raster(doc, input, lang, raster::HEAD_WIDTH_DOTS)
        }
    }
}

/// The facture as text bytes, whatever the language.
///
/// The twin of `render_ticket_escpos`, and it exists for the same reason:
/// the `.txt` goldens have to carry the words, in all three languages, or
/// there is nothing for `the_facture_raster_draws_the_lines_the_text_path_prints`
/// to compare the drawing against. That test is the whole guarantee that a
/// total reads the same on a French facture and on the Arabic one beside
/// it, and it cannot be written against a picture.
///
/// No shop is ever routed here in Arabic: `render_facture_escpos` above
/// sends `ar` down the raster path under either preference, because these
/// bytes on a cheap head are a box per character. A caller that names this
/// function is asking for the encoding and not for a paper.
pub fn render_facture_escpos_text(
    doc: &Document,
    input: &FactureInput<'_>,
    lang: Lang,
) -> Result<Vec<u8>, RetailError> {
    Ok(escpos::encode(&items(&view_of(doc, input, lang)?)))
}

/// The same facture as dots, at a named head width: 576 on the common 80 mm
/// head, 384 on a 58 mm one.
pub fn render_facture_escpos_raster(
    doc: &Document,
    input: &FactureInput<'_>,
    lang: Lang,
    width_dots: u32,
) -> Result<Vec<u8>, RetailError> {
    escpos::encode_raster(&draw_facture_raster(doc, input, lang, width_dots)?.bitmap)
}

/// The bitmap on its own, beside the lines it was drawn from. What the
/// tests read: the rule they keep is about the strings and the dots those
/// strings took, not about the command that carries them.
pub fn draw_facture_raster(
    doc: &Document,
    input: &FactureInput<'_>,
    lang: Lang,
    width_dots: u32,
) -> Result<raster::Drawn, RetailError> {
    raster::draw(&items(&view_of(doc, input, lang)?), lang, width_dots)
}

/// The facture's lines, for a caller that wants the line model and not the
/// bytes: the amount read-back test parses these before anything is drawn,
/// which is the point at which a wrong figure is still a string.
pub fn facture_roll_lines(
    doc: &Document,
    input: &FactureInput<'_>,
    lang: Lang,
) -> Result<Vec<String>, RetailError> {
    Ok(items(&view_of(doc, input, lang)?)
        .into_iter()
        .filter_map(|item| match item {
            Item::Line(text) => Some(text),
            Item::Align(_) | Item::Bold(_) | Item::Feed(_) | Item::Cut => None,
        })
        .collect())
}

/// The view every layout draws, built for the roll.
///
/// `Paper::Roll80` is named here and not taken from the caller: this is a
/// thermal head and there is no tray to reach into. The field it decides is
/// the `@page` size line, which no ESC/POS byte carries, so it changes
/// nothing on this wire — it is passed for the same reason the HTML roll
/// passes it, that one view has one answer for every field.
fn view_of(
    doc: &Document,
    input: &FactureInput<'_>,
    lang: Lang,
) -> Result<FactureView, RetailError> {
    built_view(doc, input, lang, Paper::Roll80)
}

/// The facture as a list of things to put on paper, in the order it says
/// them.
///
/// The order follows the A4 page: heading, the document's own identity,
/// the two parties, the lines, the totals, the words line, the ledger, how
/// it was paid, and the signatures. A reader holding both papers reads them
/// in the same order.
fn items(view: &FactureView) -> Vec<Item> {
    let mut out = Items::new();
    heading(&mut out, view);
    parties(&mut out, view);
    lines(&mut out, view);
    totals(&mut out, view);
    closing(&mut out, view);
    out.align(Align::Center);
    out.feed(1);
    out.line(view.currency);
    out.feed(2);
    out.cut();
    out.into_items()
}

/// The title, the mark of a cancelled reprint, the number, the day, and the
/// two sentences only one face of this document prints: the facture an
/// avoir corrects, and the notice that a proforma settles nothing.
fn heading(out: &mut Items, view: &FactureView) {
    out.align(Align::Center);
    out.bold(true);
    out.wrapped(view.title);
    // The A4 page draws this across the sheet on the diagonal, which a
    // thermal head cannot do. It becomes a line of its own, above the
    // number rather than over it: a word printed over the amounts on paper
    // this coarse would take the amounts with it.
    if let Some(mark) = view.cancelled_mark {
        out.wrapped(mark);
    }
    out.bold(false);
    out.align(Align::Left);
    out.row(&view.number, &view.issued_at);
    if let Some(cancellation) = &view.cancellation {
        out.wrapped(&format!("{} {}", cancellation.on_label, cancellation.at));
        out.wrapped(&format!(
            "{} {}",
            cancellation.reason_label, cancellation.reason
        ));
    }
    if let Some(reference) = &view.reference {
        out.wrapped(&format!(
            "{} {} {} {}",
            reference.label, reference.number, reference.on_label, reference.issued_at
        ));
    }
    if let Some(notice) = view.notice {
        out.wrapped(notice);
    }
    out.rule();
}

/// Both parties, stacked rather than side by side: 72 mm has no room for
/// two columns, and the identifiers are what make this a facture rather
/// than a receipt (décret 05-468 art. 3). A consumer buyer carries no
/// identifier rows at all, which `facture_view::buyer_view` decided long
/// before this file sees it.
fn parties(out: &mut Items, view: &FactureView) {
    out.align(Align::Left);
    for party in [&view.seller, &view.buyer] {
        out.bold(true);
        out.wrapped(party.title);
        out.bold(false);
        out.wrapped(&party.name);
        if let Some(address) = &party.address {
            out.wrapped(address);
        }
        if let Some(phone) = &party.phone {
            out.wrapped(phone);
        }
        for id in &party.ids {
            out.wrapped(&format!("{} {}", id.label, id.value));
        }
    }
    out.rule();
}

/// The lines. The sheet's table is six columns; the roll gives each line
/// its designation on one row and its figures on the next, the same shape
/// the HTML roll takes, so nothing is dropped to win the width.
fn lines(out: &mut Items, view: &FactureView) {
    for line in &view.lines {
        out.wrapped(&line.name);
        // The same detail row the HTML roll prints, in the same order:
        // quantity, unit price, and the rate where the régime has one. The
        // six column headings of the A4 table are not carried over, for
        // the reason that page's own roll template leaves them out — a
        // heading above a single line is a row spent saying what the
        // figures already show.
        let mut figures = format!("{} × {}", line.qty, line.unit_price);
        if let Some(rate) = &line.rate {
            figures = format!("{figures}  {rate}");
        }
        out.row(&figures, &line.total);
        if let Some(discount) = &line.discount {
            out.row(view.line_discount_label, &format!("-{discount}"));
        }
    }
    out.rule();
}

/// The totals table, row for row with the sheet's: the total before tax,
/// the global discount and the subtotal it leaves, the TVA recap with each
/// group's rate and base, the total TTC, the droit de timbre, and the row
/// the document closes on.
///
/// A row the document does not carry is not printed as a zero. Which rows
/// exist is decided in `facture_view::view` — the IFU carries no recap and
/// no TTC row, an avoir carries no stamp — and this file only lays out what
/// it is handed.
fn totals(out: &mut Items, view: &FactureView) {
    out.row(view.total_label, &view.total_amount);
    if let Some(discount) = &view.discount {
        out.row(view.discount_label, &format!("-{discount}"));
    }
    if let Some(subtotal) = &view.subtotal {
        out.row(view.subtotal_label, subtotal);
    }
    for row in &view.tva_rows {
        // The rate and the base share the label's row, because a recap row
        // read without its base cannot be checked against the line it came
        // from.
        out.row(
            &format!("{} {} {} {}", row.label, row.rate, row.base_label, row.base),
            &row.amount,
        );
    }
    if let Some(ttc) = &view.total_ttc {
        out.row(view.total_ttc_label, ttc);
    }
    if let Some(stamp) = &view.stamp {
        out.row(view.stamp_label, stamp);
    }
    out.bold(true);
    out.row(view.net_to_pay_label, &view.net_to_pay);
    out.bold(false);
    out.rule();
}

/// The amount in words, the ledger triple, how the document was paid, and
/// the two signature lines.
///
/// The words line is the one décret 05-468 art. 3 asks for in letters
/// beside the figure, and it is long in all three languages, so it is
/// wrapped rather than shortened: a facture whose written total stops at
/// the edge of the roll states half a sum.
fn closing(out: &mut Items, view: &FactureView) {
    out.wrapped(&format!("{} {}", view.in_words_label, view.in_words));
    if let Some(balance) = &view.balance {
        out.rule();
        out.wrapped(balance.title);
        out.row(balance.old_label, &balance.old);
        out.row(balance.this_label, &balance.this);
        out.row(balance.total_label, &balance.total);
    }
    if let Some(payment) = &view.payment_mode {
        out.row(payment.title, payment.mode);
    }
    // The sheet frames these; a roll names them and leaves the space. Every
    // face prints them, including the proforma, the same call the HTML roll
    // makes: a quote is signed too. `show_blocks` decides the two blocks
    // above and not these.
    out.rule();
    out.wrapped(&format!("{} · {}", view.seller_label, view.cachet_label));
    out.feed(2);
    out.wrapped(&format!("{} · {}", view.buyer_label, view.cachet_label));
}
