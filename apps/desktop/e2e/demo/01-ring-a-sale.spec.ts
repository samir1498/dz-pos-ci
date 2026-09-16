// Scene 1: ring a sale. Three things in the basket, cash, change, the
// ticket. The net carries the TVA at two rates and the droit de timbre,
// which is the thing a foreign till gets wrong for an Algerian shop.
//
// A scene, not a test: the shop is seeded through the API the way the
// till specs do it, the steps are the ones a cashier makes, and the few
// assertions only wait for the screen to catch up.

import type { APIRequestContext } from "@playwright/test";

import { apiHeaders, apiUrl } from "../api";
import { t } from "../messages";
import { beat, expect, keepClip, test } from "./scene";

const CAFE = { name: "Café moulu 250g", barcode: "6130000000017", price: 42_000, rate: 1900 };
const LAIT = { name: "Lait 1L", barcode: "6130000000024", price: 14_000, rate: 900 };
const PAIN = { name: "Pain", barcode: "6130000000031", price: 1_500, rate: 0 };

async function seed(request: APIRequestContext, p: typeof CAFE): Promise<void> {
  const res = await request.post(`${apiUrl()}/products`, {
    headers: apiHeaders(),
    data: {
      name: p.name,
      barcode: p.barcode,
      category_id: 1,
      unit: "piece",
      cost_centimes: 0,
      selling_centimes: p.price,
      wholesale_centimes: null,
      qty_on_hand_milli: 50_000,
      low_stock_at_milli: 0,
      rate_bps: p.rate,
      active: true,
    },
  });
  // 201 on an empty shop, 409 when the previous scene already put it there.
  expect([201, 409]).toContain(res.status());
}

/** Under the réel a ticket shows its TVA; the IFU shows none. Stated here
 * rather than inherited from whatever ran before. */
async function reel(request: APIRequestContext): Promise<void> {
  const current = await request.get(`${apiUrl()}/settings`, { headers: apiHeaders() });
  const settings: { regime: { regime: string } } = await current.json();
  if (settings.regime.regime === "reel") return;
  const today = new Date(Date.now() + 3_600_000).toISOString().slice(0, 10);
  const res = await request.post(`${apiUrl()}/settings/regime`, {
    headers: apiHeaders(),
    data: { regime: "reel", valid_from: today },
  });
  expect(res.ok()).toBe(true);
}

test("ring a sale", async ({ page, request }, testInfo) => {
  await reel(request);
  for (const p of [CAFE, LAIT, PAIN]) await seed(request, p);

  await page.goto("/");
  await expect(page).toHaveURL(/\/till$/);
  await beat(page);

  // A scanner reads the coffee: the barcode lands in the search box and
  // Enter puts one in the basket.
  const search = page.getByLabel(t("till_search"), { exact: true });
  await search.pressSequentially(CAFE.barcode, { delay: 40 });
  await search.press("Enter");
  await beat(page);

  // Two milks, tapped off the tiles.
  await search.pressSequentially("Lait", { delay: 80 });
  await page.getByTestId("tiles").getByRole("button", { name: LAIT.name }).click();
  await page.getByRole("button", { name: `${t("till_qty_increase")} ${LAIT.name}` }).click();
  await beat(page);

  // And the bread.
  await search.fill("");
  await search.pressSequentially("Pain", { delay: 80 });
  await page.getByTestId("tiles").getByRole("button", { name: PAIN.name }).click();
  await beat(page, 2);

  // Cash: a 1 000 DA note. The change is worked out by the till.
  await page.getByLabel(t("field_tendered"), { exact: true }).pressSequentially("1000", { delay: 120 });
  await beat(page);
  await page.getByRole("button", { name: t("action_pay"), exact: true }).click();
  await expect(page.getByRole("status")).toBeVisible();
  await beat(page, 2);

  // The ticket the printer will put out, in French, with the TVA lines and
  // the stamp.
  await page.getByRole("button", { name: t("till_print"), exact: true }).click();
  const receipt = page.getByRole("region", { name: t("till_receipt") });
  await expect(receipt.frameLocator("iframe").getByText(CAFE.name)).toBeVisible();
  await beat(page, 3);

  await keepClip(page, testInfo);
});
