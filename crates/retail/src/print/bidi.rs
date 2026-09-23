//! Which way a ticket line runs, and which face draws each stretch of it.
//!
//! Split out of `raster.rs` when the isolate fix landed and that file
//! went past the 600-line limit: drawing dots and deciding the order of the
//! things to draw are two jobs, and this is the second. Nothing here knows
//! about pixels, fonts or amounts — it reads a line as characters and says
//! what belongs with what.
//!
//! The hard part is that an Arabic ticket is not Arabic. It carries
//! Western digits, a `%`, a `×`, a date and a phone number, and the
//! Unicode bidi algorithm treats a European number as right-to-left when
//! it decides who a neighbouring space or sign belongs to. Left alone it
//! prints `10,00-` for a discount and `% 19` for a rate. The answer is to
//! hand it objects instead of loose characters.

/// What a stretch of a line is drawn with, or not drawn at all.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Piece {
    Arabic,
    Latin,
    /// An isolate mark. It exists for the bidi algorithm and has no shape:
    /// it never reaches the shaper, so it cannot be a box and cannot move
    /// the pen.
    Mark,
}

/// Split a run into stretches of one script. Arabic goes to Naskh and
/// everything else to Plex Mono, which is what puts the spaces of a padded
/// line on the monospaced grid the padding was counted in.
pub(super) fn script_runs(text: &str) -> Vec<(std::ops::Range<usize>, Piece)> {
    let mut out: Vec<(std::ops::Range<usize>, Piece)> = Vec::new();
    for (index, ch) in text.char_indices() {
        let piece = if is_isolate(ch) {
            Piece::Mark
        } else if is_arabic(ch) {
            Piece::Arabic
        } else {
            Piece::Latin
        };
        let end = index.saturating_add(ch.len_utf8());
        match out.last_mut() {
            Some((range, was)) if *was == piece => range.end = end,
            _ => out.push((index..end, piece)),
        }
    }
    out
}

/// The two invisible marks that hold a left-to-right stretch together
/// inside a right-to-left line: `LRI` opens the isolate, `PDI` closes it.
const LRI: char = '\u{2066}';
const PDI: char = '\u{2069}';

const fn is_isolate(ch: char) -> bool {
    matches!(ch, LRI | PDI)
}

/// A character that belongs inside one left-to-right object: a letter of
/// any script but Arabic, a digit, and the punctuation that lives between
/// them on a ticket — the minus of a discount, the `%` of a rate, the `×`
/// of a quantity, the `/` and `:` of a date and a time, the decimal comma,
/// the full stop, and the narrow space that groups thousands.
fn is_ltr_member(ch: char) -> bool {
    !is_arabic(ch)
        && (ch.is_alphanumeric()
            || matches!(
                ch,
                '-' | '+' | '%' | '×' | '/' | ':' | ',' | '.' | '\u{202f}'
            ))
}

/// A character that can sit inside a left-to-right object without being a
/// reason for one: anything that is neither Arabic nor a space.
///
/// This is the punctuation a shop types into a product name or an address —
/// `<`, `>`, `&`, brackets, an apostrophe, a quote — none of which
/// `is_ltr_member` names, and none of which should be allowed to cut a name
/// into pieces. It is the same reading the HTML templates get from `<bdi>`,
/// which takes its direction from what is inside it and does not care where
/// the ampersands fall.
fn is_ltr_joiner(ch: char) -> bool {
    !is_arabic(ch) && ch != ' ' && !is_isolate(ch)
}

/// Wrap each left-to-right stretch of an Arabic line in an isolate.
///
/// Without this the bidi algorithm is right and the paper is wrong. The
/// algorithm treats a European number as right-to-left when it decides
/// what a neighbouring space or sign belongs to, so `-10,00` prints as
/// `10,00-`, `19 %` prints as `% 19`, a phone number comes out in reversed
/// groups and a date swaps with the time beside it. None of that is a bug
/// in the algorithm: it is what happens when a run of Latin is handed to
/// it as loose characters instead of as one object. `LRI … PDI` says it is
/// one object, which is the same fix the HTML ticket took on 2026-09-12,
/// and the marks themselves are dropped before anything is drawn.
///
/// A stretch runs from its first left-to-right member to its last, over any
/// punctuation and any single space between them. It had to grow that far
/// when the facture came down this wire on 2026-09-21: the facture fixture
/// carries `Huile <Elio> & Co 5 L`, and with only `is_ltr_member` joining,
/// the brackets and the ampersand were neutral, so the name became four
/// objects and the Arabic paragraph laid them out right to left as
/// `Co 5 L & <Elio> Huile`. The amounts were right and the product was
/// printed backwards, which no test caught and the first picture did
/// (`a_product_name_with_punctuation_in_it_stays_one_object`).
///
/// One space joins the two sides it sits between; two or more do not. A
/// padded row's gap has to stay neutral, both because it is where the row
/// is justified and because the amount column only stays a column while
/// the amount is a separate object from the label. That is the rule that
/// keeps this from swallowing a whole line: `تخفيض  -10,00` is padded with
/// two spaces, so the label and the figure stay two objects.
///
/// A stretch with no member in it at all — the two full stops of `ت.ق.م`,
/// which are punctuation between Arabic letters — is left neutral. It is
/// not a left-to-right object; it only looks like one from here.
pub(super) fn isolate_ltr_runs(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let member_at = |index: usize| chars.get(index).is_some_and(|ch| is_ltr_member(*ch));
    let joiner_at = |index: usize| chars.get(index).is_some_and(|ch| is_ltr_joiner(*ch));
    // A space is part of the object around it only when it has a joinable
    // character on each side. A second space fails that on both counts,
    // which is what keeps a padded gap neutral.
    let inside = |index: usize| match chars.get(index) {
        Some(&ch) if is_ltr_joiner(ch) => true,
        Some(' ') => {
            index.checked_sub(1).is_some_and(joiner_at) && joiner_at(index.saturating_add(1))
        }
        _ => false,
    };
    let push = |out: &mut String, from: usize, to: usize| {
        for index in from..to {
            if let Some(ch) = chars.get(index) {
                out.push(*ch);
            }
        }
    };
    let mut out = String::with_capacity(text.len());
    let mut at = 0usize;
    while at < chars.len() {
        if !inside(at) {
            push(&mut out, at, at.saturating_add(1));
            at = at.saturating_add(1);
            continue;
        }
        let mut end = at;
        while end < chars.len() && inside(end) {
            end = end.saturating_add(1);
        }
        // The isolate opens at the first member and closes after the last:
        // punctuation hanging off either end of the stretch belongs to the
        // Arabic around it, the way a full stop ending an Arabic sentence
        // does.
        let first = (at..end).find(|index| member_at(*index));
        let last = (at..end).rfind(|index| member_at(*index));
        match (first, last) {
            (Some(first), Some(last)) => {
                push(&mut out, at, first);
                out.push(LRI);
                push(&mut out, first, last.saturating_add(1));
                out.push(PDI);
                push(&mut out, last.saturating_add(1), end);
            }
            _ => push(&mut out, at, end),
        }
        at = end;
    }
    out
}

/// The Arabic blocks, including the presentation forms a stored string may
/// already carry. Western digits are not in them, which is the point: an
/// Algerian receipt prints `0-9` and those come from the Latin face.
const fn is_arabic(ch: char) -> bool {
    matches!(ch as u32,
        0x0600..=0x06FF | 0x0750..=0x077F | 0x0870..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF)
}

/// The byte index of the last space of the widest run of two or more
/// spaces: where a padded line's slack goes. Two or more, because a single
/// space is a word break inside a name and stretching it would read as a
/// gap that is not there.
pub(super) fn gap_mark(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let (mut best, mut best_len) = (None, 1usize);
    let mut start: Option<usize> = None;
    for index in 0..=bytes.len() {
        match bytes.get(index) {
            Some(b' ') => start = start.or(Some(index)),
            _ => {
                if let Some(from) = start.take() {
                    let len = index.saturating_sub(from);
                    if len > best_len {
                        best_len = len;
                        best = index.checked_sub(1);
                    }
                }
            }
        }
    }
    best
}

#[cfg(test)]
#[path = "../../tests/unit/print_bidi.rs"]
mod tests;
