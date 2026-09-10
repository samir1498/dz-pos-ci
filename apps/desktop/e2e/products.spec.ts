// The products screen driven through a real browser against a real API.
// Expected strings come from the JSON dictionary of the Playwright project
// running the test (fr, en or ar), so a reworded message fails here
// instead of silently passing a hardcoded sentence, and the same test
// proves the screen in all three languages.

import { expect, test } from "@playwright/test";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { currentLang, t } from "./messages";

// `new URL(".", ...)` is already this file's directory (apps/desktop/e2e).
const here = fileURLToPath(new URL(".", import.meta.url));

const PRODUCT_NAME = "Semoule extra 5kg";
// 250,50 DA typed in dinars becomes 25050 centimes; formatCentimes in
// packages/shared renders that back as "250,50". Kept under 1000 DA so no
// thousands separator (a narrow no-break space) enters the assertion.
const PRICE_INPUT = "250,50";
const PRICE_RENDERED = "250,50";
const STOCK_INPUT = "12";
const STOCK_RENDERED = "12";
const EDITED_NAME = "Lait Candia 1L";
const EDITED_PRICE_INPUT = "199,99";
const EDITED_PRICE_RENDERED = "199,99";

import type { Page } from "@playwright/test";

/**
 * The kit's select is Radix's: a button that opens a listbox, not a
 * `<select>`, so `selectOption` has nothing to act on. The trigger is opened
 * and the option is clicked by the word it shows. Exact, because "9 %" is a
 * substring of "19 %".
 */
async function chooseRate(page: Page, label: string) {
  await page.getByRole("combobox", { name: t("field_rate"), exact: true }).click();
  await page.getByRole("option", { name: label, exact: true }).click();
}

/** Adds a product through the form at 9 %, the way the first test does. */
async function addProduct(page: Page, name: string) {
  await page.getByRole("button", { name: t("products_add") }).click();
  // Not an exact match on the two required fields: the kit's FormField
  // puts a required marker inside the label, so the label reads "Nom *".
  await page.getByLabel(t("field_name")).fill(name);
  await page.getByLabel(t("field_price")).fill(PRICE_INPUT);
  await page.getByLabel(t("field_stock"), { exact: true }).fill(STOCK_INPUT);
  await chooseRate(page, t("rate_900"));
  await page.getByRole("button", { name: t("action_save") }).click();
}

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

  await expect(page.getByRole("main").getByRole("heading", { name: t("products_title") })).toBeVisible();
  await expect(page.getByText(t("products_empty"))).toBeVisible();

  await addProduct(page, PRODUCT_NAME);

  const row = page.getByRole("row").filter({ hasText: PRODUCT_NAME });
  await expect(row).toBeVisible();
  await expect(row.getByRole("cell", { name: PRICE_RENDERED, exact: true })).toBeVisible();
  await expect(row.getByRole("cell", { name: STOCK_RENDERED, exact: true })).toBeVisible();
  // The rate on the row comes from the API's answer, not from the form.
  await expect(row.getByRole("cell", { name: t("rate_900"), exact: true })).toBeVisible();
  await expect(page.getByText(t("products_empty"))).toBeHidden();
  expect(postedRates).toEqual([900]);

  // Only fr and ar keep a committed screenshot: fr is the reference shot,
  // ar is the one RTL screenshot the brief asks for. en adds nothing new
  // to look at once those two exist.
  const lang = currentLang();
  if (lang === "fr") {
    await page.screenshot({ path: path.join(here, "screenshots", "products.png"), fullPage: true });
  } else if (lang === "ar") {
    await page.screenshot({
      path: path.join(here, "screenshots", "products-ar.png"),
      fullPage: true,
    });
  }
});

test("edits a product in place and the row shows the stored values", async ({ page }) => {
  // Adds its own product so the test stands alone under --grep. The PUT
  // body is captured to prove the whole product was sent, the rate the
  // shop chose included, and the row is read back from the API after.
  const putBodies: unknown[] = [];
  await page.route("**/products/*", async (route) => {
    if (route.request().method() === "PUT") putBodies.push(route.request().postDataJSON());
    await route.continue();
  });

  await page.goto("/products");
  await addProduct(page, EDITED_NAME);
  const row = page.getByRole("row").filter({ hasText: EDITED_NAME });
  await expect(row).toBeVisible();
  await row.getByRole("button", { name: `${t("products_edit")} ${EDITED_NAME}` }).click();

  const price = page.getByLabel(t("field_price"));
  await expect(price).toHaveValue(PRICE_RENDERED);
  await price.fill(EDITED_PRICE_INPUT);
  await page.getByLabel(t("field_wholesale"), { exact: true }).fill("180");
  await page.getByLabel(t("field_low_stock"), { exact: true }).fill("3");
  await chooseRate(page, t("rate_1900"));
  await page.getByRole("button", { name: t("action_save") }).click();

  await expect(page.getByRole("button", { name: t("action_save") })).toBeHidden();
  await expect(
    row.getByRole("cell", { name: EDITED_PRICE_RENDERED, exact: true }),
  ).toBeVisible();
  await expect(row.getByRole("cell", { name: t("rate_1900"), exact: true })).toBeVisible();
  expect(putBodies).toHaveLength(1);
  // Every field of the product, the ones left alone included; the barcode
  // is the number the server gave it, sent back as it came.
  expect(putBodies[0]).toEqual({
    name: EDITED_NAME,
    barcode: expect.any(String),
    category_id: 1,
    unit: "piece",
    cost_centimes: 0,
    selling_centimes: 19_999,
    wholesale_centimes: 18_000,
    qty_on_hand_milli: 12_000,
    low_stock_at_milli: 3_000,
    rate_bps: 1900,
    active: true,
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

  await page.getByLabel(t("field_price")).fill(PRICE_INPUT);
  await page.getByRole("button", { name: t("action_save") }).click();

  await expect(page.getByRole("alert").filter({ hasText: t("error_name_required") })).toBeVisible();
  // The form is still open, so nothing was saved.
  await expect(page.getByRole("button", { name: t("action_save") })).toBeVisible();
  expect(productPosts).toBe(0);
});

test("the filter bar narrows the catalogue and says so in the count", async ({ page }) => {
  // Two products so a filter has something to remove, and both added
  // through the form so the test stands alone under --grep.
  await page.goto("/products");
  await addProduct(page, "Farine dorée 5kg");
  await addProduct(page, "Sucre roux 1kg");

  const search = page.getByTestId("products-search");
  await search.fill("farine");
  await expect(page.getByRole("row").filter({ hasText: "Farine dorée 5kg" })).toBeVisible();
  await expect(page.getByRole("row").filter({ hasText: "Sucre roux 1kg" })).toHaveCount(0);
  await expect(page.getByTestId("page-header")).toContainText(`1 ${t("products_count_one")}`);

  // A search that matches nothing is not the screen a shop with no products
  // sees: it offers to clear the filter, not to add the first product.
  await search.fill("zzzz");
  await expect(page.getByText(t("products_no_match"))).toBeVisible();
  await expect(page.getByText(t("products_empty"))).toHaveCount(0);

  await page.getByTestId("products-clear-filters").click();
  await expect(page.getByRole("row").filter({ hasText: "Sucre roux 1kg" })).toBeVisible();
  await expect(search).toHaveValue("");
});
