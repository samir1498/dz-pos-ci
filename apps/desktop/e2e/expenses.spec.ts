// The expenses screen driven through a real browser against the real axum
// API and a real SQLite file. Expected strings come from the JSON dictionary
// of the Playwright project running the test (fr, en or ar), so a reworded
// message fails here instead of silently passing a hardcoded sentence.
//
// What the run proves end to end: two expenses filed in two categories reach
// the database as centimes under the right category, the month's total the
// screen shows is the one `GET /expenses?month=` answers, and the cash
// position box is the figure `GET /cash?month=` computes from the ledgers
// rather than a sum the screen made.

import { expect, test } from "./auth";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { apiHeaders, apiUrl } from "./api";
import { currentLang, t } from "./messages";

// `new URL(".", ...)` is already this file's directory (apps/desktop/e2e).
const here = fileURLToPath(new URL(".", import.meta.url));

/** 3 000,00 DA of rent and 1 500,00 DA of electricity. */
const RENT_INPUT = "3000";
const RENT_CENTIMES = 300_000;
const ELECTRICITY_INPUT = "1500";
const ELECTRICITY_CENTIMES = 150_000;
const TOTAL_CENTIMES = RENT_CENTIMES + ELECTRICITY_CENTIMES;
const TOTAL_RENDERED = "4 500,00";

interface Expenses {
  month: string;
  total_centimes: number;
  expenses: { category_id: number; amount_centimes: number; expense_date: string; note: string | null }[];
}

interface Cash {
  from: string;
  to: string;
  cash_in: { total_centimes: number };
  cash_out: { refunds_centimes: number; expenses_centimes: number; total_centimes: number };
  cash_centimes: number;
}

/** The shop's day, asked of the same clock the core dates rows with. A
 * `new Date()` here would be a second clock, and on the machine's zone
 * rather than Algeria's. */
async function shopToday(
  request: import("@playwright/test").APIRequestContext,
): Promise<string> {
  const res = await request.get(`${apiUrl()}/clock`, { headers: apiHeaders() });
  expect(res.ok()).toBe(true);
  const body: { today: string } = await res.json();
  return body.today;
}

async function categoryId(
  request: import("@playwright/test").APIRequestContext,
  key: string,
): Promise<number> {
  const res = await request.get(`${apiUrl()}/expense-categories`, { headers: apiHeaders() });
  expect(res.ok()).toBe(true);
  const all: { id: number; key: string }[] = await res.json();
  const found = all.find((c) => c.key === key);
  if (found === undefined) throw new Error(`the API has no category ${key}`);
  return found.id;
}

/** The filed rows, without the heading row above them. `DataTable` draws a
 * `rowgroup` for the head and one for the body; the rows are the second
 * group's, and nothing here reads a class name. */
function expenseRows(page: import("@playwright/test").Page) {
  return page.getByTestId("expenses-table").getByRole("rowgroup").nth(1).getByRole("row");
}

async function fileOne(
  page: import("@playwright/test").Page,
  category: string,
  amount: string,
  note: string,
): Promise<void> {
  // By test id and not by name: the empty state offers a second button with
  // the same words, which is the right thing on the screen and an ambiguous
  // locator here.
  await page.getByTestId("expenses-add").click();
  // The category is a listbox now, not a native select: the trigger opens it
  // and the option is clicked by the word the running language shows.
  await page.getByTestId("expense-category").click();
  await page.getByRole("option", { name: t(category), exact: true }).click();
  await page.getByTestId("expense-amount").fill(amount);
  await page.getByTestId("expense-note").fill(note);
  await page.getByRole("button", { name: t("action_save"), exact: true }).click();
  // The panel closes itself once the row is filed.
  await expect(page.getByTestId("expense-form")).toBeHidden();
}

test("files two expenses in two categories and shows the month's total and the cash position, and saves the expenses screenshot in Arabic", async ({
  page,
  request,
}) => {
  const today = await shopToday(request);
  const month = today.slice(0, 7);
  const rent = await categoryId(request, "rent");
  const electricity = await categoryId(request, "electricity");

  await page.goto("/expenses");
  await expect(page.getByRole("main").getByRole("heading", { name: t("expenses_title") })).toBeVisible();
  // The month the screen opens on is the shop's, read from /clock. It is
  // two boxes now instead of one month input, so each segment is read on
  // its own.
  await expect(page.getByTestId("expenses-month-month")).toHaveValue(today.slice(5, 7));
  await expect(page.getByTestId("expenses-month-year")).toHaveValue(today.slice(0, 4));
  await expect(page.getByText(t("expenses_empty"))).toBeVisible();

  await fileOne(page, "expense_category_rent", RENT_INPUT, "loyer");
  await expect(expenseRows(page)).toHaveCount(1);
  await fileOne(page, "expense_category_electricity", ELECTRICITY_INPUT, "sonelgaz");
  await expect(expenseRows(page)).toHaveCount(2);
  // The category rides in the row as a chip, in the words of the running
  // language rather than as the key the row stores. Which row is which is
  // not asserted: both were filed on the same day and the order of two rows
  // of one day is the server's business.
  const table = page.getByTestId("expenses-table");
  await expect(table).toContainText(t("expense_category_rent"));
  await expect(table).toContainText(t("expense_category_electricity"));

  // What the screen prints.
  await expect(page.getByTestId("expenses-total")).toHaveText(TOTAL_RENDERED);
  await expect(page.getByTestId("cash-out-expenses")).toHaveText(TOTAL_RENDERED);
  await expect(page.getByTestId("cash-out-total")).toHaveText(TOTAL_RENDERED);
  await expect(page.getByTestId("cash-net")).toHaveText(`-${TOTAL_RENDERED}`);

  // And what the database holds, which is where the two amounts are checked
  // as centimes under the category each was filed in.
  const listed = await request.get(`${apiUrl()}/expenses?month=${month}`, {
    headers: apiHeaders(),
  });
  expect(listed.ok()).toBe(true);
  const stored: Expenses = await listed.json();
  expect(stored.month).toBe(month);
  expect(stored.total_centimes).toBe(TOTAL_CENTIMES);
  expect(stored.expenses).toHaveLength(2);
  const byCategory = new Map(stored.expenses.map((e) => [e.category_id, e]));
  expect(byCategory.get(rent)).toMatchObject({
    amount_centimes: RENT_CENTIMES,
    expense_date: today,
    note: "loyer",
  });
  expect(byCategory.get(electricity)).toMatchObject({
    amount_centimes: ELECTRICITY_CENTIMES,
    note: "sonelgaz",
  });

  // The cash position is the core's, summed from the ledgers. Nothing has
  // been sold in this run, so the month took nothing in and the net is the
  // expenses, below zero. A refund is nothing: an avoir credits a customer's
  // ledger and brings the goods back, it does not open the drawer.
  const cash = await request.get(`${apiUrl()}/cash?month=${month}`, { headers: apiHeaders() });
  expect(cash.ok()).toBe(true);
  const position: Cash = await cash.json();
  expect(position.from.slice(0, 7)).toBe(month);
  expect(position.cash_in.total_centimes).toBe(0);
  expect(position.cash_out.expenses_centimes).toBe(TOTAL_CENTIMES);
  expect(position.cash_out.refunds_centimes).toBe(0);
  expect(position.cash_centimes).toBe(-TOTAL_CENTIMES);

  // The one committed screenshot of this screen is Arabic: it is where the
  // mirrored figures panel and the left-to-right amount cells are worth
  // looking at.
  if (currentLang() === "ar") {
    await page.screenshot({
      path: path.join(here, "screenshots", "expenses-ar.png"),
      fullPage: true,
    });
  }
});

test("a month the shop spent nothing in shows no rows, a total of zero and a cash position of zero", async ({
  page,
  request,
}) => {
  const today = await shopToday(request);
  // A month far enough back that nothing this suite writes can land in it.
  const quiet = `${Number(today.slice(0, 4)) - 1}-01`;

  await page.goto("/expenses");
  await page.getByTestId("expenses-month").fill(quiet);
  await expect(page.getByText(t("expenses_empty"))).toBeVisible();
  await expect(page.getByTestId("expenses-total")).toHaveText("0,00");
  await expect(page.getByTestId("cash-net")).toHaveText("0,00");
});
