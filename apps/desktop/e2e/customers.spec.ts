// The customers screen driven through a real browser against the real axum
// API and a real SQLite file. Expected strings come from the JSON dictionary
// of the Playwright project running the test (fr, en or ar), so a reworded
// message fails here instead of silently passing a hardcoded sentence.
//
// What the run proves end to end: a company fiche opened with an opening
// debt writes one ledger movement, an adjustment written on the customer's
// account page lands in the stored ledger, and the balance the list shows
// afterwards is the one `GET /customers/{id}` answers.
//
// The screen is two pages since the kit landed, and the run walks both: the
// list, the fiche panel that opens over it, then the account page a row's
// name links to, then back to the list.

import { expect, test } from "./auth";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { apiHeaders, apiUrl } from "./api";
import { currentLang, t } from "./messages";

// `new URL(".", ...)` is already this file's directory (apps/desktop/e2e).
const here = fileURLToPath(new URL(".", import.meta.url));

/** One name per project, so the three runs share a database file without
 * reading each other's rows. */
function customerName(): string {
  return `Entreprise Benali ${currentLang()}`;
}

/** 1 500,00 DA carried over, corrected by 500,00 DA down to 1 000,00. */
const OPENING_INPUT = "1500";
const OPENING_CENTIMES = 150_000;
const ADJUST_INPUT = "-500";
const ADJUST_CENTIMES = -50_000;
const BALANCE_RENDERED = "1 000,00";

interface Customer {
  id: number;
  name: string;
  balance_centimes: number;
  party_kind: string;
}

async function customerByName(
  request: import("@playwright/test").APIRequestContext,
  name: string,
): Promise<Customer> {
  const res = await request.get(`${apiUrl()}/customers`, { headers: apiHeaders() });
  expect(res.ok()).toBe(true);
  const all: Customer[] = await res.json();
  const found = all.find((c) => c.name === name);
  if (found === undefined) throw new Error(`the API does not know ${name}`);
  return found;
}

test("opens a company fiche with an opening debt, adjusts it, and saves the customers screenshot", async ({
  page,
  request,
}) => {
  const name = customerName();

  await page.goto("/customers");
  await expect(page.getByRole("main").getByRole("heading", { name: t("customers_title") })).toBeVisible();

  await page.getByRole("button", { name: t("customers_add") }).click();
  // The fiche is a panel now, and the kit has no radio group: the party kind
  // is a row of buttons wearing `role="radio"`, which is clicked rather than
  // checked.
  await expect(page.getByTestId("customer-fiche")).toBeVisible();
  await page.getByLabel(t("field_name"), { exact: true }).fill(name);
  await page.getByRole("radio", { name: t("party_company"), exact: true }).click();
  await page.getByLabel(t("field_phone"), { exact: true }).fill("0770 11 22 33");
  await page.getByLabel(t("field_credit_limit"), { exact: true }).fill("2000");
  await page.getByLabel(t("field_warn_threshold"), { exact: true }).fill("1000");
  await page.getByLabel(t("field_opening_debt"), { exact: true }).fill(OPENING_INPUT);
  await page.getByRole("button", { name: t("action_save") }).click();

  const row = page.getByRole("row").filter({ hasText: name });
  await expect(row).toBeVisible();
  // 1 500,00 owed against a 1 000,00 warning threshold and a 2 000,00 limit.
  await expect(row.getByText(t("status_near_limit"))).toBeVisible();

  // The opening debt is one movement in the stored ledger, read through the
  // API rather than off the screen that wrote it.
  const created = await customerByName(request, name);
  expect(created.party_kind).toBe("company");
  expect(created.balance_centimes).toBe(OPENING_CENTIMES);
  const opening = await request.get(`${apiUrl()}/customers/${created.id}/ledger`, {
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

  // The row's name is the way in to the customer's own page, where the
  // movements are.
  await row.getByRole("link", { name }).click();
  await expect(page.getByRole("heading", { name: t("customers_ledger") })).toBeVisible();
  await page.getByLabel(t("field_adjust_amount"), { exact: true }).fill(ADJUST_INPUT);
  await page.getByLabel(t("field_adjust_note"), { exact: true }).fill("erreur de saisie");
  await page.getByRole("button", { name: t("action_adjust"), exact: true }).click();
  // The adjustment asks first in our own dialog, not the browser's box.
  await page.getByTestId("customer-adjust-dialog-confirm").click();

  // The new movement and the balance it left behind, on the screen.
  await expect(page.getByText(t("customers_adjusted"))).toBeVisible();
  const adjusted = page.getByRole("row").filter({ hasText: t("debt_adjustment") });
  await expect(adjusted.getByRole("cell", { name: BALANCE_RENDERED, exact: true })).toBeVisible();

  // The account page in Arabic is where the mirrored ledger and the
  // left-to-right amount cells are worth looking at.
  if (currentLang() === "ar") {
    await page.screenshot({
      path: path.join(here, "screenshots", "customer-account-ar.png"),
      fullPage: true,
    });
  }

  // And back on the list, the same figure on the row.
  await page.getByRole("link", { name: t("action_back_to_customers") }).click();
  const listed = page.getByRole("row").filter({ hasText: name });
  await expect(listed.getByRole("cell", { name: BALANCE_RENDERED, exact: true })).toBeVisible();

  // And in the shop file: the balance the screen shows is the one the API
  // answers, not a figure the browser worked out.
  const after = await request.get(`${apiUrl()}/customers/${created.id}`, {
    headers: apiHeaders(),
  });
  const stored: Customer = await after.json();
  expect(stored.balance_centimes).toBe(OPENING_CENTIMES + ADJUST_CENTIMES);

  const rows = await request.get(`${apiUrl()}/customers/${created.id}/ledger`, {
    headers: apiHeaders(),
  });
  const storedLedger: {
    balance_centimes: number;
    entries: {
      kind: string;
      credit_centimes: number;
      balance_after_centimes: number;
      note: string | null;
    }[];
  } = await rows.json();
  expect(storedLedger.balance_centimes).toBe(OPENING_CENTIMES + ADJUST_CENTIMES);
  expect(storedLedger.entries).toHaveLength(2);
  // Newest first, each row carrying what was owed once it had landed. The
  // column is the core's running balance, so this is where the two movements
  // are checked against the figures the screen printed above.
  expect(storedLedger.entries[0]).toMatchObject({
    kind: "adjustment",
    credit_centimes: -ADJUST_CENTIMES,
    balance_after_centimes: OPENING_CENTIMES + ADJUST_CENTIMES,
    note: "erreur de saisie",
  });
  expect(storedLedger.entries[1]).toMatchObject({
    kind: "opening",
    balance_after_centimes: OPENING_CENTIMES,
  });

  // The list itself, in the two languages the committed shots cover.
  if (currentLang() === "fr" || currentLang() === "ar") {
    const shot = currentLang() === "ar" ? "customers-ar.png" : "customers.png";
    await page.screenshot({ path: path.join(here, "screenshots", shot), fullPage: true });
  }
});

test("searches the list by a piece of the name", async ({ page, request }) => {
  const name = customerName();
  const other = `Zoubir Amrani ${currentLang()}`;
  const res = await request.post(`${apiUrl()}/customers`, {
    headers: apiHeaders(),
    data: {
      name: other,
      party_kind: "consumer",
      phone: null,
      address: null,
      rc: null,
      nif: null,
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

  await page.goto("/customers");
  await expect(page.getByRole("row").filter({ hasText: other })).toBeVisible();
  await page.getByLabel(t("customers_search"), { exact: true }).fill("zoubir");
  await expect(page.getByRole("row").filter({ hasText: other })).toBeVisible();
  await expect(page.getByRole("row").filter({ hasText: name })).toBeHidden();
});
