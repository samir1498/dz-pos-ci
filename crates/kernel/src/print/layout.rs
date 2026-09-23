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

/// The page a document is drawn onto. It changes one line, the `@page size`,
/// and nothing else: an A5 facture is the same facture on a smaller sheet,
/// not a second layout to keep in step (features.md §4, "A4/A5 through the
/// OS dialog").
///
/// `A4` and `A5` are sheets a print dialog offers and a till may name on any
/// print. `Roll80` is not: it is the continuous 80 mm paper a counter printer
/// carries, and no dialog lets a cashier choose it for a page laid out for a
/// sheet. It is here because it is still the `@page size` and splitting it
/// into a second type would mean two ways to say the same line. What keeps a
/// caller from asking for it is that the facture route's query string does
/// not spell it: that handler takes a `Sheet`, which has two values, and the
/// roll arrives only through the layout that names it
/// (`FactureLayout::fixed_paper`). A comment is not a guard, so
/// `the_roll_is_not_a_sheet_a_query_string_can_name` in
/// `crates/api/tests/print_api.rs` is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Paper {
    A4,
    A5,
    /// Spelled the way the layout drawn for it is spelled, so a query string
    /// naming a paper and one naming a layout do not disagree by a
    /// character. `snake_case` alone would make this `roll80`.
    #[serde(rename = "roll_80mm")]
    Roll80,
}

impl Paper {
    /// The value of the CSS `size` descriptor. The two named page sizes are
    /// CSS's own, so the browser and the print dialog agree on the sheet
    /// without the template naming millimetres.
    ///
    /// `pub`, not `pub(crate)`: both callers, `facture_view.rs` and
    /// `statement.rs`, are in `dzpos-retail` since the kernel crate split
    /// (S3 of `a-kernel-crate-and-retail-as-the-first-module`).
    pub const fn css_size(self) -> &'static str {
        match self {
            Paper::A4 => "A4",
            Paper::A5 => "A5",
            // A roll has a width and no end, which is what `auto` says. The
            // ticket's own template has carried this literal since M2.
            Paper::Roll80 => "80mm auto",
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
///
/// `snake_case` and not `lowercase`: serde has to spell a name the way
/// `as_str` does, because one of them is what a query string carries and the
/// other is what the shop's setting stores. Under `lowercase` a two-word
/// layout is `halfsheet` on the wire and `half_sheet` in the database, which
/// is one value with two spellings and a preview that silently ignores the
/// layout it was asked for. `the_wire_and_the_store_spell_a_layout_the_same`
/// is what holds this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
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
    /// The half sheet, drawn for it rather than squeezed onto it. A counter
    /// that hands a facture over with the goods wants the smaller paper, and
    /// the A4 page on an A5 sheet is a page at 70% with margins to match,
    /// which is not the same thing as a page laid out for 148 mm.
    HalfSheet,
    /// The roll a counter printer already has. 72 mm of content, so this is
    /// the one layout that is not the standard page under another
    /// stylesheet: the two party blocks stack, and a line becomes two rows
    /// rather than six columns. Every mention décret 05-468 art. 3 asks for
    /// is still on it, which is what makes it a facture and not a ticket
    /// with a facture's title.
    ///
    /// Named outright because `snake_case` puts no boundary before a digit:
    /// it reads `Roll80` as `roll80` while `as_str` stores `roll_80mm`, and
    /// the two spellings have to be one.
    #[serde(rename = "roll_80mm")]
    Roll80,
}

impl FactureLayout {
    /// Every layout a shop may choose, in the order a settings screen lists
    /// them. The API hands this list to the UI so there is one source for it.
    pub const ALL: [FactureLayout; 4] = [
        FactureLayout::Standard,
        FactureLayout::Compact,
        FactureLayout::HalfSheet,
        FactureLayout::Roll80,
    ];

    /// The one spelling: what is stored, what crosses the wire, and what a
    /// query string carries. One name, so a value written by an older build
    /// reads back the same.
    pub const fn as_str(self) -> &'static str {
        match self {
            FactureLayout::Standard => "standard",
            FactureLayout::Compact => "compact",
            FactureLayout::HalfSheet => "half_sheet",
            FactureLayout::Roll80 => "roll_80mm",
        }
    }

    pub fn parse(value: &str) -> Option<FactureLayout> {
        match value {
            "standard" => Some(FactureLayout::Standard),
            "compact" => Some(FactureLayout::Compact),
            "half_sheet" => Some(FactureLayout::HalfSheet),
            "roll_80mm" => Some(FactureLayout::Roll80),
            _ => None,
        }
    }

    /// The sheet a layout can only be drawn on, when it has one.
    ///
    /// Most layouts have none: the standard page and the compact one are A4
    /// designs that the print dialog may put on A5, and a shop doing that
    /// gets a smaller version of the same page, which is a choice it is
    /// allowed to make. The half sheet is the other kind. Its type sizes and
    /// its column widths are measured for 148 mm, so drawing it on A4 would
    /// leave a small facture in the corner of a large sheet, and no caller
    /// asking for A4 means that.
    pub const fn fixed_paper(self) -> Option<Paper> {
        match self {
            FactureLayout::Standard | FactureLayout::Compact => None,
            FactureLayout::HalfSheet => Some(Paper::A5),
            FactureLayout::Roll80 => Some(Paper::Roll80),
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

    /// The sheet this page is actually drawn on: what the caller asked for,
    /// unless the layout only fits one, in which case that one wins.
    ///
    /// The layout wins rather than refusing the call because the two are
    /// answered by different people. The caller naming A4 is a till saying
    /// which tray to use, and the shop having chosen the half sheet is a
    /// standing decision about what its factures look like. A shop that
    /// wants A4 paper picks an A4 layout; nobody is served by a print that
    /// fails because a query string and a setting disagree.
    pub const fn paper(self) -> Paper {
        match self.layout.fixed_paper() {
            Some(fixed) => fixed,
            None => self.paper,
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/print_layout.rs"]
mod tests;
