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
    pub(crate) const fn css_size(self) -> &'static str {
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
mod tests {
    // Tests may panic; the deny is for shipped code.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    /// A layout has one name, and these are the two places it is written.
    ///
    /// `as_str` is what the shop's setting stores and what `parse` reads back;
    /// serde is what a `?layout=` query string carries and what the settings
    /// screen sends. They are declared separately, so nothing but this test
    /// stops them drifting, and the drift is silent: a preview would ask for a
    /// layout under a name the route cannot read and be handed the shop's
    /// standing one instead, with no error anywhere.
    ///
    /// It walks `ALL`, so a layout added without a thought about its spelling
    /// fails here rather than in a shop.
    #[test]
    fn the_wire_and_the_store_spell_a_layout_the_same() {
        for layout in FactureLayout::ALL {
            let on_the_wire = serde_json::to_string(&layout).expect("a layout serialises");
            assert_eq!(
                on_the_wire,
                format!("\"{}\"", layout.as_str()),
                "{layout:?} is spelled one way on the wire and another in the store"
            );
            assert_eq!(
                FactureLayout::parse(layout.as_str()),
                Some(layout),
                "{layout:?} does not read back out of its own spelling"
            );
        }
    }

    /// No two layouts answer to the same name.
    ///
    /// `ALL` is what the settings screen offers and what several tests walk,
    /// so a duplicate spelling would hide one layout behind another
    /// everywhere at once.
    #[test]
    fn no_two_layouts_share_a_name() {
        assert_eq!(
            FactureLayout::ALL.len(),
            FactureLayout::ALL
                .iter()
                .map(|layout| layout.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            "two layouts share a name"
        );
    }

    /// Every layout the enum has is in `ALL`, in the order `ALL` gives.
    ///
    /// This is the one property Rust cannot check on its own: there is no
    /// way to count an enum's variants without a derive, so a variant left
    /// out of `ALL` is invisible to any loop over `ALL` — including the two
    /// tests below and the settings route, all of which walk that constant.
    /// `ALL` carries every layout the type has, in the order it declares.
    ///
    /// A list cannot check itself, so `every` below is a second list, written
    /// out rather than read from `ALL`. Iterating `ALL` and asking whether
    /// `ALL` holds its own elements cannot fail; iterating a separate list
    /// can, and the mutation it exists for is `ALL` shrunk back to three
    /// entries with the fourth `match` arm left in place. The test lens named
    /// that one on 2026-09-20, and it passed until this shape.
    ///
    /// The wildcard-free `match` is what brings an author here: a fifth
    /// layout stops the build until it is given a position, and `every` is
    /// the line above the arm they add.
    #[test]
    fn all_carries_every_layout_the_type_has() {
        let every = [
            FactureLayout::Standard,
            FactureLayout::Compact,
            FactureLayout::HalfSheet,
            FactureLayout::Roll80,
        ];
        for (place, layout) in every.into_iter().enumerate() {
            let declared = match layout {
                FactureLayout::Standard => 0,
                FactureLayout::Compact => 1,
                FactureLayout::HalfSheet => 2,
                FactureLayout::Roll80 => 3,
            };
            assert_eq!(place, declared, "{layout:?} is not where this list puts it");
            assert_eq!(
                FactureLayout::ALL.get(place),
                Some(&layout),
                "ALL does not carry {layout:?} at {place}"
            );
        }
        assert_eq!(
            FactureLayout::ALL.len(),
            every.len(),
            "ALL carries something this list does not"
        );
    }

    /// `Page::paper` takes the layout's sheet when the layout pins one, and
    /// the caller's when it does not.
    ///
    /// The expectation comes from `fixed_paper` itself, so this proves the
    /// override is wired and not which sheet each layout pins. That second
    /// claim is held by hand-typed constants elsewhere:
    /// `the_roll_prints_on_the_roll_even_when_a_sheet_was_asked_for` and
    /// `the_half_sheet_prints_on_a5_even_when_a4_was_asked_for` name A5 and
    /// 80 mm in full.
    #[test]
    fn a_fixed_sheet_is_the_one_the_page_is_drawn_on() {
        for layout in FactureLayout::ALL {
            for asked_for in [Paper::A4, Paper::A5, Paper::Roll80] {
                let page = Page {
                    paper: asked_for,
                    layout,
                };
                match layout.fixed_paper() {
                    Some(fixed) => assert_eq!(page.paper(), fixed, "{layout:?}"),
                    None => assert_eq!(page.paper(), asked_for, "{layout:?}"),
                }
            }
        }
    }
}
