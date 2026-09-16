// Scene 5: Arabic. The same till switched to the shop's other language,
// right to left, and a sale rung up in it. One shot.

import { t, tIn } from "../messages";
import { beat, expect, keepClip, test } from "./scene";
import { CAFE, PAIN, reel, seedProduct } from "./shop";

test("arabic", async ({ page, request }, testInfo) => {
  await reel(request);
  for (const p of [CAFE, PAIN]) await seedProduct(request, p);

  await page.goto("/");
  await expect(page).toHaveURL(/\/till$/);
  await beat(page, 2);

  // The switch, in the top bar.
  await page.getByTestId("language-switcher").getByRole("button", { name: t("lang_name_ar") }).click();
  await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
  await beat(page, 3);

  // A coffee and a loaf, a 1 000 DA note, in Arabic.
  const search = page.getByLabel(tIn("ar", "till_search"), { exact: true });
  await search.pressSequentially(CAFE.barcode, { delay: 40 });
  await search.press("Enter");
  await beat(page);
  await search.pressSequentially("Pain", { delay: 80 });
  await page.getByTestId("tiles").getByRole("button", { name: PAIN.name }).click();
  await beat(page, 2);

  await page.getByLabel(tIn("ar", "field_tendered"), { exact: true }).pressSequentially("1000", { delay: 120 });
  await beat(page);
  await page.getByRole("button", { name: tIn("ar", "action_pay"), exact: true }).click();
  await expect(page.getByRole("status")).toBeVisible();
  await beat(page, 3);

  await keepClip(page, testInfo);
});
