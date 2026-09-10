// The settings screen driven through a real browser against a real API:
// the store block a ticket prints and the dated régime fiscal. Strings
// come from the JSON dictionary of the Playwright project running the
// test (fr, en or ar), so a reworded label fails here rather than passing
// a hardcoded sentence, in every language.

import { expect, test } from "@playwright/test";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { currentLang, t } from "./messages";

const here = fileURLToPath(new URL(".", import.meta.url));

const STORE_NAME = "Superette El Baraka";
const RC = "16/00-1234567 B 20";
const NIF = "000016001234567";
const PHONE = "0555 12 34 56";

/** A day as the API writes it, `YYYY-MM-DD`, on the calendar the server
 * reads "today" from: Algeria's, UTC+1 with no daylight saving. */
function day(offsetDays: number): string {
  const d = new Date(Date.now() + 3600_000);
  d.setUTCDate(d.getUTCDate() + offsetDays);
  return d.toISOString().slice(0, 10);
}

test("saves the store block, reads it back after a reload, and saves the settings screenshot in Arabic", async ({ page }) => {
  const putBodies: unknown[] = [];
  await page.route("**/settings/store", async (route) => {
    if (route.request().method() === "PUT") putBodies.push(route.request().postDataJSON());
    await route.continue();
  });

  await page.goto("/products");
  await page.getByRole("link", { name: t("nav_settings") }).click();
  await expect(page.getByRole("heading", { name: t("settings_title") })).toBeVisible();

  const name = page.getByLabel(t("field_name"), { exact: true });
  // The seeded shop, so the database is the fresh one the run started.
  await expect(name).toHaveValue("Mon magasin");
  await name.fill(STORE_NAME);
  await page.getByLabel(t("field_rc"), { exact: true }).fill(RC);
  await page.getByLabel(t("field_nif"), { exact: true }).fill(NIF);
  await page.getByLabel(t("field_phone"), { exact: true }).fill(PHONE);
  const storeForm = page.getByRole("form", { name: t("settings_store") });
  await storeForm.getByRole("button", { name: t("action_save") }).click();
  await expect(page.getByRole("status")).toHaveText(t("settings_saved"));
  expect(putBodies).toEqual([
    { name: STORE_NAME, rc: RC, nif: NIF, nis: null, ai: null, address: null, phone: PHONE },
  ]);

  // What the screen shows after a reload is what the file holds.
  await page.reload();
  await expect(page.getByLabel(t("field_name"), { exact: true })).toHaveValue(STORE_NAME);
  await expect(page.getByLabel(t("field_rc"), { exact: true })).toHaveValue(RC);
  await expect(page.getByLabel(t("field_nis"), { exact: true })).toHaveValue("");
  await expect(page.getByLabel(t("field_phone"), { exact: true })).toHaveValue(PHONE);

  // The one committed settings screenshot: Arabic, so the RTL layout of a
  // form (fields mirrored, fiscal identifiers still left to right) has a
  // reference image, the way products.png does for fr.
  if (currentLang() === "ar") {
    await page.screenshot({
      path: path.join(here, "screenshots", "settings-ar.png"),
      fullPage: true,
    });
  }
});

test("a régime change dated ahead is planned; dated back it is in force", async ({ page }) => {
  await page.goto("/settings");
  const current = page.getByTestId("regime-current");
  await expect(current).toContainText(t("regime_reel"));
  await expect(page.getByTestId("regime-planned")).toHaveCount(0);

  const regimeForm = page.getByRole("form", { name: t("settings_regime") });
  const regime = regimeForm.getByRole("combobox", { name: t("field_regime") });
  const from = regimeForm.getByLabel(t("field_valid_from"), { exact: true });
  const apply = regimeForm.getByRole("button", { name: t("action_apply") });

  // Tomorrow: the change is planned, réel stays in force.
  await regime.selectOption("ifu");
  await from.fill(day(1));
  await apply.click();
  const planned = page.getByTestId("regime-planned");
  await expect(planned).toContainText(t("regime_ifu"));
  await expect(planned).toContainText(day(1));
  await expect(current).toContainText(t("regime_reel"));

  // Yesterday: in force at once, and the planned line is gone after the
  // reload because the server's answer, not the screen, decides.
  await regime.selectOption("ifu");
  await from.fill(day(-1));
  await apply.click();
  await expect(current).toContainText(`${t("regime_ifu")} · ${t("regime_since")} ${day(-1)}`);
  await page.reload();
  await expect(page.getByTestId("regime-current")).toContainText(t("regime_ifu"));

  // Back to the réel before leaving, dated today so it is in force at once.
  // The suites share one shop file and this one runs early in the alphabet:
  // every till spec after it would otherwise ring its baskets up under the
  // IFU, where no facture prints a TVA recap. The earlier rows stay, which
  // is the point of a dated régime, so the two changes above are still on
  // the record.
  await regimeForm.getByRole("combobox", { name: t("field_regime") }).selectOption("reel");
  await regimeForm.getByLabel(t("field_valid_from"), { exact: true }).fill(day(0));
  await regimeForm.getByRole("button", { name: t("action_apply") }).click();
  await expect(page.getByTestId("regime-current")).toContainText(t("regime_reel"));
});
