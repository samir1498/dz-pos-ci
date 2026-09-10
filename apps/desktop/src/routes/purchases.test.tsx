// The purchases screens are checked for what they show from the API's answer
// and for what they send. The rules (the landed cost, what a delivery does to
// the stock and to the debt, which state takes a cancel) are the core's and
// the API crate's tests; what these hold is the wiring: the request body the
// order form builds, the running totals the order page shows, and the
// delivery and return forms posting to the two routes that mean opposite
// things.

import { beforeEach, describe, expect, test, vi } from "vitest";
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
import type { ProductDto, PurchaseDetailDto, SupplierDto } from "@dzpos/shared";
import { I18nProvider, type Lang } from "@/i18n";
import fr from "@/i18n/fr.json";
import ar from "@/i18n/ar.json";

import { OnePurchase } from "./purchases_.$id";
import { NewPurchaseScreen } from "./purchases_.new";

const amrani: SupplierDto = {
  id: 3,
  shop_id: 1,
  name: "Sarl Amrani",
  phone: null,
  address: null,
  rc: null,
  nif: null,
  nis: null,
  ai: null,
  notes: null,
  active: true,
  balance_centimes: 0,
};

const farine: ProductDto = {
  id: 4,
  shop_id: 1,
  name: "Farine 5kg",
  barcode: "200000000001",
  category_id: null,
  unit: "piece",
  cost_centimes: 20_000,
  selling_centimes: 26_000,
  wholesale_centimes: null,
  qty_on_hand_milli: 0,
  low_stock_at_milli: 0,
  rate_bps: 1900,
  active: true,
};

/** Ten ordered, four in, one already back, with 50 000 centimes of transport
 *  landed on the line: 20 000 a unit plus 5 000 of transport. */
const partly: PurchaseDetailDto = {
  purchase: {
    id: 8,
    shop_id: 1,
    supplier_id: 3,
    supplier_document_number: "BL-77",
    purchase_date: "2026-09-10",
    due_date: null,
    transport_centimes: 50_000,
    extra_costs_centimes: 0,
    status: "partially_received",
    user_id: 1,
    note: null,
    created_at: "2026-09-10 09:00:00",
  },
  lines: [
    {
      id: 11,
      product_id: 4,
      qty_ordered_milli: 10_000,
      unit_cost_centimes: 20_000,
      landed_unit_cost_centimes: 25_000,
      qty_received_milli: 4_000,
      qty_returned_milli: 1_000,
    },
  ],
  receipts: [
    {
      id: 2,
      series: "reception:2026",
      number: 1,
      received_at: "2026-09-10 09:05:00",
      user_id: 1,
      note: null,
      lines: [{ purchase_line_id: 11, qty_milli: 4_000 }],
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

/** The order page links back to the list and to the supplier's fiche, so it
 *  needs a router around it: a `<Link>` with no router is what the page would
 *  be if the route file were wrong. */
function mountOrder(lang: Lang = "fr") {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  const rootRoute = createRootRoute();
  const page = createRoute({
    getParentRoute: () => rootRoute,
    path: "/",
    component: () => <OnePurchase id={8} />,
  });
  const router = createRouter({
    routeTree: rootRoute.addChildren([page]),
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });
  return render(
    <I18nProvider lang={lang}>
      <QueryClientProvider client={client}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </I18nProvider>,
  );
}

let fetchMock: ReturnType<typeof vi.fn>;
let detail: PurchaseDetailDto;
let writeAnswer: (() => Response) | null;

beforeEach(() => {
  detail = partly;
  writeAnswer = null;
  fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
    const url = String(input);
    if (init?.method === "POST") {
      if (writeAnswer !== null) return Promise.resolve(writeAnswer());
      return Promise.resolve(json(201, detail));
    }
    if (url.includes("/products")) return Promise.resolve(json(200, [farine]));
    if (url.includes("/suppliers")) return Promise.resolve(json(200, [amrani]));
    if (url.includes("/clock")) return Promise.resolve(json(200, { today: "2026-09-10" }));
    if (url.includes("/purchases/8")) return Promise.resolve(json(200, detail));
    if (url.includes("/purchases")) return Promise.resolve(json(200, [detail.purchase]));
    throw new Error(`no stub for ${url}`);
  });
  vi.stubGlobal("fetch", fetchMock);
});

describe("one order", () => {
  test("the line shows what was ordered, what arrived and what went back", async () => {
    mountOrder();
    // Scoped to the lines table: the product is named again on the delivery
    // note under it, and the quantities are the same figures there.
    const table = await screen.findByRole("table", { name: fr.purchases_lines });
    const row = within(table).getAllByRole("row")[1];
    const cells = within(row).getAllByRole("cell").map((cell) => cell.textContent);
    // The running totals are the file's own columns, shown as they came, and
    // the landed cost is the core's: 200,00 plus 50,00 of transport a unit.
    expect(cells).toEqual(["Farine 5kg", "10", "4", "1", "200,00", "250,00"]);
    expect(screen.getByTestId("purchase-status")).toHaveTextContent(
      fr.purchase_status_partially_received,
    );
  });

  test("the delivery notes are listed with the number they took", async () => {
    mountOrder();
    expect(await screen.findByText(/reception:2026 \/ 1/)).toBeInTheDocument();
  });

  test("a delivery posts to the receipts route with the quantity typed", async () => {
    const user = userEvent.setup();
    mountOrder();
    const box = await screen.findByLabelText(`${fr.action_receive} 11`);
    await user.type(box, "3");
    await user.click(screen.getByRole("button", { name: fr.action_receive }));
    await waitFor(() => {
      const request = sent("POST");
      expect(request.url).toContain("/purchases/8/receipts");
      expect(request.body).toEqual({
        lines: [{ purchase_line_id: 11, qty_milli: 3_000 }],
        note: null,
      });
    });
  });

  test("a return posts to the returns route, which is the opposite direction", async () => {
    const user = userEvent.setup();
    mountOrder();
    const box = await screen.findByLabelText(`${fr.action_return} 11`);
    await user.type(box, "2");
    await user.click(screen.getByRole("button", { name: fr.action_return }));
    await waitFor(() => {
      const request = sent("POST");
      expect(request.url).toContain("/purchases/8/returns");
      expect(request.body).toEqual({
        lines: [{ purchase_line_id: 11, qty_milli: 2_000 }],
        note: null,
      });
    });
  });

  test("a partly received order offers the close short and not the cancel", async () => {
    mountOrder();
    expect(await screen.findByRole("button", { name: fr.action_close_short })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: fr.action_cancel_order })).toBeNull();
  });

  test("an order nothing arrived against offers the cancel and no return", async () => {
    detail = {
      ...partly,
      purchase: { ...partly.purchase, status: "ordered" },
      lines: [{ ...partly.lines[0], qty_received_milli: 0, qty_returned_milli: 0 }],
      receipts: [],
    };
    mountOrder();
    expect(await screen.findByRole("button", { name: fr.action_cancel_order })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: fr.action_close_short })).toBeNull();
    expect(screen.queryByRole("button", { name: fr.action_return })).toBeNull();
    expect(screen.getByText(fr.purchases_no_receipt)).toBeInTheDocument();
  });

  test("closing short sends the reason the server records", async () => {
    const user = userEvent.setup();
    mountOrder();
    const reason = await screen.findByLabelText(fr.purchases_reason);
    await user.type(reason, "le fournisseur ne livre plus");
    await user.click(screen.getByRole("button", { name: fr.action_close_short }));
    await waitFor(() => {
      const request = sent("POST");
      expect(request.url).toContain("/purchases/8/close-short");
      expect(request.body).toEqual({ reason: "le fournisseur ne livre plus" });
    });
  });

  test("a blank reason never leaves the screen", async () => {
    const user = userEvent.setup();
    mountOrder();
    await screen.findByLabelText(fr.purchases_reason);
    await user.click(screen.getByRole("button", { name: fr.action_close_short }));
    expect(await screen.findByRole("alert")).toHaveTextContent(fr.purchases_reason_needed);
    expect(() => sent("POST")).toThrow();
  });

  test("a finished order offers neither way of closing it", async () => {
    detail = {
      ...partly,
      purchase: { ...partly.purchase, status: "received" },
      lines: [{ ...partly.lines[0], qty_received_milli: 10_000, qty_returned_milli: 0 }],
    };
    mountOrder();
    expect(await screen.findByTestId("purchase-status")).toHaveTextContent(
      fr.purchase_status_received,
    );
    expect(screen.queryByRole("button", { name: fr.action_cancel_order })).toBeNull();
    expect(screen.queryByRole("button", { name: fr.action_close_short })).toBeNull();
    // Nothing left to take in, so the delivery form is gone; a return is
    // still offered, because goods on the shelf can still go back.
    expect(screen.queryByRole("button", { name: fr.action_receive })).toBeNull();
    expect(screen.getByRole("button", { name: fr.action_return })).toBeInTheDocument();
  });

  test("a refusal from the server is shown as the translated code", async () => {
    const user = userEvent.setup();
    writeAnswer = () =>
      json(422, { error: { code: "validation", message: "…", field: "qty_milli" } });
    mountOrder();
    const box = await screen.findByLabelText(`${fr.action_receive} 11`);
    await user.type(box, "9");
    await user.click(screen.getByRole("button", { name: fr.action_receive }));
    expect(await screen.findByRole("alert")).toHaveTextContent(fr.error_validation);
  });

  test("the Arabic screen says the same things in Arabic", async () => {
    mountOrder("ar");
    expect(await screen.findByTestId("purchase-status")).toHaveTextContent(
      ar.purchase_status_partially_received,
    );
    expect(screen.getByRole("button", { name: ar.action_receive })).toBeInTheDocument();
  });
});

/** The order form navigates to the order it made, so the test router carries
 *  both addresses: a navigate to a route the tree does not have is what the
 *  screen would do if the file name were wrong. */
function mountForm() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  const rootRoute = createRootRoute();
  const form = createRoute({
    getParentRoute: () => rootRoute,
    path: "/",
    component: () => <NewPurchaseScreen />,
  });
  const made = createRoute({
    getParentRoute: () => rootRoute,
    path: "/purchases/$id",
    component: () => <p>{fr.purchases_one}</p>,
  });
  const router = createRouter({
    routeTree: rootRoute.addChildren([form, made]),
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

describe("the order form", () => {
  test("sends centimes, thousandths and the shop's own day", async () => {
    const user = userEvent.setup();
    mountForm();
    // The options arrive with the two lists, so the test waits for the name
    // rather than for the label the select already carries.
    await screen.findByRole("option", { name: "Sarl Amrani" });
    await screen.findByRole("option", { name: "Farine 5kg" });
    await user.selectOptions(screen.getByLabelText(fr.col_supplier), "3");
    await user.selectOptions(screen.getByLabelText(fr.col_product), "4");
    await user.type(screen.getByLabelText(fr.col_qty), "10");
    await user.type(screen.getByLabelText(fr.col_unit_cost), "200,00");
    await user.type(screen.getByLabelText(fr.field_transport), "500,00");
    await user.type(screen.getByLabelText(fr.field_paid_now), "1000,00");
    await user.click(screen.getByRole("button", { name: fr.purchases_save }));
    await waitFor(() => {
      const request = sent("POST");
      expect(request.url).toContain("/purchases");
      expect(request.body).toEqual({
        supplier_id: 3,
        supplier_document_number: null,
        // The day is the server's, never the machine's.
        purchase_date: "2026-09-10",
        due_date: null,
        transport_centimes: 50_000,
        extra_costs_centimes: 0,
        note: null,
        lines: [{ product_id: 4, qty_ordered_milli: 10_000, unit_cost_centimes: 20_000 }],
        paid_now: { amount_centimes: 100_000, payment_mode: "cash" },
        // The goods came with the paper, which is the common case and the
        // box the form opens with.
        receive_now: true,
      });
    });
  });

  test("a half-filled line never leaves the screen", async () => {
    const user = userEvent.setup();
    mountForm();
    await screen.findByRole("option", { name: "Sarl Amrani" });
    await screen.findByRole("option", { name: "Farine 5kg" });
    await user.selectOptions(screen.getByLabelText(fr.col_supplier), "3");
    // A product and nothing else: the row was started and left half typed.
    await user.selectOptions(screen.getByLabelText(fr.col_product), "4");
    await user.click(screen.getByRole("button", { name: fr.purchases_save }));
    expect(await screen.findByRole("alert")).toHaveTextContent(fr.purchases_line_incomplete);
    expect(() => sent("POST")).toThrow();
  });

  test("an order with no supplier never leaves the screen", async () => {
    const user = userEvent.setup();
    mountForm();
    await screen.findByLabelText(fr.col_supplier);
    await user.click(screen.getByRole("button", { name: fr.purchases_save }));
    expect(await screen.findByRole("alert")).toHaveTextContent(fr.purchases_pick_supplier);
    expect(() => sent("POST")).toThrow();
  });
});
