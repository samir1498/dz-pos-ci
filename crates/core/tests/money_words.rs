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

/// `read_back` is the inverse parser for the language, where one exists.
/// Running it on the hand-written strings is what proves the parser
/// itself before the round-trip property leans on it.
fn check_golden(file: &str, lang: Lang, lang_tag: &str, read_back: Option<fn(&str) -> i64>) {
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
        let got = amount_in_words(Money::centimes(c.centimes), lang)
            .unwrap_or_else(|e| panic!("{file}, {} centimes: {e}", c.centimes));
        assert_eq!(got, c.words, "{file}, {} centimes", c.centimes);
        if let Some(read_back) = read_back {
            assert_eq!(
                read_back(&c.words),
                c.centimes,
                "{file} reads back, {} centimes",
                c.centimes
            );
        }
    }
}

#[test]
fn words_fr_golden_matches() {
    check_golden("words_fr_golden", Lang::Fr, "fr", Some(parse_fr));
}

#[test]
fn words_en_golden_matches() {
    check_golden("words_en_golden", Lang::En, "en", Some(parse_en));
}

#[test]
fn words_ar_golden_matches() {
    check_golden("words_ar_golden", Lang::Ar, "ar", None);
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

// ------- Test-only inverse: French and English words back to centimes -------
//
// Task T6 wants the round trip proved, so the test reads the words back
// and compares. These parsers walk the words and apply the multiplier
// rules of the language; they do not reuse the generator's split into
// groups of three, so a generator that drops a scale word or an
// agreement is caught rather than mirrored. `check_golden` runs them on
// the hand-written golden strings first, which is what pins the parsers
// themselves before the property leans on them.
//
// Arabic has no parser here. Its duals lose the nun in the construct
// state, so `ألفا` and `مائتا` are the same numbers as `ألفان` and
// `مائتان` written differently because of the word that follows, and the
// form a group ending in 01 or 02 takes is still open for the native
// reviewer (see the note in `words_ar_golden.json`). Reading Arabic back
// would mean guessing at both. The golden file carries Arabic instead.

fn fr_word(word: &str) -> Option<i64> {
    Some(match word {
        "zéro" => 0,
        "un" => 1,
        "deux" => 2,
        "trois" => 3,
        "quatre" => 4,
        "cinq" => 5,
        "six" => 6,
        "sept" => 7,
        "huit" => 8,
        "neuf" => 9,
        "dix" => 10,
        "onze" => 11,
        "douze" => 12,
        "treize" => 13,
        "quatorze" => 14,
        "quinze" => 15,
        "seize" => 16,
        "vingt" | "vingts" => 20,
        "trente" => 30,
        "quarante" => 40,
        "cinquante" => 50,
        "soixante" => 60,
        _ => return None,
    })
}

/// One hyphen-joined French chunk, so everything below one million.
fn fr_chunk(chunk: &str) -> i64 {
    let mut total = 0;
    let mut hundreds = 0;
    let mut small = 0;
    for word in chunk.split('-') {
        match word {
            "et" => {}
            "cent" | "cents" => {
                hundreds += if small == 0 { 100 } else { small * 100 };
                small = 0;
            }
            "mille" => {
                let group = hundreds + small;
                total += if group == 0 { 1_000 } else { group * 1_000 };
                hundreds = 0;
                small = 0;
            }
            _ => {
                let value = fr_word(word).unwrap_or_else(|| panic!("french word: {word}"));
                // quatre-vingt is the one place a unit multiplies a ten.
                if value == 20 && small == 4 {
                    small = 80;
                } else {
                    small += value;
                }
            }
        }
    }
    total + hundreds + small
}

fn parse_fr(words: &str) -> i64 {
    // Hyphens hold the numerals together, so a spaced " et " can only be
    // the one between the dinars and the centimes.
    let (dinars, centimes) = match words.rsplit_once(" et ") {
        Some((head, tail)) if tail.ends_with("centime") || tail.ends_with("centimes") => {
            let count = tail
                .strip_suffix(" centimes")
                .or_else(|| tail.strip_suffix(" centime"))
                .unwrap_or_else(|| panic!("french centimes: {tail}"));
            (head, fr_chunk(count))
        }
        _ => (words, 0),
    };
    let number = dinars
        .strip_suffix(" dinars")
        .or_else(|| dinars.strip_suffix(" dinar"))
        .unwrap_or_else(|| panic!("french dinars: {dinars}"));
    let number = number.strip_suffix(" de").unwrap_or(number);
    let mut total = 0;
    let mut pending = 0;
    for token in number.split(' ') {
        match token {
            "milliard" | "milliards" => {
                total += pending * 1_000_000_000;
                pending = 0;
            }
            "million" | "millions" => {
                total += pending * 1_000_000;
                pending = 0;
            }
            _ => pending = fr_chunk(token),
        }
    }
    (total + pending) * 100 + centimes
}

fn en_word(word: &str) -> Option<i64> {
    Some(match word {
        "zero" => 0,
        "one" => 1,
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        "eleven" => 11,
        "twelve" => 12,
        "thirteen" => 13,
        "fourteen" => 14,
        "fifteen" => 15,
        "sixteen" => 16,
        "seventeen" => 17,
        "eighteen" => 18,
        "nineteen" => 19,
        "twenty" => 20,
        "thirty" => 30,
        "forty" => 40,
        "fifty" => 50,
        "sixty" => 60,
        "seventy" => 70,
        "eighty" => 80,
        "ninety" => 90,
        _ => return None,
    })
}

fn en_number(words: &str) -> i64 {
    let mut total = 0;
    let mut current = 0;
    for token in words.split(' ') {
        match token {
            "and" => {}
            "hundred" => current *= 100,
            "thousand" => {
                total += current * 1_000;
                current = 0;
            }
            "million" => {
                total += current * 1_000_000;
                current = 0;
            }
            "billion" => {
                total += current * 1_000_000_000;
                current = 0;
            }
            // "twenty-one" is one token holding two words.
            _ => {
                for word in token.split('-') {
                    current += en_word(word).unwrap_or_else(|| panic!("english word: {word}"));
                }
            }
        }
    }
    total + current
}

fn parse_en(words: &str) -> i64 {
    // The number carries its own "and", so only the last one can be the
    // separator, and only when the string ends on a centime.
    let (dinars, centimes) = if words.ends_with("centime") || words.ends_with("centimes") {
        let (head, tail) = words
            .rsplit_once(" and ")
            .unwrap_or_else(|| panic!("english centimes: {words}"));
        let count = tail
            .strip_suffix(" centimes")
            .or_else(|| tail.strip_suffix(" centime"))
            .unwrap_or_else(|| panic!("english centimes: {tail}"));
        (head, en_number(count))
    } else {
        (words, 0)
    };
    let number = dinars
        .strip_suffix(" dinars")
        .or_else(|| dinars.strip_suffix(" dinar"))
        .unwrap_or_else(|| panic!("english dinars: {dinars}"));
    en_number(number) * 100 + centimes
}

/// The small end and the scale boundaries, swept rather than sampled, so
/// the round trip does not depend on what the random draw happened to
/// pick. Every centime value up to 100 dinars, then each carry.
#[test]
fn fr_and_en_read_back_on_the_small_end_and_the_carries() {
    let mut amounts: Vec<i64> = (0..=10_000).collect();
    for scale in [100i64, 10_000, 1_000_000, 100_000_000, 100_000_000_000] {
        for step in -2i64..=2 {
            let candidate = scale + step;
            if candidate >= 0 {
                amounts.push(candidate);
            }
        }
    }
    for c in amounts {
        let fr =
            amount_in_words(Money::centimes(c), Lang::Fr).unwrap_or_else(|e| panic!("fr {c}: {e}"));
        assert_eq!(parse_fr(&fr), c, "fr: {fr}");
        let en =
            amount_in_words(Money::centimes(c), Lang::En).unwrap_or_else(|e| panic!("en {c}: {e}"));
        assert_eq!(parse_en(&en), c, "en: {en}");
    }
}

/// Amounts spread over every magnitude in 0..=10^11 centimes. A flat
/// draw over the whole range lands above ten million dinars almost
/// every time, which leaves the small numbers and the shrinking untested;
/// the sub-ranges put a quarter of the cases in each decade band.
fn any_shop_amount() -> impl Strategy<Value = i64> {
    prop_oneof![
        0i64..=10_000,
        0i64..=10_000_000,
        0i64..=100_000_000,
        0i64..=100_000_000_000,
    ]
}

proptest! {
    /// T6: the words read back to the amount they were written from.
    /// French and English only; Arabic is excluded, see the note above
    /// the parsers.
    #[test]
    fn fr_and_en_words_read_back_to_the_amount(c in any_shop_amount()) {
        let fr = amount_in_words(Money::centimes(c), Lang::Fr).unwrap();
        prop_assert_eq!(parse_fr(&fr), c, "fr: {}", fr);
        let en = amount_in_words(Money::centimes(c), Lang::En).unwrap();
        prop_assert_eq!(parse_en(&en), c, "en: {}", en);
    }

    /// Amounts a shop can see: up to a billion dinars in centimes.
    #[test]
    fn any_shop_amount_is_written_cleanly(c in any_shop_amount()) {
        for lang in [Lang::Fr, Lang::En, Lang::Ar] {
            let s = amount_in_words(Money::centimes(c), lang).unwrap();
            prop_assert!(!s.is_empty(), "c={c} lang={lang:?}");
            prop_assert!(!s.contains("  "), "double space: c={c} lang={lang:?} s={s}");
            prop_assert!(s.trim() == s, "padded: c={c} lang={lang:?} s={s}");
        }
    }
}
