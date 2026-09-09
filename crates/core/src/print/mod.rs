//! Printed documents. The core renders them, not the UI, so the desktop
//! and a server with no screen print the same bytes (features.md §4).
//!
//! Each template is one file under `crates/core/templates/`, compiled into
//! the binary by askama, and each template × language is pinned by a golden
//! file under `fixtures/print/`. A template change is a reviewed golden
//! diff, never a green run nobody read.
//!
//! What every template shares lives here: the printed number, a rate as a
//! percentage, and the rule that a row worth nothing is not printed. The
//! ticket and the facture are two papers, but a document number reads the
//! same on both and a customer comparing them must not find two spellings.

pub mod facture;
pub mod strings;
pub mod ticket;

pub use facture::{render_facture, render_facture_with_reference, Paper};
pub use ticket::render_ticket;

use crate::models::document::Document;
use crate::money::format::format_centimes;
use crate::money::{Bps, Money, PaymentMode};
use crate::print::strings::Key;

/// `TK-000123`: the kind's short prefix, a hyphen, and the number in the
/// series padded to six digits. The stored `series` is the counter's name
/// (`doc_ticket`), which is a column and not something a customer quotes;
/// the prefix is the printed form of the same series and lives beside it on
/// `DocumentKind` so the two cannot drift.
const NUMBER_DIGITS: usize = 6;

/// The number a customer quotes, `{prefix}-{number:06}` (features.md §4).
/// Public because the wire carries it too: a screen that says "Facture
/// FA-000001" must read the same spelling the paper prints, not a second
/// one built out of the kind and the integer.
pub fn number(doc: &Document) -> String {
    format!(
        "{}-{:0width$}",
        doc.kind.number_prefix(),
        doc.number,
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
