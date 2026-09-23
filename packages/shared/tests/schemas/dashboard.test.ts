// Contract: what the dashboard looks like on the wire. Every amount is an
// exact integer and the day and the month carry their own shapes, because a
// screen that took a day for a month would show a figure for the wrong range
// and say nothing about it.

import { describe, expect, test } from "vitest";

import type { CashPositionDto } from "../../src/generated/CashPositionDto";
import type { DashboardDto } from "../../src/generated/DashboardDto";
import { dashboardSchema } from "../../src/schemas/dashboard";

const position: CashPositionDto = {
  from: "2026-09-15",
  to: "2026-09-15",
  cash_in: {
    sales_centimes: 302_000,
    stamp_centimes: 2_000,
    customer_payments_centimes: 0,
    total_centimes: 302_000,
  },
  cash_out: {
    refunds_centimes: 0,
    supplier_payments_centimes: 0,
    expenses_centimes: 40_000,
    total_centimes: 40_000,
  },
  cash_centimes: 262_000,
  card_in: {
    sales_centimes: 0,
    stamp_centimes: 0,
    customer_payments_centimes: 0,
    total_centimes: 0,
  },
};

const dashboard: DashboardDto = {
  day: "2026-09-15",
  month: "2026-09",
  today: {
    sales_ttc_centimes: 300_000,
    sales_count: 4,
    lines_ht_centimes: 305_000,
    discounts_centimes: 5_000,
    sales_ht_centimes: 300_000,
    cost_of_goods_centimes: 180_000,
    margin_centimes: 120_000,
    expenses_centimes: 40_000,
  },
  this_month: {
    sales_ttc_centimes: 1_200_000,
    sales_count: 17,
    lines_ht_centimes: 1_210_000,
    discounts_centimes: 10_000,
    sales_ht_centimes: 1_200_000,
    cost_of_goods_centimes: 700_000,
    margin_centimes: 500_000,
    expenses_centimes: 90_000,
  },
  cash_today: position,
  cash_this_month: { ...position, from: "2026-09-01", to: "2026-09-30" },
  low_stock: [
    { product_id: 7, name: "Ciment CPJ 42.5", qty_on_hand_milli: -2_000, low_stock_at_milli: 10_000 },
  ],
  top_by_quantity: [
    {
      product_id: 7,
      name: "Ciment CPJ 42.5",
      qty_milli: 40_000,
      lines_ht_centimes: 400_000,
      cost_of_goods_centimes: 240_000,
      margin_centimes: 160_000,
    },
  ],
  top_by_margin: [
    {
      product_id: 9,
      name: "Rond à béton",
      qty_milli: 2_000,
      lines_ht_centimes: 500_000,
      cost_of_goods_centimes: 100_000,
      margin_centimes: 400_000,
    },
  ],
  customer_debt: { total_centimes: 2_400_000, parties: 3 },
  supplier_debt: { total_centimes: 900_000, parties: 1 },
  open_purchases: 2,
};

describe("the dashboard", () => {
  test("the answer from the API parses whole", () => {
    expect(dashboardSchema.parse(dashboard)).toEqual(dashboard);
  });

  test("a month with nothing in it parses as zeros rather than as nothing", () => {
    const empty: DashboardDto = {
      ...dashboard,
      today: {
        sales_ttc_centimes: 0,
        sales_count: 0,
        lines_ht_centimes: 0,
        discounts_centimes: 0,
        sales_ht_centimes: 0,
        cost_of_goods_centimes: 0,
        margin_centimes: 0,
        expenses_centimes: 0,
      },
      low_stock: [],
      top_by_quantity: [],
      top_by_margin: [],
    };
    expect(dashboardSchema.safeParse(empty).success).toBe(true);
  });

  test("a margin below zero is taken, because a month can have one", () => {
    // More came back on credit notes than went out. Nothing on this screen is
    // bounded below zero, and a schema that refused it would blank the screen
    // on the one month a shop would want to look at.
    const returned: DashboardDto = {
      ...dashboard,
      today: { ...dashboard.today, margin_centimes: -50_000, sales_ht_centimes: -10_000 },
    };
    expect(dashboardSchema.safeParse(returned).success).toBe(true);
  });

  test("a month written as a day is refused", () => {
    const wrong = { ...dashboard, month: "2026-09-15" };
    expect(dashboardSchema.safeParse(wrong).success).toBe(false);
  });

  test("an amount JSON.parse had to round is refused", () => {
    // Past 2^53 a centime figure comes back already rounded. A shop reading
    // it off its own month would have no way to know.
    const rounded = {
      ...dashboard,
      today: { ...dashboard.today, margin_centimes: 9_007_199_254_740_993 },
    };
    expect(dashboardSchema.safeParse(rounded).success).toBe(false);
  });

  test("a cash position missing from the answer is refused", () => {
    const { cash_today: _dropped, ...short } = dashboard;
    expect(dashboardSchema.safeParse(short).success).toBe(false);
  });
});
