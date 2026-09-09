// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Writes `packages/shared/src/generated`. Types cross the Rust/TypeScript
//! boundary once, generated, never hand-written twice (architecture.md).
//!
//! This is a test rather than a `#[ts(export)]` attribute so the write
//! happens in one named place: `just types-check` runs it into a temp
//! directory and diffs that against the committed one both ways, which is
//! what makes a stale checkout fail.

use dzpos_api::dto::{
    ApiErrorDto, ApiErrorPayloadDto, CategoryDto, DatedRegimeDto, HealthDto, NewProductDto,
    ProductDto, RegimeChangeDto, RegimeDto, SettingsDto, StoreDto, UnitDto,
};
use ts_rs::{Config, TS};

const FILES: [&str; 12] = [
    "UnitDto.ts",
    "ProductDto.ts",
    "NewProductDto.ts",
    "CategoryDto.ts",
    "HealthDto.ts",
    "ApiErrorDto.ts",
    "ApiErrorPayloadDto.ts",
    "StoreDto.ts",
    "RegimeDto.ts",
    "DatedRegimeDto.ts",
    "SettingsDto.ts",
    "RegimeChangeDto.ts",
];

/// Where the bindings are written. Never the committed directory by
/// default: a plain `cargo test --workspace` used to regenerate
/// `packages/shared/src/generated` in place, so the CI diff that ran after
/// it compared fresh against fresh and a stale commit passed. The
/// `types` recipe points this at the committed dir on purpose; the
/// `types-check` recipe and CI point it at a temp dir and diff.
fn out_dir() -> std::path::PathBuf {
    match std::env::var_os("DZPOS_TS_OUT_DIR") {
        Some(dir) => std::path::PathBuf::from(dir),
        None => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/ts-bindings"),
    }
}

/// The DTO source, read at compile time so the export list below cannot
/// drift from it: a DTO that names an `export_to` file and is missing from
/// `FILES` (or the other way round) fails here, and a bare `#[ts(export)]`
/// is refused because ts-rs would then write it to `crates/api/bindings`
/// from a generated lib test, outside the directory the gate diffs.
const DTO_SOURCE: &str = include_str!("../src/dto.rs");

#[test]
fn every_exported_dto_is_in_the_list_and_none_uses_the_bare_export() {
    let mut named: Vec<&str> = DTO_SOURCE
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            line.strip_prefix("#[ts(export_to = \"")
                .and_then(|rest| rest.strip_suffix("\")]"))
        })
        .collect();
    named.sort_unstable();
    let mut listed = FILES.to_vec();
    listed.sort_unstable();
    assert_eq!(
        named, listed,
        "the export_to names in dto.rs and FILES in this test differ"
    );
    assert!(
        !DTO_SOURCE.contains("#[ts(export)]"),
        "use #[ts(export_to = \"Name.ts\")] and list it in FILES; a bare export writes to crates/api/bindings"
    );
}

/// ts-rs calls an `i64` a `bigint` by default, which would not survive
/// `JSON.parse`. Centimes are safe below 2^53, so large ints are `number`
/// (architecture.md, contract between Rust and TypeScript).
fn config(dir: &std::path::Path) -> Config {
    Config::new().with_out_dir(dir).with_large_int("number")
}

#[test]
fn export_bindings() {
    let dir = out_dir();
    std::fs::create_dir_all(&dir).unwrap();
    let cfg = config(&dir);
    UnitDto::export_all(&cfg).unwrap();
    ProductDto::export_all(&cfg).unwrap();
    NewProductDto::export_all(&cfg).unwrap();
    CategoryDto::export_all(&cfg).unwrap();
    HealthDto::export_all(&cfg).unwrap();
    ApiErrorDto::export_all(&cfg).unwrap();
    ApiErrorPayloadDto::export_all(&cfg).unwrap();
    StoreDto::export_all(&cfg).unwrap();
    RegimeDto::export_all(&cfg).unwrap();
    DatedRegimeDto::export_all(&cfg).unwrap();
    SettingsDto::export_all(&cfg).unwrap();
    RegimeChangeDto::export_all(&cfg).unwrap();

    for name in FILES {
        assert!(dir.join(name).exists(), "{name} was not written");
    }
}

#[test]
fn money_is_a_number_not_a_bigint() {
    let cfg = config(&out_dir());
    let decl = ProductDto::decl(&cfg);
    assert!(
        decl.contains("selling_centimes: number"),
        "money is not a plain number: {decl}"
    );
    assert!(
        !decl.contains("bigint"),
        "a bigint reached the wire: {decl}"
    );

    let new = NewProductDto::decl(&cfg);
    assert!(
        !new.contains("bigint"),
        "a bigint reached the request type: {new}"
    );
}

#[test]
fn an_optional_price_stays_nullable_on_the_wire() {
    // A per-field `number` override once erased the null here and the
    // client would have missed the empty wholesale price.
    let cfg = config(&out_dir());
    let decl = ProductDto::decl(&cfg);
    assert!(
        decl.contains("wholesale_centimes: number | null"),
        "an optional price lost its null: {decl}"
    );
}

#[test]
fn the_unit_union_matches_what_the_database_allows() {
    let cfg = config(&out_dir());
    let decl = UnitDto::decl(&cfg);
    for unit in ["piece", "kg", "litre", "box"] {
        assert!(
            decl.contains(&format!("\"{unit}\"")),
            "{unit} missing: {decl}"
        );
    }
}
