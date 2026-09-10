// Contract: what the till reads before it can ring anything up. The unit is
// one of four words the migration's CHECK allows, and every price and
// quantity is an exact integer, so a product whose selling price came back
// rounded never reaches a cart line.

import { describe, expect, test } from "vitest";

import type { CategoryDto } from "../generated/CategoryDto";
import type { ProductDto } from "../generated/ProductDto";
import { categorySchema, productSchema, unitSchema } from "./catalogue";

const product: ProductDto = {
  id: 1,
  shop_id: 1,
  name: "Huile Elio 5L",
  barcode: "2000010000017",
  category_id: 1,
  unit: "piece",
  cost_centimes: 820,
  selling_centimes: 920,
  wholesale_centimes: null,
  qty_on_hand_milli: 24_000,
  low_stock_at_milli: 10_000,
  rate_bps: 1900,
  active: true,
};

const category: CategoryDto = { id: 1, shop_id: 1, name: "Épicerie", default_rate_bps: 1900 };

describe("unitSchema", () => {
  test("takes a unit the migration allows", () => {
    expect(unitSchema.parse("kg")).toBe("kg");
  });

  test("refuses a unit the API does not write", () => {
    expect(unitSchema.safeParse("gramme").success).toBe(false);
  });

  test("carries the four the migration's CHECK allows and no fifth", () => {
    expect(unitSchema.options).toEqual(["piece", "kg", "litre", "box"]);
  });
});

describe("categorySchema", () => {
  test("takes the row the add-product form reads its rate from", () => {
    expect(categorySchema.parse(category)).toEqual(category);
  });

  test("refuses a rate that is not a number at all", () => {
    // The rate is checked as a number and not as an exact integer, the way
    // the guard it replaced did: basis points are the core's and the client
    // passes them through. A string or a null is still garbage, and a row
    // carrying one would put an empty TVA column on a form.
    for (const rate of ["1900", null, "dix-neuf"]) {
      expect(categorySchema.safeParse({ ...category, default_rate_bps: rate }).success).toBe(false);
    }
  });

  test("refuses a row with no rate", () => {
    const { default_rate_bps: _rate, ...withoutRate } = category;
    expect(categorySchema.safeParse(withoutRate).success).toBe(false);
  });
});

describe("productSchema", () => {
  test("takes a product with the optional fields null", () => {
    expect(productSchema.parse(product)).toEqual(product);
  });

  test("refuses a price JSON.parse had to round", () => {
    expect(productSchema.safeParse({ ...product, selling_centimes: 9.2 }).success).toBe(false);
  });
});
