// The avoir, the cancellation and the proforma end to end: a real browser,
// the real axum API, the real SQLite file. What is asserted is the stored
// answer of every step, because the whole point of these three writes is
// what they leave in the file.
//
// The products and the customers are named after this spec. The suites share
// one shop file and the till picks a tile by a piece of a name, so a name
// another spec also uses would make the click ambiguous.
//
// Playwright runs the files of a project in path order and they share one
// SQLite file, so the name places this one last but one. It seeds a product,
// two fiches and the shop's own block before it has anything to credit, and
// three earlier suites need what it would have written first: products.spec
// wants a table nobody has touched, settings.spec wants the store block as
// the migration seeded it, and till-facture.spec wants FA-000001 to be its
// own. That is the same reason settlement.spec and till-credit.spec sit
// where they do.

import { expect, test } from "@playwright/test";
import type { APIRequestContext } from "@playwright/test";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { apiHeaders, apiUrl } from "./api";
import { currentLang, t } from "./messages";

const here = fileURLToPath(new URL(".", import.meta.url));

/** Exempt of TVA and sold on credit, so the net to pay is the price: no
 *  rounding and no droit de timbre stand between a figure here and the
 *  amounts the ledger moves. */
const CEMENT = "Ciment documents e2e";
const CEMENT_BARCODE = "6130009300013";
const UNIT = 100_000;

const BUYER = "Entreprise Documents e2e";
/** Its own fiche, because the arithmetic of the credit case only works on a
 *  customer who owes nothing else: after the partial avoir the first one is
 *  1 500,00 down, and an avoir on a paid facture there would net against
 *  that rather than leaving a credit of its own. */
const PAID_BUYER = "Entreprise Avoir Payée e2e";

interface Sale {
  id: number;
  kind: string;
  series: string;
  number: number;
  printed_number: string;
  status: string;
  ref_document_id: number | null;
  cancellation: { reason: string; avoir_document_id: number | null } | null;
  balance: { remaining_debt_centimes: number } | null;
  totals: { net_to_pay_centimes: number };
  lines: { id: number; qty_milli: number }[];
}

async function seedStoreBlock(request: APIRequestContext): Promise<void> {
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
      name: CEMENT,
      barcode: CEMENT_BARCODE,
      category_id: 1,
      unit: "piece",
      cost_centimes: 0,
      selling_centimes: UNIT,
      wholesale_centimes: null,
      qty_on_hand_milli: 100_000,
      low_stock_at_milli: 0,
      rate_bps: 0,
      active: true,
    },
  });
  expect(res.status()).toBe(201);
  const created: { id: number } = await res.json();
  return created.id;
}

async function seedCustomer(request: APIRequestContext, name: string): Promise<number> {
  const res = await request.post(`${apiUrl()}/customers`, {
    headers: apiHeaders(),
    data: {
      name,
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
  qtyMilli: number,
): Promise<Sale> {
  const res = await request.post(`${apiUrl()}/sales`, {
    headers: apiHeaders(),
    data: {
      lines: [{ product_id: productId, qty_milli: qtyMilli }],
      payment_mode: "credit",
      customer_id: customerId,
      kind: "facture",
    },
  });
  expect(res.status()).toBe(201);
  return res.json();
}

async function balance(request: APIRequestContext, customerId: number): Promise<number> {
  const res = await request.get(`${apiUrl()}/customers/${customerId}`, { headers: apiHeaders() });
  expect(res.ok()).toBe(true);
  const fiche: { balance_centimes: number } = await res.json();
  return fiche.balance_centimes;
}

async function read(request: APIRequestContext, id: number): Promise<Sale> {
  const res = await request.get(`${apiUrl()}/sales/${id}`, { headers: apiHeaders() });
  expect(res.ok()).toBe(true);
  return res.json();
}

async function onHand(request: APIRequestContext, productId: number): Promise<number> {
  const res = await request.get(`${apiUrl()}/products/${productId}`, { headers: apiHeaders() });
  expect(res.ok()).toBe(true);
  const product: { qty_on_hand_milli: number } = await res.json();
  return product.qty_on_hand_milli;
}

// "screenshot" in the title on purpose: `just screenshot` greps for it, and
// documents-avoir-ar.png is committed, so the file has to be regenerable by
// that recipe.
test("credits a facture in part, cancels another whole, leaves a credit, quotes a proforma, and saves the screenshot in Arabic", async ({
  page,
  request,
}) => {
  await seedStoreBlock(request);
  const product = await seedProduct(request);
  const buyer = await seedCustomer(request, BUYER);
  const paidBuyer = await seedCustomer(request, PAID_BUYER);

  // ---- a credit facture of 3 000,00, a payment of 1 000,00, a partial
  // avoir of one line worth 500,00.
  const facture = await sellOnCredit(request, product, buyer, 3_000);
  expect(facture.totals.net_to_pay_centimes).toBe(300_000);
  const paid = await request.post(`${apiUrl()}/customers/${buyer}/payments`, {
    headers: apiHeaders(),
    data: { amount_centimes: 100_000, payment_mode: "cash", note: null },
  });
  expect(paid.status()).toBe(201);
  expect(await balance(request, buyer)).toBe(200_000);

  const avoirRes = await request.post(`${apiUrl()}/sales/${facture.id}/avoir`, {
    headers: apiHeaders(),
    data: {
      lines: [{ document_line_id: facture.lines[0].id, qty_milli: 500 }],
      reason: "retour marchandise",
    },
  });
  expect(avoirRes.status()).toBe(201);
  const avoir: Sale = await avoirRes.json();
  expect(avoir.kind).toBe("avoir");
  expect(avoir.series).toBe("doc_avoir");
  expect(avoir.ref_document_id).toBe(facture.id);
  expect(avoir.totals.net_to_pay_centimes).toBe(50_000);

  // The unpaid part of the facture came down by it, and so did the balance:
  // 2 000,00 less 500,00.
  expect((await read(request, facture.id)).balance?.remaining_debt_centimes).toBe(150_000);
  expect(await balance(request, buyer)).toBe(150_000);

  // ---- a second credit facture of 2 000,00 with no payment, cancelled
  // whole: annulée, an avoir of 2 000,00, and the balance back where it was
  // before that facture existed.
  const beforeSecond = await balance(request, buyer);
  const second = await sellOnCredit(request, product, buyer, 2_000);
  expect(second.totals.net_to_pay_centimes).toBe(200_000);
  const cancelRes = await request.post(`${apiUrl()}/sales/${second.id}/cancel`, {
    headers: apiHeaders(),
    data: { reason: "commande annulée" },
  });
  expect(cancelRes.status()).toBe(200);
  const cancelled: Sale = await cancelRes.json();
  expect(cancelled.status).toBe("cancelled");
  expect(cancelled.number).toBe(second.number);
  const cancelAvoirId = cancelled.cancellation?.avoir_document_id ?? null;
  expect(cancelAvoirId).not.toBeNull();
  const cancelAvoir = await read(request, cancelAvoirId as number);
  expect(cancelAvoir.totals.net_to_pay_centimes).toBe(200_000);
  expect(await balance(request, buyer)).toBe(beforeSecond);

  // ---- an avoir on a fully paid facture of 1 000,00, on a fiche of its own,
  // leaves that customer holding a credit of 1 000,00.
  const settled = await sellOnCredit(request, product, paidBuyer, 1_000);
  const settling = await request.post(`${apiUrl()}/customers/${paidBuyer}/payments`, {
    headers: apiHeaders(),
    data: { amount_centimes: 100_000, payment_mode: "cash", note: null },
  });
  expect(settling.status()).toBe(201);
  expect(await balance(request, paidBuyer)).toBe(0);
  const whole = await request.post(`${apiUrl()}/sales/${settled.id}/avoir`, {
    headers: apiHeaders(),
    data: { lines: null, reason: "retour complet" },
  });
  expect(whole.status()).toBe(201);
  // Nothing was owed, so the whole of it is credit the shop is holding: the
  // only way a balance goes below zero.
  expect(await balance(request, paidBuyer)).toBe(-100_000);

  // ---- a proforma with a customer moves no stock.
  const stockBefore = await onHand(request, product);
  const quote = await request.post(`${apiUrl()}/sales`, {
    headers: apiHeaders(),
    data: {
      lines: [{ product_id: product, qty_milli: 4_000 }],
      payment_mode: "credit",
      customer_id: buyer,
      kind: "proforma",
    },
  });
  expect(quote.status()).toBe(201);
  const proforma: Sale = await quote.json();
  expect(proforma.kind).toBe("proforma");
  expect(proforma.series).toBe("doc_proforma");
  expect(await onHand(request, product)).toBe(stockBefore);
  expect(await balance(request, buyer)).toBe(beforeSecond);

  // ---- and the documents screen finds every one of them.
  await page.goto("/documents");
  await expect(page.getByRole("heading", { name: t("documents_title") })).toBeVisible();
  await expect(page.getByRole("button", { name: facture.printed_number })).toBeVisible();
  await expect(page.getByRole("button", { name: avoir.printed_number })).toBeVisible();
  await expect(page.getByRole("button", { name: proforma.printed_number })).toBeVisible();

  // The cancelled facture is on the list and says so, number kept.
  const cancelledRow = page
    .getByRole("row")
    .filter({ has: page.getByRole("button", { name: cancelled.printed_number }) });
  await expect(cancelledRow.getByText(t("documents_cancelled"))).toBeVisible();

  // A row opens the document, with its lines and the sheet the core
  // rendered beside them.
  await page.getByRole("button", { name: facture.printed_number }).click();
  const detail = page.getByRole("region", { name: t("documents_detail") });
  await expect(detail.getByText(CEMENT)).toBeVisible();
  await expect(detail.getByText(avoir.printed_number)).toBeVisible();
  await expect(page.getByTestId("documents-sheet")).toBeVisible();

  if (currentLang() === "ar") {
    // The one committed picture of this screen: the document list and an
    // open facture with its credit note under it, in the mirrored layout,
    // with the figures still left to right.
    await page.screenshot({
      path: path.join(here, "screenshots", "documents-avoir-ar.png"),
      fullPage: true,
    });
  }
});
