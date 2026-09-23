//! Printed documents. The core renders them, not the UI, so the desktop
//! and a server with no screen print the same bytes (features.md §4).
//!
//! Each template is one file under `crates/retail/templates/`, compiled into
//! the binary by askama, and each template × language is pinned by a golden
//! file under `fixtures/print/`. A template change is a reviewed golden
//! diff, never a green run nobody read.
//!
//! What every template shares lives here: the printed number, a rate as a
//! percentage, and the rule that a row worth nothing is not printed. The
//! ticket and the facture are two papers, but a document number reads the
//! same on both and a customer comparing them must not find two spellings.

pub mod barcode_label;
mod bidi;
pub mod debt_slip;
pub mod escpos;
pub mod facture;
pub mod facture_roll;
pub(crate) mod facture_view;
mod png;
pub mod raster;
mod refusals;
pub mod statement;
pub mod strings;
pub mod ticket;

// `layout` and `thermal` are in the kernel: `services::preferences` (a
// domain-free service) stores a shop's chosen `FactureLayout` and
// `ThermalMode`, unlike `mod.rs`, `escpos.rs` and `raster.rs`. Both moved to
// `dzpos_kernel::print` in the kernel crate split (S3 of
// `a-kernel-crate-and-retail-as-the-first-module`) and are re-exported here
// as modules so `crate::print::layout::Paper` keeps resolving unchanged.
// `strings` is this crate's own module now, not a re-export: S4 of the same
// plan moved the fourteen shop words `Key` used to carry out of the
// kernel's copy into `strings::ShopKey` here, and `strings::{text, Key}`
// re-export the kernel's own so `crate::print::strings::{text, Key}` keeps
// resolving unchanged for every call site that only ever needed those.
pub use dzpos_kernel::print::{layout, thermal};

pub use barcode_label::{render_label, render_label_sheet};
pub use debt_slip::render_debt_slip;
pub use dzpos_kernel::print::{FactureLayout, Page, Paper, ThermalMode};
pub use escpos::{
    draw_ticket_raster, dump_ticket_escpos, dump_ticket_escpos_png, render_ticket_escpos,
    render_ticket_escpos_in, render_ticket_escpos_raster, send_ticket_escpos_tcp,
    write_ticket_escpos_to_file,
};
pub use facture::{
    render_facture, render_facture_with, render_facture_with_reference, Cancellation, FactureInput,
};
pub use facture_roll::{
    draw_facture_raster, facture_roll_lines, render_facture_escpos, render_facture_escpos_raster,
    render_facture_escpos_text,
};
pub use raster::HEAD_WIDTH_DOTS;
pub use statement::render_statement;
pub use ticket::render_ticket;

use crate::models::document::Document;
use crate::print::strings::Key;
use dzpos_kernel::money::format::format_centimes;
use dzpos_kernel::money::{Bps, Money, PaymentMode};

/// `TK-2026-000123`: the kind's short prefix, the year the series counts in,
/// and the number inside that year padded to six digits. The stored `series`
/// is the counter's name (`doc_ticket:2026`), which is a column and not
/// something a customer quotes; the prefix is the printed form of the same
/// series and lives beside it on `DocumentKind` so the two cannot drift.
const NUMBER_DIGITS: usize = 6;

/// The number a customer quotes, `{prefix}-{year}-{number:06}`
/// (features.md §4). Public because the wire carries it too: a screen that
/// says "Facture FA-2026-000001" must read the same spelling the paper
/// prints, not a second one built out of the kind, the year and the integer.
pub fn number(doc: &Document) -> String {
    number_of(doc.kind, doc.series_year, doc.number)
}

/// The same number, for a caller that has the kind, the year and the number
/// without the document: a statement names the document a movement cites and
/// reads three columns of it, never the whole row.
///
/// The year is the document's own and never the one being printed in, so an
/// avoir written this year against last year's facture prints that facture's
/// year: `FA-2025-000042` is the paper the customer is holding.
pub(crate) fn number_of(
    kind: crate::models::document::DocumentKind,
    year: i32,
    number: i64,
) -> String {
    format!(
        "{}-{year}-{:0width$}",
        kind.number_prefix(),
        number,
        width = NUMBER_DIGITS
    )
}

/// A row that is only there when there is something on it. Zero is not a
/// discount and not a stamp; no document says anything about either.
pub(crate) fn some_amount(amount: Money) -> Option<String> {
    (amount != Money::ZERO).then(|| format_centimes(amount))
}

/// A rate in basis points as a person reads it: 1900 is `19 %`, 950 is
/// `9,5 %`. The space before the sign is a narrow no-break one, the same
/// character the thousands separator uses, so a rate never breaks across
/// two lines of a 72 mm column.
pub(crate) fn percent(rate: Bps) -> String {
    let bps = rate.as_u32();
    let whole = bps / 100;
    let rest = bps % 100;
    if rest == 0 {
        return format!("{whole}\u{202f}%");
    }
    let decimals = format!("{rest:02}");
    format!("{whole},{}\u{202f}%", decimals.trim_end_matches('0'))
}

/// The word for how the document was paid. Same three modes on both
/// papers: the ticket and the facture name a card sale the same way.
pub(crate) const fn payment_mode_key(mode: PaymentMode) -> Key {
    match mode {
        PaymentMode::Cash => Key::Cash,
        PaymentMode::Card => Key::Card,
        PaymentMode::Credit => Key::Credit,
    }
}

#[cfg(test)]
#[path = "../../tests/unit/print_mod.rs"]
mod tests;
