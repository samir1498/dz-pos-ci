// The customers screen is checked for what it shows from the API's answer
// and for what it sends. The rules (a blank name, a zero adjustment, a field
// too long) are the API crate's tests; what these hold is the wiring: the
// centimes, the party kind, the whole row on an update and the ledger
// refreshing after a correction.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { CustomerDto, CustomerLedgerDto, CustomerPaymentsDto } from "@dzpos/shared";
import { I18nProvider, type Lang } from "@/i18n";
import fr from "@/i18n/fr.json";
import ar from "@/i18n/ar.json";
import { CustomersScreen } from "./customers";

const benali: CustomerDto = {
  id: 3,
  shop_id: 1,
  name: "Entreprise Benali",
  party_kind: "company",
  phone: "0770 11 22 33",
  address: null,
  rc: "16/00-7654321 B 22",
  nif: null,
  nis: null,
  ai: null,
  credit_limit_centimes: 200_000,
  warn_threshold_centimes: 150_000,
  notes: null,
  active: true,
  balance_centimes: 150_000,
};

const overLimit: CustomerDto = {
  ...benali,
  id: 4,
  name: "Zoubir Amrani",
  party_kind: "consumer",
  phone: null,
  rc: null,
  balance_centimes: 250_000,
};

const noCredit: CustomerDto = {
  ...benali,
  id: 5,
  name: "Ali Cash",
  credit_limit_centimes: 0,
  warn_threshold_centimes: null,
  balance_centimes: 0,
};

/** Owing exactly the limit is not over it: the tag turns on the figure
 *  above, and what shows here is the warning threshold below. */
const atLimit: CustomerDto = {
  ...benali,
  id: 6,
  name: "Farid Exact",
  balance_centimes: 200_000,
};

/** No credit at all and something owed anyway. Two tags apply and the worse
 *  one shows. */
const owingWithNoCredit: CustomerDto = {
  ...noCredit,
  id: 7,
  name: "Kamel Ardoise",
  balance_centimes: 50_000,
};

/** A fiche the shop has stopped selling to. Money can still be collected on
 *  it and mistakes still corrected, which is what the screen has to say. */
const closed: CustomerDto = {
  ...benali,
  id: 8,
  name: "Nadir Fermé",
  active: false,
};

/** An avoir left more on the fiche than was owed, so the balance is
 *  negative: the shop holds the customer's money, not the other way round.
 *  The sign alone would read as a debt of minus something, which is not a
 *  sentence anyone says at a counter. */
const holdingCredit: CustomerDto = {
  ...benali,
  id: 9,
  name: "Yacine Avoir",
  balance_centimes: -100_000,
};

const ledger: CustomerLedgerDto = {
  customer_id: 3,
  balance_centimes: 150_000,
  entries: [
    {
      id: 11,
      customer_id: 3,
      document_id: null,
      kind: "opening",
      debit_centimes: 150_000,
      credit_centimes: 0,
      balance_after_centimes: 150_000,
      user_id: 1,
      note: "solde de départ",
      created_at: "2026-09-09 10:00:00",
    },
  ],
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

/** The URL and JSON body of the last request made with `method`. */
function sent(method: string): { url: string; body: Record<string, unknown> } {
  const calls = fetchMock.mock.calls.filter((call) => {
    const init: unknown = call[1];
    return isInit(init) && init.method === method;
  });
  const last = calls[calls.length - 1];
  if (last === undefined) throw new Error(`no ${method} was made`);
  const init: unknown = last[1];
  if (!isInit(init) || typeof init.body !== "string") {
    throw new Error(`the ${method} had no JSON body`);
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

function mount(lang: Lang = "fr") {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <I18nProvider lang={lang}>
      <QueryClientProvider client={client}>
        <CustomersScreen />
      </QueryClientProvider>
    </I18nProvider>,
  );
}

const noPayments: CustomerPaymentsDto = {
  customer_id: 3,
  balance_centimes: 150_000,
  payments: [],
};

/** One payment of 1 000,00 spread over two factures, the older one settled
 *  in full. What the server answers after the money has landed. */
const paid: CustomerPaymentsDto = {
  customer_id: 3,
  balance_centimes: 50_000,
  payments: [
    {
      ledger_id: 21,
      customer_id: 3,
      amount_centimes: 100_000,
      payment_mode: "cash",
      note: "acompte",
      balance_after_centimes: 50_000,
      allocations: [
        { document_id: 8, amount_centimes: 60_000 },
        { document_id: 9, amount_centimes: 40_000 },
      ],
      created_at: "2026-09-12 16:30:00",
    },
  ],
};

let fetchMock: ReturnType<typeof vi.fn>;
let list: CustomerDto[];
let rows: CustomerLedgerDto;
let payments: CustomerPaymentsDto;
let writeAnswer: (() => Response) | null;

beforeEach(() => {
  list = [benali];
  rows = ledger;
  payments = noPayments;
  writeAnswer = null;
  vi.spyOn(window, "confirm").mockReturnValue(true);
  fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
    const url = String(input);
    if (init?.method === "POST" || init?.method === "PUT") {
      if (writeAnswer !== null) return Promise.resolve(writeAnswer());
      if (url.endsWith("/payments")) {
        payments = paid;
        list = list.map((c) =>
          c.id === 3 ? { ...c, balance_centimes: paid.balance_centimes } : c,
        );
        return Promise.resolve(json(201, paid));
      }
      if (url.endsWith("/adjustments")) {
        const body: unknown = JSON.parse(String(init.body));
        const amount =
          isInit(body) && "amount_centimes" in body && typeof body.amount_centimes === "number"
            ? body.amount_centimes
            : 0;
        rows = {
          ...rows,
          balance_centimes: rows.balance_centimes + amount,
          entries: [
            {
              ...ledger.entries[0],
              id: 12,
              kind: "adjustment",
              debit_centimes: amount > 0 ? amount : 0,
              credit_centimes: amount < 0 ? -amount : 0,
              balance_after_centimes: rows.balance_centimes + amount,
              note: "erreur de saisie",
            },
            ...rows.entries,
          ],
        };
        list = list.map((c) =>
          c.id === 3 ? { ...c, balance_centimes: rows.balance_centimes } : c,
        );
        return Promise.resolve(json(201, rows));
      }
      return Promise.resolve(json(init.method === "POST" ? 201 : 200, benali));
    }
    if (url.includes("/ledger")) return Promise.resolve(json(200, rows));
    if (url.includes("/payments")) return Promise.resolve(json(200, payments));
    if (url.includes("/statement")) {
      return Promise.resolve(
        new Response("<html><body>RELEVÉ</body></html>", {
          status: 200,
          headers: { "content-type": "text/html" },
        }),
      );
    }
    if (url.includes("/customers")) {
      const q = new URL(url).searchParams.get("q");
      const found =
        q === null
          ? list
          : list.filter((c) => c.name.toLowerCase().includes(q.toLowerCase()));
      return Promise.resolve(json(200, found));
    }
    return Promise.resolve(json(404, { error: { code: "not_found", message: "no" } }));
  });
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("the list", () => {
  test("shows the debt, the limit and the tag each balance earns", async () => {
    list = [benali, overLimit, noCredit];
    mount();

    const near = await screen.findByRole("row", { name: /Entreprise Benali/ });
    expect(within(near).getByText("1 500,00")).toBeInTheDocument();
    expect(within(near).getByText("2 000,00")).toBeInTheDocument();
    expect(within(near).getByText(fr.status_near_limit)).toBeInTheDocument();

    const over = screen.getByRole("row", { name: /Zoubir Amrani/ });
    expect(within(over).getByText(fr.status_over_limit)).toBeInTheDocument();

    const cash = screen.getByRole("row", { name: /Ali Cash/ });
    expect(within(cash).getByText(fr.status_no_credit)).toBeInTheDocument();
  });

  test("the worse of two tags is the one that shows, and the limit itself is not over it", async () => {
    list = [atLimit, owingWithNoCredit];
    mount();

    const exact = await screen.findByRole("row", { name: /Farid Exact/ });
    expect(within(exact).getByText(fr.status_near_limit)).toBeInTheDocument();
    expect(within(exact).queryByText(fr.status_over_limit)).not.toBeInTheDocument();

    const ardoise = screen.getByRole("row", { name: /Kamel Ardoise/ });
    expect(within(ardoise).getByText(fr.status_over_limit)).toBeInTheDocument();
    expect(within(ardoise).queryByText(fr.status_no_credit)).not.toBeInTheDocument();
  });

  test("the search travels to the API and the list follows it", async () => {
    list = [benali, overLimit];
    mount();
    await screen.findByRole("row", { name: /Zoubir Amrani/ });

    await userEvent.type(screen.getByLabelText(fr.customers_search), "benali");
    await waitFor(() => {
      expect(screen.queryByRole("row", { name: /Zoubir Amrani/ })).not.toBeInTheDocument();
    });
    expect(fetched().some((url) => url.endsWith("/customers?q=benali"))).toBe(true);
  });

  test("says so when the shop has no customer yet", async () => {
    list = [];
    mount();
    expect(await screen.findByText(fr.customers_empty)).toBeInTheDocument();
  });
});

describe("the fiche", () => {
  test("creates one, posting centimes, the party kind and the opening debt", async () => {
    list = [];
    mount();
    await screen.findByText(fr.customers_empty);

    await userEvent.click(screen.getByRole("button", { name: fr.customers_add }));
    await userEvent.type(screen.getByLabelText(fr.field_name), "Entreprise Benali");
    await userEvent.click(screen.getByRole("radio", { name: fr.party_company }));
    await userEvent.type(screen.getByLabelText(fr.field_phone), "0770 11 22 33");
    await userEvent.type(screen.getByLabelText(fr.field_credit_limit), "2000");
    await userEvent.type(screen.getByLabelText(fr.field_warn_threshold), "1500");
    await userEvent.type(screen.getByLabelText(fr.field_opening_debt), "1500,50");
    await userEvent.click(screen.getByRole("button", { name: fr.action_save }));

    await waitFor(() => expect(sent("POST").url).toMatch(/\/customers$/));
    expect(sent("POST").body).toEqual({
      name: "Entreprise Benali",
      party_kind: "company",
      phone: "0770 11 22 33",
      address: null,
      rc: null,
      nif: null,
      nis: null,
      ai: null,
      credit_limit_centimes: 200_000,
      warn_threshold_centimes: 150_000,
      notes: null,
      active: true,
      opening_debt_centimes: 150_050,
    });
  });

  test("an update sends the whole row, with a null for every field cleared", async () => {
    mount();
    await userEvent.click(
      await screen.findByRole("button", { name: `${fr.customers_edit} Entreprise Benali` }),
    );

    await userEvent.clear(screen.getByLabelText(fr.field_rc));
    await userEvent.clear(screen.getByLabelText(fr.field_credit_limit));
    await userEvent.click(screen.getByRole("radio", { name: fr.party_consumer }));
    await userEvent.click(screen.getByLabelText(fr.field_customer_active));
    await userEvent.click(screen.getByRole("button", { name: fr.action_save }));

    await waitFor(() => expect(sent("PUT").url).toMatch(/\/customers\/3$/));
    expect(sent("PUT").body).toEqual({
      name: "Entreprise Benali",
      party_kind: "consumer",
      phone: "0770 11 22 33",
      address: null,
      rc: null,
      nif: null,
      nis: null,
      ai: null,
      credit_limit_centimes: null,
      warn_threshold_centimes: 150_000,
      notes: null,
      active: false,
    });
    expect(sent("PUT").body).not.toHaveProperty("opening_debt_centimes");
  });

  test("the opening debt is asked for once and never on an existing fiche", async () => {
    mount();
    await userEvent.click(
      await screen.findByRole("button", { name: `${fr.customers_edit} Entreprise Benali` }),
    );
    expect(screen.queryByLabelText(fr.field_opening_debt)).not.toBeInTheDocument();
  });

  test("the server's refusal is shown in the screen's own words", async () => {
    writeAnswer = () =>
      json(422, { error: { code: "validation", message: "name is required" } });
    list = [];
    mount();
    await screen.findByText(fr.customers_empty);
    await userEvent.click(screen.getByRole("button", { name: fr.customers_add }));
    await userEvent.type(screen.getByLabelText(fr.field_name), "Entreprise Benali");
    await userEvent.click(screen.getByRole("button", { name: fr.action_save }));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent(fr.error_validation);
    expect(alert.textContent).not.toContain("name is required");
  });
});

describe("the ledger", () => {
  test("reads the movements with the balance the core ran up", async () => {
    mount();
    await userEvent.click(
      await screen.findByRole("button", { name: `${fr.customers_edit} Entreprise Benali` }),
    );

    const row = await screen.findByRole("row", { name: /solde de départ/ });
    expect(within(row).getByText(fr.debt_opening)).toBeInTheDocument();
    expect(within(row).getAllByText("1 500,00")).toHaveLength(2);
  });

  test("an adjustment posts signed centimes and the ledger comes back changed", async () => {
    mount();
    await userEvent.click(
      await screen.findByRole("button", { name: `${fr.customers_edit} Entreprise Benali` }),
    );
    await screen.findByRole("row", { name: /solde de départ/ });

    await userEvent.type(screen.getByLabelText(fr.field_adjust_amount), "-500");
    await userEvent.type(screen.getByLabelText(fr.field_adjust_note), "erreur de saisie");
    await userEvent.click(screen.getByRole("button", { name: fr.action_adjust }));

    await waitFor(() => expect(sent("POST").url).toMatch(/\/customers\/3\/adjustments$/));
    expect(sent("POST").body).toEqual({
      amount_centimes: -50_000,
      note: "erreur de saisie",
    });
    expect(window.confirm).toHaveBeenCalledWith(fr.customers_adjust_confirm);

    const adjusted = await screen.findByRole("row", { name: /erreur de saisie/ });
    expect(within(adjusted).getByText(fr.debt_adjustment)).toBeInTheDocument();
    expect(within(adjusted).getByText("1 000,00")).toBeInTheDocument();
    expect(await screen.findByRole("status")).toHaveTextContent(fr.customers_adjusted);
  });

  test("a cancelled confirmation posts nothing", async () => {
    vi.spyOn(window, "confirm").mockReturnValue(false);
    mount();
    await userEvent.click(
      await screen.findByRole("button", { name: `${fr.customers_edit} Entreprise Benali` }),
    );
    await userEvent.type(screen.getByLabelText(fr.field_adjust_amount), "-500");
    await userEvent.click(screen.getByRole("button", { name: fr.action_adjust }));

    expect(() => sent("POST")).toThrow();
  });

  test("a refused correction keeps the figure that was typed", async () => {
    mount();
    await userEvent.click(
      await screen.findByRole("button", { name: `${fr.customers_edit} Entreprise Benali` }),
    );
    await screen.findByRole("row", { name: /solde de départ/ });

    writeAnswer = () =>
      json(422, { error: { code: "validation", message: "amount_centimes is too large" } });
    await userEvent.type(screen.getByLabelText(fr.field_adjust_amount), "-500");
    await userEvent.type(screen.getByLabelText(fr.field_adjust_note), "erreur de saisie");
    await userEvent.click(screen.getByRole("button", { name: fr.action_adjust }));

    expect(await screen.findByRole("alert")).toHaveTextContent(fr.error_validation);
    // Emptying the box on a refusal means retyping the figure to find out
    // what was wrong with it.
    expect(screen.getByLabelText(fr.field_adjust_amount)).toHaveValue("-500");
    expect(screen.getByLabelText(fr.field_adjust_note)).toHaveValue("erreur de saisie");
  });

  test("an adjustment of nothing is refused before it leaves the screen", async () => {
    mount();
    await userEvent.click(
      await screen.findByRole("button", { name: `${fr.customers_edit} Entreprise Benali` }),
    );
    await userEvent.type(screen.getByLabelText(fr.field_adjust_amount), "0");
    await userEvent.click(screen.getByRole("button", { name: fr.action_adjust }));

    expect(await screen.findByText(fr.error_amount_zero)).toBeInTheDocument();
    expect(() => sent("POST")).toThrow();
  });
});

describe("a credit balance", () => {
  test("a negative balance is named a credit rather than shown as a minus debt", async () => {
    list = [holdingCredit];
    rows = { ...ledger, customer_id: 9, balance_centimes: -100_000, entries: [] };
    payments = { ...noPayments, customer_id: 9, balance_centimes: -100_000 };
    mount();

    // The list says it on the row, before anything is opened.
    const row = await screen.findByRole("row", { name: /Yacine Avoir/ });
    expect(within(row).getByText(fr.customers_credit)).toBeInTheDocument();
    expect(within(row).getByText("1 000,00")).toBeInTheDocument();
    expect(within(row).queryByText("-1 000,00")).toBeNull();

    // And the fiche labels the figure the same way, beside the amount rather
    // than as a column header the whole table shares.
    await userEvent.click(
      screen.getByRole("button", { name: `${fr.customers_edit} Yacine Avoir` }),
    );
    const heading = await screen.findByRole("heading", { name: fr.customers_ledger });
    const fiche = heading.closest("section");
    if (fiche === null) throw new Error("the ledger heading sits in no section");
    const said = within(fiche);
    expect(said.getByText(fr.customers_credit)).toBeInTheDocument();
    expect(said.getByText("1 000,00")).toBeInTheDocument();
    expect(said.queryByText(fr.customers_balance)).toBeNull();
  });
});

describe("payments", () => {
  async function openTheFiche() {
    mount();
    await userEvent.click(
      await screen.findByRole("button", { name: `${fr.customers_edit} Entreprise Benali` }),
    );
    await screen.findByRole("row", { name: /solde de départ/ });
  }

  test("a closed fiche still takes a payment, and says why the form is there", async () => {
    list = [closed];
    mount();
    await userEvent.click(
      await screen.findByRole("button", { name: `${fr.customers_edit} Nadir Fermé` }),
    );

    // The form stays: a shop closes a fiche to stop selling, not to stop
    // collecting, and the note next to it says so.
    expect(await screen.findByLabelText(fr.field_payment_amount)).toBeInTheDocument();
    expect(screen.getByText(fr.customers_closed_still_collects)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: fr.action_take_payment })).toBeEnabled();
  });

  test("a payment posts the amount, the mode and the note, and the allocations come back", async () => {
    await openTheFiche();

    await userEvent.type(screen.getByLabelText(fr.field_payment_amount), "1000");
    await userEvent.click(screen.getByRole("radio", { name: fr.payment_cash }));
    await userEvent.type(screen.getByLabelText(fr.field_payment_note), "acompte");
    await userEvent.click(screen.getByRole("button", { name: fr.action_take_payment }));

    await waitFor(() => expect(sent("POST").url).toMatch(/\/customers\/3\/payments$/));
    expect(sent("POST").body).toEqual({
      amount_centimes: 100_000,
      payment_mode: "cash",
      note: "acompte",
    });
    expect(window.confirm).toHaveBeenCalledWith(fr.customers_pay_confirm);

    // The documents the money landed on are shown open: which facture a
    // payment settled is what a customer asks at the counter.
    const rows = await screen.findAllByTestId("customer-payment");
    expect(within(rows[0]).getByText("600,00")).toBeInTheDocument();
    expect(within(rows[0]).getByText("400,00")).toBeInTheDocument();
    expect(within(rows[0]).getByText("1 000,00")).toBeInTheDocument();
    expect(await screen.findByRole("status")).toHaveTextContent(fr.customers_paid);
  });

  test("a payment above the debt says so and names what is still owed", async () => {
    await openTheFiche();
    writeAnswer = () =>
      json(422, {
        error: {
          code: "validation",
          message: "a payment is never more than what the customer owes",
          field: "amount_centimes",
          outstanding_centimes: 150_000,
        },
      });

    await userEvent.type(screen.getByLabelText(fr.field_payment_amount), "2000");
    await userEvent.click(screen.getByRole("button", { name: fr.action_take_payment }));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent(fr.error_payment_above_debt);
    // "Too much" is useless without the amount that would not have been.
    expect(alert).toHaveTextContent("1 500,00");
    // A refused payment keeps the figure that was typed.
    expect(screen.getByLabelText(fr.field_payment_amount)).toHaveValue("2000");
  });

  test("a payment of nothing is refused before it leaves the screen", async () => {
    await openTheFiche();

    await userEvent.type(screen.getByLabelText(fr.field_payment_amount), "0");
    await userEvent.click(screen.getByRole("button", { name: fr.action_take_payment }));

    expect(await screen.findByText(fr.error_payment_amount_zero)).toBeInTheDocument();
    expect(() => sent("POST")).toThrow();
  });

  test("the statement is asked for over the range and shown as the page the core rendered", async () => {
    await openTheFiche();

    await userEvent.click(screen.getByRole("button", { name: fr.action_statement }));

    const frame = await screen.findByTestId("customer-statement");
    expect(frame).toHaveAttribute("sandbox", "");
    expect(frame.getAttribute("srcdoc")).toContain("RELEVÉ");
    const asked = fetched().find((url) => url.includes("/statement"));
    expect(asked).toMatch(/\/customers\/3\/statement\?from=\d{4}-01-01&to=\d{4}-\d{2}-\d{2}&lang=fr$/);
  });

  test("a range that ends before it starts asks for nothing", async () => {
    await openTheFiche();

    const from = screen.getByLabelText(fr.field_statement_from);
    await userEvent.clear(from);
    await userEvent.type(from, "2027-12-31");

    expect(await screen.findByText(fr.error_statement_range_invalid)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: fr.action_statement })).toBeDisabled();
    expect(fetched().some((url) => url.includes("/statement"))).toBe(false);
  });
});

describe("Arabic", () => {
  test("shows the Arabic wording and keeps the amounts left to right", async () => {
    mount("ar");
    const row = await screen.findByRole("row", { name: /Entreprise Benali/ });
    expect(within(row).getByText(ar.status_near_limit)).toBeInTheDocument();
    // The amount cells carry dir="ltr" so the bidi algorithm cannot reorder
    // the groups and the sign inside a right-to-left row.
    const amount = within(row).getByText("1 500,00");
    expect(amount).toHaveAttribute("dir", "ltr");
  });
});
