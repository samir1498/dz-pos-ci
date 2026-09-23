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
