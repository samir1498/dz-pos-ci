// What the shop ordered, what arrived of it, and the papers each delivery
// was written on (features.md §1, Purchase).
//
// Quantities are thousandths of the unit and amounts are centimes, both exact
// integers: a landed cost JSON.parse had to round is a margin a shop would
// read wrong for the life of the stock.

import { z } from "zod";

import type { CloseOrderDto } from "../generated/CloseOrderDto";
import type { NewPurchaseDto } from "../generated/NewPurchaseDto";
import type { NewPurchaseLineDto } from "../generated/NewPurchaseLineDto";
import type { NewReceiptDto } from "../generated/NewReceiptDto";
import type { PaidNowDto } from "../generated/PaidNowDto";
import type { PurchaseDetailDto } from "../generated/PurchaseDetailDto";
import type { PurchaseDto } from "../generated/PurchaseDto";
import type { PurchaseLineDto } from "../generated/PurchaseLineDto";
import type { PurchaseReceiptDto } from "../generated/PurchaseReceiptDto";
import type { PurchaseReceiptLineDto } from "../generated/PurchaseReceiptLineDto";
import type { PurchaseStatusDto } from "../generated/PurchaseStatusDto";
import type { ReceiveLineDto } from "../generated/ReceiveLineDto";
import { day, exactInteger } from "./common";
import { paymentMethodSchema } from "./customer";
import type { Assert, Matches } from "./drift";

/** The five states of an order. The whole union crosses from the first
 *  version, because a screen that met an unknown state could only refuse the
 *  whole answer. */
export const purchaseStatusSchema = z.enum([
  "ordered",
  "partially_received",
  "received",
  "cancelled",
  "closed_short",
]) satisfies z.ZodType<PurchaseStatusDto>;
type _PurchaseStatus = Assert<Matches<PurchaseStatusDto, typeof purchaseStatusSchema>>;

/** The order as the list reads it. `purchase_date` and `due_date` are days on
 *  the shop's calendar; `created_at` is the moment the row was written. */
export const purchaseSchema = z.object({
  id: z.number(),
  shop_id: z.number(),
  supplier_id: z.number(),
  /** The bound every stored printed field carries (`MAX_FIELD_CHARS` in the
   *  core). */
  supplier_document_number: z.string().max(200).nullable(),
  purchase_date: day,
  due_date: day.nullable(),
  transport_centimes: exactInteger,
  extra_costs_centimes: exactInteger,
  status: purchaseStatusSchema,
  user_id: z.number(),
  note: z.string().max(200).nullable(),
  created_at: z.string(),
}) satisfies z.ZodType<PurchaseDto>;
type _Purchase = Assert<Matches<PurchaseDto, typeof purchaseSchema>>;

/** One product on the order. Both running totals are the file's own columns,
 *  so a screen counting the receipts itself would be a second answer. */
export const purchaseLineSchema = z.object({
  id: z.number(),
  product_id: z.number(),
  qty_ordered_milli: exactInteger,
  unit_cost_centimes: exactInteger,
  landed_unit_cost_centimes: exactInteger,
  qty_received_milli: exactInteger,
  qty_returned_milli: exactInteger,
}) satisfies z.ZodType<PurchaseLineDto>;
type _PurchaseLine = Assert<Matches<PurchaseLineDto, typeof purchaseLineSchema>>;

export const purchaseReceiptLineSchema = z.object({
  purchase_line_id: z.number(),
  qty_milli: exactInteger,
}) satisfies z.ZodType<PurchaseReceiptLineDto>;
type _PurchaseReceiptLine = Assert<
  Matches<PurchaseReceiptLineDto, typeof purchaseReceiptLineSchema>
>;

/** One bon de réception. Its series is `reception:<year>` and its number is
 *  the shop's own inside that year, the way a document's is inside its own. */
export const purchaseReceiptSchema = z.object({
  id: z.number(),
  series: z.string(),
  number: exactInteger,
  received_at: z.string(),
  user_id: z.number(),
  note: z.string().max(200).nullable(),
  lines: z.array(purchaseReceiptLineSchema),
}) satisfies z.ZodType<PurchaseReceiptDto>;
type _PurchaseReceipt = Assert<Matches<PurchaseReceiptDto, typeof purchaseReceiptSchema>>;

/** The whole order, which is what every route that changes one answers. */
export const purchaseDetailSchema = z.object({
  purchase: purchaseSchema,
  lines: z.array(purchaseLineSchema),
  receipts: z.array(purchaseReceiptSchema),
}) satisfies z.ZodType<PurchaseDetailDto>;
type _PurchaseDetail = Assert<Matches<PurchaseDetailDto, typeof purchaseDetailSchema>>;

/** A line of the form. A quantity of nothing orders nothing and a negative
 *  unit cost is money the supplier owes for delivering, so both are refused
 *  before the request leaves. */
export const newPurchaseLineSchema = z.object({
  product_id: z.number(),
  qty_ordered_milli: exactInteger.positive(),
  unit_cost_centimes: exactInteger.min(0),
}) satisfies z.ZodType<NewPurchaseLineDto>;
type _NewPurchaseLine = Assert<Matches<NewPurchaseLineDto, typeof newPurchaseLineSchema>>;

/** Money handed over as the order is written. The mode travels with it
 *  because the ledger's file refuses a payment that does not say how it was
 *  taken. */
export const paidNowSchema = z.object({
  amount_centimes: exactInteger.positive(),
  payment_mode: paymentMethodSchema,
}) satisfies z.ZodType<PaidNowDto>;
type _PaidNow = Assert<Matches<PaidNowDto, typeof paidNowSchema>>;

/** The order as the form sends it. An order with no line is refused here as
 *  well as in the core: the button should not be sending one. */
export const newPurchaseSchema = z.object({
  supplier_id: z.number(),
  supplier_document_number: z.string().max(200).nullable(),
  purchase_date: day,
  due_date: day.nullable(),
  transport_centimes: exactInteger.min(0),
  extra_costs_centimes: exactInteger.min(0),
  note: z.string().max(200).nullable(),
  lines: z.array(newPurchaseLineSchema).min(1),
  paid_now: paidNowSchema.nullable(),
  receive_now: z.boolean(),
}) satisfies z.ZodType<NewPurchaseDto>;
type _NewPurchase = Assert<Matches<NewPurchaseDto, typeof newPurchaseSchema>>;

export const receiveLineSchema = z.object({
  purchase_line_id: z.number(),
  qty_milli: exactInteger.positive(),
}) satisfies z.ZodType<ReceiveLineDto>;
type _ReceiveLine = Assert<Matches<ReceiveLineDto, typeof receiveLineSchema>>;

/** A delivery, or a return: the same shape asked in two directions, and the
 *  route is what says which. */
export const newReceiptSchema = z.object({
  lines: z.array(receiveLineSchema).min(1),
  note: z.string().max(200).nullable(),
}) satisfies z.ZodType<NewReceiptDto>;
type _NewReceipt = Assert<Matches<NewReceiptDto, typeof newReceiptSchema>>;

/** Why an order was cancelled or closed short. Never blank: writing off goods
 *  that never came is a decision, and the audit log is where it is written
 *  down. */
export const closeOrderSchema = z.object({
  reason: z.string().min(1).max(200),
}) satisfies z.ZodType<CloseOrderDto>;
type _CloseOrder = Assert<Matches<CloseOrderDto, typeof closeOrderSchema>>;
