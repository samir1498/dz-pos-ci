//! The TTC total written out in words. Décret 05-468 requires it "en
//! chiffres et en lettres", so every facture prints `net_to_pay` twice.
//!
//! The golden files `fixtures/money/words_{fr,en,ar}_golden.json` are the
//! spelling contract; this module follows them, byte for byte. French uses
//! the 1990 hyphenation the Journal Officiel uses, with the Académie
//! agreement on `vingt` and `cent`. Arabic is unreviewed until research R6.
//!
//! The module inherits `#![deny(clippy::arithmetic_side_effects)]` from
//! `money`, so the digits come out by division and remainder against
//! literals and every word comes from a `match`, never from index maths.

use super::Money;

/// The print language of a document. It lives in `crate::lang` now that a
/// printed template needs it outside the money module; this re-export is
/// what keeps `money::words::Lang` the name every caller already uses.
pub use crate::lang::Lang;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WordsError {
    /// A facture never prints a negative total; an avoir prints its own.
    #[error("a negative amount has no written form on a document")]
    Negative,
    /// Past `milliard` the scale words differ per language and nobody has
    /// picked them. No real document reaches this.
    #[error("amount is past the milliard scale")]
    TooLarge,
}

/// The largest amount in dinars the three scales cover: 999 999 999 999.
const MAX_DINARS: u64 = 999_999_999_999;

/// `net_to_pay` written out in `lang`, dinars and centimes.
pub fn amount_in_words(amount: Money, lang: Lang) -> Result<String, WordsError> {
    let cents = amount.as_centimes();
    if cents < 0 {
        return Err(WordsError::Negative);
    }
    // Checked non-negative just above, so neither conversion loses a digit.
    let dinars = (cents / 100).unsigned_abs();
    let sub = (cents % 100).unsigned_abs();
    if dinars > MAX_DINARS {
        return Err(WordsError::TooLarge);
    }
    Ok(match lang {
        Lang::Fr => fr(dinars, sub),
        Lang::En => en(dinars, sub),
        Lang::Ar => ar(dinars, sub),
    })
}

/// The four groups of three digits, high to low: milliards, millions,
/// thousands, units.
fn groups(n: u64) -> (u64, u64, u64, u64) {
    (
        n / 1_000_000_000,
        (n / 1_000_000) % 1_000,
        (n / 1_000) % 1_000,
        n % 1_000,
    )
}

// ---------------------------------------------------------------- French

fn fr_unit(u: u64) -> &'static str {
    match u {
        1 => "un",
        2 => "deux",
        3 => "trois",
        4 => "quatre",
        5 => "cinq",
        6 => "six",
        7 => "sept",
        8 => "huit",
        9 => "neuf",
        _ => "",
    }
}

/// The word for `10 + u`, which the seventies and the nineties reuse.
fn fr_ten_plus(u: u64) -> &'static str {
    match u {
        0 => "dix",
        1 => "onze",
        2 => "douze",
        3 => "treize",
        4 => "quatorze",
        5 => "quinze",
        6 => "seize",
        7 => "dix-sept",
        8 => "dix-huit",
        9 => "dix-neuf",
        _ => "",
    }
}

/// `plural_s` is false when a number word follows, which is what stops the
/// `s` of `quatre-vingts` before `mille`.
fn fr_under_100(n: u64, plural_s: bool) -> String {
    let tens = n / 10;
    let unit = n % 10;
    match tens {
        0 => fr_unit(unit).to_owned(),
        1 => fr_ten_plus(unit).to_owned(),
        2..=6 => {
            let base = match tens {
                2 => "vingt",
                3 => "trente",
                4 => "quarante",
                5 => "cinquante",
                _ => "soixante",
            };
            match unit {
                0 => base.to_owned(),
                1 => format!("{base}-et-un"),
                _ => format!("{base}-{}", fr_unit(unit)),
            }
        }
        7 => match unit {
            0 => "soixante-dix".to_owned(),
            1 => "soixante-et-onze".to_owned(),
            _ => format!("soixante-{}", fr_ten_plus(unit)),
        },
        8 => match unit {
            0 if plural_s => "quatre-vingts".to_owned(),
            0 => "quatre-vingt".to_owned(),
            _ => format!("quatre-vingt-{}", fr_unit(unit)),
        },
        _ => format!("quatre-vingt-{}", fr_ten_plus(unit)),
    }
}

fn fr_group(g: u64, plural_s: bool) -> String {
    let hundreds = g / 100;
    let rest = g % 100;
    let head = match hundreds {
        0 => String::new(),
        1 => "cent".to_owned(),
        // `cent` takes the s only when it is multiplied and final.
        _ => {
            let mark = if rest == 0 && plural_s { "s" } else { "" };
            format!("{}-cent{mark}", fr_unit(hundreds))
        }
    };
    if rest == 0 {
        return head;
    }
    let tail = fr_under_100(rest, plural_s);
    if hundreds == 0 {
        tail
    } else {
        format!("{head}-{tail}")
    }
}

/// The number in words, and whether it ends on `million` or `milliard`.
/// Those two are nouns, so what they count takes `de`: `deux millions de
/// dinars`, against `deux millions deux-cents dinars`.
fn fr_number(n: u64) -> (String, bool) {
    if n == 0 {
        return ("zéro".to_owned(), false);
    }
    let (milliards, millions, thousands, units) = groups(n);
    let mut parts: Vec<String> = Vec::new();
    if milliards > 0 {
        let mark = if milliards > 1 { "s" } else { "" };
        parts.push(format!("{} milliard{mark}", fr_group(milliards, true)));
    }
    if millions > 0 {
        let mark = if millions > 1 { "s" } else { "" };
        parts.push(format!("{} million{mark}", fr_group(millions, true)));
    }
    // `mille` is invariable and hyphenates onto the units group.
    let mut tail: Vec<String> = Vec::new();
    match thousands {
        0 => {}
        1 => tail.push("mille".to_owned()),
        _ => tail.push(format!("{}-mille", fr_group(thousands, false))),
    }
    if units > 0 {
        tail.push(fr_group(units, true));
    }
    let tail = tail.join("-");
    let ends_on_a_noun = tail.is_empty();
    if !tail.is_empty() {
        parts.push(tail);
    }
    (parts.join(" "), ends_on_a_noun)
}

fn fr(dinars: u64, sub: u64) -> String {
    let (number, ends_on_a_noun) = fr_number(dinars);
    let unit = if dinars <= 1 { "dinar" } else { "dinars" };
    let liaison = if ends_on_a_noun { "de " } else { "" };
    let mut out = format!("{number} {liaison}{unit}");
    if sub > 0 {
        let centime = if sub == 1 { "centime" } else { "centimes" };
        out.push(' ');
        out.push_str(&format!("et {} {centime}", fr_under_100(sub, true)));
    }
    out
}

// --------------------------------------------------------------- English

fn en_under_20(n: u64) -> &'static str {
    match n {
        1 => "one",
        2 => "two",
        3 => "three",
        4 => "four",
        5 => "five",
        6 => "six",
        7 => "seven",
        8 => "eight",
        9 => "nine",
        10 => "ten",
        11 => "eleven",
        12 => "twelve",
        13 => "thirteen",
        14 => "fourteen",
        15 => "fifteen",
        16 => "sixteen",
        17 => "seventeen",
        18 => "eighteen",
        19 => "nineteen",
        _ => "",
    }
}

fn en_under_100(n: u64) -> String {
    if n < 20 {
        return en_under_20(n).to_owned();
    }
    let base = match n / 10 {
        2 => "twenty",
        3 => "thirty",
        4 => "forty",
        5 => "fifty",
        6 => "sixty",
        7 => "seventy",
        8 => "eighty",
        _ => "ninety",
    };
    match n % 10 {
        0 => base.to_owned(),
        unit => format!("{base}-{}", en_under_20(unit)),
    }
}

fn en_group(g: u64) -> String {
    let hundreds = g / 100;
    let rest = g % 100;
    match (hundreds, rest) {
        (0, _) => en_under_100(rest),
        (_, 0) => format!("{} hundred", en_under_20(hundreds)),
        _ => format!(
            "{} hundred and {}",
            en_under_20(hundreds),
            en_under_100(rest)
        ),
    }
}

fn en_number(n: u64) -> String {
    if n == 0 {
        return "zero".to_owned();
    }
    let (billions, millions, thousands, units) = groups(n);
    let mut parts: Vec<String> = Vec::new();
    if billions > 0 {
        parts.push(format!("{} billion", en_group(billions)));
    }
    if millions > 0 {
        parts.push(format!("{} million", en_group(millions)));
    }
    if thousands > 0 {
        parts.push(format!("{} thousand", en_group(thousands)));
    }
    if units > 0 {
        // British usage: "one thousand and one", "one thousand two hundred".
        if !parts.is_empty() && units < 100 {
            parts.push(format!("and {}", en_under_100(units)));
        } else {
            parts.push(en_group(units));
        }
    }
    parts.join(" ")
}

fn en(dinars: u64, sub: u64) -> String {
    let unit = if dinars == 1 { "dinar" } else { "dinars" };
    let mut out = format!("{} {unit}", en_number(dinars));
    if sub > 0 {
        let centime = if sub == 1 { "centime" } else { "centimes" };
        out.push(' ');
        out.push_str(&format!("and {} {centime}", en_under_100(sub)));
    }
    out
}

// ---------------------------------------------------------------- Arabic

/// A scale noun in the forms the counted-noun rules need. `dual_construct`
/// is the dual with the nun dropped, which is what a scale noun takes when
/// the word it counts follows it directly.
struct ArScale {
    one: &'static str,
    dual: &'static str,
    dual_construct: &'static str,
    few: &'static str,
    accusative: &'static str,
}

const AR_THOUSAND: ArScale = ArScale {
    one: "ألف",
    dual: "ألفان",
    dual_construct: "ألفا",
    few: "آلاف",
    accusative: "ألفًا",
};

const AR_MILLION: ArScale = ArScale {
    one: "مليون",
    dual: "مليونان",
    dual_construct: "مليونا",
    few: "ملايين",
    accusative: "مليونًا",
};

const AR_MILLIARD: ArScale = ArScale {
    one: "مليار",
    dual: "ملياران",
    dual_construct: "مليارا",
    few: "مليارات",
    accusative: "مليارًا",
};

fn ar_under_20(n: u64) -> &'static str {
    match n {
        1 => "واحد",
        2 => "اثنان",
        3 => "ثلاثة",
        4 => "أربعة",
        5 => "خمسة",
        6 => "ستة",
        7 => "سبعة",
        8 => "ثمانية",
        9 => "تسعة",
        10 => "عشرة",
        11 => "أحد عشر",
        12 => "اثنا عشر",
        13 => "ثلاثة عشر",
        14 => "أربعة عشر",
        15 => "خمسة عشر",
        16 => "ستة عشر",
        17 => "سبعة عشر",
        18 => "ثمانية عشر",
        19 => "تسعة عشر",
        _ => "",
    }
}

fn ar_under_100(n: u64) -> String {
    if n < 20 {
        return ar_under_20(n).to_owned();
    }
    let base = match n / 10 {
        2 => "عشرون",
        3 => "ثلاثون",
        4 => "أربعون",
        5 => "خمسون",
        6 => "ستون",
        7 => "سبعون",
        8 => "ثمانون",
        _ => "تسعون",
    };
    match n % 10 {
        0 => base.to_owned(),
        // Arabic reads the unit before the ten: "واحد وعشرون".
        unit => format!("{} و{base}", ar_under_20(unit)),
    }
}

/// `construct` drops the nun of the dual `مائتان` when the counted word
/// follows it directly: `مائتا دينار جزائري`.
fn ar_hundred(h: u64, construct: bool) -> &'static str {
    match h {
        1 => "مائة",
        2 if construct => "مائتا",
        2 => "مائتان",
        3 => "ثلاثمائة",
        4 => "أربعمائة",
        5 => "خمسمائة",
        6 => "ستمائة",
        7 => "سبعمائة",
        8 => "ثمانمائة",
        9 => "تسعمائة",
        _ => "",
    }
}

fn ar_group(g: u64, construct: bool) -> String {
    let hundreds = g / 100;
    let rest = g % 100;
    if hundreds == 0 {
        return ar_under_100(rest);
    }
    let head = ar_hundred(hundreds, construct && rest == 0);
    if rest == 0 {
        head.to_owned()
    } else {
        format!("{head} و{}", ar_under_100(rest))
    }
}

/// `last` means nothing follows this group, so the scale noun is in
/// construct with the currency and loses its tanwin.
fn ar_scale(count: u64, scale: &ArScale, last: bool) -> String {
    match count {
        1 => scale.one.to_owned(),
        2 if last => scale.dual_construct.to_owned(),
        2 => scale.dual.to_owned(),
        _ => {
            // The counted-noun form follows the last two digits.
            let form = match count % 100 {
                3..=10 => scale.few,
                11..=99 if last => scale.one,
                11..=99 => scale.accusative,
                _ => scale.one,
            };
            format!("{} {form}", ar_group(count, true))
        }
    }
}

fn ar_number(n: u64) -> String {
    if n == 0 {
        return "صفر".to_owned();
    }
    let (milliards, millions, thousands, units) = groups(n);
    let mut parts: Vec<String> = Vec::new();
    if milliards > 0 {
        let last = millions == 0 && thousands == 0 && units == 0;
        parts.push(ar_scale(milliards, &AR_MILLIARD, last));
    }
    if millions > 0 {
        let last = thousands == 0 && units == 0;
        parts.push(ar_scale(millions, &AR_MILLION, last));
    }
    if thousands > 0 {
        parts.push(ar_scale(thousands, &AR_THOUSAND, units == 0));
    }
    if units > 0 {
        parts.push(ar_group(units, true));
    }
    parts.join(" و")
}

/// The dinar in the form the count requires: dual for two, plural genitive
/// for three to ten, singular accusative for eleven to ninety-nine,
/// singular genitive after a round hundred or thousand.
fn ar_dinar(n: u64) -> &'static str {
    match n % 100 {
        3..=10 => "دنانير جزائرية",
        11..=99 => "دينارًا جزائريًا",
        _ => "دينار جزائري",
    }
}

fn ar_centime(n: u64) -> &'static str {
    match n % 100 {
        3..=10 => "سنتيمات",
        11..=99 => "سنتيمًا",
        _ => "سنتيم",
    }
}

fn ar(dinars: u64, sub: u64) -> String {
    // One and two take the noun itself, not a numeral before it.
    let mut out = match dinars {
        1 => "دينار جزائري واحد".to_owned(),
        2 => "ديناران جزائريان".to_owned(),
        _ => format!("{} {}", ar_number(dinars), ar_dinar(dinars)),
    };
    if sub > 0 {
        let piece = match sub {
            1 => "سنتيم واحد".to_owned(),
            2 => "سنتيمان".to_owned(),
            _ => format!("{} {}", ar_under_100(sub), ar_centime(sub)),
        };
        out.push(' ');
        out.push_str(&format!("و{piece}"));
    }
    out
}
