// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The 58 × 40 mm barcode label and its A4 sheet, against their golden
//! files (features.md §4: "Golden-file test for every template × language
//! against fixed fixtures. A template change is a reviewed golden diff").
//!
//! Regenerate with `UPDATE_GOLDENS=1 cargo test -p dzpos-core --test
//! print_barcode_label`. The run that rewrites them fails on purpose.
//!
//! The bars are read back out of the golden as well as the digits: a label
//! whose picture and whose printed number stop agreeing is a label that
//! scans as another product, and the number under the bars is the only
//! thing a person can check by eye.

use std::path::PathBuf;

use chrono::NaiveDate;
use dzpos_retail::error::{CoreError, RetailError};
use dzpos_retail::lang::Lang;
use dzpos_retail::models::product::{Product, Unit};
use dzpos_retail::money::{Bps, Money};
use dzpos_retail::print::{render_label, render_label_sheet};

mod common;

/// The one product every golden prints. An in-store code in the restricted
/// range GS1 reserves for a shop's own numbering (`services::products`), so
/// the label a shop really prints is the label pinned here. Its check digit
/// is 4, and the encoder is handed the whole thirteen so it recomputes that
/// digit and refuses a code whose own is wrong.
const BARCODE: &str = "2000010000074";
const PRICE: i64 = 32_050;
const NAME: &str = "Café moulu 250 g";

/// A second product, for the sheet: two labels prove the grid repeats and a
/// long name proves it wraps rather than pushing the bars off the label.
const SECOND_BARCODE: &str = "6130001234563";
const SECOND_PRICE: i64 = 8_000;
const SECOND_NAME: &str = "Pain de campagne au levain, cuit sur pierre";

fn goldens_dir() -> PathBuf {
    common::goldens_dir("barcode_label")
}

fn product(name: &str, barcode: &str, selling: i64) -> Product {
    let at = NaiveDate::from_ymd_opt(2026, 9, 9)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap();
    Product {
        id: 1,
        shop_id: 1,
        name: name.to_owned(),
        barcode: Some(barcode.to_owned()),
        category_id: None,
        unit: Unit::Piece,
        cost: Money::centimes(20_000),
        selling: Money::centimes(selling),
        wholesale: None,
        qty_on_hand_milli: 12_000,
        low_stock_at_milli: 3_000,
        rate_bps: Bps::new(1900).unwrap(),
        active: true,
        created_at: at,
        updated_at: at,
    }
}

fn golden(name: &str, rendered: &str, updated: &mut Vec<String>) -> String {
    let path = goldens_dir().join(name);
    if std::env::var_os("UPDATE_GOLDENS").is_some() {
        std::fs::create_dir_all(goldens_dir()).unwrap();
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        if existing != rendered {
            std::fs::write(&path, rendered).unwrap();
            updated.push(name.to_owned());
        }
        return rendered.to_owned();
    }
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}; run UPDATE_GOLDENS=1 to write it", path.display()))
}

fn refuse_a_silent_regeneration(updated: &[String]) {
    assert!(
        updated.is_empty(),
        "rewrote {updated:?}: read the diff, then run the test again without UPDATE_GOLDENS"
    );
}

/// The text of the one span carrying `marker`.
fn spans(html: &str, marker: &str) -> Vec<String> {
    let opening = format!("<span class=\"{marker}\">");
    html.split(&opening)
        .skip(1)
        .map(|rest| {
            let end = rest.find("</span>").expect("a span never closes");
            rest[..end].to_owned()
        })
        .collect()
}

/// The `<rect>` elements of the bars, as the string of ones and zeroes they
/// stand for. Read off the golden, by a reader that shares no code with the
/// encoder: the bars a shop's scanner sees are what this checks.
fn bars(html: &str) -> String {
    let svg = html
        .split_once("<svg")
        .and_then(|(_, rest)| rest.split_once("</svg>"))
        .map(|(inside, _)| inside.to_owned())
        .expect("the label carries no bars");
    // Every dark module is one `fill="#000000"` rect at its own x; the light
    // ones are the one background rect the generator lays down first.
    let dark: Vec<u32> = svg
        .split("<rect")
        .skip(2)
        .filter_map(|rect| {
            let (_, rest) = rect.split_once("x=\"")?;
            let (value, _) = rest.split_once('"')?;
            value.parse().ok()
        })
        .collect();
    let width = svg
        .split_once("viewBox=\"0 0 ")
        .and_then(|(_, rest)| rest.split_once(' '))
        .and_then(|(w, _)| w.parse::<u32>().ok())
        .expect("the bars carry no viewBox");
    (0..width)
        .map(|x| if dark.contains(&x) { '1' } else { '0' })
        .collect()
}

#[test]
fn the_label_prints_the_name_the_price_and_the_digits_in_all_three_languages() {
    let mut updated = Vec::new();
    for lang in Lang::ALL {
        let rendered = render_label(&product(NAME, BARCODE, PRICE), lang).unwrap();
        let file = golden(&format!("{}.html", lang.tag()), &rendered, &mut updated);
        assert!(file.contains(NAME), "the label drops the product name");
        assert_eq!(spans(&file, "code"), vec![BARCODE.to_owned()]);
        // The price is the amount the fiche stores, read back out of the
        // golden by a parser that shares no code with the formatter.
        let printed = spans(&file, "amount");
        assert_eq!(printed.len(), 1, "{printed:?}");
        let digits: String = printed[0].chars().filter(char::is_ascii_digit).collect();
        assert_eq!(digits.parse::<i64>().unwrap(), PRICE);
        assert!(
            file.contains("58mm 40mm"),
            "the label is not on the 58 x 40 paper"
        );
    }
    refuse_a_silent_regeneration(&updated);
}

#[test]
fn the_bars_encode_the_twelve_digits_and_the_check_digit_the_code_carries() {
    let mut updated = Vec::new();
    let rendered = render_label(&product(NAME, BARCODE, PRICE), Lang::Fr).unwrap();
    let file = golden("fr.html", &rendered, &mut updated);

    // An EAN-13 is 95 modules: 3 guard, 42 left, 5 centre, 42 right, 3 guard.
    let modules = bars(&file);
    assert_eq!(modules.len(), 95, "{modules}");
    assert!(modules.starts_with("101"), "no left guard: {modules}");
    assert!(modules.ends_with("101"), "no right guard: {modules}");
    assert_eq!(&modules[45..50], "01010", "no centre guard: {modules}");

    // And the bars really are this product's number: decoded here off the
    // golden with the GS1 tables written out below, which share no code with
    // the encoder that drew them. A label whose picture and whose digits
    // parted company is a label that scans as another product.
    let decoded = decode_ean13(&modules);
    assert_eq!(decoded, BARCODE, "the bars encode {decoded}, not {BARCODE}");
    assert_eq!(
        check_digit(&decoded[..12]),
        decoded.chars().nth(12).unwrap_or(' '),
        "the thirteenth digit of the decoded bars is not its own check digit"
    );
    refuse_a_silent_regeneration(&updated);
}

/// The GS1 encodings of a digit in the three character sets: odd-parity left
/// (A), even-parity left (B) and right (C). Written out rather than derived,
/// so this decoder cannot be wrong in the same way the encoder is.
const A: [&str; 10] = [
    "0001101", "0011001", "0010011", "0111101", "0100011", "0110001", "0101111", "0111011",
    "0110111", "0001011",
];
const B: [&str; 10] = [
    "0100111", "0110011", "0011011", "0100001", "0011101", "0111001", "0000101", "0010001",
    "0001001", "0010111",
];
const C: [&str; 10] = [
    "1110010", "1100110", "1101100", "1000010", "1011100", "1001110", "1010000", "1000100",
    "1001000", "1110100",
];
/// Which of A and B each of the six left digits uses, per first digit. The
/// first digit of an EAN-13 has no bars of its own: it is this pattern.
const PARITY: [&str; 10] = [
    "AAAAAA", "AABABB", "AABBAB", "AABBBA", "ABAABB", "ABBAAB", "ABBBAA", "ABABAB", "ABABBA",
    "ABBABA",
];

fn decode_ean13(modules: &str) -> String {
    let mut parity = String::new();
    let mut digits = String::new();
    for index in 0..6 {
        let at = 3 + index * 7;
        let seven = &modules[at..at + 7];
        if let Some(d) = A.iter().position(|c| *c == seven) {
            parity.push('A');
            digits.push_str(&d.to_string());
        } else if let Some(d) = B.iter().position(|c| *c == seven) {
            parity.push('B');
            digits.push_str(&d.to_string());
        } else {
            panic!("left digit {index} is not an EAN-13 encoding: {seven}");
        }
    }
    let first = PARITY
        .iter()
        .position(|p| *p == parity)
        .unwrap_or_else(|| panic!("no first digit has the parity {parity}"));
    let mut right = String::new();
    for index in 0..6 {
        let at = 50 + index * 7;
        let seven = &modules[at..at + 7];
        let d = C
            .iter()
            .position(|c| *c == seven)
            .unwrap_or_else(|| panic!("right digit {index} is not an EAN-13 encoding: {seven}"));
        right.push_str(&d.to_string());
    }
    format!("{first}{digits}{right}")
}

/// The GS1 check digit of twelve digits: odd positions weigh 1, even weigh
/// 3, and the digit brings the total to the next multiple of ten.
fn check_digit(body: &str) -> char {
    let mut sum = 0_u32;
    for (index, c) in body.chars().enumerate() {
        let d = c.to_digit(10).unwrap_or(0);
        sum += if index % 2 == 0 { d } else { d * 3 };
    }
    char::from_digit((10 - (sum % 10)) % 10, 10).unwrap_or('?')
}

#[test]
fn a_code_whose_check_digit_is_wrong_gets_no_label_at_all() {
    // 2000010000075: the same twelve digits with the wrong thirteenth. A
    // label printed from it would scan as nothing, or worse as something,
    // and there is no honest picture of it.
    let refused = render_label(&product(NAME, "2000010000075", PRICE), Lang::Fr).unwrap_err();
    assert_eq!(refused.code(), "validation");

    let no_code = Product {
        barcode: None,
        ..product(NAME, BARCODE, PRICE)
    };
    let empty = render_label(&no_code, Lang::Fr).unwrap_err();
    assert_eq!(empty.code(), "validation");
    // An empty column and a code that is not an EAN-13 are two different
    // things to do something about, so they are two fields.
    match &empty {
        RetailError::Kernel(CoreError::Validation { field, .. }) => assert_eq!(*field, "barcode"),
        other => panic!("{other:?}"),
    }

    // Not thirteen digits at all: a supplier reference typed into the fiche.
    let typed = render_label(&product(NAME, "REF-4471", PRICE), Lang::Fr).unwrap_err();
    assert_eq!(typed.code(), "validation");
}

/// The twelve digits of `BARCODE` without its check digit. barcoders takes
/// a twelve-digit EAN-13 and appends the check digit it computes, so the
/// bars would carry thirteen digits while the label printed the twelve off
/// the fiche: a scanner and a person reading the same sticker would come
/// away with different numbers.
const TWELVE: &str = "200001000007";

/// The geometry the sheet's stylesheet lays down, read back out of the
/// golden rather than restated here: A4 is 210 x 297 mm, the page margin
/// and the gap come off the file, and what the arithmetic has to give is
/// three labels across and six down.
fn millimetres(css: &str, after: &str) -> f64 {
    let (_, rest) = css
        .split_once(after)
        .unwrap_or_else(|| panic!("no {after}"));
    let digits: String = rest
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    digits
        .parse()
        .unwrap_or_else(|_| panic!("no number after {after}"))
}

#[test]
fn the_sheet_puts_eighteen_labels_on_an_a4_page() {
    let mut updated = Vec::new();
    // Eighteen products, so a full page is what the golden holds and a
    // nineteenth would be the one that proves the grid wrapped onto a
    // second page rather than shrinking to fit.
    let products: Vec<Product> = (0..18)
        .map(|i| product(&format!("Article {i}"), BARCODE, PRICE + i64::from(i)))
        .collect();
    let rendered = render_label_sheet(&products, Lang::Fr).unwrap();
    let file = golden("sheet-full-fr.html", &rendered, &mut updated);

    assert_eq!(
        file.matches("<article class=\"label\">").count(),
        18,
        "the sheet does not carry the eighteen labels it was given"
    );

    // And they fit on one A4 page, by the sheet's own numbers.
    let page_margin = millimetres(&file, "@page { size: A4; margin:");
    let gap = millimetres(&file, ".sheet { display: flex; flex-wrap: wrap; gap:");
    let width = millimetres(&file, ".label { box-sizing: border-box; inline-size:");
    let height = millimetres(&file, "block-size:");

    let usable_across = 210.0 - page_margin * 2.0;
    let usable_down = 297.0 - page_margin * 2.0;
    // n labels take n widths and n-1 gaps.
    let across = ((usable_across + gap) / (width + gap)).floor();
    let down = ((usable_down + gap) / (height + gap)).floor();
    assert_eq!(across, 3.0, "{width} mm labels no longer go three across");
    assert_eq!(down, 6.0, "{height} mm labels no longer go six down");
    assert_eq!(across * down, 18.0, "a page no longer holds eighteen");

    refuse_a_silent_regeneration(&updated);
}

#[test]
fn a_code_of_twelve_digits_is_refused_rather_than_completed_by_the_encoder() {
    let refused = render_label(&product(NAME, TWELVE, PRICE), Lang::Fr).unwrap_err();
    assert_eq!(refused.code(), "validation");
    // Its own field, so the screen can say "not an EAN-13" about a fiche
    // that visibly carries a code, rather than "no barcode".
    match &refused {
        RetailError::Kernel(CoreError::Validation { field, .. }) => {
            assert_eq!(*field, "barcode_digits")
        }
        other => panic!("{other:?}"),
    }

    // The same rule on the sheet, and one more shape a fiche really holds:
    // thirteen characters that are not all digits.
    assert_eq!(
        render_label_sheet(&[product(NAME, TWELVE, PRICE)], Lang::Fr)
            .unwrap_err()
            .code(),
        "validation"
    );
    assert_eq!(
        render_label(&product(NAME, "20000100000A4", PRICE), Lang::Fr)
            .unwrap_err()
            .code(),
        "validation"
    );
    // Fourteen digits is an ITF-14 carton code, not a shelf label.
    assert_eq!(
        render_label(&product(NAME, "20000100000749", PRICE), Lang::Fr)
            .unwrap_err()
            .code(),
        "validation"
    );
}

#[test]
fn the_sheet_lays_the_labels_out_on_a4_and_refuses_when_one_of_them_cannot_be_printed() {
    let mut updated = Vec::new();
    let products = [
        product(NAME, BARCODE, PRICE),
        product(SECOND_NAME, SECOND_BARCODE, SECOND_PRICE),
    ];
    let rendered = render_label_sheet(&products, Lang::Fr).unwrap();
    let file = golden("sheet-fr.html", &rendered, &mut updated);

    assert!(file.contains("size: A4"), "the sheet is not on A4");
    assert_eq!(
        spans(&file, "code"),
        vec![BARCODE.to_owned(), SECOND_BARCODE.to_owned()]
    );
    assert!(file.contains(SECOND_NAME), "the second label is missing");

    let with_a_bad_one = [
        product(NAME, BARCODE, PRICE),
        product("Sans code", "REF-4471", PRICE),
    ];
    let refused = render_label_sheet(&with_a_bad_one, Lang::Fr).unwrap_err();
    assert_eq!(refused.code(), "validation");

    // A sheet of nothing is a blank page nobody asked for.
    assert_eq!(
        render_label_sheet(&[], Lang::Fr).unwrap_err().code(),
        "validation"
    );
    refuse_a_silent_regeneration(&updated);
}
