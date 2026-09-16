// Scene 1: ring a sale. Three things in the basket, cash, change, the
// ticket. The net carries the TVA at two rates and the droit de timbre,
// which is the thing a foreign till gets wrong for an Algerian shop.
//
// A scene, not a test: the shop is seeded through the API the way the
// till specs do it, the steps are the ones a cashier makes, and the few
// assertions only wait for the screen to catch up.

import { t } from "../messages";
import { beat, expect, keepClip, test } from "./scene";
import { CAFE, LAIT, PAIN, reel, seedProduct } from "./shop";

test("ring a sale", async ({ page, request }, testInfo) => {
  await reel(request);
  for (const p of [CAFE, LAIT, PAIN]) await seedProduct(request, p);

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
