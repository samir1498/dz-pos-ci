// The stock recount: what one run found and what the last one left behind.
// Quantities are thousandths of a unit, so every figure here is an integer
// the same way a centime figure is: a fractional one means the answer came
// from something other than the server this client was generated against.

import { z } from "zod";

import type { LastStockRecountDto } from "../generated/LastStockRecountDto";
import type { StockDriftDto } from "../generated/StockDriftDto";
import type { StockRecountDto } from "../generated/StockRecountDto";
import { day, exactInteger } from "./common";
import type { Assert, Matches } from "./drift";

export const stockDriftSchema = z.object({
  product_id: exactInteger,
  name: z.string(),
  cached_milli: exactInteger,
  ledger_milli: exactInteger,
  difference_milli: exactInteger,
}) satisfies z.ZodType<StockDriftDto>;
type _StockDrift = Assert<Matches<StockDriftDto, typeof stockDriftSchema>>;

export const stockRecountSchema = z.object({
  day,
  products_checked: exactInteger,
  drifts: z.array(stockDriftSchema),
}) satisfies z.ZodType<StockRecountDto>;
type _StockRecount = Assert<Matches<StockRecountDto, typeof stockRecountSchema>>;

/** `last_run_day` is null on a shop that has never recounted, which is what a
 *  file says before the daily loop has woken once. */
export const lastStockRecountSchema = z.object({
  last_run_day: day.nullable(),
  drifts: z.array(stockDriftSchema),
}) satisfies z.ZodType<LastStockRecountDto>;
type _LastStockRecount = Assert<Matches<LastStockRecountDto, typeof lastStockRecountSchema>>;
