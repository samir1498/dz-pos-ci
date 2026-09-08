// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Writes `packages/shared/src/generated`. Types cross the Rust/TypeScript
//! boundary once, generated, never hand-written twice (architecture.md).
//!
//! This is a test rather than a `#[ts(export)]` attribute so the write
//! happens in one named place: `just types-check` runs it and then asks git
//! whether anything moved, which is what makes a stale checkout fail.

use dzpos_api::dto::{
    ApiErrorDto, ApiErrorPayloadDto, CategoryDto, HealthDto, NewProductDto, ProductDto, UnitDto,
};
use ts_rs::{Config, TS};

const FILES: [&str; 7] = [
    "UnitDto.ts",
    "ProductDto.ts",
    "NewProductDto.ts",
    "CategoryDto.ts",
    "HealthDto.ts",
    "ApiErrorDto.ts",
    "ApiErrorPayloadDto.ts",
];

fn out_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages/shared/src/generated")
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
