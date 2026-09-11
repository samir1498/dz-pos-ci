// The dashboard driven through a real browser against the real axum API and a
// real SQLite file.
//
// What the run proves end to end: a shop that has sold, bought, borrowed and
// spent is read back by `GET /dashboard` and `GET /dashboard/series`, and
// every figure the screen paints is the one the API answered rather than one
// the screen summed. The expected amounts are fetched from the API inside the
// test and compared with what is on the page, so a screen that agreed with a
// server storing the wrong thing would still fail, and no number here has to
// be kept in step by hand.
//
// Named to run last, after `zz-exports-and-labels.spec.ts`. Every spec in a
// run shares one database in filename order, and this one fills a shop with
// sales, debts and a month of expenses; the specs that start from an empty
// catalogue or an empty month have to have run first.
//
// It seeds everything it reads, rather than living off what the specs before
// it left, because `just screenshot` runs it alone (`-g screenshot`) against a
// database that was just deleted: the committed picture has to show a shop
// either way.
//
// The one thing the picture cannot show is a curve of sales. `issued_at` is
// not on the wire on purpose (crates/api/src/dto.rs: the moment a sale
// happened is the server's), so every sale a test can make lands on the shop's
// today and the sales bars are one bar. An expense carries its own
// `expense_date`, so the expenses line is the one that runs across the window.

import { expect, test } from "./auth";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { formatCentimes } from "@dzpos/shared";
import { apiHeaders, apiUrl } from "./api";
import { currentLang, t } from "./messages";

// `new URL(".", ...)` is already this file's directory (apps/desktop/e2e).
const here = fileURLToPath(new URL(".", import.meta.url));

type Request = import("@playwright/test").APIRequestContext;

/** The window the screen draws and the one the API defaults to. */
const CHART_DAYS = 30;

interface Figures {
  sales_ttc_centimes: number;
  sales_count: number;
  margin_centimes: number;
  expenses_centimes: number;
}

interface Owed {
  total_centimes: number;
  parties: number;
}

interface Dashboard {
  day: string;
  today: Figures;
  this_month: Figures;
  cash_today: { cash_centimes: number };
  cash_this_month: { cash_centimes: number };
  low_stock: { product_id: number; name: string }[];
  top_by_quantity: { name: string; margin_centimes: number }[];
  top_by_margin: { name: string; margin_centimes: number }[];
  customer_debt: Owed;
  supplier_debt: Owed;
  open_purchases: number;
}

interface Series {
  days: { from: string; to: string; figures: Figures }[];
  weeks: { from: string; to: string }[];
}

/** One set of names per project, so the three language runs can share a
 * database file without tripping over a unique name. */
function suffix(): string {
  return currentLang();
}

/**
 * What the shop sells in this test. Prices and costs are whole dinars written
 * as centimes; the margins differ on purpose, so the two top tens rank the
 * catalogue in two different orders and a screen that drew one list twice
 * would show it.
 */
const CATALOGUE = [
  { key: "semoule", name: "Semoule extra 10 kg", cost: 90_000, price: 120_000, stock: 60_000 },
  { key: "huile", name: "Huile de table 5 L", cost: 62_000, price: 78_000, stock: 45_000 },
  { key: "cafe", name: "Café moulu 250 g", cost: 21_000, price: 34_000, stock: 80_000 },
  { key: "lait", name: "Lait UHT 1 L", cost: 8_500, price: 11_000, stock: 120_000 },
  // The one under its threshold: what the low-stock list is for.
  { key: "sucre", name: "Sucre blanc 1 kg", cost: 9_000, price: 12_500, stock: 3_000 },
] as const;

/** Thousandths of a unit sold of each product today, one basket per row. The
 * numbers are fixed rather than random: a committed screenshot has to be the
 * same picture on every machine. */
const BASKETS: readonly (readonly { key: string; qty: number }[])[] = [
  [
    { key: "semoule", qty: 4_000 },
    { key: "cafe", qty: 6_000 },
  ],
  [
    { key: "huile", qty: 5_000 },
    { key: "lait", qty: 12_000 },
  ],
  [
    { key: "cafe", qty: 9_000 },
    { key: "sucre", qty: 2_000 },
  ],
  [
    { key: "semoule", qty: 3_000 },
    { key: "huile", qty: 2_000 },
    { key: "lait", qty: 8_000 },
  ],
  [{ key: "cafe", qty: 11_000 }],
];

/**
 * How many days back an expense is filed and what it cost, in whole dinars as
 * centimes. Fourteen days of the thirty carry one, which is what gives the
 * chart's expenses line something to trace; the amounts rise and fall rather
 * than climbing, so the line is a line and not a ramp.
 */
const EXPENSES: readonly { back: number; centimes: number; category: string }[] = [
  { back: 29, centimes: 180_000, category: "transport" },
  { back: 27, centimes: 420_000, category: "electricity" },
  { back: 25, centimes: 95_000, category: "maintenance" },
  { back: 22, centimes: 640_000, category: "salaries" },
  { back: 20, centimes: 150_000, category: "transport" },
  { back: 18, centimes: 310_000, category: "water" },
  { back: 16, centimes: 880_000, category: "rent" },
  { back: 13, centimes: 205_000, category: "transport" },
  { back: 11, centimes: 470_000, category: "electricity" },
  { back: 9, centimes: 120_000, category: "other" },
  { back: 7, centimes: 355_000, category: "maintenance" },
  { back: 5, centimes: 690_000, category: "salaries" },
  { back: 3, centimes: 240_000, category: "transport" },
  { back: 1, centimes: 165_000, category: "water" },
];

const CUSTOMER_OPENING_DEBT = 4_850_000;
const SUPPLIER_OPENING_DEBT = 7_300_000;

/** The shop's day, asked of the same clock the core dates rows with. A
 * `new Date()` here would be a second clock, and on the machine's zone rather
 * than Algeria's. */
async function shopToday(request: Request): Promise<string> {
  const res = await request.get(`${apiUrl()}/clock`, { headers: apiHeaders() });
  expect(res.ok()).toBe(true);
  const body: { today: string } = await res.json();
  return body.today;
}

/** `days` days before `day`, as `YYYY-MM-DD`. The arithmetic is on the day the
 * server gave us, at noon UTC so no zone can push it over a boundary; nothing
 * here reads the machine's clock. */
function daysBefore(day: string, days: number): string {
  const at = new Date(`${day}T12:00:00Z`);
  at.setUTCDate(at.getUTCDate() - days);
  return at.toISOString().slice(0, 10);
}

async function aProduct(
  request: Request,
  product: { name: string; cost: number; price: number; stock: number },
  lowStockAt: number,
): Promise<number> {
  const res = await request.post(`${apiUrl()}/products`, {
    headers: apiHeaders(),
    data: {
      name: product.name,
      barcode: null,
      category_id: null,
      unit: "piece",
      cost_centimes: product.cost,
      selling_centimes: product.price,
      wholesale_centimes: null,
      qty_on_hand_milli: product.stock,
      low_stock_at_milli: lowStockAt,
      rate_bps: 1900,
      active: true,
    },
  });
  expect(res.status()).toBe(201);
  const made: { id: number } = await res.json();
  return made.id;
}

async function aCashSale(
  request: Request,
  lines: readonly { product_id: number; qty_milli: number }[],
): Promise<void> {
  const res = await request.post(`${apiUrl()}/sales`, {
    headers: apiHeaders(),
    data: {
      lines: lines.map((line) => ({
        product_id: line.product_id,
        qty_milli: line.qty_milli,
        unit_price_centimes: null,
        line_discount_centimes: 0,
      })),
      global_discount_centimes: 0,
      payment_mode: "cash",
      // Well over any of these baskets: the change is the server's business
      // and this test is not about it.
      tendered_centimes: 100_000_000,
      customer_id: null,
      override: false,
      kind: "ticket",
    },
  });
  expect(res.status()).toBe(201);
}

async function categoryId(request: Request, key: string): Promise<number> {
  const res = await request.get(`${apiUrl()}/expense-categories`, { headers: apiHeaders() });
  expect(res.ok()).toBe(true);
  const all: { id: number; key: string }[] = await res.json();
  const found = all.find((c) => c.key === key);
  if (found === undefined) throw new Error(`the API has no category ${key}`);
  return found.id;
}

async function anExpense(
  request: Request,
  category: number,
  centimes: number,
  day: string,
): Promise<void> {
  const res = await request.post(`${apiUrl()}/expenses`, {
    headers: apiHeaders(),
    data: {
      category_id: category,
      amount_centimes: centimes,
      expense_date: day,
      note: null,
    },
  });
  expect(res.status()).toBe(201);
}

/** A shop with a catalogue, a day of trading, both debts, an order still open
 * and a month of expenses behind it. Everything through the API, so every rule
 * the core holds applies to these rows as it would to a real shop's. */
async function aTradingShop(request: Request, today: string): Promise<void> {
  const ids = new Map<string, number>();
  for (const product of CATALOGUE) {
    const lowStockAt = product.key === "sucre" ? 12_000 : 5_000;
    ids.set(product.key, await aProduct(request, { ...product, name: `${product.name} ${suffix()}` }, lowStockAt));
  }

  for (const basket of BASKETS) {
    await aCashSale(
      request,
      basket.map((line) => {
        const id = ids.get(line.key);
        if (id === undefined) throw new Error(`no product seeded for ${line.key}`);
        return { product_id: id, qty_milli: line.qty };
      }),
    );
  }

  const customer = await request.post(`${apiUrl()}/customers`, {
    headers: apiHeaders(),
    data: {
      name: `Boulangerie El Firdaws ${suffix()}`,
      party_kind: "company",
      phone: null,
      address: null,
      rc: null,
      nif: null,
      nis: null,
      ai: null,
      credit_limit_centimes: 10_000_000,
      warn_threshold_centimes: 8_000_000,
      notes: null,
      active: true,
      opening_debt_centimes: CUSTOMER_OPENING_DEBT,
    },
  });
  expect(customer.status()).toBe(201);

  const supplier = await request.post(`${apiUrl()}/suppliers`, {
    headers: apiHeaders(),
    data: {
      name: `Grossiste Bab Ezzouar ${suffix()}`,
      phone: null,
      address: null,
      rc: null,
      nif: null,
      nis: null,
      ai: null,
      notes: null,
      active: true,
      opening_debt_centimes: SUPPLIER_OPENING_DEBT,
    },
  });
  expect(supplier.status()).toBe(201);
  const madeSupplier: { id: number } = await supplier.json();

  // Ordered and not received: what "orders still open" counts.
  const semoule = ids.get("semoule");
  if (semoule === undefined) throw new Error("no product seeded for semoule");
  const order = await request.post(`${apiUrl()}/purchases`, {
    headers: apiHeaders(),
    data: {
      supplier_id: madeSupplier.id,
      supplier_document_number: null,
      purchase_date: today,
      due_date: null,
      transport_centimes: 0,
      extra_costs_centimes: 0,
      note: null,
      lines: [{ product_id: semoule, qty_ordered_milli: 20_000, unit_cost_centimes: 88_000 }],
      paid_now: null,
      receive_now: false,
    },
  });
  expect(order.status()).toBe(201);

  for (const expense of EXPENSES) {
    await anExpense(
      request,
      await categoryId(request, expense.category),
      expense.centimes,
      daysBefore(today, expense.back),
    );
  }
}

async function dashboardOf(request: Request, day: string): Promise<Dashboard> {
  const res = await request.get(`${apiUrl()}/dashboard?day=${day}`, { headers: apiHeaders() });
  expect(res.ok()).toBe(true);
  return res.json();
}

async function seriesOf(request: Request, day: string): Promise<Series> {
  const res = await request.get(`${apiUrl()}/dashboard/series?day=${day}&days=${CHART_DAYS}`, {
    headers: apiHeaders(),
  });
  expect(res.ok()).toBe(true);
  return res.json();
}

test("shows the shop's day and month as the API answered them, and saves the dashboard screenshot", async ({
  page,
  request,
}) => {
  const today = await shopToday(request);
  // What the shop was already carrying. In a whole run the specs above have
  // sold on credit, opened fiches and filed this month's expenses, so the
  // figures below are read as differences: the seeding's own effect is what
  // this test knows, and it is the same number on an empty database and on a
  // full one.
  const before = await dashboardOf(request, today);
  await aTradingShop(request, today);

  const answered = await dashboardOf(request, today);
  const chart = await seriesOf(request, today);

  // The seeding is worth nothing if it left an empty shop, and every
  // assertion below would pass against zeros.
  expect(answered.day).toBe(today);
  expect(answered.today.sales_count).toBeGreaterThan(before.today.sales_count);
  expect(answered.today.margin_centimes).toBeGreaterThan(before.today.margin_centimes);
  expect(answered.low_stock.length).toBeGreaterThan(0);
  expect(answered.top_by_quantity.length).toBeGreaterThan(0);
  expect(answered.top_by_margin.length).toBeGreaterThan(0);
  expect(answered.customer_debt.total_centimes - before.customer_debt.total_centimes).toBe(
    CUSTOMER_OPENING_DEBT,
  );
  expect(answered.supplier_debt.total_centimes - before.supplier_debt.total_centimes).toBe(
    SUPPLIER_OPENING_DEBT,
  );
  expect(answered.open_purchases - before.open_purchases).toBe(1);
  // Thirty buckets, oldest first and none skipped, and the expenses filed
  // across them are what the chart's line traces. At least fourteen days
  // carry one: a whole run has the expenses spec's two on top of these.
  expect(chart.days).toHaveLength(CHART_DAYS);
  expect(chart.days.filter((d) => d.figures.expenses_centimes > 0).length).toBeGreaterThanOrEqual(
    EXPENSES.length,
  );

  await page.goto("/dashboard");
  await expect(
    page.getByRole("main").getByRole("heading", { name: t("dashboard_title") }),
  ).toBeVisible();

  // The four figure cards: the day's amount and the month's, each where it
  // belongs, and both of them the server's.
  await expect(page.getByTestId("figure-sales-today")).toHaveText(
    formatCentimes(answered.today.sales_ttc_centimes),
  );
  await expect(page.getByTestId("figure-sales-month")).toHaveText(
    formatCentimes(answered.this_month.sales_ttc_centimes),
  );
  await expect(page.getByTestId("figure-margin-today")).toHaveText(
    formatCentimes(answered.today.margin_centimes),
  );
  await expect(page.getByTestId("figure-expenses-month")).toHaveText(
    formatCentimes(answered.this_month.expenses_centimes),
  );
  await expect(page.getByTestId("figure-cash-today")).toHaveText(
    formatCentimes(answered.cash_today.cash_centimes),
  );
  await expect(page.getByTestId("figure-customer-debt-total")).toHaveText(
    formatCentimes(answered.customer_debt.total_centimes),
  );
  await expect(page.getByTestId("figure-supplier-debt-total")).toHaveText(
    formatCentimes(answered.supplier_debt.total_centimes),
  );
  await expect(page.getByTestId("figure-open-purchases")).toContainText(
    String(answered.open_purchases),
  );

  // The three lists, each naming what the API put in it. The low-stock row
  // looked for is this test's own product, which a whole run finds among the
  // ones the specs above left behind.
  const sugar = CATALOGUE.find((p) => p.key === "sucre");
  if (sugar === undefined) throw new Error("the catalogue above has no sucre");
  const mine = answered.low_stock.find((row) => row.name === `${sugar.name} ${suffix()}`);
  if (mine === undefined) throw new Error("the API answered no low-stock row for the seeded sugar");
  await expect(page.getByTestId("dashboard-low-stock")).toContainText(mine.name);
  await expect(page.getByTestId("low-stock-pill")).toHaveCount(answered.low_stock.length);

  const busiest = answered.top_by_quantity[0];
  const richest = answered.top_by_margin[0];
  if (busiest === undefined || richest === undefined) {
    throw new Error("the API answered an empty top ten");
  }
  await expect(page.getByTestId("dashboard-top-quantity")).toContainText(busiest.name);
  await expect(page.getByTestId("dashboard-top-margin")).toContainText(
    formatCentimes(richest.margin_centimes),
  );

  const drawn = page.getByTestId("dashboard-chart");
  await expect(drawn).toBeVisible();
  // The three series are all drawn: a bar layer and two line layers.
  await expect(page.locator(".recharts-bar")).toHaveCount(1);
  await expect(page.locator(".recharts-line")).toHaveCount(2);
  await expect(drawn).toHaveAttribute("data-buckets", String(chart.days.length));
  // The week view is the array the same answer already carried: switching it
  // folds the buckets and asks the server for nothing.
  const requests: string[] = [];
  page.on("request", (asked) => requests.push(asked.url()));
  await page.getByTestId("chart-grain-weeks").click();
  await expect(drawn).toHaveAttribute("data-buckets", String(chart.weeks.length));
  expect(requests.filter((url) => url.includes("/dashboard"))).toEqual([]);
  // Back to the day view, which is what the committed picture shows.
  await page.getByTestId("chart-grain-days").click();
  await expect(drawn).toHaveAttribute("data-buckets", String(chart.days.length));

  // Only fr and ar keep a committed screenshot: fr is the reference shot, ar
  // is the RTL one, where the cards and the tables mirror and the chart does
  // not. en adds nothing the other two do not already show.
  const lang = currentLang();
  if (lang === "fr") {
    await page.screenshot({ path: path.join(here, "screenshots", "dashboard.png"), fullPage: true });
  } else if (lang === "ar") {
    await page.screenshot({
      path: path.join(here, "screenshots", "dashboard-ar.png"),
      fullPage: true,
    });
  }
});
