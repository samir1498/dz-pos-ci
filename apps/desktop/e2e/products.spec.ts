// The products screen driven through a real browser against a real API.
// Expected strings come from src/i18n/fr.json (fr is the default language
// in src/i18n/index.tsx), so a reworded message fails here instead of
// silently passing a hardcoded sentence.

import { expect, test } from "@playwright/test";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

// `new URL(".", ...)` is already this file's directory (apps/desktop/e2e).
const here = fileURLToPath(new URL(".", import.meta.url));
const frPath = path.join(here, "..", "src", "i18n", "fr.json");

/** Reads fr.json without a type assertion: every value is checked. */
function readMessages(): Record<string, string> {
  const parsed: unknown = JSON.parse(readFileSync(frPath, "utf8"));
  if (typeof parsed !== "object" || parsed === null) {
    throw new Error(`${frPath} is not a JSON object`);
  }
  const messages: Record<string, string> = {};
  for (const [key, value] of Object.entries(parsed)) {
    if (typeof value !== "string") throw new Error(`${frPath}: ${key} is not a string`);
    messages[key] = value;
  }
  return messages;
}

const messages = readMessages();

function t(key: string): string {
  const value = messages[key];
  if (value === undefined) throw new Error(`${frPath} has no key ${key}`);
  return value;
}

const PRODUCT_NAME = "Semoule extra 5kg";
// 250,50 DA typed in dinars becomes 25050 centimes; formatCentimes in
// packages/shared renders that back as "250,50". Kept under 1000 DA so no
// thousands separator (a narrow no-break space) enters the assertion.
const PRICE_INPUT = "250,50";
const PRICE_RENDERED = "250,50";
const STOCK_INPUT = "12";
const STOCK_RENDERED = "12";

test("adds a product and saves the products screenshot", async ({ page }) => {
  // The rate is chosen by hand (9 %) rather than inherited from the
  // category, and the request body is captured to prove what was posted.
  const postedRates: unknown[] = [];
  await page.route("**/products", async (route) => {
    if (route.request().method() === "POST") {
      const body: unknown = route.request().postDataJSON();
      if (typeof body === "object" && body !== null && "rate_bps" in body) {
        postedRates.push(body.rate_bps);
      }
    }
    await route.continue();
  });

  await page.goto("/products");

  await expect(page.getByRole("heading", { name: t("products_title") })).toBeVisible();
  await expect(page.getByText(t("products_empty"))).toBeVisible();

  await page.getByRole("button", { name: t("products_add") }).click();

  await page.getByLabel(t("field_name"), { exact: true }).fill(PRODUCT_NAME);
  await page.getByLabel(t("field_price"), { exact: true }).fill(PRICE_INPUT);
  await page.getByLabel(t("field_stock"), { exact: true }).fill(STOCK_INPUT);
  // By role, not by label: a <label> wrapping a <select> has the option
  // texts in its own text, so an exact label match never resolves.
  await page
    .getByRole("combobox", { name: t("field_rate"), exact: true })
    .selectOption({ label: t("rate_900") });
  await page.getByRole("button", { name: t("action_save") }).click();

  const row = page.getByRole("row").filter({ hasText: PRODUCT_NAME });
  await expect(row).toBeVisible();
  await expect(row.getByRole("cell", { name: PRICE_RENDERED, exact: true })).toBeVisible();
  await expect(row.getByRole("cell", { name: STOCK_RENDERED, exact: true })).toBeVisible();
  await expect(page.getByText(t("products_empty"))).toBeHidden();
  expect(postedRates).toEqual([900]);

  await page.screenshot({
    path: path.join(here, "screenshots", "products.png"),
    fullPage: true,
  });
});

test("refuses a product with an empty name", async ({ page }) => {
  // The form validator runs before the client ever posts. Counting the
  // POSTs proves that: the message below is the UI's own, and the API's
  // `validation` code has no path to the screen from this form.
  let productPosts = 0;
  await page.route("**/products", async (route) => {
    if (route.request().method() === "POST") productPosts += 1;
    await route.continue();
  });

  await page.goto("/products");
  await page.getByRole("button", { name: t("products_add") }).click();

  await page.getByLabel(t("field_price"), { exact: true }).fill(PRICE_INPUT);
  await page.getByRole("button", { name: t("action_save") }).click();

  await expect(page.getByRole("alert").filter({ hasText: t("error_name_required") })).toBeVisible();
  // The form is still open, so nothing was saved.
  await expect(page.getByRole("button", { name: t("action_save") })).toBeVisible();
  expect(productPosts).toBe(0);
});
