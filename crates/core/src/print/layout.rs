//! What a facture is drawn on, and how.
//!
//! Two questions that arrive together and are answered by different people,
//! which is why they are two types and not one.
//!
//! `Paper` is the sheet, and the till knows it: the same facture goes on A4
//! in the office and on A5 at the counter, so it is named on every print and
//! is not a setting (features.md §4).
//!
//! `FactureLayout` is how the page is drawn, and the shop knows it: chosen
//! once in settings, every facture follows it. Lumina, the competitor a shop
//! compares us against, ships eleven of these plus an A5 and a thermal one,
//! and a buyer looking at a menu against a single page draws the obvious
//! conclusion.
//!
//! This module holds the choice. `facture.rs` holds the page it produces, and
//! is the only place that knows which template a layout is drawn by.

/// The sheet the OS print dialog is given. It changes one line of the page,
/// the `@page size`, and nothing else: an A5 facture is the same facture on
/// a smaller sheet, not a second layout to keep in step (features.md §4,
/// "A4/A5 through the OS dialog").
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Paper {
    A4,
    A5,
}

impl Paper {
    /// The value of the CSS `size` descriptor. The two named page sizes are
    /// CSS's own, so the browser and the print dialog agree on the sheet
    /// without the template naming millimetres.
    pub(crate) const fn css_size(self) -> &'static str {
        match self {
            Paper::A4 => "A4",
            Paper::A5 => "A5",
        }
    }
}

/// Which of the shop's facture layouts a page is drawn in.
///
/// Not the same question as `Paper`, and the two are kept apart on purpose.
/// The paper is the sheet in the tray, which the till knows and the shop does
/// not: the same facture goes on A4 in the office and on A5 at the counter.
/// The layout is how the page is drawn, which the shop chooses once and every
/// facture then follows.
///
/// Lumina, the competitor a shop compares us against, ships eleven of these
/// plus an A5 and a thermal one, and a buyer looking at a menu against a
/// single page draws the obvious conclusion.
///
/// A name this build cannot read falls back to `Standard` rather than
/// failing, the way an unknown theme reads as no choice: the cost of being
/// wrong is a facture on the wrong layout, and the alternative is a shop that
/// cannot print because a newer build once wrote a name this one never heard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FactureLayout {
    /// The page this repo has printed since M2. Generous spacing, one line
    /// per article with room to read it.
    #[default]
    Standard,
    /// The same information, drawn tight, so a long basket that spilled onto
    /// a second sheet fits on one. Every mention décret 05-468 art. 3 asks
    /// for is on it; what changes is the size and the spacing, because a
    /// layout that saved a line by dropping the NIF would not be a facture.
    Compact,
}

impl FactureLayout {
    /// Every layout a shop may choose, in the order a settings screen lists
    /// them. The API hands this list to the UI so there is one source for it.
    pub const ALL: [FactureLayout; 2] = [FactureLayout::Standard, FactureLayout::Compact];

    /// The one spelling: what is stored, what crosses the wire, and what a
    /// query string carries. One name, so a value written by an older build
    /// reads back the same.
    pub const fn as_str(self) -> &'static str {
        match self {
            FactureLayout::Standard => "standard",
            FactureLayout::Compact => "compact",
        }
    }

    pub fn parse(value: &str) -> Option<FactureLayout> {
        match value {
            "standard" => Some(FactureLayout::Standard),
            "compact" => Some(FactureLayout::Compact),
            _ => None,
        }
    }
}

/// The sheet and the layout together: what is being drawn, and what it is
/// being drawn on.
///
/// One argument rather than two because they arrive together at every call
/// and neither is much use alone, and because a layout that only fits one
/// sheet will want to say so here rather than in each caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    pub paper: Paper,
    pub layout: FactureLayout,
}

impl Page {
    /// The default page: the standard layout on A4.
    pub const A4: Page = Page {
        paper: Paper::A4,
        layout: FactureLayout::Standard,
    };
}
