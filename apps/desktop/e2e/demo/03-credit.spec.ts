// Scene 3: credit. A regular buys on the book; the till shows the new
// balance against the limit the shop set. Then the customer's own page,
// where a cash payment brings the balance down.

import { t } from "../messages";
import { beat, expect, keepClip, test } from "./scene";
import { CAFE, LAIT, MEZIANE, reel, seedCustomer, seedProduct } from "./shop";

test("credit", async ({ page, request }, testInfo) => {
  await reel(request);
  for (const p of [CAFE, LAIT]) await seedProduct(request, p);
  await seedCustomer(request, MEZIANE);

  await page.goto("/");
  await expect(page).toHaveURL(/\/till$/);
  await beat(page);

  // Who is buying.
  await page.getByLabel(t("till_customer_search"), { exact: true }).pressSequentially("Meziane", { delay: 80 });
  await page.getByRole("group", { name: t("till_customer") }).getByRole("button", { name: MEZIANE.name }).click();
  await beat(page);

  // Two coffees and a milk.
  const search = page.getByLabel(t("till_search"), { exact: true });
  await search.pressSequentially("Café", { delay: 80 });
  await page.getByTestId("tiles").getByRole("button", { name: CAFE.name }).click();
  await page.getByRole("button", { name: `${t("till_qty_increase")} ${CAFE.name}` }).click();
  await search.fill("");
  await search.pressSequentially("Lait", { delay: 80 });
  await page.getByTestId("tiles").getByRole("button", { name: LAIT.name }).click();
  await beat(page, 2);

  // On the book. The confirmation shows what they now owe.
  await page.getByRole("radio", { name: t("pay_credit"), exact: true }).click();
  await beat(page);
  await page.getByRole("button", { name: t("action_pay"), exact: true }).click();
  await expect(page.getByRole("status").getByTestId("till-new-balance")).toBeVisible();
  await beat(page, 3);

  // The customer's page: the balance, the movements, and a payment.
  await page.getByTestId("nav-customers").click();
  await expect(page).toHaveURL(/\/customers$/);
  await beat(page, 2);
  await page.getByRole("row").filter({ hasText: MEZIANE.name }).getByRole("link", { name: MEZIANE.name }).click();
  await expect(page.getByTestId("customer-balance")).toBeVisible();
  await beat(page, 2);

  await page.getByRole("button", { name: t("customers_pay"), exact: true }).click();
  await expect(page.getByTestId("customer-pay-dialog")).toBeVisible();
  await beat(page);
  await page.getByLabel(t("field_payment_amount"), { exact: true }).pressSequentially("500", { delay: 120 });
  await page.getByRole("radio", { name: t("payment_cash"), exact: true }).click();
  await beat(page);
  await page.getByRole("button", { name: t("action_take_payment"), exact: true }).click();
  await expect(page.getByText(t("customers_paid"))).toBeVisible();
  await beat(page, 3);

  await keepClip(page, testInfo);
});
