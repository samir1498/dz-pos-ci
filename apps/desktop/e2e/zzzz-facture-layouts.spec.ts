// The layout a shop picks, and the facture it gets afterwards.
//
// The picker's own wiring is covered beside the component, and that each
// layout draws what décret 05-468 art. 3 asks for is covered by the goldens
// in `crates/core`. What is only true end to end is the sentence a shop
// owner would say: pick a layout in settings, open a facture, and the page
// is the one that was picked.
//
// The name is the ordering. The suites share one database and Playwright
// runs them by filename, and this spec seeds a product and rings up a
// facture, which is the kind of row every later spec can see: a product in
// the till's grid, a document in the list, a sale in the day's totals. Four
// committed screenshots and the dashboard's figures moved the first time
// this file was called `till-layouts.spec.ts`. So it sorts after
// `zzz-dashboard.spec.ts`, the last spec that photographs or counts
// anything, and leaves the shop back on the standard layout at the end.
//
// The restore is the last step and not a `finally`, so a failure between
// the two picks leaves the shop on the roll. That reaches nobody but a
// developer re-running this one file against a server they left up: every
// `just e2e` invocation deletes the database before its first test
// (`playwright.config.ts`, the webServer command).

// `test` from ./auth and not from Playwright: every route but the auth ones
// is behind the session guard, and that fixture signs both the browser and
// the standalone request context in before a spec runs.
import { expect, test } from "./auth";
import type { APIRequestContext, Page } from "@playwright/test";

import { apiHeaders, apiUrl } from "./api";
import { t } from "./messages";

const PRODUCT = "Tôle layout e2e";
const BARCODE = "6130000090011";
const PRICE = 120_000;
const BUYER = "Entreprise Layout e2e";

async function seedStoreBlock(request: APIRequestContext): Promise<void> {
  // The core refuses a facture until the seller's own identifiers are on
  // file. Put there the way a shop owner would have done before their first
  // facture; the screen that does it is tested elsewhere.
  const res = await request.put(`${apiUrl()}/settings/store`, {
    headers: apiHeaders(),
    data: {
      name: "Supérette El Bahdja",
      rc: "16/00-1234567 B 21",
      nif: "000216001234567",
      nis: "098216001234567",
      ai: "16123456789",
      address: "Rue Didouche Mourad, Alger",
      phone: "021 00 00 00",
    },
  });
  expect(res.status()).toBe(200);
}

async function seedProduct(request: APIRequestContext): Promise<number> {
  const res = await request.post(`${apiUrl()}/products`, {
    headers: apiHeaders(),
    data: {
      name: PRODUCT,
      barcode: BARCODE,
      category_id: 1,
      unit: "piece",
      cost_centimes: 0,
      selling_centimes: PRICE,
      wholesale_centimes: null,
      qty_on_hand_milli: 40_000,
      low_stock_at_milli: 0,
      rate_bps: 1900,
      active: true,
    },
  });
  expect(res.status()).toBe(201);
  const created: { id: number } = await res.json();
  return created.id;
}

async function seedBuyer(request: APIRequestContext): Promise<number> {
  const res = await request.post(`${apiUrl()}/customers`, {
    headers: apiHeaders(),
    data: {
      name: BUYER,
      party_kind: "company",
      phone: null,
      address: "Zone industrielle, Rouiba",
      rc: "16/00-7654321 B 22",
      nif: null,
      nis: "098216007654321",
      ai: null,
      credit_limit_centimes: null,
      warn_threshold_centimes: null,
      notes: null,
      active: true,
      opening_debt_centimes: null,
    },
  });
  expect(res.status()).toBe(201);
  const created: { id: number } = await res.json();
  return created.id;
}

async function sellOnCredit(
  request: APIRequestContext,
  productId: number,
  customerId: number,
): Promise<{ id: number; printed_number: string }> {
  const res = await request.post(`${apiUrl()}/sales`, {
    headers: apiHeaders(),
    data: {
      lines: [{ product_id: productId, qty_milli: 2_000 }],
      payment_mode: "credit",
      customer_id: customerId,
      kind: "facture",
    },
  });
  expect(res.status()).toBe(201);
  return res.json();
}

/** Pick a layout in the printing room and wait for the screen to read it
 *  back, which only happens once the server has answered.
 *
 *  Reached by clicking the sidebar, not by `page.goto`. A `goto` is a fresh
 *  load and throws the query cache away with the rest of the page, which
 *  would hide the thing this spec is here for: an owner does not reload the
 *  app between changing the setting and looking at the document, and the
 *  facture already in memory is keyed by the document, the language and the
 *  sheet, with no mention of the layout. */
async function pickLayout(page: Page, label: string): Promise<void> {
  await page.getByTestId("nav-settings").click();
  await page.getByRole("link", { name: t("settings_nav_printing") }).click();
  const picker = page.getByRole("combobox", { name: t("settings_facture_layout") });
  await picker.click();
  await page.getByRole("option", { name: label }).click();
  await expect(picker).toContainText(label);
}

/** Open a document on the documents screen and hand back its printed page.
 *  Through the sidebar for the same reason `pickLayout` is. */
async function openSheet(page: Page, printedNumber: string) {
  await page.getByTestId("nav-documents").click();
  await page.getByRole("button", { name: printedNumber }).click();
  await expect(page.getByTestId("documents-sheet")).toBeVisible();
  return page.frameLocator('[data-testid="documents-sheet"]');
}

test("a facture comes back in the layout the shop picked, and again when it changes", async ({
  page,
  request,
}) => {
  await seedStoreBlock(request);
  const product = await seedProduct(request);
  const buyer = await seedBuyer(request);
  const facture = await sellOnCredit(request, product, buyer);

  // One load, and every move after it is a click inside the app, so the
  // query cache lives across the whole test the way it does for an owner.
  await page.goto("/till");

  // The shop is on the standard sheet, which lays its lines out in a table
  // with a head over its columns. The head and not the table: the sheet has
  // a second one for the totals, and the column captions are the thing a
  // layout without columns cannot have.
  const standard = await openSheet(page, facture.printed_number);
  await expect(standard.locator("thead")).toHaveCount(1);
  await expect(standard.locator(".amount-net-to-pay")).toHaveCount(1);
  const net = await standard.locator(".amount-net-to-pay").textContent();
  expect(net).not.toBeNull();

  // The roll. 72 mm has no room for six columns, so the table is gone and a
  // line is two rows; the net to pay is still on it, and still the same
  // figure, because the layout is how the page is drawn and not what it
  // says.
  await pickLayout(page, t("facture_layout_roll_80mm"));
  const roll = await openSheet(page, facture.printed_number);
  // The counts are what catch a stale page: a cache that handed the sheet
  // back would answer two tables and no `.item`. The net to pay is here to
  // say the roll carries the same figure and not to catch the cache, and it
  // cannot: served the old page, it would match trivially. Anything that
  // replaces the counts has to keep something that tells the two apart.
  await expect(roll.locator("table")).toHaveCount(0);
  await expect(roll.locator(".item")).toHaveCount(1);
  await expect(roll.locator(".amount-line")).toHaveCount(1);
  await expect(roll.locator(".amount-net-to-pay")).toHaveText(net ?? "");

  // Both parties are still named, which is the line between a facture on a
  // roll and a ticket with a facture's title.
  await expect(roll.getByText(BUYER)).toBeVisible();
  await expect(roll.getByText("098216007654321")).toBeVisible();

  // And back to the standard sheet. This is the half a cache makes fail: the
  // standard page was rendered at the top of this test and is still in
  // memory under a key that says nothing about the layout, so the choice has
  // to drop it by hand or the screen hands the roll's predecessor straight
  // back. Leaving the shop on the standard layout is also what the specs
  // after this one were written against.
  await pickLayout(page, t("facture_layout_standard"));
  const again = await openSheet(page, facture.printed_number);
  await expect(again.locator("thead")).toHaveCount(1);
  await expect(again.locator(".amount-net-to-pay")).toHaveText(net ?? "");
});
