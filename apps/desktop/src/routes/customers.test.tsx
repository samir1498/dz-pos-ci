// The customers screen is checked for what it shows from the API's answer
// and for what it sends. The rules (a blank name, a zero adjustment, a field
// too long) are the API crate's tests; what these hold is the wiring: the
// centimes, the party kind, the whole row on an update and the ledger
// refreshing after a correction.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  RouterProvider,
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
} from "@tanstack/react-router";
import type { CustomerDto, CustomerLedgerDto, CustomerPaymentsDto } from "@dzpos/shared";
import { I18nProvider, type Lang } from "@/i18n";
import fr from "@/i18n/fr.json";
import ar from "@/i18n/ar.json";
import { CustomerFiche, CustomersScreen } from "./customers";

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

/** What the server says the day is. Deliberately a day the machine is not
 * on, so a screen that read `new Date()` would fail here. */
const SHOP_TODAY = "2027-03-04";

let fetchMock: ReturnType<typeof vi.fn>;
let list: CustomerDto[];
let rows: CustomerLedgerDto;
let payments: CustomerPaymentsDto;
let writeAnswer: (() => Response) | null;
let clockAnswer: (() => Response) | null;

beforeEach(() => {
  list = [benali];
  rows = ledger;
  payments = noPayments;
  writeAnswer = null;
  clockAnswer = null;
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
    // One fiche by id, which is what the `/customers/$id` route reads. Before
    // the list branch below: `/customers/3` has no `q` and would otherwise
    // come back as the whole list.
    const one = /\/customers\/(\d+)$/.exec(url);
    if (one !== null) {
      const found = list.find((c) => c.id === Number(one[1]));
      return Promise.resolve(
        found === undefined
          ? json(404, { error: { code: "not_found", message: "no" } })
          : json(200, found),
      );
    }
    // The shop's day, which the statement panel asks for before it offers a
    // range. A fixed one so the defaults it fills in are assertable.
    if (url.endsWith("/clock")) {
      if (clockAnswer !== null) return Promise.resolve(clockAnswer());
      return Promise.resolve(json(200, { today: SHOP_TODAY }));
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
    if (url.includes("/debt-slip")) {
      return Promise.resolve(
        new Response("<html><body>SITUATION</body></html>", {
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
      // Nothing on this fiche's account, so no reason was asked for and none
      // is sent: the field travels as a null the way every empty one does.
      close_reason: null,
    });
    expect(sent("PUT").body).not.toHaveProperty("opening_debt_centimes");
  });

  // features.md §2: closing a fiche over an account that is still open is a
  // decision, and the reason goes into the log beside the balance. The screen
  // asks for it the moment the box comes off; the core is what refuses
  // without it, so the rule is written once.
  test("closing a fiche that still carries a balance asks why", async () => {
    mount();
    await userEvent.click(
      await screen.findByRole("button", { name: `${fr.customers_edit} Entreprise Benali` }),
    );
    expect(screen.queryByTestId("customer-close-reason")).not.toBeInTheDocument();

    await userEvent.click(screen.getByLabelText(fr.field_customer_active));
    const reason = await screen.findByTestId("customer-close-reason");
    expect(screen.getByText(fr.customers_close_reason)).toBeInTheDocument();

    await userEvent.type(reason, "dossier au contentieux");
    await userEvent.click(screen.getByRole("button", { name: fr.action_save }));

    await waitFor(() => expect(sent("PUT").url).toMatch(/\/customers\/3$/));
    expect(sent("PUT").body.active).toBe(false);
    expect(sent("PUT").body.close_reason).toBe("dossier au contentieux");
  });

  test("closing a settled fiche asks nothing", async () => {
    list = [noCredit];
    mount();
    await userEvent.click(
      await screen.findByRole("button", { name: `${fr.customers_edit} Ali Cash` }),
    );
    await userEvent.click(screen.getByLabelText(fr.field_customer_active));
    expect(screen.queryByTestId("customer-close-reason")).not.toBeInTheDocument();
  });

  // A fiche whose balance nets to nothing can still have a facture asking to
  // be paid, and the screen cannot see that document. The server's refusal
  // names the field, and that is what opens the box.
  test("a refusal naming the reason opens the box the screen could not know to open", async () => {
    list = [noCredit];
    writeAnswer = () =>
      json(422, {
        error: {
          code: "validation",
          message: "this customer still has an account open",
          field: "reason",
        },
      });
    mount();
    await userEvent.click(
      await screen.findByRole("button", { name: `${fr.customers_edit} Ali Cash` }),
    );
    await userEvent.click(screen.getByLabelText(fr.field_customer_active));
    expect(screen.queryByTestId("customer-close-reason")).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: fr.action_save }));
    expect(await screen.findByTestId("customer-close-reason")).toBeInTheDocument();
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
    // The whole range, exactly: the year opens on 1 January and closes on
    // the day the server calls today.
    expect(asked?.endsWith(`/customers/3/statement?from=2027-01-01&to=${SHOP_TODAY}&lang=fr`)).toBe(
      true,
    );
  });

  test("the range opens on the shop's day, which the server says and the browser does not", async () => {
    await openTheFiche();

    // The stub answers a fixed `/clock`; a screen reading `new Date()` would
    // date the range from whatever zone the machine is in, which is a day
    // either side of the ledger for a shop open past midnight.
    expect(screen.getByLabelText(fr.field_statement_to)).toHaveValue(SHOP_TODAY);
    expect(screen.getByLabelText(fr.field_statement_from)).toHaveValue("2027-01-01");
    expect(fetched().some((url) => url.endsWith("/clock"))).toBe(true);
  });

  test("a clock the server will not answer is an error line with a retry, not a wait", async () => {
    // The range waits for the shop's day, and the day is a call that can
    // fail. Read as "not here yet", a refusal would leave the panel on its
    // loading line for as long as the fiche stayed open and say nothing.
    clockAnswer = () => json(500, { error: { code: "storage", message: "no" } });
    await openTheFiche();

    const failed = await screen.findByText(fr.error_storage);
    expect(failed).toHaveAttribute("role", "alert");
    expect(screen.queryByLabelText(fr.field_statement_to)).not.toBeInTheDocument();

    clockAnswer = null;
    await userEvent.click(screen.getByRole("button", { name: fr.action_retry }));

    expect(await screen.findByLabelText(fr.field_statement_to)).toHaveValue(SHOP_TODAY);
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

  test("the debt slip is asked for on the button and shown as the page the core rendered", async () => {
    await openTheFiche();
    // Nothing is fetched before the button: a slip nobody asked for is a
    // render of a page nobody is going to print.
    expect(fetched().some((url) => url.includes("/debt-slip"))).toBe(false);

    await userEvent.click(screen.getByRole("button", { name: fr.action_debt_slip }));

    const frame = await screen.findByTestId("customer-debt-slip");
    expect(frame).toHaveAttribute("sandbox", "");
    expect(frame.getAttribute("srcdoc")).toContain("SITUATION");
    const asked = fetched().find((url) => url.includes("/debt-slip"));
    expect(asked).toMatch(/\/customers\/3\/debt-slip\?lang=fr$/);
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

/** The fiche on a page of its own, which is what `/customers/$id` opens. The
 * router here is not the app's; it is the smallest one that lets the fiche's
 * own `Link` render, the way the till's tests build theirs. */
describe("one customer by id", () => {
  function mountFiche(id: number) {
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
    const rootRoute = createRootRoute();
    const ficheRoute = createRoute({
      getParentRoute: () => rootRoute,
      path: "/",
      component: () => <CustomerFiche id={id} />,
    });
    const router = createRouter({
      routeTree: rootRoute.addChildren([ficheRoute]),
      history: createMemoryHistory({ initialEntries: ["/"] }),
    });
    return render(
      <I18nProvider lang="fr">
        <QueryClientProvider client={client}>
          <RouterProvider router={router} />
        </QueryClientProvider>
      </I18nProvider>,
    );
  }

  test("reads the one fiche by id and shows the ledger under it", async () => {
    mountFiche(3);

    expect(await screen.findByDisplayValue("Entreprise Benali")).toBeInTheDocument();
    // The fiche it shows came from the customer's own route, not from the
    // list: a page addressed by id must not depend on a list being loaded.
    expect(fetched().some((url) => /\/customers\/3$/.test(url))).toBe(true);
    expect(fetched().some((url) => url.endsWith("/customers"))).toBe(false);
    expect(
      await screen.findByRole("heading", { name: fr.customers_ledger }),
    ).toBeInTheDocument();
    // And the same two papers the panel offers.
    expect(screen.getByRole("button", { name: fr.action_debt_slip })).toBeInTheDocument();
  });

  test("a customer this shop does not have says so instead of showing a blank fiche", async () => {
    mountFiche(4242);

    expect(await screen.findByRole("alert")).toHaveTextContent(fr.error_not_found);
    expect(screen.queryByRole("heading", { name: fr.customers_ledger })).toBeNull();
  });
});
