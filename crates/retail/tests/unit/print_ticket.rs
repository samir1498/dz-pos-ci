#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

/// A label too long to sit beside its figure takes the rows above it,
/// and the figure keeps a row of its own.
///
/// No golden reaches this arm: every label on the three facture
/// fixtures is short enough for `pair`. A shop reaches it on its first
/// paper all the same — "Montant de la taxe sur la valeur ajoutée"
/// beside a five-figure sum is past the budget on a 42-column head —
/// and what must not happen there is the figure losing its thousands
/// to the edge of the roll. A label a reader loses half of is still a
/// label they can name; half an amount is a different amount.
#[test]
fn a_label_too_long_to_sit_beside_its_amount_leaves_the_amount_a_row() {
    let label = "Montant de la taxe sur la valeur ajoutee";
    // The narrow no-break space the formatter groups thousands with,
    // so this is the shape of a real figure and not a test's idea of
    // one.
    let amount = "12\u{202f}345,67";
    assert_eq!(label.chars().count(), 40);
    assert_eq!(amount.chars().count(), 9);
    assert!(
        label.chars().count() + amount.chars().count() >= WIDTH,
        "this label and amount fit the budget, so the test misses the arm"
    );

    let mut items = Items::new();
    items.row(label, amount);
    let lines: Vec<String> = items
        .into_items()
        .into_iter()
        .map(|item| match item {
            Item::Line(line) => line,
            _ => panic!("row wrote something that is not a line"),
        })
        .collect();

    assert_eq!(lines.len(), 2, "{lines:?}");
    assert_eq!(lines[0], label, "the label did not get the row to itself");
    assert!(
        !lines[0].contains("345,67"),
        "the figure rode along on the label's row: {lines:?}"
    );
    assert_eq!(
        lines[1].trim_start(),
        amount,
        "the figure is not alone on its row: {lines:?}"
    );
    assert!(
        lines[1].starts_with(' '),
        "the figure is not right-aligned: {lines:?}"
    );
    assert_eq!(
        lines[1].chars().count(),
        WIDTH,
        "the figure's row does not reach the right edge: {lines:?}"
    );
    for line in &lines {
        assert!(
            line.chars().count() <= WIDTH,
            "{line:?} is wider than the head"
        );
    }
}
