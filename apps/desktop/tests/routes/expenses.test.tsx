// The expenses screen is checked for the request body it sends and for the
// state it renders from the API's answer. The rules (an amount of nothing, a
// retired category, a month that is not a month) are the core's and the API
// crate's tests; what these hold is the wiring: the month comes from the shop
// clock, the amount is posted in centimes, the category is posted as an id,
// and the figures on the screen are the ones the server summed.

import { afterEach, beforeAll, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { CashPositionDto, ExpenseCategoryDto, ExpensesDto } from "@dzpos/shared";
import { I18nProvider, type Lang } from "@/i18n";
import fr from "@/i18n/fr.json";
import ar from "@/i18n/ar.json";
import { ExpensesScreen } from "../../src/routes/expenses";

// The entry sheet and the category picker are Radix overlays, and Radix asks
// the DOM for three things jsdom does not implement: pointer capture and
// `scrollIntoView` on the way to opening a select, and nothing else. They are
// stubbed here rather than in the shared setup because this is the one file
// in the suite that opens one; the browser has all three, and e2e is what
// proves the panel really opens.
beforeAll(() => {
  Element.prototype.scrollIntoView = () => {};
  Element.prototype.hasPointerCapture = () => false;
  Element.prototype.setPointerCapture = () => {};
  Element.prototype.releasePointerCapture = () => {};
});

/** The filed rows, without the heading row above them. `DataTable` draws one
 *  `rowgroup` for the head and one for the body, and the rows are the second
 *  group's; nothing here reads a class name. */
function expenseRows(): HTMLElement[] {
  const groups = within(screen.getByTestId("expenses-table")).getAllByRole("rowgroup");
  const body = groups[1];
  if (body === undefined) throw new Error("the table has no body");
  return within(body).getAllByRole("row");
}

/** Opens the entry sheet and waits for the amount field to be reachable. */
async function openSheet(user: ReturnType<typeof userEvent.setup>): Promise<void> {
  await user.click(await screen.findByTestId("expenses-add"));
  await screen.findByTestId("expense-amount");
}

/** What the server says the day is. Deliberately a day the machine is not
 *  on, so a screen that read `new Date()` would fail here. */
const SHOP_TODAY = "2027-03-04";
const SHOP_MONTH = "2027-03";

const categories: ExpenseCategoryDto[] = [
  { id: 1, key: "rent", sort_order: 1, active: true },
  { id: 2, key: "electricity", sort_order: 2, active: true },
  { id: 3, key: "water", sort_order: 3, active: false },
];

const month: ExpensesDto = {
  month: SHOP_MONTH,
  total_centimes: 3_150_000,
  expenses: [
    {
      id: 2,
      category_id: 2,
      amount_centimes: 150_000,
      expense_date: "2027-03-03",
      note: "sonelgaz",
    },
    {
      id: 1,
      category_id: 1,
      amount_centimes: 3_000_000,
      expense_date: "2027-03-01",
      note: null,
    },
  ],
};

const position: CashPositionDto = {
  from: "2027-03-01",
  to: "2027-03-31",
  cash_in: {
    sales_centimes: 802_000,
    stamp_centimes: 2_000,
    customer_payments_centimes: 30_000,
    total_centimes: 832_000,
  },
  cash_out: {
    refunds_centimes: 0,
    supplier_payments_centimes: 40_000,
    expenses_centimes: 3_150_000,
    total_centimes: 3_190_000,
  },
  cash_centimes: -2_358_000,
  card_in: {
    sales_centimes: 50_000,
    stamp_centimes: 0,
    customer_payments_centimes: 0,
    total_centimes: 50_000,
  },
};

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function isInit(value: unknown): value is RequestInit {
  return typeof value === "object" && value !== null;
}

/** The URL and JSON body of the last POST. */
function posted(): { url: string; body: Record<string, unknown> } {
  const calls = fetchMock.mock.calls.filter((call) => {
    const init: unknown = call[1];
    return isInit(init) && init.method === "POST";
  });
  const last = calls[calls.length - 1];
  if (last === undefined) throw new Error("no POST was made");
  const init: unknown = last[1];
  if (!isInit(init) || typeof init.body !== "string") {
    throw new Error("the POST had no JSON body");
  }
  return { url: String(last[0]), body: JSON.parse(init.body) };
}

/** Every URL asked for with GET, in order. */
function fetched(): string[] {
  return fetchMock.mock.calls
    .filter((call) => {
      const init: unknown = call[1];
      return !isInit(init) || init.method === undefined || init.method === "GET";
    })
    .map((call) => String(call[0]));
}

let fetchMock: ReturnType<typeof vi.fn>;
let listed: ExpensesDto;
let writeAnswer: (() => Response) | null;

beforeEach(() => {
  listed = month;
  writeAnswer = null;
  fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
    const url = String(input);
    if (init?.method === "POST") {
      if (writeAnswer !== null) return Promise.resolve(writeAnswer());
      return Promise.resolve(
        json(201, {
          id: 9,
          category_id: 1,
          amount_centimes: 1_000,
          expense_date: SHOP_TODAY,
          note: null,
        }),
      );
    }
    if (url.includes("/clock")) return Promise.resolve(json(200, { today: SHOP_TODAY }));
    if (url.includes("/expense-categories")) return Promise.resolve(json(200, categories));
    if (url.includes("/expenses")) return Promise.resolve(json(200, listed));
    if (url.includes("/cash")) return Promise.resolve(json(200, position));
    return Promise.resolve(json(404, { error: { code: "not_found", message: "no" } }));
  });
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

function mount(lang: Lang = "fr") {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <I18nProvider lang={lang}>
      <QueryClientProvider client={client}>
        <ExpensesScreen />
      </QueryClientProvider>
    </I18nProvider>,
  );
}

describe("the month the screen opens on", () => {
  test("is the shop's, read from the clock and not from the machine", async () => {
    mount();
    await screen.findByTestId("expenses-month");
    const [year, month] = SHOP_MONTH.split("-");
    expect(screen.getByTestId("expenses-month-month")).toHaveValue(month);
    expect(screen.getByTestId("expenses-month-year")).toHaveValue(year);
    await waitFor(() => {
      expect(fetched().some((u) => u.includes(`/expenses?month=${SHOP_MONTH}`))).toBe(true);
    });
    expect(fetched().some((u) => u.includes(`/cash?month=${SHOP_MONTH}`))).toBe(true);
  });

  test("another month is asked of the server rather than filtered on the screen", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByTestId("expenses-month");
    listed = { month: "2027-02", total_centimes: 0, expenses: [] };
    await user.clear(screen.getByTestId("expenses-month-month"));
    await user.type(screen.getByTestId("expenses-month-month"), "02");
    await user.clear(screen.getByTestId("expenses-month-year"));
    await user.type(screen.getByTestId("expenses-month-year"), "2027");
    await waitFor(() => {
      expect(fetched().some((u) => u.includes("/expenses?month=2027-02"))).toBe(true);
    });
    expect(await screen.findByText(fr.expenses_empty)).toBeInTheDocument();
  });
});

describe("what the month shows", () => {
  test("the rows, their category labels and the total the server summed", async () => {
    mount();
    expect(await screen.findByTestId("expenses-total")).toHaveTextContent("31 500,00");
    await screen.findByTestId("expenses-table");
    const rows = expenseRows();
    expect(rows).toHaveLength(2);
    // Newest first, and the label comes from the app's own words by the
    // key the row's category carries.
    expect(rows[0]).toHaveTextContent("2027-03-03");
    expect(rows[0]).toHaveTextContent(fr.expense_category_electricity);
    expect(rows[0]).toHaveTextContent("1 500,00");
    expect(rows[1]).toHaveTextContent(fr.expense_category_rent);
  });

  test("the cash position box is the server's figures, the net below zero included", async () => {
    mount();
    expect(await screen.findByTestId("cash-in-total")).toHaveTextContent("8 320,00");
    // The stamp shows on its own, inside the takings above it.
    expect(screen.getByTestId("cash-in-stamp")).toHaveTextContent("20,00");
    expect(screen.getByTestId("cash-out-expenses")).toHaveTextContent("31 500,00");
    expect(screen.getByTestId("cash-out-total")).toHaveTextContent("31 900,00");
    expect(screen.getByTestId("cash-net")).toHaveTextContent("-23 580,00");
    expect(screen.getByTestId("card-in-total")).toHaveTextContent("500,00");
  });

  test("the same screen in Arabic reads its own words and mirrors", async () => {
    mount("ar");
    expect(await screen.findByRole("heading", { name: ar.expenses_title })).toBeInTheDocument();
    await screen.findByTestId("expenses-table");
    expect(expenseRows()[1]).toHaveTextContent(ar.expense_category_rent);
  });
});

describe("adding one", () => {
  test("posts the amount in centimes, the category id and the day", async () => {
    const user = userEvent.setup();
    mount();
    await openSheet(user);
    await user.type(screen.getByTestId("expense-amount"), "1250,50");
    await user.type(screen.getByTestId("expense-note"), "  taxi  ");
    await user.click(screen.getByRole("button", { name: fr.action_save }));
    await waitFor(() => {
      expect(posted().url).toContain("/expenses");
    });
    expect(posted().body).toEqual({
      category_id: 1,
      amount_centimes: 125_050,
      expense_date: SHOP_TODAY,
      // Trimmed by the screen as well as by the core, so what the form
      // shows and what is stored are the same string.
      note: "taxi",
    });
  });

  test("a retired category is not offered", async () => {
    const user = userEvent.setup();
    mount();
    await openSheet(user);
    await user.click(screen.getByTestId("expense-category"));
    const options = await screen.findAllByRole("option");
    expect(options.map((o) => o.textContent)).toEqual([
      fr.expense_category_rent,
      fr.expense_category_electricity,
    ]);
  });

  test("an amount of nothing is refused by the form and no request goes out", async () => {
    const user = userEvent.setup();
    mount();
    await openSheet(user);
    await user.type(screen.getByTestId("expense-amount"), "0");
    await user.click(screen.getByRole("button", { name: fr.action_save }));
    expect(await screen.findByText(fr.error_expense_amount_zero)).toBeInTheDocument();
    expect(() => posted()).toThrow();
  });

  test("an amount nobody can read is refused before it reaches the server", async () => {
    const user = userEvent.setup();
    mount();
    await openSheet(user);
    await user.type(screen.getByTestId("expense-amount"), "douze");
    await user.click(screen.getByRole("button", { name: fr.action_save }));
    expect(await screen.findByText(fr.error_expense_amount_invalid)).toBeInTheDocument();
    expect(() => posted()).toThrow();
  });

  test("the server's refusal is shown as the screen's own words", async () => {
    const user = userEvent.setup();
    mount();
    writeAnswer = () =>
      json(422, {
        error: { code: "validation", message: "no", field: "category_id" },
      });
    await openSheet(user);
    await user.type(screen.getByTestId("expense-amount"), "10");
    await user.click(screen.getByRole("button", { name: fr.action_save }));
    expect(await screen.findByText(fr.error_validation)).toBeInTheDocument();
  });

  test("a saved row refreshes the month and the cash position", async () => {
    const user = userEvent.setup();
    mount();
    await openSheet(user);
    await user.type(screen.getByTestId("expense-amount"), "10");
    const before = fetched().filter((u) => u.includes("/cash?month=")).length;
    await user.click(screen.getByRole("button", { name: fr.action_save }));
    await waitFor(() => {
      expect(fetched().filter((u) => u.includes("/cash?month=")).length).toBeGreaterThan(before);
    });
    expect(fetched().filter((u) => u.includes(`/expenses?month=${SHOP_MONTH}`)).length).toBeGreaterThan(
      1,
    );
  });
});

// What the rewrite on the kit brought that the old screen did not have: a
// month with nothing in it is a state rather than a sentence, and the form
// lives in a panel that has to close itself.
describe("the empty month and the entry panel", () => {
  test("a month the shop spent nothing in offers the one thing to do about it", async () => {
    const user = userEvent.setup();
    listed = { month: SHOP_MONTH, total_centimes: 0, expenses: [] };
    mount();
    const empty = await screen.findByTestId("empty-state");
    expect(empty).toHaveTextContent(fr.expenses_empty);
    expect(empty).toHaveTextContent(fr.expenses_empty_hint);
    // The total is still the server's zero, not a blank: a month that took
    // nothing out is a fact, and the card says so.
    expect(screen.getByTestId("expenses-total")).toHaveTextContent("0,00");
    await user.click(within(empty).getByRole("button", { name: fr.expenses_add }));
    expect(await screen.findByTestId("expense-amount")).toBeInTheDocument();
  });

  test("the panel closes once the row is filed, and stays open when it is refused", async () => {
    const user = userEvent.setup();
    mount();
    await openSheet(user);
    await user.type(screen.getByTestId("expense-amount"), "10");
    await user.click(screen.getByRole("button", { name: fr.action_save }));
    await waitFor(() => {
      expect(screen.queryByTestId("expense-form")).not.toBeInTheDocument();
    });

    writeAnswer = () => json(422, { error: { code: "validation", message: "no" } });
    await openSheet(user);
    await user.type(screen.getByTestId("expense-amount"), "10");
    await user.click(screen.getByRole("button", { name: fr.action_save }));
    expect(await screen.findByText(fr.error_validation)).toBeInTheDocument();
    // Still there: a refused row is a row the shop has to correct, and a
    // panel that closed on it would throw the typing away.
    expect(screen.getByTestId("expense-form")).toBeInTheDocument();
  });
});
