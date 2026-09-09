// Types generated from the Rust structs in crates/api. Run `just types-check`
// after changing them; CI fails on a stale checkout.
export type { ApiErrorDto } from "./generated/ApiErrorDto";
export type { ApiErrorPayloadDto } from "./generated/ApiErrorPayloadDto";
export type { CategoryDto } from "./generated/CategoryDto";
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
export type { SaleDto } from "./generated/SaleDto";
export type { SaleLineDto } from "./generated/SaleLineDto";
export type { SaleTvaDto } from "./generated/SaleTvaDto";
export type { SaleTotalsDto } from "./generated/SaleTotalsDto";
export type { NewSaleDto } from "./generated/NewSaleDto";
export type { NewSaleLineDto } from "./generated/NewSaleLineDto";

export {
  ApiError,
  createClient,
  isApiErrorBody,
  isBackup,
  isBackups,
  isCategory,
  isRestore,
  isSale,
  isSettings,
  isStore,
} from "./client";
export type { ClientOptions } from "./client";
export type { ApiClient } from "./client";
export { formatCentimes, formatQty, parseAmountToCentimes, parseQtyToMilli } from "./money";
