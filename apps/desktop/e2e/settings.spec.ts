// The settings screen driven through a real browser against a real API:
// the store block a ticket prints and the dated régime fiscal. Strings
// come from the JSON dictionary of the Playwright project running the
// test (fr, en or ar), so a reworded label fails here rather than passing
// a hardcoded sentence, in every language.

import { expect, test } from "./auth";
import type { APIRequestContext, Locator, Page } from "@playwright/test";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { apiHeaders, apiUrl } from "./api";
import { currentLang, t } from "./messages";
import { fillDate } from "./date-field";

const here = fileURLToPath(new URL(".", import.meta.url));

/**
 * Picks a régime in the kit's select. It is a listbox the app draws, not the
 * browser's own menu, so it is opened and its option is clicked; that is also
 * the only place the keyboard and the pointer path of the control are proved,
 * because jsdom cannot open it.
 */
async function chooseRegime(page: Page, form: Locator, label: string): Promise<void> {
  await form.getByRole("combobox", { name: t("field_regime") }).click();
  await page.getByRole("option", { name: label }).click();
}

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
  await expect(page.getByRole("main").getByRole("heading", { name: t("settings_title") })).toBeVisible();

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
  await page.goto("/settings/regime");
  const current = page.getByTestId("regime-current");
  await expect(current).toContainText(t("regime_reel"));
  await expect(page.getByTestId("regime-planned")).toHaveCount(0);

  const regimeForm = page.getByRole("form", { name: t("settings_regime") });
  const apply = regimeForm.getByRole("button", { name: t("action_apply") });

  // Tomorrow: the change is planned, réel stays in force.
  await chooseRegime(page, regimeForm, t("regime_ifu"));
  await fillDate(regimeForm, "regime-valid-from", day(1));
  await apply.click();
  const planned = page.getByTestId("regime-planned");
  await expect(planned).toContainText(t("regime_ifu"));
  await expect(planned).toContainText(day(1));
  await expect(current).toContainText(t("regime_reel"));

  // Yesterday: in force at once, and the planned line is gone after the
  // reload because the server's answer, not the screen, decides.
  await chooseRegime(page, regimeForm, t("regime_ifu"));
  await fillDate(regimeForm, "regime-valid-from", day(-1));
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
  await chooseRegime(page, regimeForm, t("regime_reel"));
  await fillDate(regimeForm, "regime-valid-from", day(0));
  await regimeForm.getByRole("button", { name: t("action_apply") }).click();
  await expect(page.getByTestId("regime-current")).toContainText(t("regime_reel"));
});

/**
 * The layout picker through a real browser.
 *
 * The picker's own wiring is covered by the jsdom test beside the component.
 * What only a browser can say is this: the kit's select is a listbox the app
 * draws, and jsdom cannot open it. This test is the choice surviving a round
 * trip to the server; the facture that comes back drawn in it is
 * `zzzz-facture-layouts.spec.ts`, which runs last because it needs a document to
 * open and this file runs before the till.
 */
test("the chosen facture layout survives a reload", async ({ page }) => {
  await page.goto("/settings/printing");
  const picker = page.getByRole("combobox", { name: t("settings_facture_layout") });
  await expect(picker).toContainText(t("facture_layout_standard"));

  await picker.click();
  await page.getByRole("option", { name: t("facture_layout_compact") }).click();
  await expect(picker).toContainText(t("facture_layout_compact"));

  // Read back off the server, not out of the screen's own memory.
  await page.reload();
  const reloaded = page.getByRole("combobox", { name: t("settings_facture_layout") });
  await expect(reloaded).toContainText(t("facture_layout_compact"));

  // Hand the shop back on the standard layout. Every spec in a run shares
  // one database and the suites are ordered by filename, so the specs that
  // print a facture after this one must meet the sheet they were written
  // against.
  await reloaded.click();
  await page.getByRole("option", { name: t("facture_layout_standard") }).click();
  await expect(reloaded).toContainText(t("facture_layout_standard"));
});

const PRINT_LANG_PRODUCT = "Café print-lang e2e";

async function seedPrintLangProduct(request: APIRequestContext): Promise<number> {
  const res = await request.post(`${apiUrl()}/products`, {
    headers: apiHeaders(),
    data: {
      name: PRINT_LANG_PRODUCT,
      barcode: null,
      category_id: null,
      unit: "piece",
      cost_centimes: 0,
      selling_centimes: 15_000,
      wholesale_centimes: null,
      qty_on_hand_milli: 10_000,
      low_stock_at_milli: 0,
      rate_bps: 1900,
      active: true,
    },
  });
  expect(res.status()).toBe(201);
  const created: { id: number } = await res.json();
  return created.id;
}

async function ringUpCashTicket(
  request: APIRequestContext,
  productId: number,
): Promise<{ printed_number: string }> {
  const res = await request.post(`${apiUrl()}/sales`, {
    headers: apiHeaders(),
    data: {
      lines: [
        {
          product_id: productId,
          qty_milli: 1_000,
          unit_price_centimes: null,
          line_discount_centimes: 0,
        },
      ],
      global_discount_centimes: 0,
      payment_mode: "cash",
      tendered_centimes: 100_000_000,
      customer_id: null,
      override: false,
      kind: "ticket",
    },
  });
  expect(res.status()).toBe(201);
  return res.json();
}

const LANG_NAME_KEY = {
  fr: "lang_name_fr",
  en: "lang_name_en",
  ar: "lang_name_ar",
} as const;

/**
 * The stored print language through a real browser, on a real printed
 * ticket.
 *
 * The picker's own wiring is covered by the jsdom test beside the
 * component; what only a browser and a real document prove is the sentence
 * the ruling is about: a shop stores a print language, and every fiscal
 * paper comes back in it even when the screen printing it is open in a
 * different one (`context/plans/20260920-a-print-language-the-shop-keeps.md`).
 *
 * The target is deliberately never the suite's own UI language: this file
 * runs once per Playwright project (fr, en, ar) against its own empty
 * database, and setting the print language to the language the suite is
 * already running in would prove nothing, since the ticket would come back
 * in it whether the stored setting won or the screen's own language did.
 */
test("the stored print language wins over the screen printing the ticket", async ({
  page,
  request,
}) => {
  const target = currentLang() === "ar" ? "fr" : "ar";
  const product = await seedPrintLangProduct(request);
  const ticket = await ringUpCashTicket(request, product);

  await page.goto("/settings/printing");
  const picker = page.getByRole("combobox", { name: t("settings_print_lang") });
  await picker.click();
  await page.getByRole("option", { name: t(LANG_NAME_KEY[target]) }).click();
  await expect(picker).toContainText(t(LANG_NAME_KEY[target]));

  await page.getByTestId("nav-documents").click();
  await page.getByRole("button", { name: ticket.printed_number }).click();
  const sheet = page.frameLocator('[data-testid="documents-sheet"]');
  await expect(sheet.locator("html")).toHaveAttribute("lang", target);
  await expect(sheet.locator("html")).toHaveAttribute("dir", target === "ar" ? "rtl" : "ltr");

  // Hand the shop back on following the till. Every spec in a run shares
  // one database, and the specs after this one were written against that
  // default, the way `zzzz-facture-layouts.spec.ts` restores the standard
  // layout it leaves with.
  await page.getByTestId("nav-settings").click();
  await page.getByRole("link", { name: t("settings_nav_printing") }).click();
  const reloaded = page.getByRole("combobox", { name: t("settings_print_lang") });
  await reloaded.click();
  await page.getByRole("option", { name: t("print_lang_follow_till") }).click();
  await expect(reloaded).toContainText(t("print_lang_follow_till"));
});
