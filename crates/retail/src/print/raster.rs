//! The ticket drawn into dots, for a head that cannot spell Arabic.
//!
//! A cheap 80 mm head selects a single-byte table and prints one byte per
//! column. No such table has Arabic in it, and even if one did, Arabic
//! letters change shape with their neighbours and run right to left, which
//! a text-mode head does not do. So the Arabic ticket is not sent as text
//! at all: it is drawn here and sent as a bitmap
//! (`context/plans/20260921-arabic-on-a-cheap-thermal-head.md`).
//!
//! **Nothing in this file decides what the ticket says.** It is handed
//! `ticket::Item`s — the same list `escpos::encode` turns into text bytes,
//! with every amount already formatted — and it draws those strings. That
//! is the one rule the money tests here pin
//! (`the_raster_draws_the_lines_the_text_path_prints`): if an amount were
//! formatted in this file, the two papers could disagree about a total, so
//! this file is not allowed to know what an amount is.
//!
//! Three borrowed pieces do the work nobody should write twice:
//! `unicode-bidi` decides which runs of a line go right to left,
//! `rustybuzz` decides which glyph each letter takes beside its
//! neighbours, and `ab_glyph` turns the chosen outlines into pixels. The
//! two faces are vendored under `crates/retail/fonts/` (see the README
//! there) and compiled in, so no machine's font list can change the paper.

use std::borrow::Cow;

use ab_glyph::{point, Font as _, FontRef, GlyphId, PxScale};
use rustybuzz::{script, Direction, Face, UnicodeBuffer};
use unicode_bidi::{Level, ParagraphBidiInfo};

use crate::error::RetailError;
use crate::lang::Lang;
use crate::print::bidi::{gap_mark, isolate_ltr_runs, script_runs, Piece};
use crate::print::ticket::{Align, Item, WIDTH};

/// The common 80 mm head prints 576 dots across (8 dots to the millimetre
/// over 72 mm of paper). A 58 mm head is 384, which is the same code with
/// one argument changed — ruling 8 of the closing-gaps loop, which also
/// says the work does not wait on the preference that will carry it.
pub const HEAD_WIDTH_DOTS: u32 = 576;

/// How much of a pixel a glyph has to cover before the head burns a dot.
/// There is no grey on thermal paper: every dot is on or off, and half
/// coverage is where a stroke's edge looks right at this size. Lower and
/// Naskh's thin joins smear; higher and they break.
const INK: f32 = 0.5;

const NASKH: &[u8] = include_bytes!("../../fonts/NotoNaskhArabic-Regular.ttf");
const PLEX: &[u8] = include_bytes!("../../fonts/IBMPlexMono-Regular.ttf");

/// One bit per dot, packed the way `GS v 0` eats it: `width / 8` bytes to a
/// row, the leftmost dot in the high bit, a set bit meaning the head burns.
pub struct Bitmap {
    width: usize,
    height: usize,
    rows: Vec<u8>,
}

impl Bitmap {
    pub(super) fn new(width: usize, height: usize) -> Self {
        let stride = width.div_ceil(8);
        Self {
            width,
            height,
            rows: vec![0; stride.saturating_mul(height)],
        }
    }

    pub const fn width(&self) -> usize {
        self.width
    }

    pub const fn height(&self) -> usize {
        self.height
    }

    pub const fn stride(&self) -> usize {
        self.width.div_ceil(8)
    }

    pub fn rows(&self) -> &[u8] {
        &self.rows
    }

    /// Whether the head burns the dot at `(x, y)`. Out of the paper is
    /// white, which is what the tests want to ask about a margin.
    pub fn is_black(&self, x: usize, y: usize) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let index = y.saturating_mul(self.stride()) + x / 8;
        let mask = 0x80u8 >> (x % 8);
        self.rows.get(index).is_some_and(|byte| byte & mask != 0)
    }

    pub(super) fn set(&mut self, x: i32, y: i32) {
        let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y)) else {
            return;
        };
        if x >= self.width || y >= self.height {
            return;
        }
        let index = y.saturating_mul(self.stride()) + x / 8;
        let mask = 0x80u8 >> (x % 8);
        if let Some(byte) = self.rows.get_mut(index) {
            *byte |= mask;
        }
    }
}

/// A line as it was actually drawn: the string it came from, how many dots
/// it took, and how many of its characters the fonts had no glyph for.
pub struct DrawnLine {
    pub text: String,
    pub advance: u32,
    /// Glyph 0, `.notdef`, the box. Counted rather than refused: a
    /// customer's product name may carry a character no vendored face has,
    /// and a shop with a Chinese brand on one line still needs the other
    /// forty printed. The tests assert it is zero for every fixture, which
    /// is where a missing `×` or `%` would show up.
    pub notdef: u32,
}

/// The whole ticket as dots, beside the lines it was drawn from.
pub struct Drawn {
    pub bitmap: Bitmap,
    pub lines: Vec<DrawnLine>,
    /// Dots per column: `width / 42`. The tests read it to say where a
    /// column should be without recomputing the layout.
    pub cell: u32,
    pub line_height: u32,
}

/// Draw `items` at `width_dots` across.
///
/// A line that does not fit is an error, never a truncation: a ticket with
/// a total cut off at the edge of the paper is worse than no ticket, and
/// the caller can see the failure while a customer cannot see the missing
/// digits.
pub(crate) fn draw(items: &[Item], lang: Lang, width_dots: u32) -> Result<Drawn, RetailError> {
    let width = usize::try_from(width_dots).unwrap_or(0);
    if width == 0 || width % 8 != 0 {
        return Err(RetailError::render(
            "a head's width is a whole number of bytes of dots (576 for 80 mm, 384 for 58 mm)",
        ));
    }
    let cell = width / WIDTH;
    if cell == 0 {
        return Err(RetailError::render(
            "a head narrower than 42 dots cannot carry the ticket's columns",
        ));
    }
    let fonts = Fonts::load()?;
    let metrics = Metrics::new(&fonts, cell);
    let rtl = lang.dir() == "rtl";

    // First pass: shape every line and count the rows it sits on, so the
    // bitmap is allocated once at the size the ticket actually needs. A
    // feed is blank rows in the bitmap rather than a command beside it, so
    // the picture a reviewer opens is the whole ticket with its spacing.
    let mut align = Align::Left;
    let mut bold = false;
    let mut row = 0usize;
    let mut placed: Vec<(Line, Align, bool, usize)> = Vec::new();
    for item in items {
        match item {
            Item::Align(next) => align = *next,
            Item::Bold(on) => bold = *on,
            Item::Feed(lines) => row = row.saturating_add(usize::from(*lines)),
            Item::Cut => {}
            Item::Line(text) => {
                placed.push((place(text, rtl, &fonts, &metrics, width)?, align, bold, row));
                row = row.saturating_add(1);
            }
        }
    }

    let mut bitmap = Bitmap::new(width, row.saturating_mul(metrics.line_height));
    let mut lines = Vec::with_capacity(placed.len());
    for (line, align, bold, row) in placed {
        let left = match align {
            Align::Center => (width as f32 - line.advance) / 2.0,
            Align::Left if rtl => width as f32 - line.advance,
            Align::Left => 0.0,
        };
        let baseline = (row.saturating_mul(metrics.line_height) + metrics.baseline) as f32;
        for glyph in &line.glyphs {
            ink(&mut bitmap, &fonts, &metrics, glyph, left, baseline, bold);
        }
        lines.push(line.drawn);
    }

    Ok(Drawn {
        bitmap,
        lines,
        cell: u32::try_from(cell).unwrap_or(u32::MAX),
        line_height: u32::try_from(metrics.line_height).unwrap_or(u32::MAX),
    })
}

/// Burn one glyph's outline into the bitmap. Bold is drawn twice a dot
/// apart: there is one weight of each face vendored, and a second file for
/// the four bold lines of a ticket is 150 KB to say what a dot of smear
/// says on paper this coarse.
fn ink(
    bitmap: &mut Bitmap,
    fonts: &Fonts,
    metrics: &Metrics,
    glyph: &Placed,
    left: f32,
    baseline: f32,
    bold: bool,
) {
    let (font, scale) = if glyph.arabic {
        (&fonts.arabic_px, metrics.arabic_scale)
    } else {
        (&fonts.latin_px, metrics.latin_scale)
    };
    let passes: &[f32] = if bold { &[0.0, 1.0] } else { &[0.0] };
    for smear in passes {
        let at = point(left + glyph.x + smear, baseline + glyph.y);
        let Some(outline) =
            font.outline_glyph(GlyphId(glyph.id).with_scale_and_position(scale, at))
        else {
            continue;
        };
        let bounds = outline.px_bounds();
        let (x0, y0) = (bounds.min.x as i32, bounds.min.y as i32);
        outline.draw(|gx, gy, coverage| {
            if coverage >= INK {
                // A glyph's ink may reach a dot or two past the pen at the
                // very edge of the paper (side bearings are not advances);
                // `Bitmap::set` drops what falls off the roll. The contract
                // this file keeps is the advance, which `place` refuses to
                // let past the width.
                bitmap.set(x0.saturating_add(gx as i32), y0.saturating_add(gy as i32));
            }
        });
    }
}

/// A glyph chosen and positioned, in dots from the line's left edge.
struct Placed {
    arabic: bool,
    id: u16,
    x: f32,
    y: f32,
}

struct Line {
    glyphs: Vec<Placed>,
    advance: f32,
    drawn: DrawnLine,
}

/// Shape one ticket line and put its glyphs where they go.
fn place(
    text: &str,
    rtl: bool,
    fonts: &Fonts,
    metrics: &Metrics,
    width: usize,
) -> Result<Line, RetailError> {
    // The narrow no-break space the money formatter groups thousands with
    // is sent to the head as a plain space (`escpos::Buf::text`), and the
    // first draft substituted it here too. It printed "1 000,00" as
    // "000,00 1": U+202F is a *common separator* to the bidi algorithm and
    // holds a number together, while a plain space is whitespace and takes
    // the paragraph's own direction, which cuts the number in half and
    // reorders the pieces. The character stays, and Plex Mono carries it at
    // the same 600 units as every other glyph, so it is still one column.
    let (glyphs, advance, notdef) = run(text, rtl, fonts, metrics, 0.0);
    // A label and an amount on one row are padded to exactly the column
    // budget by `ticket::Items::pair`, and in Latin that lands on
    // `42 × cell` dots because the face is monospaced. Arabic is not
    // monospaced, so the same row comes up short and the amount column
    // would wander from line to line. The gap between the two halves takes
    // the difference: no word moves, the space between them stretches, and
    // every padded row spans the same dots.
    let budget = WIDTH.saturating_mul(metrics.cell) as f32;
    let slack = budget - advance;
    let padded = text.chars().count() == WIDTH && slack > 0.0 && gap_mark(text).is_some();
    let (glyphs, advance, notdef) = if padded {
        run(text, rtl, fonts, metrics, slack)
    } else {
        (glyphs, advance, notdef)
    };
    if advance > width as f32 {
        // Named, not trimmed. A ticket whose total is cut off at the edge
        // of the roll is worse than no ticket, and the line has to be in
        // the message or the shop is told only that printing failed.
        // `RetailError::render` takes a `&'static str`; this one line needs
        // the offending text, which is why it builds the variant directly.
        return Err(RetailError::Render(askama::Error::custom(format!(
            "a ticket line needs {advance:.0} dots of a {width}-dot head: {text}"
        ))));
    }
    Ok(Line {
        glyphs,
        advance,
        drawn: DrawnLine {
            text: text.to_owned(),
            advance: advance.ceil().max(0.0) as u32,
            notdef,
        },
    })
}

/// Bidi, then shaping, then the pen. `slack` is inserted once, after the
/// glyph that ends the line's widest space run, which is what justifies a
/// padded row; zero for every other line.
fn run(
    text: &str,
    rtl: bool,
    fonts: &Fonts,
    metrics: &Metrics,
    slack: f32,
) -> (Vec<Placed>, f32, u32) {
    // A right-to-left line hands the algorithm its Latin stretches as
    // objects rather than as loose characters; see `isolate_ltr_runs`. A
    // left-to-right one needs nothing: its Latin already runs the way the
    // paragraph does, and leaving the string untouched is what keeps the
    // French and English drawings exactly as they were.
    let held: Cow<'_, str> = if rtl {
        Cow::Owned(isolate_ltr_runs(text))
    } else {
        Cow::Borrowed(text)
    };
    let text: &str = &held;
    let mark = if slack > 0.0 { gap_mark(text) } else { None };
    let level = if rtl { Level::rtl() } else { Level::ltr() };
    let bidi = ParagraphBidiInfo::new(text, Some(level));
    let (levels, runs) = bidi.visual_runs(0..text.len());
    let mut glyphs = Vec::new();
    let mut pen = 0.0f32;
    let mut notdef = 0u32;
    // `visual_runs` hands the runs back left to right, so the pen only ever
    // moves forward and the paragraph's direction is already accounted for.
    for visual in runs {
        let run_rtl = levels
            .get(visual.start)
            .is_some_and(unicode_bidi::Level::is_rtl);
        let Some(slice) = text.get(visual.clone()) else {
            continue;
        };
        // A run is one direction but can be two scripts: the digits inside
        // an Arabic total are Latin, and the space beside them is neither.
        // The face is chosen by what the characters are, not by the bidi
        // level, or `%` and `×` would be asked of a face that has no Latin.
        let mut pieces = script_runs(slice);
        if run_rtl {
            pieces.reverse();
        }
        for (piece, kind) in pieces {
            if kind == Piece::Mark {
                continue;
            }
            let arabic = kind == Piece::Arabic;
            let Some(sub) = slice.get(piece.clone()) else {
                continue;
            };
            let face = if arabic { &fonts.arabic } else { &fonts.latin };
            let mut buffer = UnicodeBuffer::new();
            buffer.push_str(sub);
            buffer.set_direction(if run_rtl {
                Direction::RightToLeft
            } else {
                Direction::LeftToRight
            });
            buffer.set_script(if arabic {
                script::ARABIC
            } else {
                script::LATIN
            });
            let shaped = rustybuzz::shape(face, &[], buffer);
            let scale = if arabic {
                metrics.arabic_unit
            } else {
                metrics.latin_unit
            };
            for (info, pos) in shaped
                .glyph_infos()
                .iter()
                .zip(shaped.glyph_positions().iter())
            {
                let id = u16::try_from(info.glyph_id).unwrap_or(0);
                if id == 0 {
                    notdef = notdef.saturating_add(1);
                }
                glyphs.push(Placed {
                    arabic,
                    id,
                    x: pen + pos.x_offset as f32 * scale,
                    // The bitmap's y grows downward and the font's grows
                    // up, so a mark raised above a letter moves to a
                    // smaller row.
                    y: -(pos.y_offset as f32) * scale,
                });
                pen += pos.x_advance as f32 * scale;
                let at = visual.start.saturating_add(piece.start);
                if mark.is_some_and(|m| m == at.saturating_add(info.cluster as usize)) {
                    pen += slack;
                }
            }
        }
    }
    (glyphs, pen, notdef)
}

struct Fonts {
    arabic: Face<'static>,
    latin: Face<'static>,
    arabic_px: FontRef<'static>,
    latin_px: FontRef<'static>,
}

impl Fonts {
    fn load() -> Result<Self, RetailError> {
        // Both faces are `include_bytes!` of files in this repository, so
        // a failure here is a corrupted checkout and not a runtime
        // condition; it is still an error rather than a panic, because the
        // one thing a printing path must never do is take the process
        // down while a sale is on the screen.
        const UNREADABLE: &str = "a vendored ticket font is not a font this build can read";
        Ok(Self {
            arabic: Face::from_slice(NASKH, 0).ok_or_else(|| RetailError::render(UNREADABLE))?,
            latin: Face::from_slice(PLEX, 0).ok_or_else(|| RetailError::render(UNREADABLE))?,
            arabic_px: FontRef::try_from_slice(NASKH)
                .map_err(|_| RetailError::render(UNREADABLE))?,
            latin_px: FontRef::try_from_slice(PLEX).map_err(|_| RetailError::render(UNREADABLE))?,
        })
    }
}

/// The one place a size is decided.
///
/// The em is set so that 42 glyphs of the monospaced face are 42 columns of
/// the head: `cell = width / 42`, and Plex Mono advances 600 of its 1000
/// units for every glyph, so an em of `cell × 1000 / 600` makes a column a
/// column. Everything else follows from the em, and the Arabic face is
/// drawn at the same em beside it.
struct Metrics {
    cell: usize,
    line_height: usize,
    baseline: usize,
    /// Font units to dots, per face.
    latin_unit: f32,
    arabic_unit: f32,
    /// `ab_glyph` scales a glyph by `scale / (ascent - descent)` rather
    /// than by the em, so the number handed to it is the em restated in
    /// that font's own height. Advances all come from the shaper, which
    /// works in font units; this only draws.
    latin_scale: PxScale,
    arabic_scale: PxScale,
}

impl Metrics {
    fn new(fonts: &Fonts, cell: usize) -> Self {
        let latin_em = fonts.latin_px.units_per_em().unwrap_or(1000.0);
        let arabic_em = fonts.arabic_px.units_per_em().unwrap_or(1000.0);
        let advance = fonts
            .latin_px
            .h_advance_unscaled(fonts.latin_px.glyph_id('0'));
        let em = if advance > 0.0 {
            cell as f32 * latin_em / advance
        } else {
            cell as f32
        };
        let latin_unit = em / latin_em;
        let arabic_unit = em / arabic_em;
        // Tall enough for the tallest ascender and the deepest descender of
        // either face, so Naskh's long tails are not sliced by the line
        // below them.
        let ascent = (fonts.latin_px.ascent_unscaled() * latin_unit)
            .max(fonts.arabic_px.ascent_unscaled() * arabic_unit);
        let descent = (-fonts.latin_px.descent_unscaled() * latin_unit)
            .max(-fonts.arabic_px.descent_unscaled() * arabic_unit);
        let gap = (fonts.latin_px.line_gap_unscaled() * latin_unit)
            .max(fonts.arabic_px.line_gap_unscaled() * arabic_unit);
        Self {
            cell,
            line_height: (ascent + descent + gap).ceil().max(1.0) as usize,
            baseline: ascent.ceil().max(0.0) as usize,
            latin_unit,
            arabic_unit,
            latin_scale: PxScale::from(em * fonts.latin_px.height_unscaled() / latin_em),
            arabic_scale: PxScale::from(em * fonts.arabic_px.height_unscaled() / arabic_em),
        }
    }
}

#[cfg(test)]
mod tests {
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
}
