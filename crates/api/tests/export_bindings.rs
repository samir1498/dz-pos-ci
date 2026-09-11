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
    AdjustmentDto, ApiErrorDto, ApiErrorPayloadDto, AuditEntryDto, AuditLogDto, AuditUserDto,
    AvoirLineDto, BackupDto, BackupsDto, CancelDocumentDto, CashPositionDto, CategoryDto, ClockDto,
    CloseOrderDto, CloseSupplierDto, CustomerDto, CustomerLedgerDto, CustomerPaymentsDto,
    CustomerWriteDto, DashboardDto, DashboardFiguresDto, DashboardSeriesDto,
    DashboardSeriesPointDto, DatedRegimeDto, DebtEntryDto, DebtKindDto, DiscountThresholdChangeDto,
    DocumentKindDto, DocumentStatusDto, ExpenseCategoryDto, ExpenseDto, ExpensesDto, HealthDto,
    ImportAppliedDto, ImportDryRunDto, ImportOutcomeDto, ImportRowDto, LabelSheetDto,
    LastStockRecountDto, LoginDto, LowStockDto, MeDto, NewAvoirDto, NewCustomerDto, NewExpenseDto,
    NewPaymentDto, NewProductDto, NewPurchaseDto, NewPurchaseLineDto, NewReceiptDto, NewSaleDto,
    NewSaleLineDto, NewSupplierDto, OutgoingsDto, OwedDto, PaidNowDto, PartyKindDto,
    PaymentAllocationDto, PaymentDto, PaymentMethodDto, PaymentModeDto, PermissionDto, ProductDto,
    PurchaseDetailDto, PurchaseDto, PurchaseLineDto, PurchaseReceiptDto, PurchaseReceiptLineDto,
    PurchaseStatusDto, ReceiveLineDto, RegimeChangeDto, RegimeDto, RestoreDto, RoleDto,
    SaleBalanceDto, SaleCancelEffectDto, SaleCancellationDto, SaleDto, SaleKindDto, SaleLineDto,
    SaleTotalsDto, SaleTvaDto, SaleWarningDto, SessionDto, SessionIdleDto, SettingsDto,
    StockDriftDto, StockRecountDto, StoreDto, SupplierAllocationDto, SupplierDebtKindDto,
    SupplierDto, SupplierEntryDto, SupplierLedgerDto, SupplierStatementDto, SupplierWriteDto,
    TakingsDto, ThemeChoiceDto, ThemeDto, TopProductDto, UnitDto,
};
use ts_rs::{Config, TS};

const FILES: [&str; 101] = [
    "LoginDto.ts",
    "MeDto.ts",
    "SessionDto.ts",
    "SessionIdleDto.ts",
    "RoleDto.ts",
    "PermissionDto.ts",
    "UnitDto.ts",
    "ProductDto.ts",
    "NewProductDto.ts",
    "CategoryDto.ts",
    "HealthDto.ts",
    "ClockDto.ts",
    "ApiErrorDto.ts",
    "ApiErrorPayloadDto.ts",
    "StoreDto.ts",
    "RegimeDto.ts",
    "DatedRegimeDto.ts",
    "ThemeDto.ts",
    "ThemeChoiceDto.ts",
    "SettingsDto.ts",
    "RegimeChangeDto.ts",
    "BackupDto.ts",
    "BackupsDto.ts",
    "RestoreDto.ts",
    "PaymentModeDto.ts",
    "DocumentKindDto.ts",
    "DocumentStatusDto.ts",
    "SaleLineDto.ts",
    "SaleTvaDto.ts",
    "SaleTotalsDto.ts",
    "SaleBalanceDto.ts",
    "SaleWarningDto.ts",
    "SaleDto.ts",
    "NewSaleLineDto.ts",
    "NewSaleDto.ts",
    "SaleKindDto.ts",
    "SaleCancelEffectDto.ts",
    "SaleCancellationDto.ts",
    "AvoirLineDto.ts",
    "NewAvoirDto.ts",
    "CancelDocumentDto.ts",
    "PartyKindDto.ts",
    "DebtKindDto.ts",
    "DiscountThresholdChangeDto.ts",
    "CustomerDto.ts",
    "CustomerWriteDto.ts",
    "NewCustomerDto.ts",
    "DebtEntryDto.ts",
    "CustomerLedgerDto.ts",
    "AdjustmentDto.ts",
    "PaymentMethodDto.ts",
    "PaymentAllocationDto.ts",
    "PaymentDto.ts",
    "CustomerPaymentsDto.ts",
    "NewPaymentDto.ts",
    "SupplierDebtKindDto.ts",
    "SupplierDto.ts",
    "SupplierWriteDto.ts",
    "NewSupplierDto.ts",
    "CloseSupplierDto.ts",
    "SupplierAllocationDto.ts",
    "SupplierEntryDto.ts",
    "SupplierLedgerDto.ts",
    "SupplierStatementDto.ts",
    "PurchaseStatusDto.ts",
    "PurchaseDto.ts",
    "PurchaseLineDto.ts",
    "PurchaseReceiptLineDto.ts",
    "PurchaseReceiptDto.ts",
    "PurchaseDetailDto.ts",
    "NewPurchaseLineDto.ts",
    "PaidNowDto.ts",
    "NewPurchaseDto.ts",
    "ReceiveLineDto.ts",
    "NewReceiptDto.ts",
    "CloseOrderDto.ts",
    "ExpenseCategoryDto.ts",
    "ExpenseDto.ts",
    "ExpensesDto.ts",
    "NewExpenseDto.ts",
    "TakingsDto.ts",
    "OutgoingsDto.ts",
    "CashPositionDto.ts",
    "DashboardFiguresDto.ts",
    "LowStockDto.ts",
    "TopProductDto.ts",
    "OwedDto.ts",
    "DashboardDto.ts",
    "DashboardSeriesPointDto.ts",
    "DashboardSeriesDto.ts",
    "StockDriftDto.ts",
    "StockRecountDto.ts",
    "LastStockRecountDto.ts",
    "ImportOutcomeDto.ts",
    "ImportRowDto.ts",
    "ImportDryRunDto.ts",
    "ImportAppliedDto.ts",
    "LabelSheetDto.ts",
    "AuditUserDto.ts",
    "AuditEntryDto.ts",
    "AuditLogDto.ts",
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
    RoleDto::export_all(&cfg).unwrap();
    PermissionDto::export_all(&cfg).unwrap();
    MeDto::export_all(&cfg).unwrap();
    LoginDto::export_all(&cfg).unwrap();
    SessionDto::export_all(&cfg).unwrap();
    SessionIdleDto::export_all(&cfg).unwrap();
    UnitDto::export_all(&cfg).unwrap();
    ProductDto::export_all(&cfg).unwrap();
    NewProductDto::export_all(&cfg).unwrap();
    CategoryDto::export_all(&cfg).unwrap();
    HealthDto::export_all(&cfg).unwrap();
    ClockDto::export_all(&cfg).unwrap();
    ApiErrorDto::export_all(&cfg).unwrap();
    ApiErrorPayloadDto::export_all(&cfg).unwrap();
    StoreDto::export_all(&cfg).unwrap();
    RegimeDto::export_all(&cfg).unwrap();
    DatedRegimeDto::export_all(&cfg).unwrap();
    ThemeDto::export_all(&cfg).unwrap();
    ThemeChoiceDto::export_all(&cfg).unwrap();
    SettingsDto::export_all(&cfg).unwrap();
    RegimeChangeDto::export_all(&cfg).unwrap();
    DiscountThresholdChangeDto::export_all(&cfg).unwrap();
    BackupDto::export_all(&cfg).unwrap();
    BackupsDto::export_all(&cfg).unwrap();
    RestoreDto::export_all(&cfg).unwrap();
    PaymentModeDto::export_all(&cfg).unwrap();
    DocumentKindDto::export_all(&cfg).unwrap();
    DocumentStatusDto::export_all(&cfg).unwrap();
    SaleLineDto::export_all(&cfg).unwrap();
    SaleTvaDto::export_all(&cfg).unwrap();
    SaleTotalsDto::export_all(&cfg).unwrap();
    SaleBalanceDto::export_all(&cfg).unwrap();
    SaleWarningDto::export_all(&cfg).unwrap();
    SaleDto::export_all(&cfg).unwrap();
    NewSaleLineDto::export_all(&cfg).unwrap();
    NewSaleDto::export_all(&cfg).unwrap();
    SaleKindDto::export_all(&cfg).unwrap();
    SaleCancelEffectDto::export_all(&cfg).unwrap();
    SaleCancellationDto::export_all(&cfg).unwrap();
    AvoirLineDto::export_all(&cfg).unwrap();
    NewAvoirDto::export_all(&cfg).unwrap();
    CancelDocumentDto::export_all(&cfg).unwrap();
    PartyKindDto::export_all(&cfg).unwrap();
    DebtKindDto::export_all(&cfg).unwrap();
    CustomerDto::export_all(&cfg).unwrap();
    CustomerWriteDto::export_all(&cfg).unwrap();
    NewCustomerDto::export_all(&cfg).unwrap();
    DebtEntryDto::export_all(&cfg).unwrap();
    CustomerLedgerDto::export_all(&cfg).unwrap();
    AdjustmentDto::export_all(&cfg).unwrap();
    PaymentMethodDto::export_all(&cfg).unwrap();
    PaymentAllocationDto::export_all(&cfg).unwrap();
    PaymentDto::export_all(&cfg).unwrap();
    CustomerPaymentsDto::export_all(&cfg).unwrap();
    NewPaymentDto::export_all(&cfg).unwrap();
    SupplierDebtKindDto::export_all(&cfg).unwrap();
    SupplierDto::export_all(&cfg).unwrap();
    SupplierWriteDto::export_all(&cfg).unwrap();
    NewSupplierDto::export_all(&cfg).unwrap();
    CloseSupplierDto::export_all(&cfg).unwrap();
    SupplierAllocationDto::export_all(&cfg).unwrap();
    SupplierEntryDto::export_all(&cfg).unwrap();
    SupplierLedgerDto::export_all(&cfg).unwrap();
    SupplierStatementDto::export_all(&cfg).unwrap();
    PurchaseStatusDto::export_all(&cfg).unwrap();
    PurchaseDto::export_all(&cfg).unwrap();
    PurchaseLineDto::export_all(&cfg).unwrap();
    PurchaseReceiptLineDto::export_all(&cfg).unwrap();
    PurchaseReceiptDto::export_all(&cfg).unwrap();
    PurchaseDetailDto::export_all(&cfg).unwrap();
    NewPurchaseLineDto::export_all(&cfg).unwrap();
    PaidNowDto::export_all(&cfg).unwrap();
    NewPurchaseDto::export_all(&cfg).unwrap();
    ReceiveLineDto::export_all(&cfg).unwrap();
    NewReceiptDto::export_all(&cfg).unwrap();
    CloseOrderDto::export_all(&cfg).unwrap();
    ExpenseCategoryDto::export_all(&cfg).unwrap();
    ExpenseDto::export_all(&cfg).unwrap();
    ExpensesDto::export_all(&cfg).unwrap();
    NewExpenseDto::export_all(&cfg).unwrap();
    TakingsDto::export_all(&cfg).unwrap();
    OutgoingsDto::export_all(&cfg).unwrap();
    CashPositionDto::export_all(&cfg).unwrap();
    StockDriftDto::export_all(&cfg).unwrap();
    StockRecountDto::export_all(&cfg).unwrap();
    LastStockRecountDto::export_all(&cfg).unwrap();
    ImportOutcomeDto::export_all(&cfg).unwrap();
    ImportRowDto::export_all(&cfg).unwrap();
    ImportDryRunDto::export_all(&cfg).unwrap();
    ImportAppliedDto::export_all(&cfg).unwrap();
    LabelSheetDto::export_all(&cfg).unwrap();
    DashboardFiguresDto::export_all(&cfg).unwrap();
    LowStockDto::export_all(&cfg).unwrap();
    TopProductDto::export_all(&cfg).unwrap();
    OwedDto::export_all(&cfg).unwrap();
    DashboardDto::export_all(&cfg).unwrap();
    DashboardSeriesPointDto::export_all(&cfg).unwrap();
    DashboardSeriesDto::export_all(&cfg).unwrap();
    AuditUserDto::export_all(&cfg).unwrap();
    AuditEntryDto::export_all(&cfg).unwrap();
    AuditLogDto::export_all(&cfg).unwrap();

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
