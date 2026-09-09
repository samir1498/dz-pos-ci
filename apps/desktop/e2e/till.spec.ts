// A cash sale end to end: a real browser, the real axum API, the real
// SQLite file. Two products are seeded through the API, the cashier scans
// one and weighs the other, pays in cash, and three things are then true at
// once: the till is what "/" opens on, the document the API issued carries
// the totals the money fixture names, and the stock went down by exactly the
// quantities sold.
//
// The expectation is the fixture, not the screen: the case
// `till_cash_sale_two_rates` in fixtures/money/tva_rounding_once_per_rate is
// read here and its numbers compared with SaleDto.totals, so a screen that
// computed a wrong preview and an API that agreed with it would still fail.

import { expect, test } from "@playwright/test";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { APIRequestContext, Page } from "@playwright/test";
import { apiHeaders, apiUrl } from "./api";
import { currentLang, t } from "./messages";

const here = fileURLToPath(new URL(".", import.meta.url));

const COFFEE = "Café e2e 250g";
const COFFEE_BARCODE = "6130009000011";
const TOMATO = "Tomate e2e";
const TOMATO_BARCODE = "6130009000028";

/** 10 pieces and 5 kg on hand; the sale takes 2 and 1,5. */
const COFFEE_STOCK_MILLI = 10_000;
const TOMATO_STOCK_MILLI = 5_000;
const COFFEE_SOLD_MILLI = 2_000;
const TOMATO_SOLD_MILLI = 1_500;

/** 15,00 DA handed over for a 12,92 DA basket. */
const TENDERED = "1500";

interface TotalsCase {
  name: string;
  expected: {
    total_ht: number;
    discount: number;
    subtotal_ht: number;
    tva_by_rate: { rate_bps: number; base: number; amount: number }[];
    tva: number;
    total_ttc: number;
    stamp: number;
    net_to_pay: number;
  };
}

function fixtureCase(): TotalsCase["expected"] {
  const file = path.join(
    here,
    "..",
    "..",
    "..",
    "fixtures",
    "money",
    "tva_rounding_once_per_rate.json",
  );
  const parsed: { totals_cases: TotalsCase[] } = JSON.parse(readFileSync(file, "utf8"));
  const found = parsed.totals_cases.find((c) => c.name.startsWith("till_cash_sale_two_rates"));
  if (found === undefined) throw new Error("the fixture has no till_cash_sale_two_rates case");
  return found.expected;
}

async function seed(
  request: APIRequestContext,
  input: { name: string; barcode: string; price: number; rate: number; unit: string; stock: number },
): Promise<void> {
  const res = await request.post(`${apiUrl()}/products`, {
    headers: apiHeaders(),
    data: {
      name: input.name,
      barcode: input.barcode,
      category_id: 1,
      unit: input.unit,
      cost_centimes: 0,
      selling_centimes: input.price,
      wholesale_centimes: null,
      qty_on_hand_milli: input.stock,
      low_stock_at_milli: 0,
      rate_bps: input.rate,
      active: true,
    },
  });
  expect(res.status()).toBe(201);
}

/** A day as the API writes it, on the calendar the server reads "today"
 * from: Algeria's, UTC+1 with no daylight saving. */
function today(): string {
  return new Date(Date.now() + 3_600_000).toISOString().slice(0, 10);
}

/**
 * The fixture case prints TVA, so the sale has to be issued under the réel.
 * The settings suite runs before this file in the same database and leaves
 * the shop on the IFU (it proves a backdated change takes effect), and under
 * the IFU a ticket carries no TVA row at all. Stating the régime here rather
 * than depending on the order of two spec files is what keeps this test
 * about the till.
 */
async function useReelRegime(request: APIRequestContext): Promise<void> {
  const res = await request.post(`${apiUrl()}/settings/regime`, {
    headers: apiHeaders(),
    data: { regime: "reel", valid_from: today() },
  });
  expect(res.ok()).toBe(true);
}

async function stockOf(request: APIRequestContext, barcode: string): Promise<number> {
  const res = await request.get(`${apiUrl()}/products`, { headers: apiHeaders() });
  expect(res.ok()).toBe(true);
  const rows: { barcode: string | null; qty_on_hand_milli: number }[] = await res.json();
  const found = rows.find((p) => p.barcode === barcode);
  if (found === undefined) throw new Error(`no product with barcode ${barcode}`);
  return found.qty_on_hand_milli;
}

/** The quantity box of one cart line. */
function qtyBox(page: Page, name: string) {
  return page.getByLabel(`${t("field_qty")} ${name}`, { exact: true });
}

test("sells two rates for cash, matches the fixture totals and reduces the stock", async ({
  page,
  request,
}) => {
  await useReelRegime(request);
  await seed(request, {
    name: COFFEE,
    barcode: COFFEE_BARCODE,
    price: 40_000,
    rate: 1900,
    unit: "piece",
    stock: COFFEE_STOCK_MILLI,
  });
  await seed(request, {
    name: TOMATO,
    barcode: TOMATO_BARCODE,
    price: 20_000,
    rate: 900,
    unit: "kg",
    stock: TOMATO_STOCK_MILLI,
  });

  // The till is the home: "/" lands on it without a click.
  await page.goto("/");
  await expect(page).toHaveURL(/\/till$/);

  // A scanner types the barcode and sends Enter; one unit lands in the cart
  // and the box is empty for the next scan.
  const search = page.getByLabel(t("till_search"), { exact: true });
  await expect(search).toBeFocused();
  await search.fill(COFFEE_BARCODE);
  await search.press("Enter");
  await expect(search).toHaveValue("");
  await expect(qtyBox(page, COFFEE)).toHaveValue("1");
  await page.getByRole("button", { name: `${t("till_qty_increase")} ${COFFEE}` }).click();
  await expect(qtyBox(page, COFFEE)).toHaveValue("2");

  // The second one is weighed, so its quantity is typed in the shop's
  // comma-decimal.
  await search.fill("Tomate e2e");
  await page.getByTestId("tiles").getByRole("button", { name: TOMATO }).click();
  await qtyBox(page, TOMATO).fill("1,5");

  await page.getByLabel(t("field_tendered"), { exact: true }).fill(TENDERED);

  const issued = page.waitForResponse(
    (res) => res.url().endsWith("/sales") && res.request().method() === "POST",
  );
  await page.getByRole("button", { name: t("action_pay"), exact: true }).click();
  const response = await issued;
  expect(response.status()).toBe(201);

  // The document the API issued, column for column against the fixture.
  const sale: {
    series: string;
    number: number;
    totals: Record<string, number>;
    tva: { rate_bps: number; base_centimes: number; amount_centimes: number }[];
    tendered_centimes: number | null;
    change_centimes: number | null;
  } = await response.json();
  const expected = fixtureCase();
  expect(sale.totals.total_ht_centimes).toBe(expected.total_ht);
  expect(sale.totals.discount_centimes).toBe(expected.discount);
  expect(sale.totals.subtotal_ht_centimes).toBe(expected.subtotal_ht);
  expect(sale.totals.tva_centimes).toBe(expected.tva);
  expect(sale.totals.total_ttc_centimes).toBe(expected.total_ttc);
  expect(sale.totals.stamp_centimes).toBe(expected.stamp);
  expect(sale.totals.net_to_pay_centimes).toBe(expected.net_to_pay);
  expect(sale.tva).toEqual(
    expected.tva_by_rate.map((g) => ({
      rate_bps: g.rate_bps,
      base_centimes: g.base,
      amount_centimes: g.amount,
    })),
  );
  expect(sale.tendered_centimes).toBe(150_000);
  expect(sale.change_centimes).toBe(150_000 - expected.net_to_pay);

  // What the cashier is shown is the document, so the number is the one the
  // server assigned and the cart is empty behind it.
  const done = page.getByRole("status");
  await expect(done).toContainText(sale.series);
  await expect(done).toContainText(String(sale.number));
  await expect(page.getByText(t("till_cart_empty"))).toBeVisible();

  // The print stub reads the stored document back rather than the screen.
  await page.getByRole("button", { name: t("till_print"), exact: true }).click();
  const receipt = page.getByRole("region", { name: t("till_receipt") });
  await expect(receipt.getByText(COFFEE)).toBeVisible();
  await expect(receipt.getByText(TOMATO)).toBeVisible();

  // Stock left by exactly what was sold, not by a rounded unit.
  expect(await stockOf(request, COFFEE_BARCODE)).toBe(COFFEE_STOCK_MILLI - COFFEE_SOLD_MILLI);
  expect(await stockOf(request, TOMATO_BARCODE)).toBe(TOMATO_STOCK_MILLI - TOMATO_SOLD_MILLI);

  // The one committed till screenshot: Arabic, so the mirrored cart, the
  // numeric cells still read left to right, and the totals block have a
  // picture a reviewer can look at.
  if (currentLang() === "ar") {
    await page.screenshot({ path: path.join(here, "screenshots", "till-ar.png"), fullPage: true });
  }
});
