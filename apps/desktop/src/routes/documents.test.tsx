// The documents screen is checked for what it shows from the API's answer
// and for what it sends. Every rule behind an avoir and a cancellation is
// the core's and has its tests there; what these hold is the wiring: the
// list columns, the quantities the avoir form puts on the wire, and the
// confirm naming the avoir exactly when the document carries debt.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { SaleDto } from "@dzpos/shared";
import { I18nProvider, type Lang } from "@/i18n";
import fr from "@/i18n/fr.json";
import ar from "@/i18n/ar.json";
import { DocumentsScreen } from "./documents";

const totals = {
  total_ht_centimes: 300_000,
  discount_centimes: 0,
  subtotal_ht_centimes: 300_000,
  tva_centimes: 0,
  total_ttc_centimes: 300_000,
  stamp_centimes: 0,
  net_to_pay_centimes: 300_000,
};

const facture: SaleDto = {
  id: 7,
  shop_id: 1,
  kind: "facture",
  series: "doc_facture",
  number: 4,
  printed_number: "FA-000004",
  issued_at: "2026-09-09 10:00:00",
  user_id: 1,
  regime: "reel",
  payment_mode: "credit",
  seller: {
    name: "Supérette El Bahdja",
    rc: "16/00-1234567 B 21",
    nif: null,
    nis: "098216001234567",
    ai: null,
    address: null,
    phone: null,
  },
  customer_id: 3,
  ref_document_id: null,
  buyer_name: "Entreprise Benali",
  balance: {
    old_balance_centimes: 0,
    remaining_debt_centimes: 300_000,
    total_debt_centimes: 300_000,
  },
  totals,
  tva: [],
  tendered_centimes: null,
  change_centimes: null,
  status: "issued",
  cancellation: null,
  cancel_effect: null,
  lines: [
    {
      id: 11,
      position: 0,
      product_id: 1,
      name: "Ciment CPJ 45",
      barcode: null,
      qty_milli: 3_000,
      unit_price_centimes: 100_000,
      line_discount_centimes: 0,
      rate_bps: 0,
      line_total_centimes: 300_000,
      ref_line_id: null,
    },
  ],
  warning: null,
};

/** A cash ticket: it owed nobody anything, so cancelling it is the goods and
 *  nothing else. */
const ticket: SaleDto = {
  ...facture,
  id: 8,
  kind: "ticket",
  series: "doc_ticket",
  number: 12,
  printed_number: "TK-000012",
  issued_at: "2026-09-08 16:30:00",
  payment_mode: "cash",
  customer_id: null,
  buyer_name: null,
  balance: null,
  tendered_centimes: 400_000,
  change_centimes: 100_000,
  lines: [{ ...facture.lines[0], id: 21 }],
};

/** One avoir of 500,00 already written against the facture: one unit of the
 *  three came back, so the form offers the other two. */
const avoir: SaleDto = {
  ...facture,
  id: 9,
  kind: "avoir",
  series: "doc_avoir",
  number: 1,
  printed_number: "AV-000001",
  ref_document_id: 7,
  balance: {
    old_balance_centimes: 300_000,
    remaining_debt_centimes: -100_000,
    total_debt_centimes: 200_000,
  },
  totals: { ...totals, total_ht_centimes: 100_000, net_to_pay_centimes: 100_000 },
  lines: [{ ...facture.lines[0], id: 31, qty_milli: 1_000, ref_line_id: 11 }],
};

/** The row a cell sits in. `closest` answers null when nothing matches, and
 *  a test that read through that null would fail on a property rather than on
 *  the sentence it means to check. */
function rowOf(cell: HTMLElement): HTMLElement {
  const row = cell.closest("tr");
  if (row === null) throw new Error("the cell sits in no row");
  return row;
}

/** `JSON.parse` answers `any`, which spreads through everything it touches.
 *  The return type is what turns it back into `unknown`, and it does it
 *  without an assertion. */
function parsed(body: BodyInit | null | undefined): unknown {
  return JSON.parse(String(body));
}

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

/** What the server says cancelling each document would do. The screen shows
 *  the server's answer and never re-derives it. */
let effect: (id: number) => unknown;

let list: SaleDto[];
let avoirs: SaleDto[];
let posted: { url: string; body: unknown }[];
let fetchMock: ReturnType<typeof vi.fn>;

beforeEach(() => {
  list = [facture, ticket];
  effect = (id) =>
    id === ticket.id
      ? { effect: "stock_back" }
      : { effect: "stock_back_and_avoir", amount_centimes: 300_000 };
  avoirs = [];
  posted = [];
  fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
    const url = String(input);
    if (init?.method === "POST") {
      posted.push({ url, body: parsed(init.body) });
      if (url.includes("/avoir")) return Promise.resolve(json(201, avoir));
      if (url.includes("/cancel")) {
        return Promise.resolve(
          json(200, {
            ...facture,
            status: "cancelled",
            cancellation: {
              cancelled_at: "2026-09-10 09:00:00",
              cancelled_by: 1,
              reason: "commande annulée",
              avoir_document_id: 9,
            },
          }),
        );
      }
    }
    if (url.includes("/avoirs")) return Promise.resolve(json(200, avoirs));
    if (url.includes("/facture?") || url.includes("/ticket?")) {
      return Promise.resolve(
        new Response("<html><body>FACTURE</body></html>", {
          status: 200,
          headers: { "content-type": "text/html" },
        }),
      );
    }
    // One document by id, which is the detail panel's own read.
    const byId = /\/sales\/(\d+)$/.exec(url);
    if (byId !== null) {
      const id = Number(byId[1]);
      const found = [...list, avoir].find((d) => d.id === id);
      // A read of one document carries what cancelling it would do; the list
      // does not, which is why the panel reads the document again.
      return Promise.resolve(json(200, { ...(found ?? facture), cancel_effect: effect(id) }));
    }
    if (url.includes("/sales")) {
      const kind = new URL(url).searchParams.get("kind");
      const narrowed = kind === null ? list : list.filter((d) => d.kind === kind);
      return Promise.resolve(json(200, narrowed));
    }
    return Promise.resolve(json(200, []));
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
        <DocumentsScreen />
      </QueryClientProvider>
    </I18nProvider>,
  );
}

describe("the document list", () => {
  test("lists a ticket and a facture with the columns a shop looks them up by", async () => {
    mount();
    await screen.findByText("FA-000004");
    const cells = within(rowOf(screen.getByText("FA-000004")));
    expect(cells.getByText("2026-09-09")).toBeTruthy();
    expect(cells.getByText(fr.documents_kind_facture)).toBeTruthy();
    expect(cells.getByText("Entreprise Benali")).toBeTruthy();
    expect(cells.getByText("3 000,00")).toBeTruthy();
    expect(cells.getByText(fr.documents_issued)).toBeTruthy();

    // The ticket is there too, and it names no buyer: it was sold to
    // whoever walked in.
    const till = rowOf(screen.getByText("TK-000012"));
    expect(within(till).getByText(fr.documents_kind_ticket)).toBeTruthy();
  });

  test("the kind filter narrows the call rather than the rendered list", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByText("FA-000004");
    await user.click(screen.getByLabelText(fr.documents_kind_facture));
    await waitFor(() => {
      expect(screen.queryByText("TK-000012")).toBeNull();
    });
    // The narrowing is the server's: the screen asked for one kind.
    const asked = fetchMock.mock.calls.map((c) => String(c[0]));
    expect(asked.some((u) => u.includes("kind=facture"))).toBe(true);
  });

  test("the list reads in Arabic through the same keys", async () => {
    mount("ar");
    await screen.findByText("FA-000004");
    expect(screen.getByText(ar.documents_title)).toBeTruthy();
  });
});

describe("the avoir", () => {
  test("the JSON carries the lines and the quantities typed into them", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByText("FA-000004");
    await user.click(screen.getByRole("button", { name: "FA-000004" }));
    await user.click(await screen.findByRole("button", { name: fr.documents_avoir_new }));

    // Two of the three units back.
    const qty = await screen.findByLabelText(`${fr.documents_avoir_qty} Ciment CPJ 45`);
    await user.type(qty, "2");
    await user.type(screen.getByLabelText(fr.documents_reason), "retour");
    await user.click(screen.getByRole("button", { name: fr.documents_avoir_write }));

    await waitFor(() => {
      expect(posted.some((p) => p.url.includes("/avoir"))).toBe(true);
    });
    const sent = posted.find((p) => p.url.includes("/avoir"));
    expect(sent?.body).toEqual({
      // Thousandths on the wire, the way every quantity is.
      lines: [{ document_line_id: 11, qty_milli: 2_000 }],
      reason: "retour",
    });
  });

  test("the whole button sends no lines at all, which is what asks for the rest", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByText("FA-000004");
    await user.click(screen.getByRole("button", { name: "FA-000004" }));
    await user.click(await screen.findByRole("button", { name: fr.documents_avoir_new }));
    await user.click(screen.getByRole("button", { name: fr.documents_avoir_whole }));

    await waitFor(() => {
      expect(posted.some((p) => p.url.includes("/avoir"))).toBe(true);
    });
    expect(posted.find((p) => p.url.includes("/avoir"))?.body).toEqual({
      lines: null,
      reason: null,
    });
  });

  test("the quantity a line offers is what earlier avoirs left on it", async () => {
    avoirs = [avoir];
    const user = userEvent.setup();
    mount();
    await screen.findByText("FA-000004");
    await user.click(screen.getByRole("button", { name: "FA-000004" }));
    // The avoir already written is listed under the facture.
    const listed = await screen.findByText(/AV-000001/);
    expect(listed).toBeTruthy();

    // And one of the three units having come back, the form offers two.
    await user.click(screen.getByRole("button", { name: fr.documents_avoir_new }));
    const qty = await screen.findByLabelText(`${fr.documents_avoir_qty} Ciment CPJ 45`);
    expect(qty.getAttribute("max")).toBe("2");
  });
});

describe("the cancellation", () => {
  test("the confirm names the amount coming off the account, from the server's answer", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByText("FA-000004");
    await user.click(screen.getByRole("button", { name: "FA-000004" }));
    await user.click(await screen.findByRole("button", { name: fr.documents_cancel }));
    expect(
      screen.getByText(fr.documents_cancel_with_avoir.replace("{amount}", "3 000,00")),
    ).toBeTruthy();
    expect(screen.queryByText(fr.documents_cancel_stock_only)).toBeNull();
  });

  test("a facture already credited in full says that nothing will move", async () => {
    // Every field the screen could re-derive this from says otherwise: it
    // carries debt, it was sold on credit and it names a customer.
    effect = () => ({ effect: "nothing_to_reverse" });
    const user = userEvent.setup();
    mount();
    await screen.findByText("FA-000004");
    await user.click(screen.getByRole("button", { name: "FA-000004" }));
    await user.click(await screen.findByRole("button", { name: fr.documents_cancel }));
    const said = screen.getByRole("status");
    expect(said.textContent).toBe(fr.documents_cancel_nothing);
  });

  test("a cash ticket owed nobody anything, so the confirm is the stock alone", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByText("TK-000012");
    await user.click(screen.getByRole("button", { name: "TK-000012" }));
    await user.click(await screen.findByRole("button", { name: fr.documents_cancel }));
    expect(screen.getByText(fr.documents_cancel_stock_only)).toBeTruthy();
    expect(screen.queryByText(fr.documents_cancel_with_avoir)).toBeNull();
  });

  test("the narrowed list is read again, not only the unfiltered one", async () => {
    // The list is cached under the kind it asked for, so a cancel that
    // refreshed only the "every kind" entry would leave a shop looking at
    // FA-000004 still marked issued for as long as the filter is on.
    const user = userEvent.setup();
    mount();
    await screen.findByText("FA-000004");
    await user.click(screen.getByLabelText(fr.documents_kind_facture));
    await waitFor(() => {
      expect(screen.queryByText("TK-000012")).toBeNull();
    });
    const before = fetchMock.mock.calls.filter((c) =>
      String(c[0]).includes("kind=facture"),
    ).length;

    await user.click(screen.getByRole("button", { name: "FA-000004" }));
    await user.click(await screen.findByRole("button", { name: fr.documents_cancel }));
    await user.type(screen.getByLabelText(fr.documents_reason), "commande annulée");
    await user.click(screen.getByRole("button", { name: fr.documents_cancel_confirm }));

    await waitFor(() => {
      const after = fetchMock.mock.calls.filter((c) =>
        String(c[0]).includes("kind=facture"),
      ).length;
      expect(after).toBeGreaterThan(before);
    });
  });

  test("the sheet is fetched again, because the paper it renders now says annulée", async () => {
    // The panel holds the page the core rendered. A cancellation changes that
    // page, so a cache left alone would keep showing a facture that no longer
    // exists in that form, and the shop would print it.
    const user = userEvent.setup();
    mount();
    await screen.findByText("FA-000004");
    await user.click(screen.getByRole("button", { name: "FA-000004" }));
    await screen.findByTestId("documents-sheet");
    const before = fetchMock.mock.calls.filter((c) =>
      String(c[0]).includes("/facture?"),
    ).length;
    expect(before).toBeGreaterThan(0);

    await user.click(await screen.findByRole("button", { name: fr.documents_cancel }));
    await user.type(screen.getByLabelText(fr.documents_reason), "commande annulée");
    await user.click(screen.getByRole("button", { name: fr.documents_cancel_confirm }));

    await waitFor(() => {
      const after = fetchMock.mock.calls.filter((c) =>
        String(c[0]).includes("/facture?"),
      ).length;
      expect(after).toBeGreaterThan(before);
    });
  });

  test("the reason travels and the answer's block is shown", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByText("FA-000004");
    await user.click(screen.getByRole("button", { name: "FA-000004" }));
    await user.click(await screen.findByRole("button", { name: fr.documents_cancel }));
    await user.type(screen.getByLabelText(fr.documents_reason), "commande annulée");
    await user.click(screen.getByRole("button", { name: fr.documents_cancel_confirm }));

    await waitFor(() => {
      expect(posted.some((p) => p.url.includes("/cancel"))).toBe(true);
    });
    expect(posted.find((p) => p.url.includes("/cancel"))?.body).toEqual({
      reason: "commande annulée",
    });
  });
});
