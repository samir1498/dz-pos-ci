// The suppliers screen is checked for what it shows from the API's answer and
// for what it sends. The rules (a blank name, a payment above the debt, a
// close over an open account) are the API crate's tests; what these hold is
// the wiring: the centimes, the whole row on an update, the close carrying
// its reason, and the ledger refreshing after a payment.
//
// The screen is two pages now, so the tests mount a router at a path rather
// than a component: `/suppliers` is the list with its panel, `/suppliers/{id}`
// is the statement with the ledger and the two dialogs. The panel and the
// dialogs are Radix overlays, which put `aria-hidden` on everything behind
// them: a query by role after one opens sees the overlay and nothing else,
// which is why the tests that go back to the list close the panel first.

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
import type { SupplierDto, SupplierLedgerDto } from "@dzpos/shared";
import { I18nProvider, type Lang } from "@/i18n";
import fr from "@/i18n/fr.json";
import ar from "@/i18n/ar.json";
import { SupplierFiche, SuppliersScreen } from "./suppliers";

/**
 * The amounts as they are read back. packages/shared groups thousands with
 * U+202F, a narrow no-break space, and testing-library normalises every space
 * character to an ordinary one before it matches; so these carry the ordinary
 * space, and a test that pasted the narrow one would pass for the wrong
 * reason rather than fail. The formatting itself is money.test.ts's.
 */
const ONE_FIVE_HUNDRED = "1 500,00";
const ONE_THOUSAND = "1 000,00";
/** The same figure inside a form control. `toHaveValue` reads the raw
 *  string, with no normalising, so this is the narrow space itself. */
const TWO_THOUSAND_TYPED = "2\u202f000,00";

const amrani: SupplierDto = {
  id: 3,
  shop_id: 1,
  name: "Sarl Amrani",
  phone: "0770 11 22 33",
  address: null,
  rc: "16/00-7654321 B 22",
  nif: null,
  nis: null,
  ai: null,
  notes: null,
  active: true,
  balance_centimes: 150_000,
};

/** A return past what was due: the supplier owes the shop, which is an
 *  advance and not a credit the shop is holding. */
const inAdvance: SupplierDto = {
  ...amrani,
  id: 4,
  name: "Bensalem",
  phone: null,
  balance_centimes: -50_000,
};

/** A fiche the shop has stopped buying from. Money can still be paid on it
 *  and mistakes still corrected, which is what the screen has to say. */
const closed: SupplierDto = {
  ...amrani,
  id: 5,
  name: "Zoubir Fermé",
  active: false,
  balance_centimes: 0,
};

const ledger: SupplierLedgerDto = {
  supplier_id: 3,
  balance_centimes: 150_000,
  entries: [
    {
      id: 11,
      supplier_id: 3,
      purchase_id: null,
      kind: "opening",
      debit_centimes: 150_000,
      credit_centimes: 0,
      balance_after_centimes: 150_000,
      payment_mode: null,
      user_id: 1,
      note: "solde de départ",
      allocations: [],
      created_at: "2026-09-09 10:00:00",
    },
  ],
};

/** What the server answers once 1 000,00 has been paid against an order the
 *  ledger was carrying. */
const afterPayment: SupplierLedgerDto = {
  supplier_id: 3,
  balance_centimes: 50_000,
  entries: [
    {
      id: 12,
      supplier_id: 3,
      purchase_id: null,
      kind: "payment",
      debit_centimes: 0,
      credit_centimes: 100_000,
      balance_after_centimes: 50_000,
      payment_mode: "cash",
      user_id: 1,
      note: "acompte",
      allocations: [{ purchase_id: 8, amount_centimes: 100_000 }],
      created_at: "2026-09-12 16:30:00",
    },
    ...ledger.entries,
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

/**
 * Both pages behind one router, entered at the path under test. The list
 * links to the statement, so a bare `render(<SuppliersScreen />)` would throw
 * on the first row it drew; and the statement links back, which is what the
 * route file would be wrong about if it were wrong.
 */
function mountAt(path: string, lang: Lang = "fr") {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  const rootRoute = createRootRoute();
  const listRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/suppliers",
    component: SuppliersScreen,
  });
  const ficheRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/suppliers/$id",
    component: OneSupplier,
  });
  function OneSupplier() {
    const { id } = ficheRoute.useParams();
    return <SupplierFiche id={Number(id)} />;
  }
  const router = createRouter({
    routeTree: rootRoute.addChildren([listRoute, ficheRoute]),
    history: createMemoryHistory({ initialEntries: [path] }),
  });
  return render(
    <I18nProvider lang={lang}>
      <QueryClientProvider client={client}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </I18nProvider>,
  );
}

const mount = (lang: Lang = "fr") => mountAt("/suppliers", lang);
const mountFiche = (id: number) => mountAt(`/suppliers/${id}`);

/** Opens the fiche panel over the list, the way a row's own button does. */
async function openPanel(name: string) {
  const row = await screen.findByRole("row", { name: new RegExp(name) });
  await userEvent.click(within(row).getByRole("button", { name: `${fr.suppliers_edit} ${name}` }));
}

let fetchMock: ReturnType<typeof vi.fn>;
let list: SupplierDto[];
let rows: SupplierLedgerDto;
let writeAnswer: (() => Response) | null;

beforeEach(() => {
  list = [amrani];
  rows = ledger;
  writeAnswer = null;
  fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
    const url = String(input);
    if (init?.method === "POST" || init?.method === "PUT") {
      if (writeAnswer !== null) return Promise.resolve(writeAnswer());
      if (url.endsWith("/payments")) {
        rows = afterPayment;
        list = list.map((s) =>
          s.id === 3 ? { ...s, balance_centimes: rows.balance_centimes } : s,
        );
        return Promise.resolve(json(201, rows));
      }
      if (url.endsWith("/adjustments")) {
        const body: unknown = JSON.parse(String(init.body));
        const amount =
          isInit(body) && "amount_centimes" in body && typeof body.amount_centimes === "number"
            ? body.amount_centimes
            : 0;
        rows = { ...rows, balance_centimes: rows.balance_centimes + amount };
        list = list.map((s) =>
          s.id === 3 ? { ...s, balance_centimes: rows.balance_centimes } : s,
        );
        return Promise.resolve(json(201, rows));
      }
      if (url.endsWith("/close")) {
        list = list.map((s) => (s.id === 3 ? { ...s, active: false } : s));
        return Promise.resolve(json(200, { ...amrani, active: false }));
      }
      return Promise.resolve(json(init.method === "POST" ? 201 : 200, amrani));
    }
    // One fiche by id, which is what the `/suppliers/$id` route reads. Before
    // the list branch below: `/suppliers/3` has no `q` and would otherwise
    // come back as the whole list.
    const one = /\/suppliers\/(\d+)$/.exec(url);
    if (one !== null) {
      const found = list.find((s) => s.id === Number(one[1]));
      return Promise.resolve(
        found === undefined
          ? json(404, { error: { code: "not_found", message: "no" } })
          : json(200, found),
      );
    }
    if (url.includes("/ledger")) return Promise.resolve(json(200, rows));
    if (url.includes("/suppliers")) {
      const q = new URL(url).searchParams.get("q");
      const found =
        q === null ? list : list.filter((s) => s.name.toLowerCase().includes(q.toLowerCase()));
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
  test("shows what the shop owes each supplier, drawn positive either way", async () => {
    list = [amrani, inAdvance];
    mount();

    const owing = await screen.findByRole("row", { name: /Sarl Amrani/ });
    expect(within(owing).getByText(ONE_FIVE_HUNDRED)).toBeInTheDocument();
    expect(within(owing).queryByText(fr.suppliers_advance)).not.toBeInTheDocument();

    // An advance is named rather than shown as a minus: the amount is drawn
    // positive and the word carries the direction.
    const advance = screen.getByRole("row", { name: /Bensalem/ });
    expect(within(advance).getByText("500,00")).toBeInTheDocument();
    expect(within(advance).getByText(fr.suppliers_advance)).toBeInTheDocument();
  });

  test("a settled account and an open one wear different pills", async () => {
    list = [amrani, closed];
    mount();
    const owing = await screen.findByRole("row", { name: /Sarl Amrani/ });
    expect(within(owing).getByText(fr.pill_open)).toBeInTheDocument();
    const settled = screen.getByRole("row", { name: /Zoubir Fermé/ });
    expect(within(settled).getByText(fr.pill_paid)).toBeInTheDocument();
  });

  test("the search travels to the API and the list follows it", async () => {
    list = [amrani, inAdvance];
    mount();
    await screen.findByRole("row", { name: /Bensalem/ });

    await userEvent.type(screen.getByLabelText(fr.suppliers_search), "bensa");
    await waitFor(() => {
      expect(screen.queryByRole("row", { name: /Sarl Amrani/ })).not.toBeInTheDocument();
    });
    const asked = fetchMock.mock.calls.map((call) => String(call[0]));
    expect(asked.some((url) => url.includes("/suppliers?q=bensa"))).toBe(true);
  });

  test("a closed fiche is marked and stays in the list", async () => {
    list = [closed];
    mount();
    const row = await screen.findByRole("row", { name: /Zoubir Fermé/ });
    expect(within(row).getByText(fr.suppliers_inactive)).toBeInTheDocument();
  });

  test("an empty list says what would fill it rather than showing a bare frame", async () => {
    list = [];
    mount();
    expect(await screen.findByText(fr.suppliers_empty)).toBeInTheDocument();
    expect(screen.getByText(fr.suppliers_empty_hint)).toBeInTheDocument();
  });

  test("the panel gives the list back when it is dismissed", async () => {
    // The panel is a Radix overlay, so everything behind it is `aria-hidden`
    // while it is open: a row is not there to be found, and it is there again
    // the moment the panel goes. A test that never closed one would pass
    // against a screen that had lost its list.
    mount();
    await openPanel("Sarl Amrani");
    expect(await screen.findByRole("dialog")).toBeInTheDocument();
    expect(screen.queryByRole("row", { name: /Sarl Amrani/ })).not.toBeInTheDocument();

    await userEvent.keyboard("{Escape}");
    expect(await screen.findByRole("row", { name: /Sarl Amrani/ })).toBeInTheDocument();
  });
});

describe("the fiche panel", () => {
  test("a new fiche sends the fields and the opening debt in centimes", async () => {
    mount();
    await screen.findByRole("row", { name: /Sarl Amrani/ });
    await userEvent.click(screen.getByRole("button", { name: fr.suppliers_add }));

    await userEvent.type(screen.getByLabelText(fr.field_name), "Sarl Nouvelle");
    await userEvent.type(screen.getByLabelText(fr.field_phone), "0550 44 33 22");
    await userEvent.type(screen.getByLabelText(fr.field_supplier_opening_debt), "1500");
    await userEvent.click(screen.getByRole("button", { name: fr.action_save }));

    await waitFor(() => {
      const { url, body } = sent("POST");
      expect(url).toMatch(/\/suppliers$/);
      expect(body).toMatchObject({
        name: "Sarl Nouvelle",
        phone: "0550 44 33 22",
        opening_debt_centimes: 150_000,
        active: true,
      });
      // A field the fiche does not have is never invented at the edge.
      expect(body).not.toHaveProperty("credit_limit_centimes");
      expect(body).not.toHaveProperty("party_kind");
    });
  });

  test("an update sends the whole fiche, a cleared field as null, and no opening debt", async () => {
    mount();
    await openPanel("Sarl Amrani");

    await userEvent.clear(screen.getByLabelText(fr.field_phone));
    await userEvent.click(screen.getByRole("button", { name: fr.action_save }));

    await waitFor(() => {
      const { url, body } = sent("PUT");
      expect(url).toMatch(/\/suppliers\/3$/);
      expect(body).toMatchObject({ name: "Sarl Amrani", phone: null, active: true });
      expect(body).not.toHaveProperty("opening_debt_centimes");
    });
  });

  test("a name another supplier already carries is said in the shop's words", async () => {
    mount();
    await screen.findByRole("row", { name: /Sarl Amrani/ });
    await userEvent.click(screen.getByRole("button", { name: fr.suppliers_add }));
    await userEvent.type(screen.getByLabelText(fr.field_name), "Sarl Amrani");
    writeAnswer = () =>
      json(409, {
        error: { code: "conflict", message: "taken", field: "name" },
      });
    await userEvent.click(screen.getByRole("button", { name: fr.action_save }));

    expect(await screen.findByText(fr.error_supplier_name_taken)).toBeInTheDocument();
  });

  test("a name too long is said as a length, not as a name already taken", async () => {
    // Both refusals used to arrive as a validation on `name`, and a paste
    // gone wrong read as "another supplier already carries that name".
    mount();
    await screen.findByRole("row", { name: /Sarl Amrani/ });
    await userEvent.click(screen.getByRole("button", { name: fr.suppliers_add }));
    await userEvent.type(screen.getByLabelText(fr.field_name), "a".repeat(201));
    await userEvent.click(screen.getByRole("button", { name: fr.action_save }));

    expect(await screen.findByText(fr.error_name_too_long)).toBeInTheDocument();
    expect(screen.queryByText(fr.error_supplier_name_taken)).not.toBeInTheDocument();
    // And nothing was sent: the bound is the core's, and the screen holds it
    // too so a paste gone wrong does not make the round trip.
    expect(
      fetchMock.mock.calls.filter((call) => {
        const init: unknown = call[1];
        return isInit(init) && init.method === "POST";
      }),
    ).toHaveLength(0);
  });

  test("the opening debt is only on a new fiche", async () => {
    mount();
    await openPanel("Sarl Amrani");
    expect(screen.queryByLabelText(fr.field_supplier_opening_debt)).not.toBeInTheDocument();
  });
});

describe("the statement", () => {
  test("shows the movements with the balance the core computed", async () => {
    mountFiche(3);

    const opening = await screen.findByRole("row", { name: new RegExp(fr.debt_opening) });
    // The debit column and the running balance the core computed: two cells,
    // one figure, and the second is the one no screen adds up.
    expect(within(opening).getAllByText(ONE_FIVE_HUNDRED)).toHaveLength(2);
    expect(within(opening).getByText("solde de départ")).toBeInTheDocument();
  });

  test("a payment sends centimes and the mode, and what it settled comes back", async () => {
    mountFiche(3);
    await userEvent.click(await screen.findByTestId("supplier-pay-open"));

    await userEvent.type(screen.getByLabelText(fr.field_payment_amount), "1000");
    await userEvent.type(screen.getByLabelText(fr.field_payment_note), "acompte");
    await userEvent.click(screen.getByRole("button", { name: fr.payment_card }));
    await userEvent.click(screen.getByRole("button", { name: fr.action_pay_supplier }));

    await waitFor(() => {
      const { url, body } = sent("POST");
      expect(url).toMatch(/\/suppliers\/3\/payments$/);
      expect(body).toMatchObject({
        amount_centimes: 100_000,
        payment_mode: "card",
        note: "acompte",
      });
    });
    expect(await screen.findByText(fr.suppliers_paid)).toBeInTheDocument();
    // What the payment settled comes from the server's answer, not from a
    // figure the screen worked out.
    const settled = await screen.findByTestId("supplier-allocations");
    expect(within(settled).getByText(ONE_THOUSAND)).toBeInTheDocument();
  });

  test("a payment above the debt says how much is actually owed", async () => {
    mountFiche(3);
    await userEvent.click(await screen.findByTestId("supplier-pay-open"));
    writeAnswer = () =>
      json(422, {
        error: {
          code: "validation",
          message: "above",
          field: "amount_centimes",
          outstanding_centimes: 150_000,
        },
      });
    await userEvent.type(screen.getByLabelText(fr.field_payment_amount), "2000");
    await userEvent.click(screen.getByRole("button", { name: fr.action_pay_supplier }));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent(fr.error_supplier_payment_above_debt);
    expect(alert).toHaveTextContent(ONE_FIVE_HUNDRED);
    // The refused amount is still in the box and the dialog is still open:
    // the message says what is owed, and an emptied box means typing the
    // figure again to find out why.
    expect(screen.getByLabelText(fr.field_payment_amount)).toHaveValue(TWO_THOUSAND_TYPED);
  });

  test("a zero payment is refused on the screen, before the round trip", async () => {
    mountFiche(3);
    await userEvent.click(await screen.findByTestId("supplier-pay-open"));
    await userEvent.type(screen.getByLabelText(fr.field_payment_amount), "0");
    await userEvent.click(screen.getByRole("button", { name: fr.action_pay_supplier }));

    expect(await screen.findByText(fr.error_payment_amount_zero)).toBeInTheDocument();
    expect(
      fetchMock.mock.calls.filter((call) => {
        const init: unknown = call[1];
        return isInit(init) && init.method === "POST";
      }),
    ).toHaveLength(0);
  });

  test("a correction sends the signed amount and the ledger it answers is shown", async () => {
    mountFiche(3);
    await userEvent.click(await screen.findByTestId("supplier-adjust-open"));

    await userEvent.type(screen.getByLabelText(fr.field_adjust_amount), "-500");
    await userEvent.type(screen.getByLabelText(fr.field_adjust_note), "erreur de saisie");
    await userEvent.click(screen.getByRole("button", { name: fr.action_adjust }));

    await waitFor(() => {
      const { url, body } = sent("POST");
      expect(url).toMatch(/\/suppliers\/3\/adjustments$/);
      expect(body).toMatchObject({ amount_centimes: -50_000, note: "erreur de saisie" });
    });
    expect(await screen.findByText(fr.suppliers_adjusted)).toBeInTheDocument();
  });
});

describe("closing", () => {
  test("a fiche that owes something asks for a reason and sends it", async () => {
    mount();
    await openPanel("Sarl Amrani");

    await userEvent.type(screen.getByTestId("supplier-close-reason"), "le fournisseur a fermé");
    await userEvent.click(screen.getByRole("button", { name: fr.action_close_supplier }));

    await waitFor(() => {
      const { url, body } = sent("POST");
      expect(url).toMatch(/\/suppliers\/3\/close$/);
      expect(body).toEqual({ reason: "le fournisseur a fermé" });
    });
  });

  test("a reason the server asks for brings the box back on a settled fiche", async () => {
    // The balance is nil, so the screen cannot see the account is open: an
    // order still asking to be paid is a fact only the server holds.
    list = [{ ...amrani, balance_centimes: 0 }];
    mount();
    await openPanel("Sarl Amrani");
    expect(screen.queryByTestId("supplier-close-reason")).not.toBeInTheDocument();

    writeAnswer = () =>
      json(422, { error: { code: "validation", message: "reason", field: "reason" } });
    await userEvent.click(screen.getByRole("button", { name: fr.action_close_supplier }));

    expect(await screen.findByTestId("supplier-close-reason")).toBeInTheDocument();
  });

  test("a closed fiche says money still moves on it and offers to reopen", async () => {
    list = [closed];
    mount();
    await openPanel("Zoubir Fermé");

    expect(screen.getByText(fr.suppliers_closed_still_pays)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: fr.action_reopen_supplier }));
    await waitFor(() => {
      const { url, body } = sent("PUT");
      expect(url).toMatch(/\/suppliers\/5$/);
      expect(body).toMatchObject({ active: true, close_reason: null });
    });
  });
});

describe("the statement on its own page", () => {
  test("reads one supplier by id and shows the same ledger", async () => {
    mountFiche(3);
    expect(await screen.findByRole("heading", { name: fr.suppliers_ledger })).toBeInTheDocument();
    const asked = fetchMock.mock.calls.map((call) => String(call[0]));
    expect(asked.some((url) => url.endsWith("/suppliers/3"))).toBe(true);
  });

  test("a supplier this shop does not have says so rather than showing an empty fiche", async () => {
    mountFiche(404);
    expect(await screen.findByRole("alert")).toHaveTextContent(fr.error_not_found);
  });
});

describe("Arabic", () => {
  test("every string on the screen comes from the dictionary", async () => {
    mount("ar");
    expect(
      await screen.findByRole("heading", { name: ar.suppliers_title }),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: ar.suppliers_add })).toBeInTheDocument();
    expect(screen.getByLabelText(ar.suppliers_search)).toBeInTheDocument();
  });
});
