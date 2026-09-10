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
import { formatCentimes } from "@dzpos/shared";
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

/** 1 500,00 DA handed over for a 1 292,00 DA basket. */
const TENDERED = "1500";

/** The product of the pad and F9 test: one piece at 300,00 DA, five on hand.
 * Its code is its own: the suite runs against one database per language, and
 * the credit spec already sells 6130009000035 to a customer. */
const SOAP = "Savon e2e";
const SOAP_BARCODE = "6130009000066";
const SOAP_STOCK_MILLI = 5_000;

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
 *
 * It asks before it writes: the core refuses a régime the shop is already
 * under on that day (services/settings.rs, so a comptable's "since" date is
 * never moved to the day of a click). Posting blind works only when the
 * settings suite has just switched the shop to the IFU, which is the very
 * dependency this function exists to remove.
 */
async function useReelRegime(request: APIRequestContext): Promise<void> {
  const current = await request.get(`${apiUrl()}/settings`, { headers: apiHeaders() });
  expect(current.ok()).toBe(true);
  const settings: { regime: { regime: string } } = await current.json();
  if (settings.regime.regime === "reel") return;
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

// "screenshot" in the title on purpose: `just screenshot` greps for it, and
// till-ar.png is committed, so the file has to be regenerable by that recipe.
test("sells two rates for cash, matches the fixture totals, reduces the stock and saves the till screenshot in Arabic", async ({
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
  await expect(done).toContainText(String(sale.number));
  // The series is the core's document code ("doc_ticket"), untranslatable
  // and not for a cashier to read.
  await expect(done).not.toContainText(sale.series);
  await expect(page.getByText(t("till_cart_empty"))).toBeVisible();

  // The print button shows the page the core rendered from the stored
  // document, in the language the till is being used in, so what is on
  // screen is the paper the printer will put out.
  await page.getByRole("button", { name: t("till_print"), exact: true }).click();
  const receipt = page.getByRole("region", { name: t("till_receipt") });
  const ticket = receipt.frameLocator("iframe");
  await expect(ticket.getByText(COFFEE)).toBeVisible();
  await expect(ticket.getByText(TOMATO)).toBeVisible();
  // The amount too, not only the names: the page is rendered from the stored
  // document, so its net to pay is the fixture's, formatted the way the shop
  // reads money. This is the one assertion in the suite that reads the core's
  // own formatting rather than the client's, and they have to agree.
  await expect(ticket.locator(".amount-net-to-pay")).toHaveText(
    formatCentimes(expected.net_to_pay),
  );

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

/**
 * The two ways a cashier's hands work that the test above does not drive: the
 * pad, and the one function key the till has. F9 pays the way a till keyboard
 * does, and it is asserted here rather than in a unit test because the
 * listener is on the window and what has the focus decides whether the key
 * ever arrives.
 */
test("the pad counts the notes and F9 takes the sale", async ({ page, request }) => {
  await seed(request, {
    name: SOAP,
    barcode: SOAP_BARCODE,
    price: 30_000,
    rate: 1900,
    unit: "piece",
    stock: SOAP_STOCK_MILLI,
  });

  await page.goto("/");
  const search = page.getByLabel(t("till_search"), { exact: true });
  await search.fill(SOAP_BARCODE);
  await search.press("Enter");

  // 1 000 DA handed over, typed on the pad the way a thumb types it: the pad
  // counts whole dinars, so four keys are four digits and not four centimes.
  const pad = page.getByTestId("keypad");
  for (const key of ["1", "0", "0", "0"]) {
    await pad.getByRole("button", { name: key, exact: true }).click();
  }
  await expect(page.getByTestId("till-change")).toBeVisible();

  const issued = page.waitForResponse(
    (res) => res.url().endsWith("/sales") && res.request().method() === "POST",
  );
  await page.keyboard.press("F9");
  const response = await issued;
  expect(response.status()).toBe(201);

  const sale: { tendered_centimes: number | null; change_centimes: number | null } =
    await response.json();
  expect(sale.tendered_centimes).toBe(100_000);
  await expect(page.getByRole("status")).toBeVisible();
  expect(await stockOf(request, SOAP_BARCODE)).toBe(SOAP_STOCK_MILLI - 1_000);
});
