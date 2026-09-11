// The suppliers screen driven through a real browser against the real axum
// API and a real SQLite file. Expected strings come from the JSON dictionary
// of the Playwright project running the test (fr, en or ar), so a reworded
// message fails here instead of silently passing a hardcoded sentence.
//
// What the run proves end to end: a fiche opened with an opening debt writes
// one ledger movement, a payment taken on the statement settles part of it,
// and the balance the two pages show is the one `GET /suppliers/{id}`
// answers.
//
// The fiche is a panel and the payment is a dialog, and Radix puts
// `aria-hidden` on the page behind either one. A role query then finds the
// overlay and nothing else, so every assertion about the list or the ledger
// here is made once the overlay that wrote it has gone.

import { expect, test } from "./auth";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { apiHeaders, apiUrl } from "./api";
import { currentLang, t } from "./messages";

// `new URL(".", ...)` is already this file's directory (apps/desktop/e2e).
const here = fileURLToPath(new URL(".", import.meta.url));

/** One name per project, so the three runs share a database file without
 * tripping over the unique name a supplier fiche carries. */
function supplierName(): string {
  return `Sarl Amrani ${currentLang()}`;
}

/** 1 500,00 DA carried over, 500,00 of it paid, 1 000,00 left. */
const OPENING_INPUT = "1500";
const OPENING_CENTIMES = 150_000;
const PAYMENT_INPUT = "500";
const PAYMENT_CENTIMES = 50_000;
const BALANCE_RENDERED = "1 000,00";

interface Supplier {
  id: number;
  name: string;
  balance_centimes: number;
  active: boolean;
}

async function supplierByName(
  request: import("@playwright/test").APIRequestContext,
  name: string,
): Promise<Supplier> {
  const res = await request.get(`${apiUrl()}/suppliers`, { headers: apiHeaders() });
  expect(res.ok()).toBe(true);
  const all: Supplier[] = await res.json();
  const found = all.find((s) => s.name === name);
  if (found === undefined) throw new Error(`the API does not know ${name}`);
  return found;
}

test("opens a supplier with an opening debt, pays part of it, and saves the suppliers screenshot", async ({
  page,
  request,
}) => {
  const name = supplierName();

  await page.goto("/suppliers");
  await expect(page.getByRole("main").getByRole("heading", { name: t("suppliers_title") })).toBeVisible();

  // From the page header: an empty list offers the same button in its middle,
  // and which of the two is clicked depends on whether an earlier project
  // left a supplier behind.
  await page.getByTestId("page-header").getByRole("button", { name: t("suppliers_add") }).click();
  await page.getByLabel(t("field_name"), { exact: true }).fill(name);
  await page.getByLabel(t("field_phone"), { exact: true }).fill("0770 11 22 33");
  await page.getByLabel(t("field_rc"), { exact: true }).fill("16/00-7654321 B 22");
  await page.getByLabel(t("field_supplier_opening_debt"), { exact: true }).fill(OPENING_INPUT);
  await page.getByRole("button", { name: t("action_save") }).click();

  // The panel closes itself once the fiche is written, which is what puts the
  // list back within reach of a role query.
  await expect(page.getByTestId("supplier-sheet")).toBeHidden();
  const row = page.getByRole("row").filter({ hasText: name });
  await expect(row).toBeVisible();

  // The opening debt is one movement in the stored ledger, read through the
  // API rather than off the screen that wrote it.
  const created = await supplierByName(request, name);
  expect(created.balance_centimes).toBe(OPENING_CENTIMES);
  const opening = await request.get(`${apiUrl()}/suppliers/${created.id}/ledger`, {
    headers: apiHeaders(),
  });
  const openingLedger: {
    balance_centimes: number;
    entries: { kind: string; debit_centimes: number; balance_after_centimes: number }[];
  } = await opening.json();
  expect(openingLedger.balance_centimes).toBe(OPENING_CENTIMES);
  expect(openingLedger.entries).toHaveLength(1);
  expect(openingLedger.entries[0]).toMatchObject({
    kind: "opening",
    debit_centimes: OPENING_CENTIMES,
    balance_after_centimes: OPENING_CENTIMES,
  });

  // The name is the way from the list to the statement.
  await row.getByRole("link", { name }).click();
  await expect(page.getByRole("heading", { name: t("suppliers_ledger") })).toBeVisible();

  await page.getByTestId("supplier-pay-open").click();
  await page.getByLabel(t("field_payment_amount"), { exact: true }).fill(PAYMENT_INPUT);
  await page.getByLabel(t("field_payment_note"), { exact: true }).fill("acompte");
  await page.getByRole("button", { name: t("action_pay_supplier"), exact: true }).click();

  // The new movement and the balance it left behind, on the statement.
  await expect(page.getByText(t("suppliers_paid"))).toBeVisible();
  const paid = page.getByRole("row").filter({ hasText: t("debt_payment") });
  await expect(paid.getByRole("cell", { name: BALANCE_RENDERED, exact: true })).toBeVisible();

  // The one committed screenshot of this screen is Arabic: it is where the
  // mirrored tables and the left-to-right amount cells are worth looking at.
  if (currentLang() === "ar") {
    await page.screenshot({
      path: path.join(here, "screenshots", "supplier-statement-ar.png"),
      fullPage: true,
    });
  }

  // And on the list, which reads the same balance from the same query.
  await page.getByRole("link", { name: t("action_back_to_suppliers") }).click();
  const back = page.getByRole("row").filter({ hasText: name });
  await expect(back.getByRole("cell", { name: BALANCE_RENDERED, exact: true })).toBeVisible();

  if (currentLang() === "ar") {
    await page.screenshot({
      path: path.join(here, "screenshots", "suppliers-ar.png"),
      fullPage: true,
    });
  }

  // And in the shop file: the balance the screens show is the one the API
  // answers, not a figure the browser worked out.
  const after = await request.get(`${apiUrl()}/suppliers/${created.id}`, {
    headers: apiHeaders(),
  });
  const stored: Supplier = await after.json();
  expect(stored.balance_centimes).toBe(OPENING_CENTIMES - PAYMENT_CENTIMES);

  const rows = await request.get(`${apiUrl()}/suppliers/${created.id}/ledger`, {
    headers: apiHeaders(),
  });
  const storedLedger: {
    balance_centimes: number;
    entries: {
      kind: string;
      credit_centimes: number;
      payment_mode: string | null;
      balance_after_centimes: number;
      note: string | null;
      allocations: unknown[];
    }[];
  } = await rows.json();
  expect(storedLedger.balance_centimes).toBe(OPENING_CENTIMES - PAYMENT_CENTIMES);
  expect(storedLedger.entries).toHaveLength(2);
  // Newest first, each row carrying what was owed once it had landed. The
  // column is the core's running balance, so this is where the two movements
  // are checked against the figures the screen printed above.
  expect(storedLedger.entries[0]).toMatchObject({
    kind: "payment",
    credit_centimes: PAYMENT_CENTIMES,
    payment_mode: "cash",
    balance_after_centimes: OPENING_CENTIMES - PAYMENT_CENTIMES,
    note: "acompte",
  });
  // An opening balance carries no order, so the money settled no paper: what
  // a payment settles is a purchase, and the purchases screen writes one.
  expect(storedLedger.entries[0].allocations).toEqual([]);
  expect(storedLedger.entries[1]).toMatchObject({
    kind: "opening",
    balance_after_centimes: OPENING_CENTIMES,
  });
});

test("closes a fiche that still owes, with the reason the server asks for", async ({
  page,
  request,
}) => {
  const name = `Bensalem ${currentLang()}`;
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
      opening_debt_centimes: 100_000,
    },
  });
  expect(res.status()).toBe(201);

  await page.goto("/suppliers");
  const row = page.getByRole("row").filter({ hasText: name });
  await row.getByRole("button", { name: `${t("suppliers_edit")} ${name}` }).click();

  // The balance says the account is open, so the box is there before the
  // server has to ask for it.
  await page.getByTestId("supplier-close-reason").fill("le fournisseur a fermé");
  await page.getByRole("button", { name: t("action_close_supplier"), exact: true }).click();

  await expect(page.getByTestId("supplier-sheet")).toBeHidden();
  await expect(row.getByText(t("suppliers_inactive"))).toBeVisible();
  const closed = await supplierByName(request, name);
  expect(closed.active).toBe(false);
  // Closing settles nothing: the shop still owes what it owed.
  expect(closed.balance_centimes).toBe(100_000);
});

test("the panel gives the list back when it is dismissed", async ({ page, request }) => {
  // Its own fiche rather than one an earlier test left: what is being checked
  // is the overlay, and a test that depended on the order of the file would
  // fail for a reason that has nothing to do with it.
  const name = `Meziane ${currentLang()}`;
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

  await page.goto("/suppliers");
  const row = page.getByRole("row").filter({ hasText: name });
  await row.getByRole("button", { name: `${t("suppliers_edit")} ${name}` }).click();
  await expect(page.getByTestId("supplier-sheet")).toBeVisible();

  await page.keyboard.press("Escape");
  await expect(page.getByTestId("supplier-sheet")).toBeHidden();
  await expect(row).toBeVisible();
});
