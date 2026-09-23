//! `error.rs`'s own message-shaping and restart-code tests, declared here
//! with `#[path]` rather than inline there: `scripts/file-sizes.mjs` caps
//! that file at 600 lines, and this module is still a child of it
//! (`super::` below resolves `error.rs`'s own private items, `unknown_field`
//! included) so nothing about what it can reach changes.

use super::{unknown_field, ApiError, CoreError, StatusCode};

#[test]
fn the_whole_name_is_kept_even_with_a_backtick_inside() {
    let text = "Failed to deserialize the JSON body into the target type: \
                unknown field `bo`gus`, expected one of `name`, `barcode` at line 1 column 12";
    assert_eq!(unknown_field(text).as_deref(), Some("bo`gus"));
}

#[test]
fn a_plain_name_and_a_missing_marker_still_parse() {
    assert_eq!(
        unknown_field("unknown field `bogus`, expected `name`").as_deref(),
        Some("bogus")
    );
    assert_eq!(
        unknown_field("unknown field `bogus` at line 1").as_deref(),
        Some("bogus")
    );
    assert_eq!(unknown_field("something else"), None);
}

#[test]
fn a_storage_fault_keeps_the_drivers_text_off_the_wire() {
    use diesel::result::{DatabaseErrorKind, Error};
    let raw = Error::DatabaseError(
        DatabaseErrorKind::UniqueViolation,
        Box::new("UNIQUE constraint failed: products.barcode".to_owned()),
    );
    let err = ApiError::Core(CoreError::Query(raw));
    let message = err.message();
    assert!(!message.contains("constraint"), "{message}");
    assert!(!message.contains("products"), "{message}");
    assert_eq!(err.parts().1, "storage");
    assert_eq!(
        ApiError::Core(CoreError::validation("name", "is empty")).message(),
        CoreError::validation("name", "is empty").to_string(),
        "a rule's own message still goes through"
    );
}

/// The two answers a closed shop file can get, and they are not the same
/// answer. Both are 500 and both mean relaunch, but one of them also says
/// the restore did not happen, and that is what decides which data the
/// owner will be looking at afterwards. A screen can only tell them apart
/// by the code.
#[test]
fn a_closed_shop_file_says_relaunch_and_says_whether_it_was_restored() {
    assert_eq!(
        ApiError::RestartNeeded.parts(),
        (StatusCode::INTERNAL_SERVER_ERROR, "restart_needed")
    );
    assert_eq!(
        ApiError::NotRestoredRestartNeeded.parts(),
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "restore_failed_restart_needed"
        )
    );

    let says = ApiError::NotRestoredRestartNeeded.message();
    assert!(says.contains("did not happen"), "{says}");
    assert!(says.contains("start it again"), "{says}");
}
