//! Invariants of the money module, not formulas. proptest shrinks any
//! counterexample to a minimal one; freeze it as a fixture case when found.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use dzpos_core::money::{Bps, Money, MoneyError};
use proptest::prelude::*;

// Amounts a shop can see: up to a billion dinars in centimes, both signs.
const RANGE: std::ops::RangeInclusive<i64> = -100_000_000_000..=100_000_000_000;

proptest! {
    #[test]
    fn pct_zero_is_zero(a in RANGE) {
        prop_assert_eq!(Money::centimes(a).pct(Bps::new(0)).unwrap(), Money::ZERO);
    }

    #[test]
    fn pct_full_is_identity(a in RANGE) {
        prop_assert_eq!(Money::centimes(a).pct(Bps::new(10_000)).unwrap(), Money::centimes(a));
    }

    #[test]
    fn pct_is_within_half_a_centime(a in RANGE, r in 0u32..=10_000) {
        let got = Money::centimes(a).pct(Bps::new(r)).unwrap().as_centimes() as i128;
        let exact_times_10k = a as i128 * r as i128;
        // |got − exact| ≤ 0.5 centime  ⇔  |got·10 000 − a·r| ≤ 5 000
        let diff = (got * 10_000 - exact_times_10k).abs();
        prop_assert!(diff <= 5_000, "a={a} r={r} got={got} diff={diff}");
    }

    #[test]
    fn pct_is_odd_in_the_amount(a in RANGE, r in 0u32..=10_000) {
        let pos = Money::centimes(a).pct(Bps::new(r)).unwrap();
        let neg = Money::centimes(-a).pct(Bps::new(r)).unwrap();
        prop_assert_eq!(pos.as_centimes(), -neg.as_centimes());
    }

    #[test]
    fn pct_never_panics_on_any_i64(a in any::<i64>(), r in 0u32..=10_000) {
        let _ = Money::centimes(a).pct(Bps::new(r));
    }

    #[test]
    fn add_is_commutative_and_overflow_is_an_error(a in any::<i64>(), b in any::<i64>()) {
        let ab = Money::centimes(a).checked_add(Money::centimes(b));
        let ba = Money::centimes(b).checked_add(Money::centimes(a));
        prop_assert_eq!(ab, ba);
        match a.checked_add(b) {
            Some(s) => prop_assert_eq!(ab, Ok(Money::centimes(s))),
            None => prop_assert_eq!(ab, Err(MoneyError::Overflow)),
        }
    }

    #[test]
    fn sub_then_add_round_trips(a in RANGE, b in RANGE) {
        let d = Money::centimes(a).checked_sub(Money::centimes(b)).unwrap();
        prop_assert_eq!(d.checked_add(Money::centimes(b)).unwrap(), Money::centimes(a));
    }

    #[test]
    fn times_qty_matches_repeated_add(unit in -1_000_000i64..=1_000_000, qty in 0i64..=50) {
        let product = Money::centimes(unit).checked_mul(qty).unwrap();
        let mut sum = Money::ZERO;
        for _ in 0..qty {
            sum = sum.checked_add(Money::centimes(unit)).unwrap();
        }
        prop_assert_eq!(product, sum);
    }
}
