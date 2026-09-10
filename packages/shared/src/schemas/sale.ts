// Every document the till issues, and the blocks one is made of.

import { z } from "zod";

import type { DocumentKindDto } from "../generated/DocumentKindDto";
import type { DocumentStatusDto } from "../generated/DocumentStatusDto";
import type { PaymentModeDto } from "../generated/PaymentModeDto";
import type { SaleBalanceDto } from "../generated/SaleBalanceDto";
import type { SaleCancelEffectDto } from "../generated/SaleCancelEffectDto";
import type { SaleCancellationDto } from "../generated/SaleCancellationDto";
import type { SaleDto } from "../generated/SaleDto";
import type { SaleKindDto } from "../generated/SaleKindDto";
import type { SaleLineDto } from "../generated/SaleLineDto";
import type { SaleTotalsDto } from "../generated/SaleTotalsDto";
import type { SaleTvaDto } from "../generated/SaleTvaDto";
import type { SaleWarningDto } from "../generated/SaleWarningDto";
import { exactInteger } from "./common";
import type { Assert, Matches } from "./drift";
import { regimeSchema, storeSchema } from "./settings";

export const documentKindSchema = z.enum([
  "ticket",
  "facture",
  "proforma",
  "bon_de_livraison",
  "avoir",
  "bon_de_reception",
  "quittance",
]) satisfies z.ZodType<DocumentKindDto>;
type _DocumentKind = Assert<Matches<DocumentKindDto, typeof documentKindSchema>>;

/** The three the till can be asked to list. A subset of the kinds above, and
 *  its own Rust enum, so it gets its own schema rather than a slice of one. */
export const saleKindSchema = z.enum([
  "ticket",
  "facture",
  "proforma",
]) satisfies z.ZodType<SaleKindDto>;
type _SaleKind = Assert<Matches<SaleKindDto, typeof saleKindSchema>>;

export const documentStatusSchema = z.enum([
  "issued",
  "cancelled",
]) satisfies z.ZodType<DocumentStatusDto>;
type _DocumentStatus = Assert<Matches<DocumentStatusDto, typeof documentStatusSchema>>;

export const paymentModeSchema = z.enum([
  "cash",
  "card",
  "credit",
]) satisfies z.ZodType<PaymentModeDto>;
type _PaymentMode = Assert<Matches<PaymentModeDto, typeof paymentModeSchema>>;

export const saleWarningSchema = z.enum(["near_limit"]) satisfies z.ZodType<SaleWarningDto>;
type _SaleWarning = Assert<Matches<SaleWarningDto, typeof saleWarningSchema>>;

export const saleLineSchema = z.object({
  id: z.number(),
  position: z.number(),
  product_id: z.number().nullable(),
  name: z.string(),
  barcode: z.string().nullable(),
  qty_milli: exactInteger,
  unit_price_centimes: exactInteger,
  line_discount_centimes: exactInteger,
  rate_bps: z.number(),
  line_total_centimes: exactInteger,
  ref_line_id: z.number().nullable(),
}) satisfies z.ZodType<SaleLineDto>;
type _SaleLine = Assert<Matches<SaleLineDto, typeof saleLineSchema>>;

/** What a cancellation left on the document it annulled. Whole or absent: a
 *  screen never has to ask whether the date is there while the reason is
 *  not, so the nullable sits at the block and not on its fields. */
export const saleCancellationSchema = z.object({
  cancelled_at: z.string(),
  cancelled_by: z.number(),
  reason: z.string(),
  avoir_document_id: z.number().nullable(),
}) satisfies z.ZodType<SaleCancellationDto>;
type _SaleCancellation = Assert<Matches<SaleCancellationDto, typeof saleCancellationSchema>>;

/** What cancelling this document would do. Discriminated, because the amount
 *  belongs to exactly one of the three: a screen that read an amount off
 *  `stock_back` would show a figure the server never sent, and the union
 *  drops it rather than passing it on. */
export const saleCancelEffectSchema = z.discriminatedUnion("effect", [
  z.object({ effect: z.literal("nothing_to_reverse") }),
  z.object({ effect: z.literal("stock_back") }),
  z.object({ effect: z.literal("stock_back_and_avoir"), amount_centimes: exactInteger }),
]) satisfies z.ZodType<SaleCancelEffectDto>;
type _SaleCancelEffect = Assert<Matches<SaleCancelEffectDto, typeof saleCancelEffectSchema>>;

export const saleTvaSchema = z.object({
  rate_bps: z.number(),
  base_centimes: exactInteger,
  amount_centimes: exactInteger,
}) satisfies z.ZodType<SaleTvaDto>;
type _SaleTva = Assert<Matches<SaleTvaDto, typeof saleTvaSchema>>;

/** The balance triple. Three exact integers or the whole block is null: two of
 *  three would be a closing balance its own opening balance does not
 *  explain. */
export const saleBalanceSchema = z.object({
  old_balance_centimes: exactInteger,
  remaining_debt_centimes: exactInteger,
  total_debt_centimes: exactInteger,
}) satisfies z.ZodType<SaleBalanceDto>;
type _SaleBalance = Assert<Matches<SaleBalanceDto, typeof saleBalanceSchema>>;

/** Every column of the totals table, each an exact integer of centimes: a
 *  total JSON.parse had to round is refused rather than printed. */
export const saleTotalsSchema = z.object({
  total_ht_centimes: exactInteger,
  discount_centimes: exactInteger,
  subtotal_ht_centimes: exactInteger,
  tva_centimes: exactInteger,
  total_ttc_centimes: exactInteger,
  stamp_centimes: exactInteger,
  net_to_pay_centimes: exactInteger,
}) satisfies z.ZodType<SaleTotalsDto>;
type _SaleTotals = Assert<Matches<SaleTotalsDto, typeof saleTotalsSchema>>;

export const saleSchema = z.object({
  id: z.number(),
  shop_id: z.number(),
  kind: documentKindSchema,
  series: z.string(),
  number: exactInteger,
  printed_number: z.string(),
  issued_at: z.string(),
  user_id: z.number(),
  regime: regimeSchema,
  payment_mode: paymentModeSchema,
  seller: storeSchema,
  customer_id: z.number().nullable(),
  ref_document_id: z.number().nullable(),
  buyer_name: z.string().nullable(),
  balance: saleBalanceSchema.nullable(),
  totals: saleTotalsSchema,
  tva: z.array(saleTvaSchema),
  tendered_centimes: exactInteger.nullable(),
  change_centimes: exactInteger.nullable(),
  status: documentStatusSchema,
  cancellation: saleCancellationSchema.nullable(),
  lines: z.array(saleLineSchema),
  cancel_effect: saleCancelEffectSchema.nullable(),
  warning: saleWarningSchema.nullable(),
}) satisfies z.ZodType<SaleDto>;
type _Sale = Assert<Matches<SaleDto, typeof saleSchema>>;
