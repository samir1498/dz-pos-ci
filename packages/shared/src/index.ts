// Types generated from the Rust structs in crates/api. Run `just types-check`
// after changing them; CI fails on a stale checkout.
export type { ApiErrorDto } from "./generated/ApiErrorDto";
export type { ApiErrorPayloadDto } from "./generated/ApiErrorPayloadDto";
export type { HealthDto } from "./generated/HealthDto";
export type { NewProductDto } from "./generated/NewProductDto";
export type { ProductDto } from "./generated/ProductDto";
export type { UnitDto } from "./generated/UnitDto";

export { ApiError, createClient, isApiErrorBody } from "./client";
export type { ApiClient } from "./client";
export { formatCentimes, formatQty, parseAmountToCentimes } from "./money";
