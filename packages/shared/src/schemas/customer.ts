// The fiche, what it owes, and the money that came against it.

import { z } from "zod";

import type { CustomerDto } from "../generated/CustomerDto";
import type { CustomerLedgerDto } from "../generated/CustomerLedgerDto";
import type { CustomerPaymentsDto } from "../generated/CustomerPaymentsDto";
import type { DebtEntryDto } from "../generated/DebtEntryDto";
import type { DebtKindDto } from "../generated/DebtKindDto";
import type { PartyKindDto } from "../generated/PartyKindDto";
import type { PaymentAllocationDto } from "../generated/PaymentAllocationDto";
import type { PaymentDto } from "../generated/PaymentDto";
import type { PaymentMethodDto } from "../generated/PaymentMethodDto";
import { exactInteger } from "./common";
import type { Assert, Matches } from "./drift";

export const partyKindSchema = z.enum(["company", "consumer"]) satisfies z.ZodType<PartyKindDto>;
type _PartyKind = Assert<Matches<PartyKindDto, typeof partyKindSchema>>;

export const debtKindSchema = z.enum([
  "opening",
  "sale",
  "payment",
  "avoir",
  "adjustment",
]) satisfies z.ZodType<DebtKindDto>;
type _DebtKind = Assert<Matches<DebtKindDto, typeof debtKindSchema>>;

/** A payment crosses in cash or by card and never on credit, which is why
 *  this is its own enum and not the sale's payment mode. */
export const paymentMethodSchema = z.enum(["cash", "card"]) satisfies z.ZodType<PaymentMethodDto>;
type _PaymentMethod = Assert<Matches<PaymentMethodDto, typeof paymentMethodSchema>>;

/** Every amount is an exact integer: a debt JSON.parse had to round is
 *  refused rather than shown to a shop. */
export const customerSchema = z.object({
  id: z.number(),
  shop_id: z.number(),
  name: z.string(),
  party_kind: partyKindSchema,
  phone: z.string().nullable(),
  address: z.string().nullable(),
  rc: z.string().nullable(),
  nif: z.string().nullable(),
  nis: z.string().nullable(),
  ai: z.string().nullable(),
  credit_limit_centimes: exactInteger.nullable(),
  warn_threshold_centimes: exactInteger.nullable(),
  notes: z.string().nullable(),
  active: z.boolean(),
  balance_centimes: exactInteger,
}) satisfies z.ZodType<CustomerDto>;
type _Customer = Assert<Matches<CustomerDto, typeof customerSchema>>;

export const debtEntrySchema = z.object({
  id: z.number(),
  customer_id: z.number(),
  document_id: z.number().nullable(),
  kind: debtKindSchema,
  debit_centimes: exactInteger,
  credit_centimes: exactInteger,
  balance_after_centimes: exactInteger,
  user_id: z.number(),
  note: z.string().nullable(),
  created_at: z.string(),
}) satisfies z.ZodType<DebtEntryDto>;
type _DebtEntry = Assert<Matches<DebtEntryDto, typeof debtEntrySchema>>;

export const customerLedgerSchema = z.object({
  customer_id: z.number(),
  balance_centimes: exactInteger,
  entries: z.array(debtEntrySchema),
}) satisfies z.ZodType<CustomerLedgerDto>;
type _CustomerLedger = Assert<Matches<CustomerLedgerDto, typeof customerLedgerSchema>>;

export const paymentAllocationSchema = z.object({
  document_id: z.number(),
  amount_centimes: exactInteger,
}) satisfies z.ZodType<PaymentAllocationDto>;
type _PaymentAllocation = Assert<Matches<PaymentAllocationDto, typeof paymentAllocationSchema>>;

export const paymentSchema = z.object({
  ledger_id: z.number(),
  customer_id: z.number(),
  amount_centimes: exactInteger,
  payment_mode: paymentMethodSchema.nullable(),
  note: z.string().nullable(),
  balance_after_centimes: exactInteger,
  allocations: z.array(paymentAllocationSchema),
  created_at: z.string(),
}) satisfies z.ZodType<PaymentDto>;
type _Payment = Assert<Matches<PaymentDto, typeof paymentSchema>>;

/** The balance in the envelope is the whole ledger's, not the newest
 *  payment's: a sale written after the last payment moved it. */
export const customerPaymentsSchema = z.object({
  customer_id: z.number(),
  balance_centimes: exactInteger,
  payments: z.array(paymentSchema),
}) satisfies z.ZodType<CustomerPaymentsDto>;
type _CustomerPayments = Assert<Matches<CustomerPaymentsDto, typeof customerPaymentsSchema>>;
