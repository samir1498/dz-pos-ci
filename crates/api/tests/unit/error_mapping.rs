//! S4 of `a-kernel-crate-and-retail-as-the-first-module` split one error
//! enum (`git show 9cc5255:crates/kernel/src/error.rs`) into `CoreError`
//! plus `RetailError`, and split this file's own `status_for`/`code`/
//! `figures` the same way (`git show 9cc5255:crates/api/src/error.rs`).
//! This pins that every variant of both enums still maps to the exact
//! HTTP status, `code` and figures it had before the split. Every
//! expected value below is a literal copied from that pre-split file,
//! never read off `status_for`, `status_for_retail`, `CoreError::code` or
//! `RetailError::code` themselves: a mapping swapped in either function
//! fails a test here rather than agreeing with its own new answer.
//!
//! Declared from `error.rs` with `#[path]` rather than written inline
//! there: `scripts/file-sizes.mjs` caps that file at 600 lines, and this
//! module is still a child of it (`super::` below resolves `error.rs`'s
//! own private items) so nothing about what it can reach changes.
#[cfg(feature = "retail")]
use super::RetailError;
use super::{ApiError, CoreError, Figures, StatusCode};
use dzpos_core::db::DbError;
#[cfg(feature = "retail")]
use dzpos_core::money::Money;
use dzpos_core::money::MoneyError;
use dzpos_core::services::permissions::Permission;

#[test]
fn every_core_error_variant_keeps_its_pre_split_status_code_and_figures() {
    assert_eq!(
        ApiError::Core(CoreError::validation("qty", "must be positive")).parts(),
        (StatusCode::UNPROCESSABLE_ENTITY, "validation")
    );
    assert_eq!(
        ApiError::Core(CoreError::validation("qty", "must be positive")).figures(),
        Figures {
            field: Some("qty".to_string()),
            ..Figures::NONE
        }
    );

    assert_eq!(
        ApiError::Core(CoreError::NotFound {
            entity: "product",
            id: 7
        })
        .parts(),
        (StatusCode::NOT_FOUND, "not_found")
    );
    assert_eq!(
        ApiError::Core(CoreError::NotFound {
            entity: "product",
            id: 7
        })
        .figures(),
        Figures::NONE
    );
    // Not pre-split: C3 of the clinic plan added it for a text id. It owes
    // the same answer as its integer twin above, written out rather than
    // read off that one.
    let text_id = CoreError::NotFoundText {
        entity: "patient",
        id: "0199a0c1-0000-7000-8000-000000000000".to_string(),
    };
    assert_eq!(
        text_id.to_string(),
        "patient 0199a0c1-0000-7000-8000-000000000000 does not exist in this shop"
    );
    let text_id = ApiError::Core(text_id);
    assert_eq!(text_id.parts(), (StatusCode::NOT_FOUND, "not_found"));
    assert_eq!(text_id.figures(), Figures::NONE);

    assert_eq!(
        ApiError::Core(CoreError::conflict("barcode", "already used")).parts(),
        (StatusCode::CONFLICT, "conflict")
    );
    assert_eq!(
        ApiError::Core(CoreError::conflict("barcode", "already used")).figures(),
        Figures {
            field: Some("barcode".to_string()),
            ..Figures::NONE
        }
    );

    assert_eq!(
        ApiError::Core(CoreError::Exhausted {
            series: "doc_facture:2026".to_string()
        })
        .parts(),
        (StatusCode::CONFLICT, "exhausted")
    );

    assert_eq!(
        ApiError::Core(CoreError::AuthRefused).parts(),
        (StatusCode::UNAUTHORIZED, "auth_refused")
    );

    assert_eq!(
        ApiError::Core(CoreError::LockedOut {
            retry_after_seconds: 30
        })
        .parts(),
        (StatusCode::TOO_MANY_REQUESTS, "locked_out")
    );
    assert_eq!(
        ApiError::Core(CoreError::LockedOut {
            retry_after_seconds: 30
        })
        .figures(),
        Figures {
            retry_after_seconds: Some(30),
            ..Figures::NONE
        }
    );

    assert_eq!(
        ApiError::Core(CoreError::Hash(argon2::password_hash::Error::Password)).parts(),
        (StatusCode::INTERNAL_SERVER_ERROR, "storage")
    );

    assert_eq!(
        ApiError::Core(CoreError::forbidden(Permission::Sell)).parts(),
        (StatusCode::FORBIDDEN, "forbidden")
    );
    assert_eq!(
        ApiError::Core(CoreError::forbidden(Permission::Sell)).figures(),
        Figures {
            permission: Some("sell"),
            ..Figures::NONE
        }
    );

    assert_eq!(
        ApiError::Core(CoreError::Money(MoneyError::Overflow)).parts(),
        (StatusCode::INTERNAL_SERVER_ERROR, "money")
    );

    assert_eq!(
        ApiError::Core(CoreError::Db(DbError::Migrate("boom".to_string()))).parts(),
        (StatusCode::INTERNAL_SERVER_ERROR, "storage")
    );

    assert_eq!(
        ApiError::Core(CoreError::Query(diesel::result::Error::NotFound)).parts(),
        (StatusCode::INTERNAL_SERVER_ERROR, "storage")
    );

    assert_eq!(
        ApiError::Core(CoreError::Io(std::io::Error::other("disk full"))).parts(),
        (StatusCode::INTERNAL_SERVER_ERROR, "storage")
    );
}

#[cfg(feature = "retail")]
#[test]
fn every_retail_error_variant_keeps_its_pre_split_status_code_and_figures() {
    // `Kernel` delegates; `NotFound` here proves the delegation lands on
    // the same status and code the plain `CoreError::NotFound` case above
    // gets on its own, so which of the two enums a service returns it
    // through never changes what a caller sees.
    assert_eq!(
        ApiError::Retail(RetailError::Kernel(CoreError::NotFound {
            entity: "product",
            id: 7
        }))
        .parts(),
        (StatusCode::NOT_FOUND, "not_found")
    );

    assert_eq!(
        ApiError::Retail(RetailError::DuplicateBarcode("6111".to_string())).parts(),
        (StatusCode::CONFLICT, "duplicate_barcode")
    );

    assert_eq!(
        ApiError::Retail(RetailError::PaymentAboveDebt {
            outstanding_centimes: 5_000
        })
        .parts(),
        (StatusCode::UNPROCESSABLE_ENTITY, "validation")
    );
    assert_eq!(
        ApiError::Retail(RetailError::PaymentAboveDebt {
            outstanding_centimes: 5_000
        })
        .figures(),
        Figures {
            field: Some("amount_centimes".to_string()),
            outstanding_centimes: Some(5_000),
            ..Figures::NONE
        }
    );

    assert_eq!(
        ApiError::Retail(RetailError::CreditLimit {
            balance_after: Money::centimes(12_000),
            credit_limit: Money::centimes(10_000),
        })
        .parts(),
        (StatusCode::UNPROCESSABLE_ENTITY, "credit_limit")
    );
    assert_eq!(
        ApiError::Retail(RetailError::CreditLimit {
            balance_after: Money::centimes(12_000),
            credit_limit: Money::centimes(10_000),
        })
        .figures(),
        Figures {
            balance_after_centimes: Some(12_000),
            credit_limit_centimes: Some(10_000),
            ..Figures::NONE
        }
    );

    assert_eq!(
        ApiError::Retail(RetailError::PartyIds {
            seller_missing: vec!["nif"],
            buyer_missing: vec!["rc"],
        })
        .parts(),
        (StatusCode::UNPROCESSABLE_ENTITY, "party_ids")
    );
    assert_eq!(
        ApiError::Retail(RetailError::PartyIds {
            seller_missing: vec!["nif"],
            buyer_missing: vec!["rc"],
        })
        .figures(),
        Figures {
            seller_missing_ids: Some(vec!["nif"]),
            buyer_missing_ids: Some(vec!["rc"]),
            ..Figures::NONE
        }
    );

    assert_eq!(
        ApiError::Retail(RetailError::Unstamped {
            entity: "cash_movement"
        })
        .parts(),
        (StatusCode::INTERNAL_SERVER_ERROR, "storage")
    );

    assert_eq!(
        ApiError::Retail(RetailError::UnpricedReversal {
            document_id: 1,
            product_id: 2,
            reason: "no movement",
        })
        .parts(),
        (StatusCode::INTERNAL_SERVER_ERROR, "storage")
    );

    assert_eq!(
        ApiError::Retail(RetailError::render("template failed")).parts(),
        (StatusCode::INTERNAL_SERVER_ERROR, "print")
    );

    assert_eq!(
        ApiError::Retail(RetailError::Workbook(
            rust_xlsxwriter::XlsxError::RowColumnLimitError
        ))
        .parts(),
        (StatusCode::INTERNAL_SERVER_ERROR, "workbook")
    );
}
