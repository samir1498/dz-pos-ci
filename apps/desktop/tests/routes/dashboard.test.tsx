// The dashboard is a screen that only reads, so what there is to hold here is
// the wiring: the day it asks about is the shop's and not the machine's, the
// two calls it makes carry that day and the thirty-day window, every amount on
// it is the server's integer put through `Money` rather than a sum the screen
// made, and the day/week switch reads the two arrays the one answer already
// carries instead of asking again.
//
// The figures below are deliberately not round and not equal to each other: a
// screen that put the month's amount where the day's belongs would still pass
// against a payload where they matched.

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import type {
  CashPositionDto,
  DashboardDto,
  DashboardFiguresDto,
  DashboardSeriesDto,
  DashboardSeriesPointDto,
} from "@dzpos/shared";

import { I18nProvider, type Lang } from "@/i18n";
import fr from "@/i18n/fr.json";
import { SessionProvider } from "@/lib/session";
import { ME_CASHIER, ME_OWNER } from "@/test/session";
import { DashboardScreen, wholeDinars } from "../../src/routes/dashboard";

/** What the server says the day is. A day the machine is not on, so a screen
 *  that read `new Date()` would fail here. */
const SHOP_TODAY = "2027-03-04";

function figures(sales: number, margin: number, expenses: number, count: number): DashboardFiguresDto {
  return {
    sales_ttc_centimes: sales,
    sales_count: count,
    lines_ht_centimes: sales,
    discounts_centimes: 0,
    sales_ht_centimes: sales,
    cost_of_goods_centimes: sales - margin,
    margin_centimes: margin,
    expenses_centimes: expenses,
  };
}

function cash(net: number): CashPositionDto {
  return {
    from: SHOP_TODAY,
    to: SHOP_TODAY,
    cash_in: {
      sales_centimes: net,
      stamp_centimes: 0,
      customer_payments_centimes: 0,
      total_centimes: net,
    },
    cash_out: {
      refunds_centimes: 0,
      supplier_payments_centimes: 0,
      expenses_centimes: 0,
      total_centimes: 0,
    },
    cash_centimes: net,
    card_in: {
      sales_centimes: 0,
      stamp_centimes: 0,
      customer_payments_centimes: 0,
      total_centimes: 0,
    },
  };
}

function point(from: string, to: string, sales: number): DashboardSeriesPointDto {
  return {
    from,
    to,
    figures: figures(sales, Math.trunc(sales / 5), 1_000, 1),
    cash_in_centimes: sales,
  };
}

const dashboard: DashboardDto = {
  day: SHOP_TODAY,
  month: "2027-03",
  today: figures(1_234_500, 231_100, 45_000, 17),
  this_month: figures(9_876_500, 1_812_300, 620_000, 143),
  cash_today: cash(981_200),
  cash_this_month: cash(7_412_800),
  low_stock: [
    { product_id: 3, name: "Café Bahdja 250 g", qty_on_hand_milli: 2_000, low_stock_at_milli: 10_000 },
    { product_id: 7, name: "Huile Elio 5 L", qty_on_hand_milli: 0, low_stock_at_milli: 6_000 },
  ],
  top_by_quantity: [
    {
      product_id: 3,
      name: "Café Bahdja 250 g",
      qty_milli: 184_000,
      lines_ht_centimes: 5_520_000,
      cost_of_goods_centimes: 4_600_000,
      margin_centimes: 920_000,
    },
  ],
  top_by_margin: [
    {
      product_id: 7,
      name: "Huile Elio 5 L",
      qty_milli: 41_000,
      lines_ht_centimes: 3_280_000,
      cost_of_goods_centimes: 2_100_000,
      margin_centimes: 1_180_000,
    },
  ],
  customer_debt: { total_centimes: 3_450_000, parties: 6 },
  supplier_debt: { total_centimes: 2_100_000, parties: 2 },
  open_purchases: 4,
};

const series: DashboardSeriesDto = {
  from: "2027-02-03",
  to: SHOP_TODAY,
  days: [
    point("2027-03-02", "2027-03-02", 800_000),
    point("2027-03-03", "2027-03-03", 1_100_000),
    point("2027-03-04", "2027-03-04", 1_234_500),
  ],
  weeks: [point("2027-02-26", "2027-03-04", 3_134_500)],
};

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

/** The box the stubbed ResizeObserver reports for the chart. */
const CHART_BOX = { width: 900, height: 256 } as const;

let fetchMock: ReturnType<typeof vi.fn>;
/** Set to make `/dashboard` refuse, so the error path can be driven. */
let refuse: boolean;
let me: typeof ME_OWNER | typeof ME_CASHIER;

/** Every URL asked for, in order. */
function fetched(): string[] {
  return fetchMock.mock.calls.map((call) => String(call[0]));
}

beforeEach(() => {
  refuse = false;
  me = ME_OWNER;
  // recharts measures its box with a ResizeObserver, which jsdom does not
  // implement, and draws nothing at all while it believes it is zero wide. So
  // the stub answers once with a plausible box; the numbers are the size the
  // card gives the chart on a 1280 px window and nothing here asserts on
  // geometry, only that the series, the axes and the legend were drawn.
  vi.stubGlobal(
    "ResizeObserver",
    class {
      private readonly callback: ResizeObserverCallback;
      constructor(callback: ResizeObserverCallback) {
        this.callback = callback;
      }
      observe(target: Element) {
        const box: ResizeObserverSize = { inlineSize: CHART_BOX.width, blockSize: CHART_BOX.height };
        this.callback(
          [
            {
              target,
              contentRect: new DOMRectReadOnly(0, 0, CHART_BOX.width, CHART_BOX.height),
              borderBoxSize: [box],
              contentBoxSize: [box],
              devicePixelContentBoxSize: [box],
            },
          ],
          this,
        );
      }
      unobserve() {}
      disconnect() {}
    },
  );
  fetchMock = vi.fn((input: unknown) => {
    const url = String(input);
    if (url.endsWith("/auth/me")) return Promise.resolve(json(200, me));
    if (url.endsWith("/auth/idle")) return Promise.resolve(json(200, { idle_minutes: 30 }));
    if (url.includes("/clock")) return Promise.resolve(json(200, { today: SHOP_TODAY }));
    // Before the plain dashboard branch: the series URL contains it too.
    if (url.includes("/dashboard/series")) return Promise.resolve(json(200, series));
    if (url.includes("/dashboard")) {
      return refuse
        ? Promise.resolve(json(503, { error: { code: "storage", message: "no" } }))
        : Promise.resolve(json(200, dashboard));
    }
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
        {/* An owner by default: the margin figures are what this file
            already tested before M4 T5 gated them on
            `see_cost_and_margin`. `me` is set to `ME_CASHIER` first by the
            tests that care who is signed in. */}
        <SessionProvider>
          <DashboardScreen />
        </SessionProvider>
      </QueryClientProvider>
    </I18nProvider>,
  );
}

describe("the day the screen reads", () => {
  test("is the shop's, and both calls name it", async () => {
    mount();
    await screen.findByTestId("dashboard");
    await waitFor(() => {
      expect(fetched().some((u) => u.includes(`/dashboard?day=${SHOP_TODAY}`))).toBe(true);
    });
    expect(
      fetched().some((u) => u.includes(`/dashboard/series?day=${SHOP_TODAY}&days=30`)),
    ).toBe(true);
  });
});

describe("the figure cards", () => {
  test("carry the day's amount and the month's, each where it belongs", async () => {
    mount();
    expect(await screen.findByTestId("figure-sales-today")).toHaveTextContent("12 345,00");
    expect(screen.getByTestId("figure-sales-month")).toHaveTextContent("98 765,00");
    expect(screen.getByTestId("figure-margin-today")).toHaveTextContent("2 311,00");
    expect(screen.getByTestId("figure-margin-month")).toHaveTextContent("18 123,00");
    expect(screen.getByTestId("figure-expenses-today")).toHaveTextContent("450,00");
    expect(screen.getByTestId("figure-cash-today")).toHaveTextContent("9 812,00");
    expect(screen.getByTestId("figure-cash-month")).toHaveTextContent("74 128,00");
  });

  test("say how many papers the month wrote", async () => {
    mount();
    expect(await screen.findByTestId("figure-sales-count")).toHaveTextContent("143");
    expect(screen.getByTestId("figure-sales")).toHaveTextContent(fr.dashboard_documents);
  });

  test("show both debts with the number of accounts behind each", async () => {
    mount();
    expect(await screen.findByTestId("figure-customer-debt-total")).toHaveTextContent("34 500,00");
    expect(screen.getByTestId("figure-customer-debt-parties")).toHaveTextContent("6");
    expect(screen.getByTestId("figure-supplier-debt-total")).toHaveTextContent("21 000,00");
    expect(screen.getByTestId("figure-open-purchases")).toHaveTextContent("4");
  });
});

describe("the three lists", () => {
  test("low stock names the product, what is on hand and its threshold", async () => {
    mount();
    const card = await screen.findByTestId("dashboard-low-stock");
    const table = within(card).getByRole("table");
    expect(within(table).getByText("Café Bahdja 250 g")).toBeInTheDocument();
    // Thousandths of a unit, shown as units: 2000 is "2", 10000 is "10".
    expect(within(table).getByText("2")).toBeInTheDocument();
    expect(within(table).getByText("10")).toBeInTheDocument();
    expect(within(card).getAllByTestId("low-stock-pill")).toHaveLength(2);
  });

  test("the two top tens are two lists, ranked on two different things", async () => {
    mount();
    const byQuantity = await screen.findByTestId("dashboard-top-quantity");
    expect(within(byQuantity).getByText("Café Bahdja 250 g")).toBeInTheDocument();
    expect(within(byQuantity).getByText("9 200,00")).toBeInTheDocument();
    const byMargin = screen.getByTestId("dashboard-top-margin");
    expect(within(byMargin).getByText("Huile Elio 5 L")).toBeInTheDocument();
    expect(within(byMargin).getByText("11 800,00")).toBeInTheDocument();
  });

  test("an empty top ten shows the empty state rather than an empty table", async () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    fetchMock.mockImplementation((input: unknown) => {
      const url = String(input);
      if (url.endsWith("/auth/me")) return Promise.resolve(json(200, ME_OWNER));
      if (url.endsWith("/auth/idle")) return Promise.resolve(json(200, { idle_minutes: 30 }));
      if (url.includes("/clock")) return Promise.resolve(json(200, { today: SHOP_TODAY }));
      if (url.includes("/dashboard/series")) return Promise.resolve(json(200, series));
      if (url.includes("/dashboard")) {
        return Promise.resolve(
          json(200, { ...dashboard, low_stock: [], top_by_quantity: [], top_by_margin: [] }),
        );
      }
      return Promise.resolve(json(404, { error: { code: "not_found", message: "no" } }));
    });
    render(
      <I18nProvider lang="fr">
        <QueryClientProvider client={client}>
          <SessionProvider>
            <DashboardScreen />
          </SessionProvider>
        </QueryClientProvider>
      </I18nProvider>,
    );
    expect(await screen.findByTestId("dashboard-low-stock-empty")).toHaveTextContent(
      fr.dashboard_low_stock_empty,
    );
    expect(screen.getByTestId("dashboard-top-quantity-empty")).toBeInTheDocument();
    expect(screen.getByTestId("dashboard-top-margin-empty")).toBeInTheDocument();
  });
});

describe("the chart", () => {
  test("draws the days the server folded, and the legend names the three series", async () => {
    mount();
    const chart = await screen.findByTestId("dashboard-chart");
    // Left to right on every language, the decision Money already takes.
    expect(chart).toHaveAttribute("dir", "ltr");
    const card = screen.getByTestId("dashboard-chart-card");
    expect(within(card).getByText(fr.dashboard_sales)).toBeInTheDocument();
    expect(within(card).getByText(fr.dashboard_margin)).toBeInTheDocument();
    expect(within(card).getByText(fr.dashboard_expenses)).toBeInTheDocument();
  });

  test("the week view is the array the same answer carried, so nothing is asked again", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByTestId("dashboard-chart");
    const before = fetched().length;
    await user.click(screen.getByTestId("chart-grain-weeks"));
    await waitFor(() => {
      expect(screen.getByTestId("chart-grain-weeks")).toHaveAttribute("data-state", "active");
    });
    expect(fetched().length).toBe(before);
  });
});

describe("when the server refuses", () => {
  test("the screen says what the code means and offers to ask again", async () => {
    refuse = true;
    mount();
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent(fr.error_storage);
    expect(screen.getByRole("button", { name: fr.action_retry })).toBeInTheDocument();
  });
});

describe("the chart's axis", () => {
  // The axis is a label and not a total, so it rounds to the whole dinar and
  // groups by hand rather than going through `Money`. It grouped with an
  // ordinary space until 2026-09-12, which put `1 284` on the axis against
  // `1 284,00` on the card under it: two separators for the same figure.
  test("groups with the narrow no-break space the amounts use", () => {
    expect(wholeDinars(128_400)).toBe("1\u202f284");
    expect(wholeDinars(-128_400)).toBe("-1\u202f284");
    expect(wholeDinars(99_900)).toBe("999");
  });
});
