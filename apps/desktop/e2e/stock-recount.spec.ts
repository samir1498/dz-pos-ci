// The stock recount driven through a real browser against the real axum API
// and a real SQLite file. features.md §1: the quantity on hand is derived
// from the movement ledger and cached on the product, and a job re-derives
// it and reports drift.
//
// What the run proves end to end is the claim the panel makes. A product is
// opened with stock and sold from, which is every kind of movement the till
// writes today, and the recount then finds nothing to correct: the cache the
// screens read really is the sum of the ledger after ordinary trading. There
// is no route that writes a cached quantity (that is the point of the
// ledger), so a forged drift cannot be made from out here; the core test
// `a_forged_cache_is_reported_corrected_and_written_into_the_log` writes one
// by raw SQL and watches it be corrected.
//
// Strings come from the JSON dictionary of the Playwright project running
// the test, so the same run proves the block in fr, en and ar.

import { expect, test } from "./auth";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { apiHeaders, apiUrl } from "./api";
import { currentLang, t } from "./messages";

// `new URL(".", ...)` is already this file's directory (apps/desktop/e2e).
const here = fileURLToPath(new URL(".", import.meta.url));

const PRODUCT_NAME = "Semoule 5kg";
const OPENING_MILLI = 24_000;
const SOLD_MILLI = 2_000;

interface Recount {
  day: string;
  products_checked: number;
  drifts: { product_id: number; name: string; difference_milli: number }[];
}

/** The shop's day, asked of the same clock the core marks a run with. A
 * `new Date()` here would be a second clock, and on the machine's zone
 * rather than Algeria's. */
async function shopToday(request: import("@playwright/test").APIRequestContext): Promise<string> {
  const res = await request.get(`${apiUrl()}/clock`, { headers: apiHeaders() });
  expect(res.ok()).toBe(true);
  const body: { today: string } = await res.json();
  return body.today;
}

test("a shop that has only traded has nothing to correct, and saves the recount screenshot in Arabic", async ({
  page,
  request,
}) => {
  // Opening stock and a sale: two movements of two kinds, each of which
  // moved the cache as it was written.
  const created = await request.post(`${apiUrl()}/products`, {
    headers: apiHeaders(),
    data: {
      name: PRODUCT_NAME,
      barcode: null,
      category_id: 1,
      unit: "kg",
      cost_centimes: 8_000,
      selling_centimes: 12_000,
      wholesale_centimes: null,
      qty_on_hand_milli: OPENING_MILLI,
      low_stock_at_milli: 0,
      rate_bps: 1900,
      active: true,
    },
  });
  expect(created.status()).toBe(201);
  const product: { id: number } = await created.json();

  const sold = await request.post(`${apiUrl()}/sales`, {
    headers: apiHeaders(),
    data: {
      lines: [{ product_id: product.id, qty_milli: SOLD_MILLI }],
      payment_mode: "cash",
      tendered_centimes: 30_000,
    },
  });
  expect(sold.status()).toBe(201);

  await page.goto("/settings");
  await expect(page.getByRole("heading", { name: t("settings_stock_recount") })).toBeVisible();
  // Nothing has run yet in this database, so the panel says the shop has
  // never been recounted rather than claiming its stock is right.
  await expect(page.getByTestId("stock-recount-day")).toHaveText(t("stock_recount_never"));
  await expect(page.getByTestId("stock-recount-clean")).toHaveCount(0);

  const answered = page.waitForResponse(
    (res) => res.url().endsWith("/stock/recount") && res.request().method() === "POST",
  );
  await page.getByRole("button", { name: t("action_recount_now") }).click();
  const report: Recount = await (await answered).json();

  // The body, not only what the screen made of it.
  const today = await shopToday(request);
  expect(report.day).toBe(today);
  expect(report.drifts).toEqual([]);
  expect(report.products_checked).toBeGreaterThan(0);

  await expect(page.getByRole("status")).toHaveText(t("stock_recount_done"));
  await expect(page.getByTestId("stock-recount-day")).toHaveText(today);
  await expect(page.getByTestId("stock-recount-checked")).toHaveText(
    String(report.products_checked),
  );
  await expect(page.getByTestId("stock-recount-clean")).toHaveText(t("stock_recount_none"));
  await expect(page.getByTestId("stock-drift-table")).toHaveCount(0);

  // The stock the fiche shows after the recount is the opening minus the
  // sale, which is what it showed before: nothing was corrected because
  // nothing was wrong.
  const read = await request.get(`${apiUrl()}/products/${product.id}`, { headers: apiHeaders() });
  expect(read.ok()).toBe(true);
  const after: { qty_on_hand_milli: number } = await read.json();
  expect(after.qty_on_hand_milli).toBe(OPENING_MILLI - SOLD_MILLI);

  // A second run the same day is still allowed from the button: the owner
  // pressed it because they want the file looked at again.
  const again = await request.post(`${apiUrl()}/stock/recount`, { headers: apiHeaders() });
  expect(again.status()).toBe(200);
  const second: Recount = await again.json();
  expect(second.day).toBe(today);
  expect(second.drifts).toEqual([]);

  // The one committed screenshot of this panel: Arabic, so the RTL layout
  // of the block (heading and hint mirrored, quantities still left to
  // right) has a reference image.
  if (currentLang() === "ar") {
    await page.screenshot({
      path: path.join(here, "screenshots", "stock-recount-ar.png"),
      fullPage: true,
    });
  }
});
