// The exports, the product import and the shelf labels driven through a
// real browser against a real API and a real SQLite file. Nothing is
// mocked: the workbook that arrives is the one rust_xlsxwriter wrote, and
// the product the import creates is read back off the products screen.
//
// Strings come from the JSON dictionary of the Playwright project running
// the test (fr, en or ar), so the same run proves the panel in all three.
//
// The name puts it last on purpose. Files run in name order against one
// database, and this one creates a product from a file: a spec that starts
// from an empty catalogue (`products.spec.ts` does) must have run already.
// Nothing runs after it, so the row it leaves is nobody's problem.

import { expect, test } from "@playwright/test";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { currentLang, t } from "./messages";

// One database, one worker: the label tests read the product the import
// test created, so a failure in that one must not read as three.
test.describe.configure({ mode: "serial" });

const here = fileURLToPath(new URL(".", import.meta.url));

/** The one row of the committed fixture. Written from the template itself
 * by `crates/core/tests/import_service.rs`, so a column renamed in the
 * service rewrites the file rather than breaking this spec silently.
 *
 * Its barcode cell is blank, the way the template's example row is, so the
 * shop's own in-store EAN-13 is what `services::products::create` puts on
 * it. Which number that is depends on how many blank-coded products the
 * run made before, so the specs below read it off the row rather than
 * naming it. */
const IMPORTED = "Café importé 250 g";

/** The first four bytes of every .xlsx: a workbook is a zip. Checked on the
 * bytes rather than on the header alone, because a server that answered
 * JSON with a spreadsheet content type would pass a header-only check and
 * hand a shop a file Excel refuses to open. */
const ZIP = [0x50, 0x4b, 0x03, 0x04];

test("the four workbooks come out as spreadsheets a shop can open", async ({ page }) => {
  await page.goto("/settings");
  await expect(page.getByRole("heading", { name: t("settings_export_import") })).toBeVisible();

  // Through the buttons, not around them: the answer read here is the one
  // the app's own call got, with the launch token the window holds, and the
  // download the panel then hands to the browser is the real path.
  for (const kind of ["products", "sales", "customers", "suppliers"]) {
    const answering = page.waitForResponse((r) => r.url().includes(`/export/${kind}`));
    const downloading = page.waitForEvent("download");
    await page.getByTestId(`export-${kind}`).click();
    const answer = await answering;
    expect(answer.status(), kind).toBe(200);
    expect(answer.headers()["content-type"], kind).toContain("spreadsheetml.sheet");
    expect(answer.headers()["content-disposition"], kind).toContain(".xlsx");
    const bytes = await answer.body();
    expect([...bytes.subarray(0, 4)], kind).toEqual(ZIP);
    // And it really reaches the shop as a file, under the server's name.
    const saved = await downloading;
    expect(saved.suggestedFilename(), kind).toContain(".xlsx");
  }
});

test("the template downloads, a filled file is checked, and applying it creates the product", async ({
  page,
}) => {
  await page.goto("/settings");
  await expect(page.getByRole("heading", { name: t("settings_export_import") })).toBeVisible();

  // The template leaves through the browser's own download, the way the
  // exports do: no save dialog is wired into the Tauri shell.
  const downloading = page.waitForEvent("download");
  await page.getByTestId("import-template").click();
  const download = await downloading;
  expect(download.suggestedFilename()).toContain(".xlsx");

  // The fixture is that template with one cell edited, which is what a shop
  // does with it. It is committed rather than written here: a workbook a
  // test builds is a workbook nobody ever opened.
  await page
    .getByTestId("import-file")
    .setInputFiles(path.join(here, "fixtures", "products-import.xlsx"));
  await page.getByTestId("import-dry-run").click();

  const rows = page.getByTestId("import-row");
  await expect(rows).toHaveCount(1);
  await expect(rows.first()).toContainText(IMPORTED);
  await expect(rows.first()).toContainText(t("import_outcome_created"));
  await expect(page.getByTestId("import-counts")).toBeVisible();

  // Nothing is written until the second decision.
  await page.goto("/products");
  await expect(page.getByRole("row").filter({ hasText: IMPORTED })).toHaveCount(0);

  await page.goto("/settings");
  await page
    .getByTestId("import-file")
    .setInputFiles(path.join(here, "fixtures", "products-import.xlsx"));
  await page.getByTestId("import-dry-run").click();
  await expect(page.getByTestId("import-row")).toHaveCount(1);
  await page.getByTestId("import-apply").click();
  await expect(page.getByTestId("import-done")).toBeVisible();

  await page.goto("/products");
  await expect(page.getByRole("row").filter({ hasText: IMPORTED })).toBeVisible();
});

test("the label of the imported product carries its bars and its digits", async ({ page }) => {
  await page.goto("/products");
  const row = page.getByRole("row").filter({ hasText: IMPORTED });
  await expect(row).toBeVisible();
  // The in-store code the shop gave it, read off the list rather than
  // named here: the label has to carry that number and no other.
  const code = (await row.getByRole("cell").nth(2).innerText()).trim();
  expect(code).toMatch(/^\d{13}$/);
  await row.getByRole("button", { name: new RegExp(t("products_edit")) }).click();
  await page.getByTestId("print-label").click();

  // The page is the core's, rendered into the sandboxed frame the ticket
  // uses. What is checked here is that it is the right product's label:
  // the goldens in dzpos-core own the layout.
  const frame = page.frameLocator('[data-testid="product-label"]');
  await expect(frame.locator("svg").first()).toBeVisible();
  await expect(frame.getByText(code)).toBeVisible();
  await expect(frame.getByText(IMPORTED)).toBeVisible();

  // The one committed screenshot for this spec: Arabic, so the RTL panel
  // with a barcode still read left to right has a reference image.
  if (currentLang() === "ar") {
    await page.screenshot({
      path: path.join(here, "screenshots", "product-label-ar.png"),
      fullPage: true,
    });
  }
});

test("a sheet is the ticked rows and nothing is offered while none is ticked", async ({ page }) => {
  await page.goto("/products");
  const sheet = page.getByTestId("print-selected-labels");
  await expect(sheet).toBeDisabled();

  const row = page.getByRole("row").filter({ hasText: IMPORTED });
  const code = (await row.getByRole("cell").nth(2).innerText()).trim();
  await row.getByRole("checkbox").check();
  await expect(sheet).toBeEnabled();
  await sheet.click();

  const frame = page.frameLocator('[data-testid="product-label"]');
  await expect(frame.getByText(code)).toBeVisible();
});
