// Scene 2: the ticket becomes a facture. A company is named on the sale,
// so the paper carries the buyer's own RC and NIS next to the seller's,
// the TVA and the stamp, and its own numbered series. The same document
// on the half sheet, one click later.

import { t } from "../messages";
import { beat, expect, keepClip, test } from "./scene";
import { CIMENT, ROUIBA, reel, seedCustomer, seedProduct, storeBlock } from "./shop";

test("the ticket", async ({ page, request }, testInfo) => {
  await reel(request);
  await storeBlock(request);
  await seedProduct(request, CIMENT);
  await seedCustomer(request, ROUIBA);

  await page.goto("/till");
  await expect(page).toHaveURL(/\/till$/);
  await beat(page);

  // The buyer first: a facture is made out to someone.
  await page.getByLabel(t("till_customer_search"), { exact: true }).pressSequentially("Rouiba", { delay: 80 });
  await page.getByRole("group", { name: t("till_customer") }).getByRole("button", { name: ROUIBA.name }).click();
  await beat(page);

  // Three bags of cement.
  const search = page.getByLabel(t("till_search"), { exact: true });
  await search.pressSequentially("Ciment", { delay: 80 });
  await page.getByTestId("tiles").getByRole("button", { name: CIMENT.name }).click();
  const more = page.getByRole("button", { name: `${t("till_qty_increase")} ${CIMENT.name}` });
  await more.click();
  await more.click();
  await beat(page, 2);

  // Facture, on the company's account.
  await page.getByRole("radio", { name: t("till_facture"), exact: true }).click();
  await beat(page);
  await page.getByRole("radio", { name: t("pay_credit"), exact: true }).click();
  await beat(page);
  await page.getByRole("button", { name: t("action_pay"), exact: true }).click();
  await expect(page.getByRole("status")).toBeVisible();
  await beat(page, 2);

  // The paper: seller's block, buyer's block, the lines, the TVA, the stamp,
  // the number in the facture series.
  await page.getByRole("button", { name: t("till_print"), exact: true }).click();
  const panel = page.getByRole("region", { name: t("till_receipt") });
  await expect(panel.frameLocator("iframe").getByText(ROUIBA.name)).toBeVisible();
  await beat(page, 3);

  // The half sheet.
  await panel.getByRole("radio", { name: t("till_paper_a5"), exact: true }).click();
  await beat(page, 3);

  await keepClip(page, testInfo);
});
