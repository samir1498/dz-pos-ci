//! An amount and a quantity as a person reads them.
//!
//! This is the paper half of `packages/shared/src/money.ts`: the same two
//! functions, the same output, pinned to the same file
//! (`fixtures/money/format_centimes.json`) by
//! `crates/core/tests/money_fixtures.rs` and by
//! `packages/shared/src/money.test.ts`. A ticket the core prints and the
//! total on the screen the customer just watched are the same figure, so
//! the two formatters are one rule with two implementations, never two
//! rules.
//!
//! Display only: nothing here is read back into a total, and the module
//! inherits `#![deny(clippy::arithmetic_side_effects)]` from `money`, so
//! the digits come out by division and remainder against literals.

use super::{Money, MILLI_PER_UNIT};

/// Centimes in one dinar.
const CENTIMES_PER_DINAR: u64 = 100;

/// `MILLI_PER_UNIT` as the unsigned type the digits are split with. Written
/// as its own constant so the divisor below is one clippy can see is not
/// zero; the assertion is what keeps it the same thousand.
const THOUSANDTHS_PER_UNIT: u64 = 1_000;
const _: () = assert!(THOUSANDTHS_PER_UNIT == MILLI_PER_UNIT.unsigned_abs());

/// Thousands are grouped with a narrow no-break space, French and Algerian
/// typography, the same character `formatCentimes` uses. An ordinary space
/// would let a line break fall inside a number.
const GROUP_SEPARATOR: char = '\u{202f}';

/// `1234` becomes `12,34`; `100000` becomes `1 000,00`. No currency word:
/// the template decides whether a row carries one, because a ticket names
/// the dinar once on its totals and not on every line.
pub fn format_centimes(amount: Money) -> String {
    let magnitude = amount.as_centimes().unsigned_abs();
    let dinars = magnitude / CENTIMES_PER_DINAR;
    let rest = magnitude % CENTIMES_PER_DINAR;
    let sign = if amount.is_negative() { "-" } else { "" };
    format!("{sign}{},{rest:02}", group(dinars))
}

/// Thousandths of a unit as a person writes them: `1500` is `1,5`, `2000`
/// is `2`, `1` is `0,001`. Never grouped, because the number is a count of
/// kilos or pieces and `1 000` there reads as a price.
pub fn format_qty(milli: i64) -> String {
    let magnitude = milli.unsigned_abs();
    let whole = magnitude / THOUSANDTHS_PER_UNIT;
    let rest = magnitude % THOUSANDTHS_PER_UNIT;
    let sign = if milli < 0 { "-" } else { "" };
    if rest == 0 {
        return format!("{sign}{whole}");
    }
    let decimals = format!("{rest:03}");
    format!("{sign}{whole},{}", decimals.trim_end_matches('0'))
}

/// The digits of a whole number in groups of three, counted from the right.
/// Counted with an index over the reversed digits rather than by slicing:
/// a slice offset would be arithmetic on a length, and this module denies
/// unchecked arithmetic.
fn group(n: u64) -> String {
    let digits = n.to_string();
    let mut reversed = String::with_capacity(digits.len());
    for (position, digit) in digits.chars().rev().enumerate() {
        if position != 0 && position % 3 == 0 {
            reversed.push(GROUP_SEPARATOR);
        }
        reversed.push(digit);
    }
    reversed.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::{format_centimes, format_qty, Money};

    /// The fixture covers the cases both runners share. These two are the
    /// edges no fixture can carry: the extremes of the type itself.
    #[test]
    fn the_smallest_amount_the_type_holds_still_formats() {
        // i64::MIN has no positive counterpart, so a naive `abs` panics
        // here and `unsigned_abs` is the reason this is a test.
        let printed = format_centimes(Money::centimes(i64::MIN));
        assert!(printed.starts_with("-92"), "{printed}");
        assert!(printed.ends_with(",08"), "{printed}");
        assert_eq!(format_qty(i64::MIN), "-9223372036854775,808");
    }

    #[test]
    fn a_group_separator_is_the_narrow_no_break_space_and_not_a_space() {
        let printed = format_centimes(Money::centimes(100_000));
        assert_eq!(printed, "1\u{202f}000,00");
        assert!(!printed.contains(' '), "an ordinary space would wrap");
    }
}
