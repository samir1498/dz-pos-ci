// Backup and restore driven through a real browser against a real API and a
// real SQLite file: the copy is taken, a product is added after it, the copy
// is restored, and the product is gone. Nothing is mocked, so a green run
// means the file on disk really was swapped underneath the running server.
//
// This spec runs first (the files run in name order, one worker, one
// database) and leaves the shop empty again, which is the state the products
// spec starts from.

import { expect, test } from "@playwright/test";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = fileURLToPath(new URL(".", import.meta.url));
const frPath = path.join(here, "..", "src", "i18n", "fr.json");

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

const PRODUCT_NAME = "Café Bonal 250g";

test("a copy taken before a product is added loses it when it is restored", async ({ page }) => {
  // The restore asks before it throws anything away; the browser's own
  // dialog is what the panel uses in M1.
  const asked: string[] = [];
  page.on("dialog", async (dialog) => {
    asked.push(dialog.message());
    await dialog.accept();
  });

  await page.goto("/settings");
  await expect(page.getByRole("heading", { name: t("settings_backups") })).toBeVisible();
  // A fresh run starts with no copy at all.
  await expect(page.getByTestId("backups-newest")).toHaveText(t("backups_none"));

  await page.getByRole("button", { name: t("action_backup_now") }).click();
  await expect(page.getByRole("status")).toHaveText(t("backups_created"));
  const rows = page.getByTestId("backup-row");
  await expect(rows).toHaveCount(1);

  // Added after the copy, so the restore has to lose it.
  await page.goto("/products");
  await page.getByRole("button", { name: t("products_add") }).click();
  await page.getByLabel(t("field_name"), { exact: true }).fill(PRODUCT_NAME);
  await page.getByLabel(t("field_price"), { exact: true }).fill("310");
  await page.getByLabel(t("field_stock"), { exact: true }).fill("6");
  await page
    .getByRole("combobox", { name: t("field_rate"), exact: true })
    .selectOption({ label: t("rate_900") });
  await page.getByRole("button", { name: t("action_save") }).click();
  await expect(page.getByRole("row").filter({ hasText: PRODUCT_NAME })).toBeVisible();

  await page.goto("/settings");
  await expect(page.getByTestId("backup-row")).toHaveCount(1);
  await page
    .getByTestId("backup-row")
    .first()
    .getByRole("button", { name: t("action_restore") })
    .click();
  await expect(page.getByRole("status")).toHaveText(t("backups_restored"));
  expect(asked).toEqual([t("backups_confirm_restore")]);

  // The same server, the same tab, the file underneath replaced.
  await page.goto("/products");
  await expect(page.getByText(t("products_empty"))).toBeVisible();
  await expect(page.getByRole("row").filter({ hasText: PRODUCT_NAME })).toHaveCount(0);

  // The copy that was restored is still there, and the safety copy taken on
  // the way is not among the thirty: it sits beside the shop file.
  await page.goto("/settings");
  await expect(page.getByTestId("backup-row")).toHaveCount(1);
});
