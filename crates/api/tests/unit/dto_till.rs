use super::{NewShift, NewShiftDto, TillCount, TillCountDto};

/// The absence proved rather than read off the struct. A body naming
/// either moment is refused outright by `deny_unknown_fields`, so there
/// is no path from the wire to a stored `opened_at` or `closed_at` at
/// all — not even one that parses the field and then ignores it, which
/// would leave the next reader of this file thinking the field works.
#[test]
fn a_body_that_names_a_moment_is_refused_rather_than_quietly_dropped() {
    let refused: Result<NewShiftDto, _> = serde_json::from_str(
        r#"{"opening_cash_centimes": 500000, "opened_at": "2020-01-01 09:00:00"}"#,
    );
    assert!(
        refused.is_err(),
        "a client named opened_at and the DTO took the body anyway"
    );
    let refused: Result<TillCountDto, _> =
        serde_json::from_str(r#"{"counted_centimes": 500000, "at": "2020-01-01 19:00:00"}"#);
    assert!(
        refused.is_err(),
        "a client named the count's moment and the DTO took the body anyway"
    );
}

/// And the body that is accepted still leaves the moment for the server:
/// `deny_unknown_fields` alone would be satisfied by a DTO that carried
/// the field under its own name and passed it through.
#[test]
fn the_accepted_body_hands_the_core_no_moment_at_all() {
    let opened = NewShift::try_from(
        serde_json::from_str::<NewShiftDto>(r#"{"opening_cash_centimes": 500000}"#).unwrap(),
    )
    .unwrap();
    assert_eq!(opened.opened_at, None);
    assert_eq!(opened.opening_cash.as_centimes(), 500_000);

    let counted = TillCount::try_from(
        serde_json::from_str::<TillCountDto>(r#"{"counted_centimes": 480000}"#).unwrap(),
    )
    .unwrap();
    assert_eq!(counted.at, None);
    assert_eq!(counted.counted.as_centimes(), 480_000);
    assert_eq!(counted.note, None);
}

/// A figure past what a JSON number carries is refused at this edge, not
/// rounded on its way through JavaScript.
#[test]
fn a_figure_beyond_the_js_safe_range_is_refused_at_the_edge() {
    assert!(NewShift::try_from(NewShiftDto {
        opening_cash_centimes: i64::MAX,
    })
    .is_err());
    assert!(TillCount::try_from(TillCountDto {
        counted_centimes: i64::MAX,
        note: None,
    })
    .is_err());
}
