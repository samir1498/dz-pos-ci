// Types generated from the Rust structs in crates/api. Run `just types-check`
// after changing them; CI fails on a stale checkout.
export type { ApiErrorDto } from "./generated/ApiErrorDto";
export type { ApiErrorPayloadDto } from "./generated/ApiErrorPayloadDto";
export type { AdjustmentDto } from "./generated/AdjustmentDto";
export type { CategoryDto } from "./generated/CategoryDto";
export type { CustomerDto } from "./generated/CustomerDto";
export type { CustomerLedgerDto } from "./generated/CustomerLedgerDto";
export type { CustomerPaymentsDto } from "./generated/CustomerPaymentsDto";
export type { CustomerWriteDto } from "./generated/CustomerWriteDto";
export type { NewPaymentDto } from "./generated/NewPaymentDto";
export type { PaymentAllocationDto } from "./generated/PaymentAllocationDto";
export type { PaymentDto } from "./generated/PaymentDto";
export type { PaymentMethodDto } from "./generated/PaymentMethodDto";
export type { DebtEntryDto } from "./generated/DebtEntryDto";
export type { DebtKindDto } from "./generated/DebtKindDto";
export type { NewCustomerDto } from "./generated/NewCustomerDto";
export type { PartyKindDto } from "./generated/PartyKindDto";
export type { ClockDto } from "./generated/ClockDto";
export type { HealthDto } from "./generated/HealthDto";
export type { NewProductDto } from "./generated/NewProductDto";
export type { ProductDto } from "./generated/ProductDto";
export type { UnitDto } from "./generated/UnitDto";
export type { StoreDto } from "./generated/StoreDto";
export type { RegimeDto } from "./generated/RegimeDto";
export type { DatedRegimeDto } from "./generated/DatedRegimeDto";
export type { SettingsDto } from "./generated/SettingsDto";
export type { RegimeChangeDto } from "./generated/RegimeChangeDto";
export type { BackupDto } from "./generated/BackupDto";
export type { BackupsDto } from "./generated/BackupsDto";
export type { RestoreDto } from "./generated/RestoreDto";
export type { PaymentModeDto } from "./generated/PaymentModeDto";
export type { DocumentKindDto } from "./generated/DocumentKindDto";
export type { DocumentStatusDto } from "./generated/DocumentStatusDto";
export type { CloseSupplierDto } from "./generated/CloseSupplierDto";
export type { NewSupplierDto } from "./generated/NewSupplierDto";
export type { SupplierAllocationDto } from "./generated/SupplierAllocationDto";
export type { SupplierDebtKindDto } from "./generated/SupplierDebtKindDto";
export type { SupplierDto } from "./generated/SupplierDto";
export type { SupplierEntryDto } from "./generated/SupplierEntryDto";
export type { SupplierLedgerDto } from "./generated/SupplierLedgerDto";
export type { SupplierStatementDto } from "./generated/SupplierStatementDto";
export type { SupplierWriteDto } from "./generated/SupplierWriteDto";
export type { CloseOrderDto } from "./generated/CloseOrderDto";
export type { NewPurchaseDto } from "./generated/NewPurchaseDto";
export type { NewPurchaseLineDto } from "./generated/NewPurchaseLineDto";
export type { NewReceiptDto } from "./generated/NewReceiptDto";
export type { PaidNowDto } from "./generated/PaidNowDto";
export type { PurchaseDetailDto } from "./generated/PurchaseDetailDto";
export type { PurchaseDto } from "./generated/PurchaseDto";
export type { PurchaseLineDto } from "./generated/PurchaseLineDto";
export type { PurchaseReceiptDto } from "./generated/PurchaseReceiptDto";
export type { PurchaseReceiptLineDto } from "./generated/PurchaseReceiptLineDto";
export type { PurchaseStatusDto } from "./generated/PurchaseStatusDto";
export type { ReceiveLineDto } from "./generated/ReceiveLineDto";
export type { SaleBalanceDto } from "./generated/SaleBalanceDto";
export type { SaleDto } from "./generated/SaleDto";
export type { SaleLineDto } from "./generated/SaleLineDto";
export type { SaleTvaDto } from "./generated/SaleTvaDto";
export type { SaleTotalsDto } from "./generated/SaleTotalsDto";
export type { SaleWarningDto } from "./generated/SaleWarningDto";
export type { SaleKindDto } from "./generated/SaleKindDto";
export type { NewSaleDto } from "./generated/NewSaleDto";
export type { NewSaleLineDto } from "./generated/NewSaleLineDto";
export type { SaleCancelEffectDto } from "./generated/SaleCancelEffectDto";
export type { SaleCancellationDto } from "./generated/SaleCancellationDto";
export type { AvoirLineDto } from "./generated/AvoirLineDto";
export type { NewAvoirDto } from "./generated/NewAvoirDto";
export type { CancelDocumentDto } from "./generated/CancelDocumentDto";
export type { CashPositionDto } from "./generated/CashPositionDto";
export type { ExpenseCategoryDto } from "./generated/ExpenseCategoryDto";
export type { ExpenseDto } from "./generated/ExpenseDto";
export type { ExpensesDto } from "./generated/ExpensesDto";
export type { NewExpenseDto } from "./generated/NewExpenseDto";
export type { OutgoingsDto } from "./generated/OutgoingsDto";
export type { LastStockRecountDto } from "./generated/LastStockRecountDto";
export type { StockDriftDto } from "./generated/StockDriftDto";
export type { StockRecountDto } from "./generated/StockRecountDto";
export type { TakingsDto } from "./generated/TakingsDto";

export { ApiError, createClient } from "./client";
// One zod schema per DTO, the check the client runs on every answer. They
// replace the hand guards this file used to export: a caller that has a
// payload from somewhere other than the client checks it with the same schema
// the client would have used, rather than with a second reading of the shape.
export * from "./schemas";
export type { ClientOptions } from "./client";
export type { ApiClient } from "./client";
export type { Download, ExportKind, PrintLang, PrintPaper } from "./client";
export type { ImportAppliedDto } from "./generated/ImportAppliedDto";
export type { ImportDryRunDto } from "./generated/ImportDryRunDto";
export type { ImportOutcomeDto } from "./generated/ImportOutcomeDto";
export type { ImportRowDto } from "./generated/ImportRowDto";
export type { LabelSheetDto } from "./generated/LabelSheetDto";
export { formatCentimes, formatQty, parseAmountToCentimes, parseQtyToMilli } from "./money";
export {
  BPS_PER_WHOLE,
  MILLI_PER_UNIT,
  MoneyError,
  STAMP_BAND_LOW,
  STAMP_BAND_MID,
  STAMP_FLOOR,
  STAMP_MIN,
  STAMP_RATE_HIGH,
  STAMP_RATE_LOW,
  STAMP_RATE_MID,
  STAMP_TRANCHE,
  computeTotals,
  lineTotal,
  pct,
  stamp,
} from "./totals";
export type {
  MoneyErrorVariant,
  Totals,
  TotalsLine,
  TotalsOptions,
  TvaGroup,
} from "./totals";
