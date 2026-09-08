//! Fixture-driven tests for the money module. One JSON file per rule in
//! `docs/features.md`; the same files feed vitest against the mockup.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use dzpos_core::money::{stamp, Bps, Money, MoneyError, PaymentMode};
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
struct RoundingFixture {
    name: String,
    pct_cases: Vec<PctCase>,
    overflow_cases: Vec<OverflowCase>,
    invalid_rate_cases: Vec<InvalidRateCase>,
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
