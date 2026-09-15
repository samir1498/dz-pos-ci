// A sale on credit end to end: a real browser, the real axum API, the real
// SQLite file. A customer with a 5 000,00 limit and a 4 000,00 warning is
// opened through the API, the cashier picks them at the till and sells
// 4 500,00 on credit, and three things are then true at once: the till says
// the customer has reached their threshold, the document carries the balance
// triple, and the customer's ledger holds one debt row naming that document.
//
// The second basket is what the limit refuses. The screen shows the two
// amounts the server sent, the cashier overrides, and the ledger ends on
// 5 500,00. A card sale to the same customer closes the file: the buyer is
// named on the document and no debt row is written, because nothing is owed.

import { expect, test } from "./auth";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { APIRequestContext, Page } from "@playwright/test";
import { formatCentimes } from "@dzpos/shared";
import { apiHeaders, apiUrl } from "./api";
import { currentLang, t } from "./messages";

const here = fileURLToPath(new URL(".", import.meta.url));

const CUSTOMER = "Entreprise Amrani e2e";
/** 5 000,00 of credit, warned at 4 000,00. */
const LIMIT = 500_000;
const WARN = 400_000;

/** Two products priced so the arithmetic is the customer's balance and
 * nothing else: both are exempt of TVA, and a credit sale carries no droit
 * de timbre (it is a cash tax), so the net to pay is the price. */
const BIG = "Ciment e2e";
const BIG_BARCODE = "6130009000035";
const BIG_PRICE = 450_000;
const SMALL = "Sable e2e";
const SMALL_BARCODE = "6130009000042";
const SMALL_PRICE = 100_000;

interface LedgerEntry {
  kind: string;
  debit_centimes: number;
  credit_centimes: number;
  document_id: number | null;
}

async function seedProduct(
  request: APIRequestContext,
  input: { name: string; barcode: string; price: number },
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
      phone: "0555 33 22 11",
      address: null,
      rc: null,
      nif: null,
      nis: null,
      ai: null,
      credit_limit_centimes: LIMIT,
      warn_threshold_centimes: WARN,
      notes: null,
      active: true,
      opening_debt_centimes: null,
    },
  });
  expect(res.status()).toBe(201);
  const created: { id: number } = await res.json();
  return created.id;
}

async function ledger(
  request: APIRequestContext,
  id: number,
): Promise<{ balance_centimes: number; entries: LedgerEntry[] }> {
  const res = await request.get(`${apiUrl()}/customers/${id}/ledger`, { headers: apiHeaders() });
  expect(res.ok()).toBe(true);
  return res.json();
}

/** Picks the customer at the till: the search box narrows the list the
 * server answers with, and the answer itself is what commits the choice. */
async function pick(page: Page, name: string): Promise<void> {
  await page.getByLabel(t("till_customer_search"), { exact: true }).fill(name);
  const list = page.getByRole("group", { name: t("till_customer") });
  await list.getByRole("button", { name }).click();
}

async function addAndPayOnCredit(page: Page, product: string): Promise<void> {
  const search = page.getByLabel(t("till_search"), { exact: true });
  await search.fill(product);
  await page.getByTestId("tiles").getByRole("button", { name: product }).click();
  await page.getByRole("radio", { name: t("pay_credit"), exact: true }).click();
  await page.getByRole("button", { name: t("action_pay"), exact: true }).click();
}

// "screenshot" in the title on purpose: `just screenshot` greps for it, and
// till-credit-ar.png is committed, so the file has to be regenerable by that
// recipe.
test("sells on credit, warns at the threshold, is refused past the limit, overrides, and saves the screenshot", async ({
  page,
  request,
}) => {
  await seedProduct(request, { name: BIG, barcode: BIG_BARCODE, price: BIG_PRICE });
  await seedProduct(request, { name: SMALL, barcode: SMALL_BARCODE, price: SMALL_PRICE });
  const customerId = await seedCustomer(request);

  await page.goto("/");
  await expect(page).toHaveURL(/\/till$/);

  // Nobody picked, so credit is not on offer at all.
  const credit = page.getByRole("radio", { name: t("pay_credit"), exact: true });
  await expect(credit).toBeDisabled();
  await pick(page, CUSTOMER);
  await expect(credit).toBeEnabled();

  // 4 500,00 on credit: through the limit's warning threshold, under the
  // limit itself.
  const first = page.waitForResponse(
    (res) => res.url().endsWith("/sales") && res.request().method() === "POST",
  );
  await addAndPayOnCredit(page, BIG);
  const firstResponse = await first;
  expect(firstResponse.status()).toBe(201);
  const firstSale: {
    id: number;
    warning: string | null;
    balance: {
      old_balance_centimes: number;
      remaining_debt_centimes: number;
      total_debt_centimes: number;
    } | null;
  } = await firstResponse.json();
  expect(firstSale.warning).toBe("near_limit");
  expect(firstSale.balance).toEqual({
    old_balance_centimes: 0,
    remaining_debt_centimes: BIG_PRICE,
    total_debt_centimes: BIG_PRICE,
  });

  // The screen says both halves: what the customer now owes, and that the
  // threshold was reached.
  const done = page.getByRole("status");
  await expect(done.getByTestId("till-near-limit")).toBeVisible();
  await expect(done.getByTestId("till-new-balance")).toHaveText(formatCentimes(BIG_PRICE));

  // One movement on the ledger, naming the document it came from.
  const afterFirst = await ledger(request, customerId);
  expect(afterFirst.balance_centimes).toBe(BIG_PRICE);
  expect(afterFirst.entries).toHaveLength(1);
  expect(afterFirst.entries[0]).toMatchObject({
    kind: "sale",
    debit_centimes: BIG_PRICE,
    credit_centimes: 0,
    document_id: firstSale.id,
  });

  // 1 000,00 more would leave 5 500,00 owed against a 5 000,00 limit.
  await pick(page, CUSTOMER);
  const refused = page.waitForResponse(
    (res) => res.url().endsWith("/sales") && res.request().method() === "POST",
  );
  await addAndPayOnCredit(page, SMALL);
  expect((await refused).status()).toBe(422);

  const over = BIG_PRICE + SMALL_PRICE;
  await expect(page.getByTestId("till-balance-after")).toHaveText(formatCentimes(over));
  await expect(page.getByTestId("till-credit-limit")).toHaveText(formatCentimes(LIMIT));
  // Nothing was written: the refusal cost the customer nothing.
  expect((await ledger(request, customerId)).balance_centimes).toBe(BIG_PRICE);

  if (currentLang() === "fr" || currentLang() === "ar") {
    // The refusal with its two amounts, in the two languages the committed
    // shots cover: fr is the reference shot, ar is the mirrored layout,
    // with the figures still left to right.
    const shot = currentLang() === "ar" ? "till-credit-ar.png" : "till-credit.png";
    await page.screenshot({
      path: path.join(here, "screenshots", shot),
      fullPage: true,
    });
  }

  // The owner takes the decision. The override asks first in our own
  // dialog, not the browser's box.
  const overridden = page.waitForResponse(
    (res) => res.url().endsWith("/sales") && res.request().method() === "POST",
  );
  await page.getByRole("button", { name: t("till_override"), exact: true }).click();
  await page.getByTestId("till-override-dialog-confirm").click();
  expect((await overridden).status()).toBe(201);

  const afterOverride = await ledger(request, customerId);
  expect(afterOverride.balance_centimes).toBe(over);
  expect(afterOverride.entries).toHaveLength(2);

  // A card sale to the same customer names the buyer and owes nothing, so
  // the ledger does not move.
  await pick(page, CUSTOMER);
  const paid = page.waitForResponse(
    (res) => res.url().endsWith("/sales") && res.request().method() === "POST",
  );
  const search = page.getByLabel(t("till_search"), { exact: true });
  await search.fill(SMALL);
  await page.getByTestId("tiles").getByRole("button", { name: SMALL }).click();
  await page.getByRole("radio", { name: t("pay_card"), exact: true }).click();
  await page.getByRole("button", { name: t("action_pay"), exact: true }).click();
  const card: { customer_id: number | null; balance: { remaining_debt_centimes: number } | null } =
    await (await paid).json();
  expect(card.customer_id).toBe(customerId);
  expect(card.balance?.remaining_debt_centimes).toBe(0);

  const afterCard = await ledger(request, customerId);
  expect(afterCard.balance_centimes).toBe(over);
  expect(afterCard.entries).toHaveLength(2);
});
