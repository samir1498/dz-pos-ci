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
/// One space joins the two sides it sits between; two or more do not. A
/// padded row's gap has to stay neutral, both because it is where the row
/// is justified and because the amount column only stays a column while
/// the amount is a separate object from the label.
pub(super) fn isolate_ltr_runs(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let member_at = |index: usize| chars.get(index).is_some_and(|ch| is_ltr_member(*ch));
    let inside = |index: usize| match chars.get(index) {
        Some(&ch) if is_ltr_member(ch) => true,
        Some(' ') => {
            index.checked_sub(1).is_some_and(member_at) && member_at(index.saturating_add(1))
        }
        _ => false,
    };
    let mut out = String::with_capacity(text.len());
    let mut open = false;
    for (index, ch) in chars.iter().enumerate() {
        let now = inside(index);
        if now && !open {
            out.push(LRI);
        } else if !now && open {
            out.push(PDI);
        }
        open = now;
        out.push(*ch);
    }
    if open {
        out.push(PDI);
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
mod tests {
    use super::{gap_mark, is_arabic, isolate_ltr_runs, script_runs, Piece, LRI, PDI};

    #[test]
    fn the_slack_of_a_padded_row_goes_between_its_two_halves() {
        // The widest run of spaces, not the first: a product name with one
        // space in it keeps that space.
        // Six Arabic letters at two bytes each, a space, five more, then
        // the three-space gap: its last space is byte 25.
        assert_eq!(gap_mark("الصافي للدفع   937,84"), Some(25));
        assert_eq!(gap_mark("Café moulu 250 g"), None);
        assert_eq!(gap_mark("------"), None);
        assert_eq!(gap_mark("a  b   c"), Some(6));
    }

    #[test]
    fn a_run_is_split_where_the_script_changes() {
        // The full stops inside the TVA label are ASCII, not Arabic, so
        // the line splits finer than an eye counts it: three Arabic
        // letters, two periods, and the digits and sign at the end. That
        // costs nothing — a letter beside a full stop does not join to it
        // anyway — and what the split is for is the face: the periods, the
        // digits and the `%` all go to Plex Mono, which has them, and
        // Naskh, which does not, is never asked.
        let runs = script_runs("ت.ق.م 19 %");
        assert_eq!(runs.len(), 6, "{runs:?}");
        let arabic = |kind: &Piece| *kind == Piece::Arabic;
        assert_eq!(runs.iter().filter(|(_, kind)| arabic(kind)).count(), 3);
        assert!(runs.first().is_some_and(|(_, kind)| arabic(kind)));
        assert!(runs.last().is_some_and(|(_, kind)| !arabic(kind)));
        assert!(is_arabic('ت') && !is_arabic('9') && !is_arabic(' '));
    }

    #[test]
    fn an_isolate_holds_a_latin_run_together_but_leaves_the_padding_alone() {
        assert_eq!(isolate_ltr_runs("-10,00"), format!("{LRI}-10,00{PDI}"));
        assert_eq!(
            isolate_ltr_runs("0555 12 34 56"),
            format!("{LRI}0555 12 34 56{PDI}")
        );
        // Two spaces are a gap, not a join: the amount stays an object of
        // its own, which is what keeps it in a column.
        assert_eq!(
            isolate_ltr_runs("19 %  300,00"),
            format!("{LRI}19 %{PDI}  {LRI}300,00{PDI}")
        );
        // Arabic is left as it is, and a line of it gains nothing.
        assert_eq!(isolate_ltr_runs("تخفيض"), "تخفيض");
        assert_eq!(
            isolate_ltr_runs("تخفيض  -10,00"),
            format!("تخفيض  {LRI}-10,00{PDI}")
        );
    }
}
