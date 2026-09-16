// Backup and restore driven through a real browser against a real API and a
// real SQLite file: the copy is taken, a product is added after it, the copy
// is restored, and the product is gone. Nothing is mocked, so a green run
// means the file on disk really was swapped underneath the running server.
//
// Strings come from the JSON dictionary of the Playwright project running
// the test (fr, en or ar), so the same run proves the block in all three.
//
// This spec runs first (the files run in name order, one worker, one
// database) and leaves the shop empty again, which is the state the products
// spec starts from.

import { expect, test } from "./auth";
import type { Locator } from "@playwright/test";
import { t } from "./messages";

const PRODUCT_NAME = "Café Bonal 250g";

/**
 * The copies a table lists. A table's rowgroups are its head and its body in
 * that order, so the second one is the rows without the header. Read by role
 * rather than by a testid on each row: the kit's `DataTable` draws every list
 * in the app and a screen does not get to decorate its rows.
 */
function copiesIn(table: Locator): Locator {
  return table.getByRole("rowgroup").nth(1).getByRole("row");
}

test("a copy taken before a product is added loses it when it is restored", async ({ page }) => {
  await page.goto("/settings/backups");
  await expect(page.getByRole("heading", { name: t("settings_backups") })).toBeVisible();
  // A fresh run starts with no copy at all.
  await expect(page.getByTestId("backups-newest")).toHaveText(t("backups_none"));

  await page.getByRole("button", { name: t("action_backup_now") }).click();
  await expect(page.getByRole("status")).toHaveText(t("backups_created"));
  const rows = copiesIn(page.getByTestId("backups-table"));
  await expect(rows).toHaveCount(1);

  // Added after the copy, so the restore has to lose it.
  await page.goto("/products");
  await page.getByRole("button", { name: t("products_add") }).click();
  // Not exact on the two required fields: the products fiche is on the
  // kit now and FormField puts a required marker inside the label. The
  // rate is the kit's select, a button and a listbox rather than a
  // <select>, so the option is clicked; exact, because "9 %" is a
  // substring of "19 %".
  await page.getByLabel(t("field_name")).fill(PRODUCT_NAME);
  await page.getByLabel(t("field_price")).fill("310");
  await page.getByLabel(t("field_stock"), { exact: true }).fill("6");
  await page.getByRole("combobox", { name: t("field_rate"), exact: true }).click();
  await page.getByRole("option", { name: t("rate_900"), exact: true }).click();
  await page.getByRole("button", { name: t("action_save") }).click();
  await expect(page.getByRole("row").filter({ hasText: PRODUCT_NAME })).toBeVisible();

  await page.goto("/settings/backups");
  const listed = copiesIn(page.getByTestId("backups-table"));
  await expect(listed).toHaveCount(1);
  // The restore asks before it throws anything away, in the app's own dialog
  // rather than the browser's box: the panel has to say what is lost in the
  // language the shop is running.
  await listed.first().getByRole("button", { name: t("action_restore") }).click();
  const asking = page.getByRole("dialog");
  await expect(asking).toContainText(t("backups_confirm_restore"));
  await asking.getByRole("button", { name: t("backups_restore_confirm") }).click();
  await expect(page.getByRole("status")).toHaveText(t("backups_restored"));

  // The same server, the same tab, the file underneath replaced.
  await page.goto("/products");
  await expect(page.getByText(t("products_empty"))).toBeVisible();
  await expect(page.getByRole("row").filter({ hasText: PRODUCT_NAME })).toHaveCount(0);

  // The copy that was restored is still there, and the copy of what it
  // replaced is listed under its own heading rather than left unmentioned
  // on the disk.
  await page.goto("/settings/backups");
  await expect(copiesIn(page.getByTestId("backups-table"))).toHaveCount(1);
  await expect(page.getByRole("heading", { name: t("settings_safety_copies") })).toBeVisible();
  const kept = copiesIn(page.getByTestId("safety-copies-table"));
  await expect(kept).toHaveCount(1);
  // It carries the same restore button as a daily copy, which is how an
  // owner undoes the restore they have just made.
  await expect(kept.getByRole("button", { name: t("action_restore") })).toHaveCount(1);
});
