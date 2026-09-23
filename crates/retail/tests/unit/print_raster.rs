use ab_glyph::Font as _;

use super::{run, Fonts, Metrics, HEAD_WIDTH_DOTS, WIDTH};

/// The five strings the first Arabic goldens got wrong, each one read
/// off the picture before this test was written: the minus printed
/// after the amount, the `%` before the rate, the phone in reversed
/// groups, the time before its date, the street number after the
/// street. Every one of them is a Latin run that the bidi algorithm
/// took apart because a number counts as right-to-left when it decides
/// what a space belongs to.
///
/// This is the test that can go red for that, which the line-for-line
/// one cannot: `DrawnLine::text` is copied from the input before a
/// glyph is chosen, so nothing about order or shaping can move it.
/// Here the glyphs are sorted by where they landed.
#[test]
fn a_latin_run_inside_an_arabic_line_is_drawn_in_its_own_order() {
    let Ok(fonts) = Fonts::load() else {
        panic!("the vendored ticket fonts are unreadable");
    };
    let metrics = Metrics::new(&fonts, HEAD_WIDTH_DOTS as usize / WIDTH);
    for (line, left, right) in [
        ("-10,00", '-', '1'),
        ("19 %", '1', '%'),
        ("0555 12 34 56", '0', '5'),
        ("09/09/2026 14:05", '0', '1'),
        ("12 rue Didouche Mourad, Alger", '1', 'r'),
    ] {
        let (glyphs, _, notdef) = run(line, true, &fonts, &metrics, 0.0);
        assert_eq!(notdef, 0, "{line}: something was drawn as a box");
        let leftmost = |ch: char| {
            let id = fonts.latin_px.glyph_id(ch).0;
            glyphs
                .iter()
                .filter(|glyph| glyph.id == id)
                .map(|glyph| glyph.x)
                .fold(None, |best: Option<f32>, x| {
                    Some(best.map_or(x, |best| best.min(x)))
                })
        };
        let (Some(first), Some(second)) = (leftmost(left), leftmost(right)) else {
            panic!("{line}: {left} or {right} was never drawn");
        };
        assert!(
            first < second,
            "{line}: {left} landed at {first} dots, right of {right} at {second}"
        );
    }
}
