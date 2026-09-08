//! Golden tests for the amount in words. Décret 05-468 wants the TTC total
//! "en chiffres et en lettres", so the three golden files under
//! `fixtures/money/` are the spelling contract: every case matches byte for
//! byte, hyphens and spaces included. A native speaker corrects the Arabic
//! file (research R6); the generator follows the file, never the other way.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use dzpos_core::money::words::{amount_in_words, Lang, WordsError};
use dzpos_core::money::Money;
use proptest::prelude::*;
use serde::Deserialize;

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/../../fixtures/money/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[derive(Deserialize)]
struct WordsCase {
    centimes: i64,
    words: String,
}

#[derive(Deserialize)]
struct WordsFixture {
    name: String,
    #[allow(dead_code)]
    rule: String,
    lang: String,
    #[serde(default)]
    reviewed_by_native_speaker: Option<bool>,
    cases: Vec<WordsCase>,
}

/// Every golden file carries these amounts. Listing them here means a file
/// that loses a case fails instead of passing on what is left.
const REQUIRED: [i64; 32] = [
    0,
    1,
    50,
    100,
    105,
    200,
    1_000,
    1_100,
    1_234,
    2_100,
    7_100,
    8_000,
    8_100,
    10_000,
    10_100,
    18_000,
    20_000,
    20_100,
    100_000,
    100_100,
    200_000,
    810_000,
    8_000_000,
    20_000_000,
    100_000_000,
    120_000_000,
    200_000_000,
    200_000_050,
    200_020_000,
    20_000_000_000,
    99_999_999_900,
    100_000_000_000,
];

fn check_golden(file: &str, lang: Lang, lang_tag: &str) {
    let f: WordsFixture = serde_json::from_str(&fixture(file)).unwrap();
    assert_eq!(f.name, file);
    assert_eq!(f.lang, lang_tag);
    for want in REQUIRED {
        assert!(
            f.cases.iter().any(|c| c.centimes == want),
            "{file} lost the case for {want} centimes"
        );
    }
    for c in &f.cases {
        let got = amount_in_words(Money::centimes(c.centimes), lang).unwrap();
        assert_eq!(got, c.words, "{file}, {} centimes", c.centimes);
    }
}

#[test]
fn words_fr_golden_matches() {
    check_golden("words_fr_golden", Lang::Fr, "fr");
}

#[test]
fn words_en_golden_matches() {
    check_golden("words_en_golden", Lang::En, "en");
}

#[test]
fn words_ar_golden_matches() {
    check_golden("words_ar_golden", Lang::Ar, "ar");
}

/// The Arabic file is unsourced until research R6 lands. The flag stays
/// false so nobody prints it as reviewed by accident.
#[test]
fn words_ar_golden_is_not_yet_reviewed() {
    let f: WordsFixture = serde_json::from_str(&fixture("words_ar_golden")).unwrap();
    assert_eq!(f.reviewed_by_native_speaker, Some(false));
}

#[test]
fn a_negative_amount_is_an_error_in_every_language() {
    for lang in [Lang::Fr, Lang::En, Lang::Ar] {
        assert_eq!(
            amount_in_words(Money::centimes(-1), lang),
            Err(WordsError::Negative)
        );
    }
}

#[test]
fn the_top_of_the_range_is_written_and_past_it_is_an_error() {
    // 999 999 999 999 dinars is the last amount the milliard scale covers.
    let top = Money::centimes(99_999_999_999_900);
    for lang in [Lang::Fr, Lang::En, Lang::Ar] {
        assert!(amount_in_words(top, lang).is_ok());
        assert_eq!(
            amount_in_words(Money::centimes(100_000_000_000_000), lang),
            Err(WordsError::TooLarge)
        );
    }
}

proptest! {
    /// Amounts a shop can see: up to a billion dinars in centimes.
    #[test]
    fn any_shop_amount_is_written_cleanly(c in 0i64..=100_000_000_000) {
        for lang in [Lang::Fr, Lang::En, Lang::Ar] {
            let s = amount_in_words(Money::centimes(c), lang).unwrap();
            prop_assert!(!s.is_empty(), "c={c} lang={lang:?}");
            prop_assert!(!s.contains("  "), "double space: c={c} lang={lang:?} s={s}");
            prop_assert!(s.trim() == s, "padded: c={c} lang={lang:?} s={s}");
        }
    }
}
