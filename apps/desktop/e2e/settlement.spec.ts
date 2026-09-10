// Settling a customer's debt, driven through a real browser against the real
// axum API and a real SQLite file. Expected strings come from the JSON
// dictionary of the Playwright project running the test (fr, en or ar), so a
// reworded message fails here instead of silently passing a hardcoded
// sentence.
//
// What the run proves end to end: two credit sales leave two documents each
// carrying its own unpaid part, a payment typed on the screen settles the
// older one in full and part of the newer, the allocations the API stored say
// exactly that, a payment above what is left is refused, and the two papers
// the core renders from that ledger, the A4 statement and the 80 mm debt
// slip, both close on the same figure.
//
// The sale is posted through the API rather than rung up at the till: this
// spec is about the settlement, and the till's own credit flow has its own.
//
// The file is named for the settlement and not for the payment so that it
// sorts after products.spec: the specs share one shop file, run one worker at
// a time in the order their names sort, and products.spec is the one that
// asserts the empty list. Anything that adds a product comes after it.

import { expect, test } from "@playwright/test";
import type { APIRequestContext } from "@playwright/test";
import { apiHeaders, apiUrl } from "./api";
import { currentLang, t } from "./messages";

/** One name per project, so the three runs share a database file without
 * reading each other's rows. */
function customerName(): string {
  return `Société Créance ${currentLang()}`;
}

/** Named after this spec and nothing else: the till's own specs pick their
 * tiles by a piece of the product name, and a name that shares a prefix with
 * one of theirs would make their click ambiguous. */
function productName(): string {
  return `Brique relevé ${currentLang()}`;
}

/** 1 000,00 then 2 000,00 sold on credit; 1 500,00 paid. The older document
 * is settled in full and 500,00 lands on the newer, leaving 1 500,00 owed. */
const FIRST_SALE_CENTIMES = 100_000;
const SECOND_SALE_CENTIMES = 200_000;
const PAYMENT_INPUT = "1500";
const PAYMENT_CENTIMES = 150_000;
const BALANCE_AFTER = FIRST_SALE_CENTIMES + SECOND_SALE_CENTIMES - PAYMENT_CENTIMES;
const BALANCE_RENDERED = "1 500,00";
/** More than the 1 500,00 still owed, so the API refuses it. */
const TOO_MUCH_INPUT = "2000";

interface Customer {
  id: number;
  name: string;
  balance_centimes: number;
}

interface Sale {
  id: number;
  balance: { remaining_debt_centimes: number } | null;
}

async function customerByName(request: APIRequestContext, name: string): Promise<Customer> {
  const res = await request.get(`${apiUrl()}/customers`, { headers: apiHeaders() });
  expect(res.ok()).toBe(true);
  const all: Customer[] = await res.json();
  const found = all.find((c) => c.name === name);
  if (found === undefined) throw new Error(`the API does not know ${name}`);
  return found;
}

/** The fiche the two sales are made out to, with room for both of them. */
async function aCustomerOnCredit(request: APIRequestContext, name: string): Promise<number> {
  const existing = await request.get(`${apiUrl()}/customers`, { headers: apiHeaders() });
  const all: Customer[] = await existing.json();
  const found = all.find((c) => c.name === name);
  if (found !== undefined) return found.id;
  const res = await request.post(`${apiUrl()}/customers`, {
    headers: apiHeaders(),
    data: {
      name,
      party_kind: "company",
      phone: "0770 44 55 66",
      address: "12 rue Didouche Mourad, Alger",
      rc: `16/00-7654321 B ${currentLang()}`,
      nif: "000916001234567",
      nis: null,
      ai: null,
      credit_limit_centimes: null,
      warn_threshold_centimes: null,
      notes: null,
      active: true,
      opening_debt_centimes: null,
    },
  });
  expect(res.status()).toBe(201);
  const made: Customer = await res.json();
  return made.id;
}

/** A product priced so one unit is exactly the sale being made. Sold at 0 %
 * so the document's net to pay is the price and the spec's figures are the
 * ones a reader can check by eye. */
async function aProductAt(request: APIRequestContext, centimes: number): Promise<number> {
  const res = await request.post(`${apiUrl()}/products`, {
    headers: apiHeaders(),
    data: {
      name: `${productName()} ${centimes}`,
      barcode: null,
      category_id: 1,
      unit: "piece",
      cost_centimes: 0,
      selling_centimes: centimes,
      wholesale_centimes: null,
      qty_on_hand_milli: 100_000,
      low_stock_at_milli: 0,
      rate_bps: 0,
      active: true,
    },
  });
  expect(res.status()).toBe(201);
  const made: { id: number } = await res.json();
  return made.id;
}

/** One sale on credit to the customer. `customer_id` and the credit mode are
 * the contract T3 writes; this spec depends on it and asserts nothing about
 * how the till gets there. */
async function aCreditSale(
  request: APIRequestContext,
  customerId: number,
  centimes: number,
): Promise<number> {
  const productId = await aProductAt(request, centimes);
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
      payment_mode: "credit",
      tendered_centimes: null,
      customer_id: customerId,
    },
  });
  expect(res.status()).toBe(201);
  const made: Sale = await res.json();
  return made.id;
}

async function remainingDebt(request: APIRequestContext, saleId: number): Promise<number> {
  const res = await request.get(`${apiUrl()}/sales/${saleId}`, { headers: apiHeaders() });
  expect(res.ok()).toBe(true);
  const sale: Sale = await res.json();
  if (sale.balance === null) throw new Error(`sale ${saleId} carries no balance triple`);
  return sale.balance.remaining_debt_centimes;
}

test("settles two credit sales oldest first, refuses more than is owed, and prints the statement", async ({
  page,
  request,
}) => {
  // Confirmed rather than dismissed: the payment asks first, and a browser
  // answers "no" to a dialog nobody handles.
  page.on("dialog", (dialog) => void dialog.accept());
  const name = customerName();
  const customerId = await aCustomerOnCredit(request, name);
  const first = await aCreditSale(request, customerId, FIRST_SALE_CENTIMES);
  const second = await aCreditSale(request, customerId, SECOND_SALE_CENTIMES);
  expect(await remainingDebt(request, first)).toBe(FIRST_SALE_CENTIMES);
  expect(await remainingDebt(request, second)).toBe(SECOND_SALE_CENTIMES);

  await page.goto("/customers");
  const row = page.getByRole("row").filter({ hasText: name });
  await expect(row).toBeVisible();
  await row.getByRole("button", { name: `${t("customers_edit")} ${name}` }).click();
  await expect(page.getByRole("heading", { name: t("customers_pay") })).toBeVisible();

  await page.getByLabel(t("field_payment_amount"), { exact: true }).fill(PAYMENT_INPUT);
  await page.getByRole("radio", { name: t("payment_cash"), exact: true }).check();
  await page.getByLabel(t("field_payment_note"), { exact: true }).fill("acompte e2e");
  await page.getByRole("button", { name: t("action_take_payment"), exact: true }).click();
  await expect(page.getByText(t("customers_paid"))).toBeVisible();

  // The screen and the shop file agree on the balance.
  await expect(row.getByRole("cell", { name: BALANCE_RENDERED, exact: true })).toBeVisible();
  const stored = await customerByName(request, name);
  expect(stored.balance_centimes).toBe(BALANCE_AFTER);

  // The money filled the older document before it touched the newer, and
  // each document's own unpaid part moved with it.
  const paid = await request.get(`${apiUrl()}/customers/${customerId}/payments`, {
    headers: apiHeaders(),
  });
  const payments: {
    balance_centimes: number;
    payments: {
      amount_centimes: number;
      payment_mode: string | null;
      note: string | null;
      allocations: { document_id: number; amount_centimes: number }[];
    }[];
  } = await paid.json();
  expect(payments.balance_centimes).toBe(BALANCE_AFTER);
  expect(payments.payments).toHaveLength(1);
  expect(payments.payments[0]).toMatchObject({
    amount_centimes: PAYMENT_CENTIMES,
    payment_mode: "cash",
    note: "acompte e2e",
  });
  expect(payments.payments[0].allocations).toEqual([
    { document_id: first, amount_centimes: FIRST_SALE_CENTIMES },
    { document_id: second, amount_centimes: PAYMENT_CENTIMES - FIRST_SALE_CENTIMES },
  ]);
  expect(await remainingDebt(request, first)).toBe(0);
  expect(await remainingDebt(request, second)).toBe(BALANCE_AFTER);

  // A payment above what is left is refused, and the refusal names what is
  // still owed rather than saying only "too much".
  await page.getByLabel(t("field_payment_amount"), { exact: true }).fill(TOO_MUCH_INPUT);
  await page.getByRole("button", { name: t("action_take_payment"), exact: true }).click();
  const refusal = page.getByRole("alert").filter({ hasText: t("error_payment_above_debt") });
  await expect(refusal).toBeVisible();
  await expect(refusal).toContainText(BALANCE_RENDERED);
  const afterRefusal = await customerByName(request, name);
  expect(afterRefusal.balance_centimes).toBe(BALANCE_AFTER);

  // The statement is the page the core rendered, and it closes on the same
  // figure.
  await page.getByRole("button", { name: t("action_statement"), exact: true }).click();
  const statement = page.frameLocator("[data-testid='customer-statement']");
  await expect(statement.locator(".amount-closing")).toHaveText(
    new RegExp(BALANCE_RENDERED.replace(/\s/g, "\\s")),
  );
  await expect(statement.locator(".in-words")).not.toBeEmpty();

  // The debt slip is the other paper the same ledger renders, and it closes
  // on the same figure with the three movements behind it: the two sales and
  // the payment. Its figure and the statement's are one balance read once.
  await page.getByRole("button", { name: t("action_debt_slip"), exact: true }).click();
  const slip = page.frameLocator("[data-testid='customer-debt-slip']");
  await expect(slip.locator(".amount-balance")).toHaveText(
    new RegExp(BALANCE_RENDERED.replace(/\s/g, "\\s")),
  );
  await expect(slip.locator(".amount-running")).toHaveCount(3);
  await expect(slip.locator(".in-words")).not.toBeEmpty();
});
