// The supplier fiche, what the shop owes it, and the movements behind that
// figure. The mirror of `./customer` on the supply side.

import { z } from "zod";

import type { SupplierAllocationDto } from "../generated/SupplierAllocationDto";
import type { SupplierDebtKindDto } from "../generated/SupplierDebtKindDto";
import type { SupplierDto } from "../generated/SupplierDto";
import type { SupplierEntryDto } from "../generated/SupplierEntryDto";
import type { SupplierLedgerDto } from "../generated/SupplierLedgerDto";
import type { SupplierStatementDto } from "../generated/SupplierStatementDto";
import { day, exactInteger } from "./common";
import { paymentMethodSchema } from "./customer";
import type { Assert, Matches } from "./drift";

/** T3 writes the `purchase` and `return` rows; the whole union crosses from
 *  the first version, because a screen that met an unknown kind could only
 *  refuse the whole answer. */
export const supplierDebtKindSchema = z.enum([
  "opening",
  "purchase",
  "payment",
  "return",
  "adjustment",
]) satisfies z.ZodType<SupplierDebtKindDto>;
type _SupplierDebtKind = Assert<Matches<SupplierDebtKindDto, typeof supplierDebtKindSchema>>;

/** Every amount is an exact integer: a debt JSON.parse had to round is
 *  refused rather than shown to a shop. */
export const supplierSchema = z.object({
  id: z.number(),
  shop_id: z.number(),
  name: z.string(),
  phone: z.string().nullable(),
  address: z.string().nullable(),
  rc: z.string().nullable(),
  nif: z.string().nullable(),
  nis: z.string().nullable(),
  ai: z.string().nullable(),
  notes: z.string().nullable(),
  active: z.boolean(),
  balance_centimes: exactInteger,
}) satisfies z.ZodType<SupplierDto>;
type _Supplier = Assert<Matches<SupplierDto, typeof supplierSchema>>;

export const supplierAllocationSchema = z.object({
  purchase_id: z.number(),
  amount_centimes: exactInteger,
}) satisfies z.ZodType<SupplierAllocationDto>;
type _SupplierAllocation = Assert<Matches<SupplierAllocationDto, typeof supplierAllocationSchema>>;

/** The allocations are empty on every kind but a payment: nothing else
 *  settles an order. */
export const supplierEntrySchema = z.object({
  id: z.number(),
  supplier_id: z.number(),
  purchase_id: z.number().nullable(),
  kind: supplierDebtKindSchema,
  debit_centimes: exactInteger,
  credit_centimes: exactInteger,
  balance_after_centimes: exactInteger,
  payment_mode: paymentMethodSchema.nullable(),
  user_id: z.number(),
  note: z.string().nullable(),
  allocations: z.array(supplierAllocationSchema),
  created_at: z.string(),
}) satisfies z.ZodType<SupplierEntryDto>;
type _SupplierEntry = Assert<Matches<SupplierEntryDto, typeof supplierEntrySchema>>;

export const supplierLedgerSchema = z.object({
  supplier_id: z.number(),
  balance_centimes: exactInteger,
  entries: z.array(supplierEntrySchema),
}) satisfies z.ZodType<SupplierLedgerDto>;
type _SupplierLedger = Assert<Matches<SupplierLedgerDto, typeof supplierLedgerSchema>>;

/** Both balances are the core's running column, so a screen showing them
 *  adds nothing up; the days are `YYYY-MM-DD` and both ends are included. */
export const supplierStatementSchema = z.object({
  supplier_id: z.number(),
  from: day,
  to: day,
  opening_centimes: exactInteger,
  entries: z.array(supplierEntrySchema),
  closing_centimes: exactInteger,
}) satisfies z.ZodType<SupplierStatementDto>;
type _SupplierStatement = Assert<Matches<SupplierStatementDto, typeof supplierStatementSchema>>;
