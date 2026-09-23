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
