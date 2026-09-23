use super::{amount, quantity, rate_percent};
use crate::money::Money;

#[test]
fn an_amount_reaches_a_cell_as_the_double_a_person_would_have_typed() {
    assert_eq!(amount(Money::centimes(123_456)).unwrap(), 1234.56_f64);
    assert_eq!(amount(Money::ZERO).unwrap(), 0.0_f64);
    assert_eq!(amount(Money::centimes(5)).unwrap(), 0.05_f64);
    // The sign survives a whole-dinar part of zero, which is where a
    // naive `centimes / 100` loses it.
    assert_eq!(amount(Money::centimes(-5)).unwrap(), -0.05_f64);
    assert_eq!(amount(Money::centimes(-123_456)).unwrap(), -1234.56_f64);
}

#[test]
fn a_quantity_reaches_a_cell_in_thousandths() {
    assert_eq!(quantity(2_500).unwrap(), 2.5_f64);
    assert_eq!(quantity(1).unwrap(), 0.001_f64);
    assert_eq!(quantity(-1_500).unwrap(), -1.5_f64);
}

#[test]
fn a_rate_reaches_a_cell_as_the_percentage_a_person_reads() {
    assert_eq!(rate_percent(1900).unwrap(), 19.0_f64);
    assert_eq!(rate_percent(0).unwrap(), 0.0_f64);
    assert_eq!(rate_percent(950).unwrap(), 9.5_f64);
}
