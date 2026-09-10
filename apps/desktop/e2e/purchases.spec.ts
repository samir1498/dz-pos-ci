// The purchases screens driven through a real browser against the real axum
// API and a real SQLite file. Expected strings come from the JSON dictionary
// of the Playwright project running the test (fr, en or ar), so a reworded
// message fails here instead of silently passing a hardcoded sentence.
//
// What the run proves end to end: an order of two products with extra costs
// is written, the extra costs land on the lines, two deliveries take it in,
// the stock rises by what arrived and the supplier's balance by what it was
// worth at the landed cost, and a return takes both back off again. Every
// figure is read through the API as well as off the screen, so a screen that
// agreed with a server that stored the wrong thing would still fail.

import { expect, test } from "@playwright/test";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { apiHeaders, apiUrl } from "./api";
import { currentLang, t } from "./messages";

// `new URL(".", ...)` is already this file's directory (apps/desktop/e2e).
const here = fileURLToPath(new URL(".", import.meta.url));

/** One name per project, so the three runs share a database file without
 * tripping over the unique name a supplier fiche carries. */
function supplierName(): string {
  return `Grossiste ${currentLang()}`;
}

function productName(which: string): string {
  return `${which} ${currentLang()}`;
}

/** Ten sacks at 200,00 and twenty at 90,00: 2 000,00 and 1 800,00 of value,
 * with 380,00 of transport spread over the two by that value. */
const FARINE_QTY_MILLI = 10_000;
const FARINE_COST = 20_000;
const SUCRE_QTY_MILLI = 20_000;
const SUCRE_COST = 9_000;
const TRANSPORT_CENTIMES = 38_000;
/** 38 000 × 200 000 / 380 000 = 20 000, over ten units: 2 000 a unit. */
const FARINE_LANDED = FARINE_COST + 2_000;
/** The remainder, 18 000, over twenty units: 900 a unit. */
const SUCRE_LANDED = SUCRE_COST + 900;

interface Line {
  id: number;
  product_id: number;
  landed_unit_cost_centimes: number;
  qty_received_milli: number;
  qty_returned_milli: number;
}

interface Detail {
  purchase: { id: number; status: string };
  lines: Line[];
  receipts: { series: string; number: number }[];
}

type Request = import("@playwright/test").APIRequestContext;
type Page = import("@playwright/test").Page;
type Locator = import("@playwright/test").Locator;

/** Pick a value in a kit select: open the trigger, click the option. The
 * listbox is in a portal, so the option is looked for on the page and not
 * inside the trigger. */
async function choose(page: Page, trigger: Locator, option: string): Promise<void> {
  await trigger.click();
  await page.getByRole("option", { name: option, exact: true }).click();
}

async function aProduct(request: Request, name: string, cost: number): Promise<number> {
  const res = await request.post(`${apiUrl()}/products`, {
    headers: apiHeaders(),
    data: {
      name,
      barcode: null,
      category_id: null,
      unit: "piece",
      cost_centimes: cost,
      selling_centimes: cost * 2,
      wholesale_centimes: null,
      qty_on_hand_milli: 0,
      low_stock_at_milli: 0,
      rate_bps: 1900,
      active: true,
    },
  });
  expect(res.status()).toBe(201);
  const made: { id: number } = await res.json();
  return made.id;
}

async function aSupplier(request: Request, name: string): Promise<number> {
  const res = await request.post(`${apiUrl()}/suppliers`, {
    headers: apiHeaders(),
    data: {
      name,
      phone: null,
      address: null,
      rc: null,
      nif: null,
      nis: null,
      ai: null,
      notes: null,
      active: true,
      opening_debt_centimes: null,
    },
  });
  expect(res.status()).toBe(201);
  const made: { id: number } = await res.json();
  return made.id;
}

async function onHand(request: Request, productId: number): Promise<number> {
  const res = await request.get(`${apiUrl()}/products`, { headers: apiHeaders() });
  const all: { id: number; qty_on_hand_milli: number }[] = await res.json();
  const found = all.find((p) => p.id === productId);
  if (found === undefined) throw new Error(`the API does not know product ${String(productId)}`);
  return found.qty_on_hand_milli;
}

async function owed(request: Request, supplierId: number): Promise<number> {
  const res = await request.get(`${apiUrl()}/suppliers/${supplierId}`, { headers: apiHeaders() });
  const one: { balance_centimes: number } = await res.json();
  return one.balance_centimes;
}

async function order(request: Request, id: number): Promise<Detail> {
  const res = await request.get(`${apiUrl()}/purchases/${id}`, { headers: apiHeaders() });
  expect(res.ok()).toBe(true);
  return res.json();
}

test("orders two products with extra costs, takes them in twice, sends some back, and saves the purchases screenshot", async ({
  page,
  request,
}) => {
  const supplier = supplierName();
  const farineName = productName("Farine 5kg");
  const sucreName = productName("Sucre 1kg");
  const supplierId = await aSupplier(request, supplier);
  const farine = await aProduct(request, farineName, FARINE_COST);
  const sucre = await aProduct(request, sucreName, SUCRE_COST);

  // The order itself is written on the screen, because the spread of the
  // extra costs over the lines is what this run is about and the form is
  // where a shop states them.
  await page.goto("/purchases/new");
  await expect(page.getByRole("heading", { name: t("purchases_new") })).toBeVisible();
  // By role and accessible name. The kit's select is a Radix trigger and a
  // listbox in a portal, not a native <select>, so the value is chosen by
  // opening it and clicking the option rather than with selectOption.
  const pickSupplier = page.getByRole("combobox", { name: t("col_supplier"), exact: true });
  await choose(page, pickSupplier, supplier);

  const pickProduct = page.getByRole("combobox", { name: t("col_product"), exact: true });
  const qty = page.getByRole("textbox", { name: t("col_qty"), exact: true });
  const cost = page.getByRole("textbox", { name: t("col_unit_cost"), exact: true });
  await choose(page, pickProduct.first(), farineName);
  await qty.first().fill("10");
  await cost.first().fill("200");

  await page.getByRole("button", { name: t("purchases_add_line") }).click();
  await choose(page, pickProduct.nth(1), sucreName);
  await qty.nth(1).fill("20");
  await cost.nth(1).fill("90");

  await page.getByRole("textbox", { name: t("field_transport"), exact: true }).fill("380");
  // The goods have not arrived: the delivery is the next step and the point
  // of the run is that nothing moves until it does.
  await page.getByRole("checkbox", { name: t("field_receive_now"), exact: true }).uncheck();
  await page.getByRole("button", { name: t("purchases_save") }).click();

  // The address of the order it made, waited for rather than read off a
  // heading: "Commande" is inside "Nouvelle commande", so a heading query
  // would still match the form the browser has not left yet.
  await page.waitForURL(/\/purchases\/\d+$/);
  await expect(page.getByRole("heading", { name: t("purchases_one"), exact: true })).toBeVisible();
  const purchaseId = Number(new URL(page.url()).pathname.split("/").pop());
  expect(Number.isInteger(purchaseId)).toBe(true);

  // The extra costs landed on the lines, by value, once and for all.
  const written = await order(request, purchaseId);
  expect(written.purchase.status).toBe("ordered");
  const farineLine = written.lines.find((l) => l.product_id === farine);
  const sucreLine = written.lines.find((l) => l.product_id === sucre);
  expect(farineLine?.landed_unit_cost_centimes).toBe(FARINE_LANDED);
  expect(sucreLine?.landed_unit_cost_centimes).toBe(SUCRE_LANDED);
  // Debt follows goods: nothing has arrived, so nothing is owed and nothing
  // is on the shelf.
  expect(await owed(request, supplierId)).toBe(0);
  expect(await onHand(request, farine)).toBe(0);

  if (farineLine === undefined || sucreLine === undefined) throw new Error("a line is missing");

  // The first delivery: four sacks of flour and nothing else. The delivery is
  // a dialog now, so it is opened before anything is typed into it.
  await page.getByRole("button", { name: t("purchases_receive"), exact: true }).click();
  await page.getByLabel(`${t("action_receive")} ${String(farineLine.id)}`).fill("4");
  await page.getByRole("button", { name: t("action_receive"), exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.getByTestId("purchase-status")).toHaveText(t("purchase_status_partially_received"));

  const afterFirst = await order(request, purchaseId);
  expect(afterFirst.purchase.status).toBe("partially_received");
  expect(afterFirst.receipts).toHaveLength(1);
  expect(await onHand(request, farine)).toBe(4_000);
  expect(await onHand(request, sucre)).toBe(0);
  expect(await owed(request, supplierId)).toBe(4 * FARINE_LANDED);

  // The second: the rest of both lines, on a bon de réception of its own.
  await page.getByRole("button", { name: t("purchases_receive"), exact: true }).click();
  await page.getByLabel(`${t("action_receive")} ${String(farineLine.id)}`).fill("6");
  await page.getByLabel(`${t("action_receive")} ${String(sucreLine.id)}`).fill("20");
  await page.getByRole("button", { name: t("action_receive"), exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.getByTestId("purchase-status")).toHaveText(t("purchase_status_received"));

  const afterSecond = await order(request, purchaseId);
  expect(afterSecond.purchase.status).toBe("received");
  expect(afterSecond.receipts).toHaveLength(2);
  expect(await onHand(request, farine)).toBe(FARINE_QTY_MILLI);
  expect(await onHand(request, sucre)).toBe(SUCRE_QTY_MILLI);
  const wholeOrder = 10 * FARINE_LANDED + 20 * SUCRE_LANDED;
  expect(await owed(request, supplierId)).toBe(wholeOrder);
  // The whole order is worth what it cost plus what it cost to get here, less
  // the centimes the per-unit division floored away.
  expect(wholeOrder).toBeLessThanOrEqual(
    10 * FARINE_COST + 20 * SUCRE_COST + TRANSPORT_CENTIMES,
  );

  // Two sacks of flour go back: the stock leaves and the debt with it. The
  // dialog closing is what says the server took it; its heading is gone with
  // it, so the closing itself is what is waited for.
  await page.getByRole("button", { name: t("purchases_return"), exact: true }).click();
  await expect(page.getByRole("heading", { name: t("purchases_return") })).toBeVisible();
  await page.getByLabel(`${t("action_return")} ${String(farineLine.id)}`).fill("2");
  await page.getByRole("button", { name: t("action_return"), exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);

  await expect
    .poll(() => onHand(request, farine))
    .toBe(FARINE_QTY_MILLI - 2_000);
  expect(await owed(request, supplierId)).toBe(wholeOrder - 2 * FARINE_LANDED);
  const afterReturn = await order(request, purchaseId);
  expect(afterReturn.lines.find((l) => l.product_id === farine)?.qty_returned_milli).toBe(2_000);

  if (currentLang() === "ar") {
    await page.screenshot({
      path: path.join(here, "screenshots", "purchases-ar.png"),
      fullPage: true,
    });
  }
});

test("refuses a cancellation once goods have arrived and closes the order short instead", async ({
  page,
  request,
}) => {
  const supplier = `Bouzid ${currentLang()}`;
  const supplierId = await aSupplier(request, supplier);
  const product = await aProduct(request, productName("Semoule 10kg"), 30_000);

  const res = await request.post(`${apiUrl()}/purchases`, {
    headers: apiHeaders(),
    data: {
      supplier_id: supplierId,
      purchase_date: "2026-09-10",
      lines: [{ product_id: product, qty_ordered_milli: 10_000, unit_cost_centimes: 30_000 }],
    },
  });
  expect(res.status()).toBe(201);
  const made: Detail = await res.json();
  const line = made.lines[0];

  await page.goto(`/purchases/${String(made.purchase.id)}`);
  // Nothing has arrived, so the cancel is the one on offer.
  await expect(page.getByRole("button", { name: t("purchases_cancel"), exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: t("purchases_close_short"), exact: true })).toHaveCount(0);

  await page.getByRole("button", { name: t("purchases_receive"), exact: true }).click();
  await page.getByLabel(`${t("action_receive")} ${String(line.id)}`).fill("3");
  await page.getByRole("button", { name: t("action_receive"), exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.getByTestId("purchase-status")).toHaveText(t("purchase_status_partially_received"));

  // And now the other way round: goods on the shelf say the order happened.
  await expect(page.getByRole("button", { name: t("purchases_cancel"), exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: t("purchases_close_short"), exact: true }).click();
  // A blank reason is refused by the screen before the server ever hears it.
  await page.getByRole("button", { name: t("action_close_short"), exact: true }).click();
  await expect(page.getByRole("alert")).toHaveText(t("purchases_reason_needed"));
  await page
    .getByRole("textbox", { name: t("purchases_reason"), exact: true })
    .fill("le reste ne viendra pas");
  await page.getByRole("button", { name: t("action_close_short"), exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);

  await expect(page.getByTestId("purchase-status")).toHaveText(t("purchase_status_closed_short"));
  const closed = await order(request, made.purchase.id);
  expect(closed.purchase.status).toBe("closed_short");
  // What arrived stays owed; what never came owes nothing.
  expect(await owed(request, supplierId)).toBe(3 * 30_000);
});
