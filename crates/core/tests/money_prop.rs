//! Invariants of the money module, not formulas. proptest shrinks any
//! counterexample to a minimal one; freeze it as a fixture case when found.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use dzpos_core::money::{
    compute_totals, stamp, Bps, Line, Money, MoneyError, PaymentMode, Regime, TotalsOptions,
};
use proptest::prelude::*;

// Amounts a shop can see: up to a billion dinars in centimes, both signs.
const RANGE: std::ops::RangeInclusive<i64> = -100_000_000_000..=100_000_000_000;

proptest! {
    #[test]
    fn pct_zero_is_zero(a in RANGE) {
        prop_assert_eq!(Money::centimes(a).pct(Bps::new(0).unwrap()).unwrap(), Money::ZERO);
    }

    #[test]
    fn pct_full_is_identity(a in RANGE) {
        prop_assert_eq!(Money::centimes(a).pct(Bps::new(10_000).unwrap()).unwrap(), Money::centimes(a));
    }

    #[test]
    fn pct_is_within_half_a_centime(a in RANGE, r in 0u32..=10_000) {
        let got = Money::centimes(a).pct(Bps::new(r).unwrap()).unwrap().as_centimes() as i128;
        let exact_times_10k = a as i128 * r as i128;
        // |got − exact| ≤ 0.5 centime  ⇔  |got·10 000 − a·r| ≤ 5 000
        let diff = (got * 10_000 - exact_times_10k).abs();
        prop_assert!(diff <= 5_000, "a={a} r={r} got={got} diff={diff}");
    }

    #[test]
    fn pct_is_odd_in_the_amount(a in RANGE, r in 0u32..=10_000) {
        let pos = Money::centimes(a).pct(Bps::new(r).unwrap()).unwrap();
        let neg = Money::centimes(-a).pct(Bps::new(r).unwrap()).unwrap();
        prop_assert_eq!(pos.as_centimes(), -neg.as_centimes());
    }

    #[test]
    fn exact_half_rounds_away_from_zero(k in -50_000_000_000i64..=50_000_000_000) {
        // odd amount × 50 % always ends in ,005: the tie goes away from zero
        let a = k.checked_mul(2).unwrap().checked_add(1).unwrap();
        let got = Money::centimes(a).pct(Bps::new(5_000).unwrap()).unwrap().as_centimes();
        let away = if a > 0 { (a + 1) / 2 } else { (a - 1) / 2 };
        prop_assert_eq!(got, away, "a={}", a);
    }

    #[test]
    fn pct_overflow_is_reported_not_wrapped(a in any::<i64>(), r in 1u32..=10_000) {
        let got = Money::centimes(a).pct(Bps::new(r).unwrap());
        let raw = a as i128 * r as i128;
        let fits = raw.abs() + 5_000 <= i64::MAX as i128;
        prop_assert_eq!(got.is_ok(), fits, "a={} r={} got={:?}", a, r, got);
    }

    #[test]
    fn rate_above_whole_is_an_error(r in 10_001u32..=u32::MAX) {
        prop_assert_eq!(Bps::new(r), Err(MoneyError::RateOutOfRange));
    }

    #[test]
    fn pct_never_panics_on_any_i64(a in any::<i64>(), r in 0u32..=10_000) {
        let _ = Money::centimes(a).pct(Bps::new(r).unwrap());
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

// ---- totals and the stamp ----

/// A basket a shop can ring up: unit prices to a million dinars, small
/// counts, a line discount never above its own line, rates from the real
/// table plus 100 % to stress the arithmetic.
fn basket() -> impl Strategy<Value = Vec<Line>> {
    let line = (
        0i64..=20,
        0i64..=100_000_000,
        prop::sample::select(vec![0u32, 900, 1900, 10_000]),
    )
        .prop_flat_map(|(qty, unit, rate)| {
            let gross = unit.saturating_mul(qty);
            (0i64..=gross).prop_map(move |line_discount| Line {
                qty,
                unit_price: Money::centimes(unit),
                line_discount: Money::centimes(line_discount),
                rate: Bps::new(rate).unwrap(),
            })
        });
    prop::collection::vec(line, 0..6)
}

fn opts(
    global_discount: Money,
    payment_mode: PaymentMode,
    stamp_enabled: bool,
    regime: Regime,
) -> TotalsOptions {
    TotalsOptions {
        global_discount,
        payment_mode,
        stamp_enabled,
        regime,
    }
}

fn any_mode() -> impl Strategy<Value = PaymentMode> {
    prop::sample::select(vec![
        PaymentMode::Cash,
        PaymentMode::Card,
        PaymentMode::Credit,
    ])
}

fn any_regime() -> impl Strategy<Value = Regime> {
    prop::sample::select(vec![Regime::Reel, Regime::Ifu])
}

/// Amounts for the stamp properties, weighted onto the band edges where an
/// off-by-one centime changes the rate per tranche: 300,00 / 30 000,00 /
/// 100 000,00 DA, each within a centime, plus the wide range.
fn stamp_amount() -> impl Strategy<Value = i64> {
    prop_oneof![
        (
            prop::sample::select(vec![30_000i64, 3_000_000, 10_000_000]),
            -1i64..=1,
        )
            .prop_map(|(edge, d)| edge.saturating_add(d)),
        0i64..=100_000_000_000,
    ]
}

/// A discount somewhere in [0, total_ht], never above it.
fn part_of(total_ht: Money, per_mille: i64) -> Money {
    Money::centimes(
        i64::try_from(
            i128::from(total_ht.as_centimes())
                .checked_mul(i128::from(per_mille))
                .unwrap()
                / 1000,
        )
        .unwrap(),
    )
}

fn sum_bases(t: &dzpos_core::money::Totals) -> Money {
    t.tva_by_rate
        .iter()
        .fold(Money::ZERO, |a, l| a.checked_add(l.base).unwrap())
}

proptest! {
    /// The columns of the totals table agree with each other whatever the
    /// basket, and the spread neither invents nor loses a centime.
    #[test]
    fn totals_columns_agree(
        lines in basket(),
        per_mille in 0i64..=1000,
        mode in any_mode(),
        stamp_enabled in any::<bool>(),
    ) {
        let bare = compute_totals(&lines, &opts(Money::ZERO, mode, false, Regime::Reel)).unwrap();
        let discount = part_of(bare.total_ht, per_mille);
        let t = compute_totals(&lines, &opts(discount, mode, stamp_enabled, Regime::Reel)).unwrap();

        prop_assert_eq!(
            t.total_ht.checked_sub(t.discount).unwrap(),
            sum_bases(&t),
            "total_ht - discount must equal the sum of the group bases"
        );
        prop_assert_eq!(sum_bases(&t), t.subtotal_ht);
        let tva = t.tva_by_rate.iter().fold(Money::ZERO, |a, l| a.checked_add(l.amount).unwrap());
        prop_assert_eq!(tva, t.tva);
        prop_assert_eq!(t.subtotal_ht.checked_add(t.tva).unwrap(), t.total_ttc);
        prop_assert_eq!(t.total_ttc.checked_add(t.stamp).unwrap(), t.net_to_pay);
    }

    /// Each group's share of the global discount is its exact proportional
    /// share rounded down, and the leftover centimes all go to one group:
    /// the one with the largest HT, the lower rate winning a tie. Sums alone
    /// cannot see this; a rule that hands the leftover to the first group,
    /// or to the smallest, conserves centimes just as well.
    #[test]
    fn the_leftover_centimes_go_to_the_largest_group(
        lines in basket(),
        per_mille in 0i64..=1000,
    ) {
        let bare = compute_totals(
            &lines,
            &opts(Money::ZERO, PaymentMode::Card, false, Regime::Reel),
        ).unwrap();
        let total_ht = bare.total_ht.as_centimes();
        if total_ht > 0 {
            let discount = part_of(bare.total_ht, per_mille);
            let t = compute_totals(
                &lines,
                &opts(discount, PaymentMode::Card, false, Regime::Reel),
            ).unwrap();
            // With no discount every base is the group's HT before the spread.
            let hts: Vec<i64> = bare.tva_by_rate.iter().map(|l| l.base.as_centimes()).collect();
            prop_assert_eq!(hts.len(), t.tva_by_rate.len());

            let floors: Vec<i64> = hts.iter().map(|ht| {
                i64::try_from(
                    i128::from(discount.as_centimes())
                        .checked_mul(i128::from(*ht)).unwrap()
                        / i128::from(total_ht),
                ).unwrap()
            }).collect();
            let leftover = discount.as_centimes() - floors.iter().sum::<i64>();
            prop_assert!(
                leftover >= 0 && leftover < i64::try_from(hts.len().max(1)).unwrap(),
                "leftover {leftover} is not a handful of centimes"
            );

            let mut got_extra = Vec::new();
            for (i, l) in t.tva_by_rate.iter().enumerate() {
                let share = hts[i] - l.base.as_centimes();
                prop_assert!(
                    share == floors[i] || share == floors[i] + leftover,
                    "group {i}: share {share} is neither the floor {} nor floor plus leftover",
                    floors[i]
                );
                if share != floors[i] {
                    got_extra.push(i);
                }
            }
            if leftover == 0 {
                prop_assert!(got_extra.is_empty());
            } else {
                prop_assert_eq!(got_extra.len(), 1, "the leftover must land on one group");
                let j = got_extra[0];
                let max_ht = *hts.iter().max().unwrap();
                prop_assert_eq!(hts[j], max_ht, "the leftover did not go to the largest HT");
                prop_assert!(
                    hts[..j].iter().all(|h| *h < max_ht),
                    "a lower rate had the same HT and should have won the tie"
                );
            }
        }
    }

    /// The stamp column is the stamp of the TTC, not of the subtotal HT and
    /// not of the net, and the shop setting switches it off.
    #[test]
    fn the_stamp_column_is_the_stamp_of_the_ttc(
        lines in basket(),
        per_mille in 0i64..=1000,
        mode in any_mode(),
        stamp_enabled in any::<bool>(),
        regime in any_regime(),
    ) {
        let bare = compute_totals(&lines, &opts(Money::ZERO, mode, false, regime)).unwrap();
        let discount = part_of(bare.total_ht, per_mille);
        let t = compute_totals(&lines, &opts(discount, mode, stamp_enabled, regime)).unwrap();
        let expected = if stamp_enabled {
            stamp(t.total_ttc, mode).unwrap()
        } else {
            Money::ZERO
        };
        prop_assert_eq!(t.stamp, expected);
    }

    /// Under the IFU there is no TVA row and no TVA, and the net is the
    /// subtotal plus the stamp (`regime_ifu_prints_no_tva`).
    #[test]
    fn ifu_carries_no_tva(lines in basket(), mode in any_mode()) {
        let t = compute_totals(&lines, &opts(Money::ZERO, mode, true, Regime::Ifu)).unwrap();
        prop_assert!(t.tva_by_rate.is_empty());
        prop_assert_eq!(sum_bases(&t), Money::ZERO);
        prop_assert_eq!(t.tva, Money::ZERO);
        prop_assert_eq!(t.total_ttc, t.subtotal_ht);
        prop_assert_eq!(t.total_ttc.checked_add(t.stamp).unwrap(), t.net_to_pay);
    }

    /// A global discount above the basket is refused, never clamped to it.
    #[test]
    fn a_discount_above_the_basket_is_refused(lines in basket(), over in 1i64..=1_000_000) {
        let bare = compute_totals(
            &lines,
            &opts(Money::ZERO, PaymentMode::Cash, false, Regime::Reel),
        ).unwrap();
        let too_much = bare.total_ht.checked_add(Money::centimes(over)).unwrap();
        let got = compute_totals(&lines, &opts(too_much, PaymentMode::Cash, false, Regime::Reel));
        prop_assert_eq!(got, Err(MoneyError::GlobalDiscountAboveTotal));
    }

    /// Nothing is due at or under 300,00 DA.
    #[test]
    fn stamp_is_zero_at_or_under_the_floor(a in -1_000_000i64..=30_000) {
        prop_assert_eq!(stamp(Money::centimes(a), PaymentMode::Cash).unwrap(), Money::ZERO);
    }

    /// The stamp never falls as the amount rises, band edges included.
    #[test]
    fn stamp_is_monotone_in_the_amount(a in stamp_amount(), b in stamp_amount()) {
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        let s_lo = stamp(Money::centimes(lo), PaymentMode::Cash).unwrap();
        let s_hi = stamp(Money::centimes(hi), PaymentMode::Cash).unwrap();
        prop_assert!(s_lo <= s_hi, "{lo} gave {s_lo:?} but {hi} gave {s_hi:?}");
    }

    /// Card and credit never carry the stamp, at any amount.
    #[test]
    fn stamp_is_zero_for_card_and_credit(a in any::<i64>()) {
        prop_assert_eq!(stamp(Money::centimes(a), PaymentMode::Card).unwrap(), Money::ZERO);
        prop_assert_eq!(stamp(Money::centimes(a), PaymentMode::Credit).unwrap(), Money::ZERO);
    }

    /// Anything above the floor owes at least the 5,00 DA minimum, and
    /// nothing at or under it owes anything. Band edges included.
    #[test]
    fn the_stamp_floor_and_minimum_hold_at_the_band_edges(a in stamp_amount()) {
        let amount = a;
        let got = stamp(Money::centimes(amount), PaymentMode::Cash).unwrap();
        if amount <= 30_000 {
            prop_assert_eq!(got, Money::ZERO, "{} is at or under the floor", amount);
        } else {
            prop_assert!(got >= Money::centimes(500), "{amount} owes {got:?}");
        }
    }
}
