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

/// A product name is typed by the shop and can carry anything. The
/// facture fixture's does: `Huile <Elio> & Co 5 L`. With only letters
/// and digits holding an object together, the brackets and the
/// ampersand were neutral, the name became four objects, and the
/// Arabic paragraph printed them right to left as
/// `Co 5 L & <Elio> Huile` — every amount on the page correct and the
/// article backwards. One object is the whole fix.
#[test]
fn a_product_name_with_punctuation_in_it_stays_one_object() {
    assert_eq!(
        isolate_ltr_runs("Huile <Elio> & Co 5 L"),
        format!("{LRI}Huile <Elio> & Co 5 L{PDI}")
    );
    assert_eq!(
        isolate_ltr_runs("Erreur <quantité> & prix"),
        format!("{LRI}Erreur <quantité> & prix{PDI}")
    );
}

/// The Arabic abbreviation for TVA, `ت.ق.م`, whose two full stops are
/// ASCII and therefore left-to-right members in their own right. Each
/// becomes an isolate of one character, which is what the raster has
/// done since it landed and is harmless: a lone neutral inside an
/// isolate has nothing to be reordered against, so the abbreviation
/// reads the same on paper either way (`ar.png`).
///
/// What this pins is that each full stop gets its own isolate and the
/// three Arabic letters stay in Arabic order around them, which is what
/// a reader of the recap row sees.
#[test]
fn the_full_stops_of_the_arabic_tva_label_are_objects_of_their_own() {
    assert_eq!(
        isolate_ltr_runs("ت.ق.م 19 %"),
        format!("ت{LRI}.{PDI}ق{LRI}.{PDI}م {LRI}19 %{PDI}")
    );
}

/// The end of a stretch is its last member, not its last character: a
/// full stop closing an Arabic sentence belongs to the Arabic.
#[test]
fn punctuation_hanging_off_an_object_stays_outside_it() {
    assert_eq!(isolate_ltr_runs("Alger،"), format!("{LRI}Alger{PDI}،"));
}
