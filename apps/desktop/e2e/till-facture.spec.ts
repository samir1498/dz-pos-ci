// A facture rung up at the till, end to end: a real browser, the real axum
// API, the real SQLite file. The shop's own block carries RC and NIS, the
// customer's fiche carries theirs, and the cashier flips the one-tap switch
// before paying. What is then true at once: the document is a `facture` in
// its own series, the confirmation says the number the paper prints, the
// page in the print panel carries the buyer's identifiers and the net, and
// the ticket series never moved for any of it.
//
// The products are named after this spec. The suites share one shop file and
// the till picks a tile by a piece of a name, so a name another spec also
// uses would make the click ambiguous.

import { expect, test } from "@playwright/test";
import type { APIRequestContext, Page } from "@playwright/test";
import { formatCentimes } from "@dzpos/shared";
import { apiHeaders, apiUrl } from "./api";
import { t } from "./messages";

const CUSTOMER = "Entreprise Facture e2e";
const CUSTOMER_RC = "16/00-7654321 B 22";
const CUSTOMER_NIS = "098216007654321";

/** Exempt of TVA and sold on credit, so the net to pay is the price: no
 * rounding and no droit de timbre stand between the tile and the paper. */
const BEAM = "Poutrelle facture e2e";
const BEAM_BARCODE = "6130009100011";
const BEAM_PRICE = 300_000;

interface Sale {
  id: number;
  kind: string;
  series: string;
  number: number;
  printed_number: string;
  totals: { net_to_pay_centimes: number };
}

async function seedStoreBlock(request: APIRequestContext): Promise<void> {
  // Décret 05-468 art. 3 asks the seller for these, and the core refuses a
  // facture until the settings carry them. The screen is tested elsewhere;
  // here the block is put in place the way a shop owner would have already.
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

async function seedProduct(request: APIRequestContext): Promise<void> {
  const res = await request.post(`${apiUrl()}/products`, {
    headers: apiHeaders(),
    data: {
      name: BEAM,
      barcode: BEAM_BARCODE,
      category_id: 1,
      unit: "piece",
      cost_centimes: 0,
      selling_centimes: BEAM_PRICE,
      wholesale_centimes: null,
      qty_on_hand_milli: 10_000,
      low_stock_at_milli: 0,
      rate_bps: 0,
      active: true,
    },
  });
  expect(res.status()).toBe(201);
}

async function seedCustomer(request: APIRequestContext): Promise<number> {
  const res = await request.post(`${apiUrl()}/customers`, {
    headers: apiHeaders(),
    data: {
      name: CUSTOMER,
      party_kind: "company",
      phone: "0555 44 33 22",
      address: "Zone industrielle, Rouiba",
      rc: CUSTOMER_RC,
      nif: null,
      nis: CUSTOMER_NIS,
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

async function readSale(request: APIRequestContext, id: number): Promise<Sale> {
  const res = await request.get(`${apiUrl()}/sales/${id}`, { headers: apiHeaders() });
  expect(res.ok()).toBe(true);
  return res.json();
}

/** The highest number the ticket series has handed out. The list carries
 * every kind now, so it is asked for the one series: a facture's number
 * comes out of its own counter and says nothing about this one. */
async function lastTicketNumber(request: APIRequestContext): Promise<number> {
  const res = await request.get(`${apiUrl()}/sales?kind=ticket`, { headers: apiHeaders() });
  expect(res.ok()).toBe(true);
  const tickets: { number: number }[] = await res.json();
  return tickets.reduce((high, one) => Math.max(high, one.number), 0);
}

async function pick(page: Page, name: string): Promise<void> {
  await page.getByLabel(t("till_customer_search"), { exact: true }).fill(name);
  const select = page.getByLabel(t("till_customer"), { exact: true });
  await expect(select.getByRole("option", { name })).toBeAttached();
  await select.selectOption({ label: name });
}

async function addBeam(page: Page): Promise<void> {
  await page.getByLabel(t("till_search"), { exact: true }).fill(BEAM);
  await page.getByTestId("tiles").getByRole("button", { name: BEAM }).click();
}

function postedSale(page: Page) {
  return page.waitForResponse(
    (res) => res.url().endsWith("/sales") && res.request().method() === "POST",
  );
}

test("rings a facture up on credit, prints it, and leaves the ticket series where it was", async ({
  page,
  request,
}) => {
  await seedStoreBlock(request);
  await seedProduct(request);
  await seedCustomer(request);
  const ticketsBefore = await lastTicketNumber(request);

  await page.goto("/");
  await expect(page).toHaveURL(/\/till$/);

  // Nobody picked, so the facture is not on offer: it is made out to a
  // named buyer (décret 05-468 art. 3).
  const facture = page.getByRole("radio", { name: t("till_facture"), exact: true });
  await expect(facture).toBeDisabled();
  await pick(page, CUSTOMER);
  await expect(facture).toBeEnabled();

  const issued = postedSale(page);
  await addBeam(page);
  await facture.click();
  await page.getByRole("radio", { name: t("pay_credit"), exact: true }).click();
  await page.getByRole("button", { name: t("action_pay"), exact: true }).click();

  const response = await issued;
  expect(response.status()).toBe(201);
  const sale: Sale = await response.json();
  expect(sale.kind).toBe("facture");
  expect(sale.series).toBe("doc_facture");
  expect(sale.number).toBe(1);
  expect(sale.printed_number).toBe("FA-000001");
  expect(sale.totals.net_to_pay_centimes).toBe(BEAM_PRICE);

  // The stored document says the same when it is read back: the kind is a
  // column, not something the create answered once.
  const stored = await readSale(request, sale.id);
  expect(stored.kind).toBe("facture");
  expect(stored.series).toBe("doc_facture");
  expect(stored.number).toBe(1);

  // And it is on the list of what the till has issued, so a cashier can
  // reach it again once the print panel is closed.
  const listed = await request.get(`${apiUrl()}/sales`, { headers: apiHeaders() });
  expect(listed.ok()).toBe(true);
  const rows: Sale[] = await listed.json();
  expect(rows.some((row) => row.id === sale.id && row.kind === "facture")).toBe(true);

  // The confirmation names the paper and the number as the paper spells it.
  const done = page.getByRole("status");
  await expect(done).toContainText(t("till_paid_facture"));
  await expect(done.getByTestId("till-document-number")).toHaveText("FA-000001");

  // The page in the panel is the facture the core rendered: the buyer's own
  // identifiers are on it, and so is the net, formatted the core's way.
  await page.getByRole("button", { name: t("till_print"), exact: true }).click();
  const panel = page.getByRole("region", { name: t("till_receipt") });
  const printed = panel.frameLocator("iframe");
  await expect(printed.getByText(CUSTOMER)).toBeVisible();
  await expect(printed.getByText(CUSTOMER_RC)).toBeVisible();
  await expect(printed.getByText(CUSTOMER_NIS)).toBeVisible();
  await expect(printed.locator(".amount-net-to-pay")).toHaveText(formatCentimes(BEAM_PRICE));

  // The half sheet is the same facture on smaller paper.
  await panel.getByRole("radio", { name: t("till_paper_a5"), exact: true }).click();
  await expect(printed.locator(".amount-net-to-pay")).toHaveText(formatCentimes(BEAM_PRICE));

  // A second basket, this time a ticket: its own series carried on from
  // where it was, untouched by the facture in between.
  await page.getByRole("button", { name: t("till_new_sale"), exact: true }).click();
  const second = postedSale(page);
  await addBeam(page);
  await page.getByLabel(t("field_tendered"), { exact: true }).fill("5000");
  await page.getByRole("button", { name: t("action_pay"), exact: true }).click();
  const ticket: Sale = await (await second).json();
  expect(ticket.kind).toBe("ticket");
  expect(ticket.series).toBe("doc_ticket");
  expect(ticket.number).toBe(ticketsBefore + 1);

  // And the facture kept its own number through all of it.
  expect((await readSale(request, sale.id)).number).toBe(1);
});
