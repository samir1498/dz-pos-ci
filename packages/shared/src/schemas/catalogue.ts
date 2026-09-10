// What the till reads to ring anything up: the units, the categories and the
// products.

import { z } from "zod";

import type { CategoryDto } from "../generated/CategoryDto";
import type { ProductDto } from "../generated/ProductDto";
import type { UnitDto } from "../generated/UnitDto";
import { exactInteger } from "./common";
import type { Assert, Matches } from "./drift";

export const unitSchema = z.enum(["piece", "kg", "litre", "box"]) satisfies z.ZodType<UnitDto>;
type _Unit = Assert<Matches<UnitDto, typeof unitSchema>>;

export const categorySchema = z.object({
  id: z.number(),
  shop_id: z.number(),
  name: z.string(),
  default_rate_bps: z.number(),
}) satisfies z.ZodType<CategoryDto>;
type _Category = Assert<Matches<CategoryDto, typeof categorySchema>>;

/** The prices and the quantities are exact integers, the ids and the rate are
 *  numbers the client passes through and never adds up. */
export const productSchema = z.object({
  id: z.number(),
  shop_id: z.number(),
  name: z.string(),
  barcode: z.string().nullable(),
  category_id: z.number().nullable(),
  unit: unitSchema,
  cost_centimes: exactInteger,
  selling_centimes: exactInteger,
  wholesale_centimes: exactInteger.nullable(),
  qty_on_hand_milli: exactInteger,
  low_stock_at_milli: exactInteger,
  rate_bps: z.number(),
  active: z.boolean(),
}) satisfies z.ZodType<ProductDto>;
type _Product = Assert<Matches<ProductDto, typeof productSchema>>;
