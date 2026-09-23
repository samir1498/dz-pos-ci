//! Fixture-driven tests for the money module. One JSON file per rule in
//! `docs/features.md`; the same files feed vitest against the mockup.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use dzpos_kernel::money::format::{format_centimes, format_qty};
use dzpos_kernel::money::{
    compute_totals, stamp, Bps, Line, Money, MoneyError, PaymentMode, Totals, TotalsOptions,
};
use serde::Deserialize;

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/../../fixtures/money/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[derive(Deserialize)]
struct PctCase {
    name: String,
    amount: i64,
    rate_bps: u32,
    expected: i64,
}

#[derive(Deserialize)]
struct OverflowCase {
    name: String,
    amount: i64,
    rate_bps: u32,
}

#[derive(Deserialize)]
struct InvalidRateCase {
    name: String,
    rate_bps: u32,
}

#[derive(Deserialize)]
struct TotalsInput {
    lines: Vec<Line>,
    opts: TotalsOptions,
}

#[derive(Deserialize)]
struct TotalsCase {
    name: String,
    input: TotalsInput,
    expected: Totals,
}

#[derive(Deserialize)]
struct TotalsErrorCase {
    name: String,
    input: TotalsInput,
    error: String,
}

#[derive(Deserialize)]
struct RoundingFixture {
    name: String,
    pct_cases: Vec<PctCase>,
    overflow_cases: Vec<OverflowCase>,
    invalid_rate_cases: Vec<InvalidRateCase>,
    totals_cases: Vec<TotalsCase>,
    totals_error_cases: Vec<TotalsErrorCase>,
}

#[test]
fn tva_rounding_once_per_rate_pct_cases() {
    let f: RoundingFixture = serde_json::from_str(&fixture("tva_rounding_once_per_rate")).unwrap();
    assert_eq!(f.name, "tva_rounding_once_per_rate");
    assert!(f.pct_cases.len() >= 10, "fixture lost its cases");
    for c in &f.invalid_rate_cases {
        assert_eq!(
            Bps::new(c.rate_bps),
            Err(MoneyError::RateOutOfRange),
            "{}",
            c.name
        );
        let json = c.rate_bps.to_string();
        let r: Result<Bps, _> = serde_json::from_str(&json);
        assert!(r.is_err(), "{} must be rejected by serde", c.name);
    }
    for c in &f.pct_cases {
        let got = Money::centimes(c.amount)
            .pct(Bps::new(c.rate_bps).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", c.name));
        assert_eq!(
            got,
            Money::centimes(c.expected),
            "{}: {} × {} bps",
            c.name,
            c.amount,
            c.rate_bps
        );
    }
    for c in &f.overflow_cases {
        let got = Money::centimes(c.amount).pct(Bps::new(c.rate_bps).unwrap());
        assert_eq!(got, Err(MoneyError::Overflow), "{}", c.name);
    }
}

#[test]
fn tva_rounding_once_per_rate_totals_cases() {
    let f: RoundingFixture = serde_json::from_str(&fixture("tva_rounding_once_per_rate")).unwrap();
    assert!(f.totals_cases.len() >= 10, "fixture lost its totals cases");
    for c in &f.totals_cases {
        let got = compute_totals(&c.input.lines, &c.input.opts).unwrap();
        // Compare field by field so a failure names the column, not the struct.
        assert_eq!(got.total_ht, c.expected.total_ht, "total_ht: {}", c.name);
        assert_eq!(got.discount, c.expected.discount, "discount: {}", c.name);
        assert_eq!(
            got.subtotal_ht, c.expected.subtotal_ht,
            "subtotal_ht: {}",
            c.name
        );
        assert_eq!(
            got.tva_by_rate, c.expected.tva_by_rate,
            "tva_by_rate: {}",
            c.name
        );
        assert_eq!(got.tva, c.expected.tva, "tva: {}", c.name);
        assert_eq!(got.total_ttc, c.expected.total_ttc, "total_ttc: {}", c.name);
        assert_eq!(got.stamp, c.expected.stamp, "stamp: {}", c.name);
        assert_eq!(
            got.net_to_pay, c.expected.net_to_pay,
            "net_to_pay: {}",
            c.name
        );
    }
}

#[test]
fn tva_rounding_once_per_rate_totals_error_cases() {
    let f: RoundingFixture = serde_json::from_str(&fixture("tva_rounding_once_per_rate")).unwrap();
    assert!(!f.totals_error_cases.is_empty(), "fixture lost its errors");
    for c in &f.totals_error_cases {
        let got = compute_totals(&c.input.lines, &c.input.opts);
        let err = got.expect_err(&c.name);
        assert_eq!(format!("{err:?}"), c.error, "{}", c.name);
    }
}

#[derive(Deserialize)]
struct LineCase {
    name: String,
    unit_price: i64,
    qty_milli: i64,
    expected: i64,
}

#[derive(Deserialize)]
struct LineOverflowCase {
    name: String,
    unit_price: i64,
    qty_milli: i64,
}

#[derive(Deserialize)]
struct FractionalQtyFixture {
    name: String,
    line_cases: Vec<LineCase>,
    line_overflow_cases: Vec<LineOverflowCase>,
    totals_cases: Vec<TotalsCase>,
}

/// A quantity is thousandths of the unit, so a kilo product never needs a
/// float. The line gross is rounded once, half away from zero, before the
/// line discount comes off.
#[test]
fn line_total_fractional_qty_cases() {
    let f: FractionalQtyFixture = serde_json::from_str(&fixture("line_total_fractional_qty"))
        .unwrap_or_else(|e| panic!("line_total_fractional_qty: {e}"));
    assert_eq!(f.name, "line_total_fractional_qty");
    assert!(f.line_cases.len() >= 10, "fixture lost its line cases");
    for c in &f.line_cases {
        let got = Money::centimes(c.unit_price)
            .checked_mul_milli(c.qty_milli)
            .unwrap_or_else(|e| panic!("{}: {e}", c.name));
        assert_eq!(
            got,
            Money::centimes(c.expected),
            "{}: {} × {} milli",
            c.name,
            c.unit_price,
            c.qty_milli
        );
    }
    for c in &f.line_overflow_cases {
        let got = Money::centimes(c.unit_price).checked_mul_milli(c.qty_milli);
        assert_eq!(got, Err(MoneyError::Overflow), "{}", c.name);
    }
}

/// Rounding once per line is what the totals cases here pin: the two-line
/// case sums to a centime more than rounding the raw product sum would.
#[test]
fn line_total_fractional_qty_totals_cases() {
    let f: FractionalQtyFixture =
        serde_json::from_str(&fixture("line_total_fractional_qty")).unwrap();
    assert!(!f.totals_cases.is_empty(), "fixture lost its totals cases");
    for c in &f.totals_cases {
        let got = compute_totals(&c.input.lines, &c.input.opts)
            .unwrap_or_else(|e| panic!("{}: {e}", c.name));
        assert_eq!(got.total_ht, c.expected.total_ht, "total_ht: {}", c.name);
        assert_eq!(
            got.subtotal_ht, c.expected.subtotal_ht,
            "subtotal_ht: {}",
            c.name
        );
        assert_eq!(
            got.tva_by_rate, c.expected.tva_by_rate,
            "tva_by_rate: {}",
            c.name
        );
        assert_eq!(got.tva, c.expected.tva, "tva: {}", c.name);
        assert_eq!(got.total_ttc, c.expected.total_ttc, "total_ttc: {}", c.name);
        assert_eq!(got.stamp, c.expected.stamp, "stamp: {}", c.name);
        assert_eq!(
            got.net_to_pay, c.expected.net_to_pay,
            "net_to_pay: {}",
            c.name
        );
    }
}

#[derive(Deserialize)]
struct StampInput {
    total_ttc: i64,
    mode: PaymentMode,
}

#[derive(Deserialize)]
struct StampExpected {
    stamp: i64,
}

#[derive(Deserialize)]
struct StampCase {
    name: String,
    input: StampInput,
    expected: StampExpected,
}

#[derive(Deserialize)]
struct StampOverflowCase {
    name: String,
    input: StampInput,
}

#[derive(Deserialize)]
struct StampFixture {
    name: String,
    cases: Vec<StampCase>,
    overflow_cases: Vec<StampOverflowCase>,
}

#[test]
fn stamp_progressive_tranches_cases() {
    let f: StampFixture = serde_json::from_str(&fixture("stamp_progressive_tranches")).unwrap();
    assert_eq!(f.name, "stamp_progressive_tranches");
    assert!(f.cases.len() >= 20, "fixture lost its cases");
    for c in &f.cases {
        let got = stamp(Money::centimes(c.input.total_ttc), c.input.mode).unwrap();
        assert_eq!(got, Money::centimes(c.expected.stamp), "{}", c.name);
    }
    for c in &f.overflow_cases {
        let got = stamp(Money::centimes(c.input.total_ttc), c.input.mode);
        assert_eq!(got, Err(MoneyError::Overflow), "{}", c.name);
    }
}

#[derive(Deserialize)]
struct Accepts {
    json: String,
    centimes: i64,
}

#[derive(Deserialize)]
struct Rejects {
    json: String,
    why: String,
}

#[derive(Deserialize)]
struct NoFloatFixture {
    accepts: Vec<Accepts>,
    rejects: Vec<Rejects>,
}

#[test]
fn money_no_float_serde() {
    let f: NoFloatFixture = serde_json::from_str(&fixture("money_no_float")).unwrap();
    for c in &f.accepts {
        let m: Money = serde_json::from_str(&c.json).unwrap_or_else(|e| panic!("{}: {e}", c.json));
        assert_eq!(m.as_centimes(), c.centimes, "{}", c.json);
        assert_eq!(
            m,
            Money::centimes(c.centimes),
            "constructor vs serde {}",
            c.json
        );
        assert_eq!(
            serde_json::to_string(&m).unwrap(),
            c.json,
            "round trip {}",
            c.json
        );
    }
    for c in &f.rejects {
        let r: Result<Money, _> = serde_json::from_str(&c.json);
        assert!(r.is_err(), "{} must be rejected: {}", c.json, c.why);
    }
}

#[derive(Deserialize)]
struct AmountCase {
    name: String,
    centimes: i64,
    expected: String,
}

#[derive(Deserialize)]
struct QtyCase {
    name: String,
    milli: i64,
    expected: String,
}

#[derive(Deserialize)]
struct FormatFixture {
    name: String,
    cases: Vec<AmountCase>,
    qty_cases: Vec<QtyCase>,
}

/// The printed ticket and the screen have to show a customer the same
/// figure, and they are two implementations: this file and
/// `packages/shared/src/money.test.ts` read the same cases so neither can
/// drift on its own.
#[test]
fn format_centimes_matches_the_shared_formatter() {
    let f: FormatFixture = serde_json::from_str(&fixture("format_centimes")).unwrap();
    assert_eq!(f.name, "format_centimes");
    assert!(f.cases.len() >= 10, "fixture lost its cases");
    for c in &f.cases {
        assert_eq!(
            format_centimes(Money::centimes(c.centimes)),
            c.expected,
            "{}: {} centimes",
            c.name,
            c.centimes
        );
    }
    assert!(!f.qty_cases.is_empty(), "fixture lost its quantity cases");
    for c in &f.qty_cases {
        assert_eq!(
            format_qty(c.milli),
            c.expected,
            "{}: {} thousandths",
            c.name,
            c.milli
        );
    }
}
