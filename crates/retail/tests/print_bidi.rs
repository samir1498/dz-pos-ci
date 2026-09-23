// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! What an Arabic page does to the values printed on it.
//!
//! A printed document is one paragraph after another, and on an Arabic page
//! every one of them is laid out right to left. That is right for the words
//! and wrong for everything the shop did not write in Arabic: a phone number,
//! a registre de commerce with a space in it, a date beside a time, a period
//! from one day to another, an amount carrying a minus sign, a quantity
//! against a unit price. Each of those is one run that reads left to right,
//! and the spaces and slashes and minus signs inside it belong to that run.
//! Left to the page, the run comes apart and its pieces lay out backwards:
//! `0770 11 22 33` prints `33 22 11 0770`, which is a phone nobody can dial.
//!
//! The templates answer that in two ways, and this file checks both. A value
//! sits in a span whose class the stylesheet isolates and points left to
//! right. Text the shop typed sits in `bdi`, which isolates it too but takes
//! its direction from what is actually in it, because a product name or an
//! address can be in either script.
//!
//! The algorithm here is the one a browser runs (UAX 9), from the crate
//! rust-url uses. Reading the markup alone would only say the rule is
//! written; running the algorithm says the value survives the page.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use unicode_bidi::{BidiInfo, Level};

/// The classes the stylesheets point left to right. A span carrying one of
/// them holds a value, never a sentence.
const ISOLATING: [&str; 5] = ["num", "amount", "qty", "rate", "code"];

/// Every golden page, as `(name, html)`. The bytes on disk and not a render:
/// what is checked is the file a reviewer reads in a diff.
fn goldens() -> Vec<(String, String)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/print");
    let mut pages = Vec::new();
    for dir in fs::read_dir(&root).unwrap() {
        let dir = dir.unwrap().path();
        if !dir.is_dir() {
            continue;
        }
        for file in fs::read_dir(&dir).unwrap() {
            let file = file.unwrap().path();
            if file.extension().and_then(|e| e.to_str()) == Some("html") {
                let name = format!(
                    "{}/{}",
                    dir.file_name().unwrap().to_string_lossy(),
                    file.file_name().unwrap().to_string_lossy()
                );
                pages.push((name, fs::read_to_string(&file).unwrap()));
            }
        }
    }
    assert!(pages.len() > 30, "only {} goldens found", pages.len());
    pages.sort();
    pages
}

/// A page read on an Arabic machine.
fn is_arabic(name: &str) -> bool {
    let file = name.split('/').next_back().unwrap_or(name);
    file.starts_with("ar")
}

/// One piece of text the page prints, and the thing that isolates it: the
/// class of the nearest span the stylesheet points left to right, or `bdi`,
/// or nothing at all.
struct Printed {
    text: String,
    isolated_by: Option<String>,
}

/// Every piece of text in the body, with what stands around it. The markup
/// these templates emit is the markup this reads: one tag per angle bracket,
/// no attribute holding a `>`, and the bars of a barcode closed on
/// themselves. A page that stopped being that simple would fail here rather
/// than be read wrongly, which is the point of the assertions inside.
fn printed(html: &str) -> Vec<Printed> {
    let start = html.find("<body>").expect("a page with no body") + "<body>".len();
    let end = html.find("</body>").expect("a body that never closes");
    let body = &html[start..end];

    let mut out = Vec::new();
    let mut stack: Vec<(String, Vec<String>)> = Vec::new();
    let mut rest = body;
    while let Some(open) = rest.find('<') {
        let (text, tail) = rest.split_at(open);
        if !text.trim().is_empty() && !stack.iter().any(|(tag, _)| tag == "svg") {
            let isolated_by = stack.iter().rev().find_map(|(tag, classes)| {
                if tag == "bdi" {
                    return Some("bdi".to_owned());
                }
                classes
                    .iter()
                    .find(|c| ISOLATING.contains(&c.as_str()))
                    .cloned()
            });
            out.push(Printed {
                text: unescape(text),
                isolated_by,
            });
        }
        let close = tail.find('>').expect("a tag that never closes");
        let inner = &tail[1..close];
        rest = &tail[close + 1..];
        if let Some(name) = inner.strip_prefix('/') {
            let (tag, _) = stack.pop().expect("a closing tag with nothing open");
            assert_eq!(tag, name, "the page closes a tag it did not open");
        } else if !inner.ends_with('/') && !inner.starts_with('!') {
            let tag = inner
                .split([' ', '\n'])
                .next()
                .expect("a tag with no name")
                .to_owned();
            let classes = class_list(inner);
            stack.push((tag, classes));
        }
    }
    assert!(stack.is_empty(), "the body ends with a tag still open");
    out
}

/// The classes on one opening tag.
fn class_list(inner: &str) -> Vec<String> {
    let Some(at) = inner.find("class=\"") else {
        return Vec::new();
    };
    let rest = &inner[at + "class=\"".len()..];
    let end = rest.find('"').expect("a class attribute that never closes");
    rest[..end].split(' ').map(str::to_owned).collect()
}

/// The three entities these pages escape, back to the characters a reader
/// sees. The algorithm reads characters, and `&#60;` is five of them where
/// the page prints one.
fn unescape(text: &str) -> String {
    text.replace("&#60;", "<")
        .replace("&#62;", ">")
        .replace("&#38;", "&")
}

/// The text in the order it is laid out on one line at the given direction.
/// `None` is what `bdi` does: the direction comes from the first strong
/// character in the text itself.
fn laid_out(text: &str, base: Option<Level>) -> String {
    let info = BidiInfo::new(text, base);
    info.paragraphs
        .iter()
        .map(|para| info.reorder_line(para, para.range.clone()).into_owned())
        .collect()
}

/// Whether the page's own stylesheet points a class left to right and cuts
/// it out of the line around it.
fn stylesheet_isolates(html: &str, class: &str) -> bool {
    let start = html.find("<style>").expect("a page with no stylesheet");
    let end = html
        .find("</style>")
        .expect("a stylesheet that never closes");
    html[start..end].lines().any(|line| {
        line.contains("direction: ltr")
            && line.contains("unicode-bidi: isolate")
            && line.split('{').next().is_some_and(|selectors| {
                selectors
                    .split(',')
                    .any(|s| s.trim() == format!(".{class}"))
            })
    })
}

/// The guard that catches the next value somebody prints without isolating
/// it. What comes apart on an Arabic line is a run of digits with neutral
/// characters in it, so no text outside an isolating element may carry a
/// digit: the words of the page are Arabic and the labels beside them
/// (`RC`, `NIF`) are letters, which survive a right-to-left line whole.
///
/// It does not catch every way a line can read wrongly, only the way every
/// defect found on these pages read wrongly.
#[test]
fn no_value_on_an_arabic_page_is_left_to_the_line_around_it() {
    let mut loose: Vec<String> = Vec::new();
    for (name, html) in goldens() {
        if !is_arabic(&name) {
            continue;
        }
        for piece in printed(&html) {
            if piece.isolated_by.is_none() && piece.text.chars().any(|c| c.is_ascii_digit()) {
                loose.push(format!("{name}: {}", piece.text.trim()));
            }
        }
    }
    assert!(
        loose.is_empty(),
        "text carrying digits sits on an Arabic line with nothing isolating it: {loose:#?}"
    );
}

/// Every class a page isolates with is a class that page's own stylesheet
/// declares. A span that carries `qty` on a template whose rule names only
/// `num` and `amount` is markup that looks isolated and is not.
#[test]
fn every_page_declares_the_rule_for_the_classes_it_isolates_with() {
    let mut missing: Vec<String> = Vec::new();
    for (name, html) in goldens() {
        let used: BTreeSet<String> = printed(&html)
            .into_iter()
            .filter_map(|p| p.isolated_by)
            .filter(|c| c != "bdi")
            .collect();
        assert!(!used.is_empty(), "{name} isolates nothing at all");
        for class in used {
            if !stylesheet_isolates(&html, &class) {
                missing.push(format!("{name}: .{class}"));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "a class is used to isolate a value and the page's stylesheet says nothing about it: {missing:#?}"
    );
}

/// An isolated value reads in the order it was written. The span is pointed
/// left to right, so this is the algorithm agreeing that left to right is
/// enough: a value that still came apart inside its own isolate would need
/// more than a rule.
#[test]
fn an_isolated_value_reads_in_the_order_it_was_written() {
    for (name, html) in goldens() {
        for piece in printed(&html) {
            let Some(by) = &piece.isolated_by else {
                continue;
            };
            if by == "bdi" {
                // `bdi` reads its own direction off the text. Where that is
                // right to left the text is Arabic and laying it out
                // backwards is what reading it means, so there is nothing to
                // compare it against.
                let info = BidiInfo::new(&piece.text, None);
                if info.paragraphs.iter().any(|p| p.level.is_rtl()) {
                    continue;
                }
                assert_eq!(
                    laid_out(&piece.text, None),
                    piece.text,
                    "{name}: bdi does not hold {:?} together",
                    piece.text
                );
            } else {
                assert_eq!(
                    laid_out(&piece.text, Some(Level::ltr())),
                    piece.text,
                    "{name}: .{by} does not hold {:?} together",
                    piece.text
                );
            }
        }
    }
}

/// The other half: without the isolation these values come apart. Each one
/// is read out of the goldens rather than typed here, because the thousands
/// separator is U+202F and a value retyped in a shell is a different string
/// that proves nothing.
///
/// Laid out on a right-to-left line, every one of these prints in an order
/// no reader can use: a phone backwards, a period from its last day to its
/// first, a minus sign on the far side of its amount.
#[test]
fn the_isolation_is_what_keeps_them_whole() {
    let breaks = [
        "0770 11 22 33",
        "16/00-7654321 B 20",
        "000216001234567 00",
        "13/09/2026 17:30",
        "01/09/2026 - 30/09/2026",
        "12 rue Didouche Mourad, Alger",
        "-20,00",
        "19\u{202f}%",
        "2 × 150,00",
    ];
    let pages = goldens();
    let printed_anywhere: BTreeSet<String> = pages
        .iter()
        .filter(|(name, _)| is_arabic(name))
        .flat_map(|(_, html)| printed(html))
        .filter(|p| p.isolated_by.is_some())
        .map(|p| p.text)
        .collect();

    for value in breaks {
        assert!(
            printed_anywhere.contains(value),
            "{value:?} is not a value any Arabic golden isolates; the list here has drifted from the pages"
        );
        assert_ne!(
            laid_out(value, Some(Level::rtl())),
            value,
            "{value:?} survives a right-to-left line on its own, so isolating it proves nothing"
        );
    }
}
