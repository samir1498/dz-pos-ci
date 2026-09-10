// The dashboard: one answer, every figure of it derived from the ledgers
// when the screen asks.

import { z } from "zod";

import type { DashboardDto } from "../generated/DashboardDto";
import type { DashboardFiguresDto } from "../generated/DashboardFiguresDto";
import type { DashboardSeriesDto } from "../generated/DashboardSeriesDto";
import type { DashboardSeriesPointDto } from "../generated/DashboardSeriesPointDto";
import type { LowStockDto } from "../generated/LowStockDto";
import type { OwedDto } from "../generated/OwedDto";
import type { TopProductDto } from "../generated/TopProductDto";
import { day, exactInteger } from "./common";
import type { Assert, Matches } from "./drift";
import { cashPositionSchema, month } from "./expense";

/** Every amount is an exact integer, and none of them is bounded below zero:
 *  a month whose credit notes came to more than its sales reads a margin
 *  under zero, and that is the honest answer rather than a floor. */
export const dashboardFiguresSchema = z.object({
  sales_ttc_centimes: exactInteger,
  sales_count: exactInteger,
  lines_ht_centimes: exactInteger,
  discounts_centimes: exactInteger,
  sales_ht_centimes: exactInteger,
  cost_of_goods_centimes: exactInteger,
  margin_centimes: exactInteger,
  expenses_centimes: exactInteger,
}) satisfies z.ZodType<DashboardFiguresDto>;
type _DashboardFigures = Assert<Matches<DashboardFiguresDto, typeof dashboardFiguresSchema>>;

/** Quantities are thousandths of the unit, the way every quantity on the wire
 *  is. `qty_on_hand_milli` goes below zero on a shop whose count was wrong
 *  before its first inventory. */
export const lowStockSchema = z.object({
  product_id: z.number(),
  name: z.string(),
  qty_on_hand_milli: exactInteger,
  low_stock_at_milli: exactInteger,
}) satisfies z.ZodType<LowStockDto>;
type _LowStock = Assert<Matches<LowStockDto, typeof lowStockSchema>>;

/** `lines_ht_centimes` is this product's lines alone: a remise given off a
 *  whole document belongs to no line, so these margins do not add up to the
 *  month's. The ranking is what they are for. */
export const topProductSchema = z.object({
  product_id: z.number(),
  name: z.string(),
  qty_milli: exactInteger,
  lines_ht_centimes: exactInteger,
  cost_of_goods_centimes: exactInteger,
  margin_centimes: exactInteger,
}) satisfies z.ZodType<TopProductDto>;
type _TopProduct = Assert<Matches<TopProductDto, typeof topProductSchema>>;

export const owedSchema = z.object({
  total_centimes: exactInteger,
  parties: exactInteger,
}) satisfies z.ZodType<OwedDto>;
type _Owed = Assert<Matches<OwedDto, typeof owedSchema>>;

export const dashboardSchema = z.object({
  day,
  month,
  today: dashboardFiguresSchema,
  this_month: dashboardFiguresSchema,
  cash_today: cashPositionSchema,
  cash_this_month: cashPositionSchema,
  low_stock: z.array(lowStockSchema),
  top_by_quantity: z.array(topProductSchema),
  top_by_margin: z.array(topProductSchema),
  customer_debt: owedSchema,
  supplier_debt: owedSchema,
  open_purchases: exactInteger,
}) satisfies z.ZodType<DashboardDto>;
type _Dashboard = Assert<Matches<DashboardDto, typeof dashboardSchema>>;

/** One bucket of the chart. `from` and `to` are both included and equal on a
 *  day, so a tooltip names the range the figure covers. `cash_in_centimes` is
 *  the drawer's side alone and goes below zero on a day the shop paid out
 *  more than it took. */
export const dashboardSeriesPointSchema = z.object({
  from: day,
  to: day,
  figures: dashboardFiguresSchema,
  cash_in_centimes: exactInteger,
}) satisfies z.ZodType<DashboardSeriesPointDto>;
type _DashboardSeriesPoint = Assert<
  Matches<DashboardSeriesPointDto, typeof dashboardSeriesPointSchema>
>;

/** The chart behind the two columns: every day of the window, oldest first
 *  and none skipped, and the weeks they fold into. The weeks are cut back
 *  from `to`, so the oldest is the short one. */
export const dashboardSeriesSchema = z.object({
  from: day,
  to: day,
  days: z.array(dashboardSeriesPointSchema),
  weeks: z.array(dashboardSeriesPointSchema),
}) satisfies z.ZodType<DashboardSeriesDto>;
type _DashboardSeries = Assert<Matches<DashboardSeriesDto, typeof dashboardSeriesSchema>>;
