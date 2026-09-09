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
export type { SaleBalanceDto } from "./generated/SaleBalanceDto";
export type { SaleDto } from "./generated/SaleDto";
export type { SaleLineDto } from "./generated/SaleLineDto";
export type { SaleTvaDto } from "./generated/SaleTvaDto";
export type { SaleTotalsDto } from "./generated/SaleTotalsDto";
export type { SaleWarningDto } from "./generated/SaleWarningDto";
export type { SaleKindDto } from "./generated/SaleKindDto";
export type { NewSaleDto } from "./generated/NewSaleDto";
export type { NewSaleLineDto } from "./generated/NewSaleLineDto";
export type { SaleCancellationDto } from "./generated/SaleCancellationDto";
export type { AvoirLineDto } from "./generated/AvoirLineDto";
export type { NewAvoirDto } from "./generated/NewAvoirDto";
export type { CancelDocumentDto } from "./generated/CancelDocumentDto";

export {
  ApiError,
  createClient,
  isApiErrorBody,
  isBackup,
  isBackups,
  isCategory,
  isCustomer,
  isCustomerLedger,
  isCustomerPayments,
  isDebtEntry,
  isPayment,
  isRestore,
  isSale,
  isSettings,
  isStore,
} from "./client";
export type { ClientOptions } from "./client";
export type { ApiClient } from "./client";
export type { PrintLang, PrintPaper } from "./client";
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
