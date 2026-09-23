// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]
// S7 of `a-kernel-crate-and-retail-as-the-first-module`: the 122 generated
// DTOs this file checks include every retail one and do not move (S5's own
// note), so `just types`/`types-check` runs this against the default,
// retail-on build; this whole file has nothing to compile with the feature
// off. C3 of `the-first-clinic-module-patients-queue-appointments` adds the
// clinic's three (`CLINIC_FILES`), and both recipes build with
// `--features clinic` on top so the one generated folder holds every DTO;
// a plain `cargo test --workspace` without the feature still runs this file
// and checks the shop's alone.
#![cfg(feature = "retail")]

//! Writes `packages/shared/src/generated`. Types cross the Rust/TypeScript
//! boundary once, generated, never hand-written twice (architecture.md).
//!
//! This is a test rather than a `#[ts(export)]` attribute so the write
//! happens in one named place: `just types-check` runs it into a temp
//! directory and diffs that against the committed one both ways, which is
//! what makes a stale checkout fail.

#[cfg(feature = "clinic")]
use dzpos_api::dto::{
    AbsenceBlockDto, AbsenceBlockWriteDto, AbsenceBlocksDto, AppointmentBookDto,
    AppointmentCallDto, AppointmentDto, AppointmentMoveDto, AppointmentsDto, BlockMadeDto,
    CallOutcomeDto, DayListDto, FreeSlotDto, OpenRangeDto, PatientDto, PatientWriteDto,
    QueueAddDto, QueueEntryDto, QueueOrderDto, SexDto, SlotMinutesDto, VisitTypeDto,
    VisitTypeWriteDto, VisitTypesDto, WorkingHoursDto, WorkingHoursWriteDto,
};
use dzpos_api::dto::{
    AdjustmentDto, ApiErrorDto, ApiErrorPayloadDto, AuditEntryDto, AuditLogDto, AuditUserDto,
    AvoirLineDto, BackupDto, BackupsDto, BuildInfoDto, CancelDocumentDto, CashPositionDto,
    CategoryDto, ClaimFirstOwnerDto, ClockDto, CloseOrderDto, CloseSupplierDto, CustomerDto,
    CustomerLedgerDto, CustomerPaymentsDto, CustomerWriteDto, DashboardDto, DashboardFiguresDto,
    DashboardSeriesDto, DashboardSeriesPointDto, DatedRegimeDto, DebtEntryDto, DebtKindDto,
    DeviceTokenDto, DiscountThresholdChangeDto, DocumentKindDto, DocumentStatusDto,
    ExpenseCategoryDto, ExpenseDto, ExpensesDto, FactureLayoutChoiceDto, FactureLayoutDto,
    HealthDto, ImportAppliedDto, ImportDryRunDto, ImportOutcomeDto, ImportRowDto, LabelSheetDto,
    LastStockRecountDto, LoginDto, LowStockDto, MeDto, NewAvoirDto, NewCustomerDto, NewExpenseDto,
    NewPaymentDto, NewProductDto, NewPurchaseDto, NewPurchaseLineDto, NewReceiptDto, NewSaleDto,
    NewSaleLineDto, NewShiftDto, NewSupplierDto, NewUserDto, OutgoingsDto, OwedDto, PaidNowDto,
    PairedDeviceDto, PairingQrDto, PartyKindDto, PaymentAllocationDto, PaymentDto,
    PaymentMethodDto, PaymentModeDto, PermissionDto, PrintLangChoiceDto, PrintLangDto, ProductDto,
    PurchaseDetailDto, PurchaseDto, PurchaseLineDto, PurchaseReceiptDto, PurchaseReceiptLineDto,
    PurchaseStatusDto, ReceiveLineDto, RegimeChangeDto, RegimeDto, RestoreDto, RoleDto,
    SaleBalanceDto, SaleCancelEffectDto, SaleCancellationDto, SaleDto, SaleKindDto, SaleLineDto,
    SaleTotalsDto, SaleTvaDto, SaleWarningDto, SessionDto, SessionIdleDto, SetPasswordDto,
    SetPinDto, SettingsDto, ShiftDto, ShiftReportDto, StaffDto, StockDriftDto, StockRecountDto,
    StoreDto, SupplierAllocationDto, SupplierDebtKindDto, SupplierDto, SupplierEntryDto,
    SupplierLedgerDto, SupplierStatementDto, SupplierWriteDto, TakingsDto, ThemeChoiceDto,
    ThemeDto, ThermalModeChoiceDto, ThermalModeDto, TillCountDto, TopProductDto, UnitDto, UserDto,
};
use ts_rs::{Config, TS};

const FILES: [&str; 122] = [
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
    "ThermalModeDto.ts",
    "ThermalModeChoiceDto.ts",
    "SettingsDto.ts",
    "RegimeChangeDto.ts",
    "BackupDto.ts",
    "BuildInfoDto.ts",
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
    "RefundDto.ts",
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
    "FactureLayoutChoiceDto.ts",
    "FactureLayoutDto.ts",
    "PrintLangDto.ts",
    "PrintLangChoiceDto.ts",
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
    "UserDto.ts",
    "NewUserDto.ts",
    "SetPinDto.ts",
    "SetPasswordDto.ts",
    "ClaimFirstOwnerDto.ts",
    "PairingQrDto.ts",
    "DeviceTokenDto.ts",
    "PairedDeviceDto.ts",
    "StaffDto.ts",
    "ShiftDto.ts",
    "ShiftReportDto.ts",
    "NewShiftDto.ts",
    "TillCountDto.ts",
];

/// The clinic's DTOs (`src/dto/patients.rs`, `src/dto/queue.rs` since C4
/// `src/dto/appointments.rs` since C5 and `src/dto/book_tools.rs` since C5b), apart from `FILES` because they only compile with the `clinic`
/// feature: the list check below reads their names off the source text in
/// every build, and the export writes them only when the feature is on.
const CLINIC_FILES: [&str; 25] = [
    "PatientDto.ts",
    "PatientWriteDto.ts",
    "SexDto.ts",
    "QueueEntryDto.ts",
    "QueueAddDto.ts",
    "QueueOrderDto.ts",
    "AppointmentDto.ts",
    "AppointmentsDto.ts",
    "AppointmentBookDto.ts",
    "AppointmentMoveDto.ts",
    "AppointmentCallDto.ts",
    "CallOutcomeDto.ts",
    "SlotMinutesDto.ts",
    "OpenRangeDto.ts",
    "WorkingHoursDto.ts",
    "WorkingHoursWriteDto.ts",
    "AbsenceBlockDto.ts",
    "AbsenceBlocksDto.ts",
    "AbsenceBlockWriteDto.ts",
    "BlockMadeDto.ts",
    "VisitTypeDto.ts",
    "VisitTypesDto.ts",
    "VisitTypeWriteDto.ts",
    "FreeSlotDto.ts",
    "DayListDto.ts",
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
///
/// Every file in `src/dto/` is named here rather than globbed, because a
/// glob is not something `include_str!` can do and a domain file nobody
/// listed would take its DTOs out of this check without failing anything.
/// A new domain file is added here in the same commit that creates it.
const DTO_SOURCE: &str = concat!(
    include_str!("../src/dto/mod.rs"),
    include_str!("../src/dto/common.rs"),
    include_str!("../src/dto/meta.rs"),
    include_str!("../src/dto/products.rs"),
    include_str!("../src/dto/categories.rs"),
    include_str!("../src/dto/settings.rs"),
    include_str!("../src/dto/backups.rs"),
    include_str!("../src/dto/sales.rs"),
    include_str!("../src/dto/customers.rs"),
    include_str!("../src/dto/suppliers.rs"),
    include_str!("../src/dto/expenses.rs"),
    include_str!("../src/dto/stock.rs"),
    include_str!("../src/dto/purchases.rs"),
    include_str!("../src/dto/dashboard.rs"),
    include_str!("../src/dto/import.rs"),
    include_str!("../src/dto/auth.rs"),
    include_str!("../src/dto/audit.rs"),
    include_str!("../src/dto/users.rs"),
    include_str!("../src/dto/pairing.rs"),
    include_str!("../src/dto/till.rs"),
    include_str!("../src/dto/patients.rs"),
    include_str!("../src/dto/queue.rs"),
    include_str!("../src/dto/appointments.rs"),
    include_str!("../src/dto/book_tools.rs")
);

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
    listed.extend(CLINIC_FILES);
    listed.sort_unstable();
    assert_eq!(
        named, listed,
        "the export_to names in crates/api/src/dto/ and FILES in this test differ"
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
    FactureLayoutDto::export_all(&cfg).unwrap();
    FactureLayoutChoiceDto::export_all(&cfg).unwrap();
    ThermalModeDto::export_all(&cfg).unwrap();
    ThermalModeChoiceDto::export_all(&cfg).unwrap();
    PrintLangDto::export_all(&cfg).unwrap();
    PrintLangChoiceDto::export_all(&cfg).unwrap();
    RegimeChangeDto::export_all(&cfg).unwrap();
    DiscountThresholdChangeDto::export_all(&cfg).unwrap();
    BackupDto::export_all(&cfg).unwrap();
    BuildInfoDto::export_all(&cfg).unwrap();
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
    UserDto::export_all(&cfg).unwrap();
    NewUserDto::export_all(&cfg).unwrap();
    SetPinDto::export_all(&cfg).unwrap();
    SetPasswordDto::export_all(&cfg).unwrap();
    ClaimFirstOwnerDto::export_all(&cfg).unwrap();
    PairingQrDto::export_all(&cfg).unwrap();
    DeviceTokenDto::export_all(&cfg).unwrap();
    PairedDeviceDto::export_all(&cfg).unwrap();
    StaffDto::export_all(&cfg).unwrap();
    ShiftDto::export_all(&cfg).unwrap();
    ShiftReportDto::export_all(&cfg).unwrap();
    NewShiftDto::export_all(&cfg).unwrap();
    TillCountDto::export_all(&cfg).unwrap();
    #[cfg(feature = "clinic")]
    {
        PatientDto::export_all(&cfg).unwrap();
        PatientWriteDto::export_all(&cfg).unwrap();
        SexDto::export_all(&cfg).unwrap();
        QueueEntryDto::export_all(&cfg).unwrap();
        QueueAddDto::export_all(&cfg).unwrap();
        QueueOrderDto::export_all(&cfg).unwrap();
        AppointmentDto::export_all(&cfg).unwrap();
        AppointmentCallDto::export_all(&cfg).unwrap();
        CallOutcomeDto::export_all(&cfg).unwrap();
        AppointmentsDto::export_all(&cfg).unwrap();
        AppointmentBookDto::export_all(&cfg).unwrap();
        AppointmentMoveDto::export_all(&cfg).unwrap();
        SlotMinutesDto::export_all(&cfg).unwrap();
        OpenRangeDto::export_all(&cfg).unwrap();
        WorkingHoursDto::export_all(&cfg).unwrap();
        WorkingHoursWriteDto::export_all(&cfg).unwrap();
        AbsenceBlockDto::export_all(&cfg).unwrap();
        AbsenceBlocksDto::export_all(&cfg).unwrap();
        AbsenceBlockWriteDto::export_all(&cfg).unwrap();
        BlockMadeDto::export_all(&cfg).unwrap();
        VisitTypeDto::export_all(&cfg).unwrap();
        VisitTypesDto::export_all(&cfg).unwrap();
        VisitTypeWriteDto::export_all(&cfg).unwrap();
        FreeSlotDto::export_all(&cfg).unwrap();
        DayListDto::export_all(&cfg).unwrap();
        for name in CLINIC_FILES {
            assert!(dir.join(name).exists(), "{name} was not written");
        }
    }

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
