use super::scaled;

#[test]
fn a_decimal_reaches_centimes_without_a_float_multiply() {
    assert_eq!(scaled("80.5", 2), Ok(8_050));
    assert_eq!(scaled("120", 2), Ok(12_000));
    assert_eq!(scaled("1 234,56", 2), Ok(123_456));
    assert_eq!(scaled("0.5", 2), Ok(50));
    assert_eq!(scaled("-1.00", 2), Ok(-100));
    assert_eq!(scaled("12.", 2), Ok(1_200));
}

#[test]
fn a_third_decimal_that_carries_value_is_refused_and_never_rounded() {
    // Ruling, 2026-09-10: a price the shop typed as 0.125 and the till
    // then charged as 0.13 is a centime nobody agreed to.
    assert_eq!(scaled("0.125", 2), Err("too_many_decimals"));
    assert_eq!(scaled("0.124", 2), Err("too_many_decimals"));
    assert_eq!(scaled("-0.125", 2), Err("too_many_decimals"));
    assert_eq!(scaled("80.505", 2), Err("too_many_decimals"));

    // Zeros past the scale are the column's format and not a decimal
    // the shop typed: 120.000 in a three-place column is the 120
    // somebody entered, and refusing it would refuse a template filled
    // in without a number being changed.
    assert_eq!(scaled("120.000", 2), Ok(12_000));
    assert_eq!(scaled("80.500", 2), Ok(8_050));
}

#[test]
fn a_quantity_is_thousandths() {
    assert_eq!(scaled("1.5", 3), Ok(1_500));
    assert_eq!(scaled("12", 3), Ok(12_000));
    // A fourth decimal on a quantity is the same refusal one place out.
    assert_eq!(scaled("0.0005", 3), Err("too_many_decimals"));
    assert_eq!(scaled("0.125", 3), Ok(125));
}

#[test]
fn anything_that_is_not_a_number_is_no_number_at_all() {
    assert_eq!(scaled("abc", 2), Err("not_a_number"));
    assert_eq!(scaled("", 2), Err("not_a_number"));
    assert_eq!(scaled("1.2.3", 2), Err("not_a_number"));
    assert_eq!(scaled("12x", 2), Err("not_a_number"));
}
