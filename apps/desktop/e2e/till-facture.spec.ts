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

import { expect, test } from "./auth";
import type { APIRequestContext, Page } from "@playwright/test";
import { formatCentimes, stamp } from "@dzpos/shared";
import { apiHeaders, apiUrl, printedNumber, seriesOf } from "./api";
import { t } from "./messages";




const CUSTOMER = "Entreprise Facture e2e";
const CUSTOMER_RC = "16/00-7654321 B 22";
const CUSTOMER_NIS = "098216007654321";

/** Exempt of TVA and sold on credit, so the net to pay is the price: no
 * rounding and no droit de timbre stand between the tile and the paper. */
const BEAM = "Poutrelle facture e2e";
const BEAM_BARCODE = "6130009100011";
const BEAM_PRICE = 300_000;

/** Taxed at the ordinary rate and paid in cash, so the second test walks the
 * other road: a TVA recap row and a droit de timbre on the same paper. */
const CEMENT = "Ciment facture e2e";
const CEMENT_BARCODE = "6130009100028";
const CEMENT_PRICE = 300_000;
const ORDINARY_RATE_BPS = 1900;

// 3 000,00 DA at 19 %: 570,00 of TVA, 3 570,00 TTC, and the timbre is one
// dinar per started tranche of 100,00 up to 30 000,00 TTC, so 36 tranches
// make 36,00. The shop has to be under the réel for any of it: under the
// IFU the recap is empty (features.md §3).
const CEMENT_TVA = 57_000;
const CEMENT_TTC = 357_000;
const CEMENT_STAMP = 3_600;
const CEMENT_NET = 360_600;
const CEMENT_TENDERED = "4000";

interface Sale {
  id: number;
  kind: string;
  series: string;
  number: number;
  printed_number: string;
  totals: {
    total_ht_centimes: number;
    tva_centimes: number;
    total_ttc_centimes: number;
    stamp_centimes: number;
    net_to_pay_centimes: number;
  };
  tva: { rate_bps: number; base_centimes: number; amount_centimes: number }[];
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

async function seedProduct(
  request: APIRequestContext,
  input: { name: string; barcode: string; price: number; rate: number },
): Promise<void> {
  const res = await request.post(`${apiUrl()}/products`, {
    headers: apiHeaders(),
    data: {
      name: input.name,
      barcode: input.barcode,
      category_id: 1,
      unit: "piece",
      cost_centimes: 0,
      selling_centimes: input.price,
      wholesale_centimes: null,
      qty_on_hand_milli: 10_000,
      low_stock_at_milli: 0,
      rate_bps: input.rate,
      active: true,
    },
  });
  expect(res.status()).toBe(201);
}

/** The day the shop dates its documents on, which is the core's and not this
 * machine's. A régime change is dated, so the day has to come from the same
 * calendar the core reads it back on. */
async function shopToday(request: APIRequestContext): Promise<string> {
  const res = await request.get(`${apiUrl()}/clock`, { headers: apiHeaders() });
  expect(res.ok()).toBe(true);
  const clock: { today: string } = await res.json();
  return clock.today;
}

/** The régime in force today, straight from the API. */
async function regime(request: APIRequestContext): Promise<string> {
  const res = await request.get(`${apiUrl()}/settings`, { headers: apiHeaders() });
  expect(res.ok()).toBe(true);
  const settings: { regime: { regime: string } } = await res.json();
  return settings.regime.regime;
}

/** Puts the shop under the réel for the sale this spec is about to ring up.
 * The suites share one shop file and this one asserts a TVA recap and a droit
 * de timbre, neither of which the IFU prints, so the régime is this spec's to
 * make sure of rather than something to inherit from whichever spec ran
 * before it. The change is only sent when it is a change: the core refuses a
 * régime that is already in force on the day given, because taking it would
 * move the "since" date a comptable reads to the day of the click. */
async function sellUnderTheReel(request: APIRequestContext): Promise<void> {
  if ((await regime(request)) !== "reel") {
    const res = await request.post(`${apiUrl()}/settings/regime`, {
      headers: apiHeaders(),
      data: { regime: "reel", valid_from: await shopToday(request) },
    });
    expect(res.status()).toBe(200);
  }
  expect(await regime(request)).toBe("reel");
}

test.beforeEach(async ({ request }) => {
  await sellUnderTheReel(request);
});

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

/** The search box narrows the list the server answers with, and the answer
 * itself is what commits the choice: the picker is the box and its answers
 * now, not a select. */
async function pick(page: Page, name: string): Promise<void> {
  await page.getByLabel(t("till_customer_search"), { exact: true }).fill(name);
  const list = page.getByRole("group", { name: t("till_customer") });
  await list.getByRole("button", { name }).click();
}

async function addOne(page: Page, name: string): Promise<void> {
  await page.getByLabel(t("till_search"), { exact: true }).fill(name);
  await page.getByTestId("tiles").getByRole("button", { name }).click();
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
  await seedProduct(request, { name: BEAM, barcode: BEAM_BARCODE, price: BEAM_PRICE, rate: 0 });
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
  await addOne(page, BEAM);
  await facture.click();
  await page.getByRole("radio", { name: t("pay_credit"), exact: true }).click();
  await page.getByRole("button", { name: t("action_pay"), exact: true }).click();

  const response = await issued;
  expect(response.status()).toBe(201);
  const sale: Sale = await response.json();
  expect(sale.kind).toBe("facture");
  expect(sale.series).toMatch(seriesOf("doc_facture"));
  expect(sale.number).toBe(1);
  expect(sale.printed_number).toMatch(printedNumber("FA", 1));
  expect(sale.totals.net_to_pay_centimes).toBe(BEAM_PRICE);

  // The stored document says the same when it is read back: the kind is a
  // column, not something the create answered once.
  const stored = await readSale(request, sale.id);
  expect(stored.kind).toBe("facture");
  expect(stored.series).toMatch(seriesOf("doc_facture"));
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
  await expect(done.getByTestId("till-document-number")).toHaveText(
    printedNumber("FA", 1),
  );

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
  await addOne(page, BEAM);
  await page.getByLabel(t("field_tendered"), { exact: true }).fill("5000");
  await page.getByRole("button", { name: t("action_pay"), exact: true }).click();
  const ticket: Sale = await (await second).json();
  expect(ticket.kind).toBe("ticket");
  expect(ticket.series).toMatch(seriesOf("doc_ticket"));
  expect(ticket.number).toBe(ticketsBefore + 1);

  // And the facture kept its own number through all of it.
  expect((await readSale(request, sale.id)).number).toBe(1);
});

test("a facture paid in cash carries the TVA recap and the droit de timbre", async ({
  page,
  request,
}) => {
  await seedStoreBlock(request);
  await seedProduct(request, {
    name: CEMENT,
    barcode: CEMENT_BARCODE,
    price: CEMENT_PRICE,
    rate: ORDINARY_RATE_BPS,
  });
  const customer = `${CUSTOMER} TVA`;
  const created = await request.post(`${apiUrl()}/customers`, {
    headers: apiHeaders(),
    data: {
      name: customer,
      party_kind: "company",
      phone: null,
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
  expect(created.status()).toBe(201);

  await page.goto("/");
  await expect(page).toHaveURL(/\/till$/);
  await pick(page, customer);

  const issued = postedSale(page);
  await addOne(page, CEMENT);
  await page.getByRole("radio", { name: t("till_facture"), exact: true }).click();
  await page.getByLabel(t("field_tendered"), { exact: true }).fill(CEMENT_TENDERED);
  await page.getByRole("button", { name: t("action_pay"), exact: true }).click();

  const sale: Sale = await (await issued).json();
  expect(sale.kind).toBe("facture");
  expect(sale.totals.total_ht_centimes).toBe(CEMENT_PRICE);
  expect(sale.totals.tva_centimes).toBe(CEMENT_TVA);
  expect(sale.totals.total_ttc_centimes).toBe(CEMENT_TTC);
  expect(sale.totals.stamp_centimes).toBe(CEMENT_STAMP);
  expect(sale.totals.net_to_pay_centimes).toBe(CEMENT_NET);
  // The client's own stamp table, which shares no code with the core's,
  // reads the same amount off the same TTC.
  expect(stamp(CEMENT_TTC, "cash")).toBe(CEMENT_STAMP);
  expect(sale.tva).toEqual([
    { rate_bps: ORDINARY_RATE_BPS, base_centimes: CEMENT_PRICE, amount_centimes: CEMENT_TVA },
  ]);

  // Read back from the file rather than from the answer to the create: the
  // recap is stored at issue so a reprint never recomputes it.
  const stored = await readSale(request, sale.id);
  expect(stored.tva).toEqual(sale.tva);
  expect(stored.totals.stamp_centimes).toBe(CEMENT_STAMP);

  // And the paper says all three: the rate, the tax, the timbre.
  await page.getByRole("button", { name: t("till_print"), exact: true }).click();
  const printed = page.getByRole("region", { name: t("till_receipt") }).frameLocator("iframe");
  await expect(printed.locator(".amount-tva")).toHaveText(formatCentimes(CEMENT_TVA));
  await expect(printed.locator(".amount-stamp")).toHaveText(formatCentimes(CEMENT_STAMP));
  await expect(printed.locator(".amount-net-to-pay")).toHaveText(formatCentimes(CEMENT_NET));
});
