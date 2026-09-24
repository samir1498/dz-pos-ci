// What the till reads to ring anything up: the units, the categories and the
// products.

import { z } from "zod";

import type { CategoryDto } from "../generated/CategoryDto";
import type { ContenanceUnitDto } from "../generated/ContenanceUnitDto";
import type { ProductDto } from "../generated/ProductDto";
import type { UnitDto } from "../generated/UnitDto";
import { exactInteger } from "./common";
import type { Assert, Matches } from "./drift";

export const unitSchema = z.enum(["piece", "kg", "litre", "box"]) satisfies z.ZodType<UnitDto>;
type _Unit = Assert<Matches<UnitDto, typeof unitSchema>>;

/** The four units a pack size is written in (T13). */
export const contenanceUnitSchema = z.enum(["g", "kg", "ml", "l"]) satisfies z.ZodType<ContenanceUnitDto>;
type _ContenanceUnit = Assert<Matches<ContenanceUnitDto, typeof contenanceUnitSchema>>;

export const categorySchema = z.object({
  id: z.number(),
  shop_id: z.number(),
  name: z.string(),
  default_rate_bps: z.number(),
}) satisfies z.ZodType<CategoryDto>;
type _Category = Assert<Matches<CategoryDto, typeof categorySchema>>;

/** The prices and the quantities are exact integers, the ids and the rate are
 *  numbers the client passes through and never adds up.
 *
 *  `cost_centimes` is nullable, not optional: the field is always on the
 *  wire, but `GET /products` redacts it to `null` for a caller who does not
 *  hold `see_cost_and_margin` (crates/api/src/routes/products.rs::redact_cost,
 *  M4 T5 review, 2026-09-11). A screen that reads it has to handle the
 *  missing case rather than assume a number. */
export const productSchema = z.object({
  id: z.number(),
  shop_id: z.number(),
  name: z.string(),
  barcode: z.string().nullable(),
  category_id: z.number().nullable(),
  unit: unitSchema,
  cost_centimes: exactInteger.nullable(),
  selling_centimes: exactInteger,
  wholesale_centimes: exactInteger.nullable(),
  qty_on_hand_milli: exactInteger,
  low_stock_at_milli: exactInteger,
  rate_bps: z.number(),
  active: z.boolean(),
  /** The pack size (T13): thousandths of the unit, both null or both set. */
  contenance_milli: exactInteger.positive().nullable(),
  contenance_unit: contenanceUnitSchema.nullable(),
  /** The name with the pack size after it, spelled by the core. */
  display_name: z.string(),
}) satisfies z.ZodType<ProductDto>;
type _Product = Assert<Matches<ProductDto, typeof productSchema>>;
