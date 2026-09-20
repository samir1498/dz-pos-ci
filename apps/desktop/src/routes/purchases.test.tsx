// The purchases screens are checked for what they show from the API's answer
// and for what they send. The rules (the landed cost, what a delivery does to
// the stock and to the debt, which state takes a cancel) are the core's and
// the API crate's tests; what these hold is the wiring: the request body the
// order form builds, the running totals the order page shows, the delivery
// and return dialogs posting to the two routes that mean opposite things, and
// the dialogs themselves opening, closing and forgetting what was typed.

import { beforeAll, beforeEach, describe, expect, test, vi } from "vitest";
import { configure, render, screen, waitFor, within } from "@testing-library/react";
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
import { SessionProvider } from "@/lib/session";
import { ME_CASHIER, ME_OWNER } from "@/test/session";

import { PurchasesScreen } from "./purchases";
import { OnePurchase } from "./purchases_.$id";
import { NewPurchaseScreen } from "./purchases_.new";

/**
 * The kit's select, dialog and their scroll lock are Radix, and Radix drives
 * them with pointer capture, element scrolling and a resize observer, none of
 * which jsdom implements. Without these four stubs a click on a select opens
 * nothing and the test reads as "the option is not there" rather than "the
 * browser this runs in has no pointer".
 *
 * They live here rather than in `src/test/setup.ts` because nine screens are
 * being rewritten at the same time and one shared file is one conflict; the
 * report says they belong there once.
 */
beforeAll(() => {
  // Nine worktrees build and test on this box at once, and the default second
  // is not enough for a query to answer under that load: the failure then
  // reads as "the table is not there" rather than "the machine was busy".
  configure({ asyncUtilTimeout: 5_000 });
  Element.prototype.scrollIntoView = () => {};
  Element.prototype.hasPointerCapture = () => false;
  Element.prototype.setPointerCapture = () => {};
  Element.prototype.releasePointerCapture = () => {};
  globalThis.ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  };
});

/** Open a kit select by its accessible name and pick the option named. */
async function pick(
  user: ReturnType<typeof userEvent.setup>,
  select: string,
  option: string,
): Promise<void> {
  await user.click(await screen.findByRole("combobox", { name: select }));
  await user.click(await screen.findByRole("option", { name: option }));
}

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
 *  landed on the line: 20 000 a unit plus 5 000 of transport.
 *
 *  `extras_centimes` is 62 500 on purpose, which is not this order's transport
 *  plus its extra costs. The API answers that field and the two screens print
 *  it; a screen adding the two columns itself would print 500,00 here, and the
 *  two assertions on 625,00 below are what catches it. */
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
    extras_centimes: 62_500,
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

/** Every address the fetch stub was asked for, in order. */
function asked(): string[] {
  return fetchMock.mock.calls.map((call) => String(call[0]));
}

function wrap(node: React.ReactNode, lang: Lang) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <I18nProvider lang={lang}>
      <QueryClientProvider client={client}>
        {/* An owner by default: the cost columns are what most of this
            file already tested before M4 T5 gated them on
            `see_cost_and_margin`. `me` is set to `ME_CASHIER` first by the
            one test that cares who is signed in. */}
        <SessionProvider>{node}</SessionProvider>
      </QueryClientProvider>
    </I18nProvider>,
  );
}

/** The order page links back to the list and to the supplier's fiche, so it
 *  needs a router around it: a `<Link>` with no router is what the page would
 *  be if the route file were wrong. */
function mountOrder(lang: Lang = "fr") {
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
  return wrap(<RouterProvider router={router} />, lang);
}

/** The list links to the form and to each order, so it needs a router too. */
function mountList(lang: Lang = "fr") {
  const rootRoute = createRootRoute();
  const page = createRoute({
    getParentRoute: () => rootRoute,
    path: "/",
    component: () => <PurchasesScreen />,
  });
  const router = createRouter({
    routeTree: rootRoute.addChildren([page]),
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });
  return wrap(<RouterProvider router={router} />, lang);
}

let fetchMock: ReturnType<typeof vi.fn>;
let detail: PurchaseDetailDto;
let list: PurchaseDetailDto["purchase"][] | null;
let writeAnswer: (() => Response) | null;
let me: typeof ME_OWNER | typeof ME_CASHIER;

beforeEach(() => {
  detail = partly;
  list = null;
  writeAnswer = null;
  me = ME_OWNER;
  fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
    const url = String(input);
    if (url.endsWith("/auth/me")) return Promise.resolve(json(200, me));
    if (url.endsWith("/auth/idle")) return Promise.resolve(json(200, { idle_minutes: 30 }));
    if (init?.method === "POST") {
      if (writeAnswer !== null) return Promise.resolve(writeAnswer());
      return Promise.resolve(json(201, detail));
    }
    if (url.includes("/products")) return Promise.resolve(json(200, [farine]));
    if (url.includes("/suppliers")) return Promise.resolve(json(200, [amrani]));
    if (url.includes("/clock")) return Promise.resolve(json(200, { today: "2026-09-10" }));
    if (url.includes("/purchases/8")) return Promise.resolve(json(200, detail));
    if (url.includes("/purchases")) {
      return Promise.resolve(json(200, list ?? [detail.purchase]));
    }
    throw new Error(`no stub for ${url}`);
  });
  vi.stubGlobal("fetch", fetchMock);
});

describe("the list of orders", () => {
  test("shows the order's day, its extra costs and the state it is in", async () => {
    mountList();
    const table = await screen.findByRole("table", { name: fr.purchases_title });
    const row = within(table).getAllByRole("row")[1];
    const cells = within(row).getAllByRole("cell").map((cell) => cell.textContent);
    // Six cells: the five columns and the one the row's own action sits in.
    expect(cells).toEqual([
      "2026-09-10",
      "Sarl Amrani",
      "BL-77",
      // The extras the API answered, not the two columns added here.
      "625,00",
      fr.purchase_status_partially_received,
      fr.purchases_open,
    ]);
  });

  test("hides the extra-costs column from a cashier, who does not hold see_cost_and_margin", async () => {
    me = ME_CASHIER;
    mountList();
    const table = await screen.findByRole("table", { name: fr.purchases_title });
    expect(within(table).queryByRole("columnheader", { name: fr.col_extra_costs })).not.toBeInTheDocument();
  });

  test("an empty list says so and offers the first order", async () => {
    list = [];
    mountList();
    expect(await screen.findByText(fr.purchases_empty)).toBeInTheDocument();
    expect(screen.getAllByRole("link", { name: new RegExp(fr.purchases_add) }).length)
      .toBeGreaterThan(0);
  });

  test("choosing a state asks the API for that state alone", async () => {
    const user = userEvent.setup();
    mountList();
    await screen.findByRole("table", { name: fr.purchases_title });
    await pick(user, fr.purchases_filter_status, fr.purchase_status_received);
    await waitFor(() => {
      expect(asked().some((url) => url.includes("status=received"))).toBe(true);
    });
  });
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

  test("hides the cost columns and the extra-costs fact from a cashier", async () => {
    me = ME_CASHIER;
    mountOrder();
    const table = await screen.findByRole("table", { name: fr.purchases_lines });
    expect(within(table).queryByRole("columnheader", { name: fr.col_unit_cost })).not.toBeInTheDocument();
    expect(within(table).queryByRole("columnheader", { name: fr.col_landed_cost })).not.toBeInTheDocument();
    expect(screen.queryByText(fr.col_extra_costs)).not.toBeInTheDocument();
  });

  /** The counterpart of the list's own extras cell. Both screens showed the
   *  transport plus the extra costs added in JSX until T9; the stub answers a
   *  figure that is neither column nor their sum, so a screen that went back to
   *  adding them reads 500,00 here and this fails. */
  test("the extra costs are the figure the API answered", async () => {
    mountOrder();
    const label = await screen.findByText(fr.col_extra_costs);
    // `Fact` renders <dt>{label}</dt><dd>{children}</dd>, so the amount is the
    // element right after the label.
    expect(label.nextElementSibling).toHaveTextContent("625,00");
  });

  test("the delivery notes are listed with the number they took", async () => {
    mountOrder();
    expect(await screen.findByText(/reception:2026 \/ 1/)).toBeInTheDocument();
  });

  test("a delivery posts to the receipts route with the quantity typed", async () => {
    const user = userEvent.setup();
    mountOrder();
    await user.click(await screen.findByRole("button", { name: fr.purchases_receive }));
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
    // The dialog closes on its own once the server has taken it.
    await waitFor(() => {
      expect(screen.queryByRole("dialog")).toBeNull();
    });
  });

  test("a return posts to the returns route, which is the opposite direction", async () => {
    const user = userEvent.setup();
    mountOrder();
    await user.click(await screen.findByRole("button", { name: fr.purchases_return }));
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

  test("a dialog closed with Escape sends nothing and forgets what was typed", async () => {
    const user = userEvent.setup();
    mountOrder();
    await user.click(await screen.findByRole("button", { name: fr.purchases_receive }));
    await user.type(await screen.findByLabelText(`${fr.action_receive} 11`), "3");
    await user.keyboard("{Escape}");
    await waitFor(() => {
      expect(screen.queryByRole("dialog")).toBeNull();
    });
    expect(() => sent("POST")).toThrow();
    await user.click(screen.getByRole("button", { name: fr.purchases_receive }));
    expect(await screen.findByLabelText(`${fr.action_receive} 11`)).toHaveValue("");
  });

  test("a partly received order offers the close short and not the cancel", async () => {
    mountOrder();
    expect(await screen.findByRole("button", { name: fr.purchases_close_short })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: fr.purchases_cancel })).toBeNull();
  });

  test("an order nothing arrived against offers the cancel and no return", async () => {
    detail = {
      ...partly,
      purchase: { ...partly.purchase, status: "ordered" },
      lines: [{ ...partly.lines[0], qty_received_milli: 0, qty_returned_milli: 0 }],
      receipts: [],
    };
    mountOrder();
    expect(await screen.findByRole("button", { name: fr.purchases_cancel })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: fr.purchases_close_short })).toBeNull();
    expect(screen.queryByRole("button", { name: fr.purchases_return })).toBeNull();
    expect(screen.getByText(fr.purchases_no_receipt)).toBeInTheDocument();
  });

  test("closing short sends the reason the server records", async () => {
    const user = userEvent.setup();
    mountOrder();
    await user.click(await screen.findByRole("button", { name: fr.purchases_close_short }));
    const dialog = await screen.findByRole("dialog");
    // By role and not by label text: the field is required, so its label
    // carries the star, and the star is out of the accessible name.
    await user.type(
      within(dialog).getByRole("textbox", { name: fr.purchases_reason }),
      "le fournisseur ne livre plus",
    );
    await user.click(within(dialog).getByRole("button", { name: fr.action_close_short }));
    await waitFor(() => {
      const request = sent("POST");
      expect(request.url).toContain("/purchases/8/close-short");
      expect(request.body).toEqual({ reason: "le fournisseur ne livre plus" });
    });
  });

  test("a blank reason never leaves the dialog", async () => {
    const user = userEvent.setup();
    mountOrder();
    await user.click(await screen.findByRole("button", { name: fr.purchases_close_short }));
    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: fr.action_close_short }));
    expect(await within(dialog).findByRole("alert")).toHaveTextContent(fr.purchases_reason_needed);
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
    expect(screen.queryByRole("button", { name: fr.purchases_cancel })).toBeNull();
    expect(screen.queryByRole("button", { name: fr.purchases_close_short })).toBeNull();
    // Nothing left to take in, so the delivery dialog is gone; a return is
    // still offered, because goods on the shelf can still go back.
    expect(screen.queryByRole("button", { name: fr.purchases_receive })).toBeNull();
    expect(screen.getByRole("button", { name: fr.purchases_return })).toBeInTheDocument();
  });

  test("a refusal from the server is shown as the translated code", async () => {
    const user = userEvent.setup();
    writeAnswer = () =>
      json(422, { error: { code: "validation", message: "…", field: "qty_milli" } });
    mountOrder();
    await user.click(await screen.findByRole("button", { name: fr.purchases_receive }));
    const dialog = await screen.findByRole("dialog");
    await user.type(within(dialog).getByLabelText(`${fr.action_receive} 11`), "9");
    await user.click(within(dialog).getByRole("button", { name: fr.action_receive }));
    expect(await within(dialog).findByRole("alert")).toHaveTextContent(fr.error_validation);
    // Refused, so the dialog stays open with the figures still in it.
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  test("the Arabic screen says the same things in Arabic", async () => {
    mountOrder("ar");
    expect(await screen.findByTestId("purchase-status")).toHaveTextContent(
      ar.purchase_status_partially_received,
    );
    expect(screen.getByRole("button", { name: ar.purchases_receive })).toBeInTheDocument();
  });
});

/** The order form navigates to the order it made, so the test router carries
 *  both addresses: a navigate to a route the tree does not have is what the
 *  screen would do if the file name were wrong. */
function mountForm() {
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
  return wrap(<RouterProvider router={router} />, "fr");
}

describe("the order form", () => {
  test("sends centimes, thousandths and the shop's own day", async () => {
    const user = userEvent.setup();
    mountForm();
    // The two lists arrive with the screen, so the option is waited for
    // inside the select rather than in the closed markup, where Radix keeps
    // nothing until it is opened.
    await pick(user, fr.col_supplier, "Sarl Amrani");
    await pick(user, fr.col_product, "Farine 5kg");
    await user.type(screen.getByRole("textbox", { name: fr.col_qty }), "10");
    await user.type(screen.getByRole("textbox", { name: fr.col_unit_cost }), "200,00");
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

  /** The mode the form opens with is cash, which the test above pins. This
   *  is the other one, and it is the only place any test opens this screen's
   *  payment list: the two names in it come from `lib/payment.ts`, shared
   *  with the customers fiche and the supplier statement since the suppliers
   *  screen split, so a wrong name there is a wrong name on three screens. */
  test("a purchase paid by card sends the mode the buyer picked", async () => {
    const user = userEvent.setup();
    mountForm();
    await pick(user, fr.col_supplier, "Sarl Amrani");
    await pick(user, fr.col_product, "Farine 5kg");
    await user.type(screen.getByRole("textbox", { name: fr.col_qty }), "10");
    await user.type(screen.getByRole("textbox", { name: fr.col_unit_cost }), "200,00");
    await user.type(screen.getByLabelText(fr.field_paid_now), "1000,00");
    await pick(user, fr.field_payment_mode, fr.payment_card);
    await user.click(screen.getByRole("button", { name: fr.purchases_save }));
    await waitFor(() => {
      expect(sent("POST").body).toMatchObject({
        paid_now: { amount_centimes: 100_000, payment_mode: "card" },
      });
    });
  });

  test("a half-filled line never leaves the screen", async () => {
    const user = userEvent.setup();
    mountForm();
    await pick(user, fr.col_supplier, "Sarl Amrani");
    // A product and nothing else: the row was started and left half typed.
    await pick(user, fr.col_product, "Farine 5kg");
    await user.click(screen.getByRole("button", { name: fr.purchases_save }));
    expect(await screen.findByRole("alert")).toHaveTextContent(fr.purchases_line_incomplete);
    expect(() => sent("POST")).toThrow();
  });

  test("an order with no supplier never leaves the screen", async () => {
    const user = userEvent.setup();
    mountForm();
    await screen.findByRole("combobox", { name: fr.col_supplier });
    await user.click(screen.getByRole("button", { name: fr.purchases_save }));
    expect(await screen.findByRole("alert")).toHaveTextContent(fr.purchases_pick_supplier);
    expect(() => sent("POST")).toThrow();
  });

  test("a second line is added and taken away again", async () => {
    const user = userEvent.setup();
    mountForm();
    await screen.findByRole("combobox", { name: fr.col_supplier });
    expect(screen.getAllByRole("textbox", { name: fr.col_qty })).toHaveLength(1);
    await user.click(screen.getByRole("button", { name: fr.purchases_add_line }));
    expect(screen.getAllByRole("textbox", { name: fr.col_qty })).toHaveLength(2);
    await user.click(screen.getByRole("button", { name: `${fr.purchases_remove_line} 2` }));
    expect(screen.getAllByRole("textbox", { name: fr.col_qty })).toHaveLength(1);
  });
});
