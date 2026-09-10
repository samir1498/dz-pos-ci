// The stock recount block on the settings screen: what it shows for a shop
// that has never recounted, what it lists after one that found something,
// and what it posts. The rules (what counts as drift, which way the
// correction goes, once a day) are the core's tests and the API crate's.

import { beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { LastStockRecountDto, StockDriftDto, StockRecountDto } from "@dzpos/shared";
import { I18nProvider } from "@/i18n";
import fr from "@/i18n/fr.json";
import { StockRecountPanel } from "./StockRecountPanel";

const drift: StockDriftDto = {
  product_id: 7,
  name: "Sucre 1kg",
  cached_milli: 99_000,
  ledger_milli: 24_000,
  difference_milli: -75_000,
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

function posts(): string[] {
  return fetchMock.mock.calls
    .filter((call) => {
      const init: unknown = call[1];
      return isInit(init) && init.method === "POST";
    })
    .map((call) => String(call[0]));
}

function mount() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <I18nProvider lang="fr">
      <QueryClientProvider client={client}>
        <StockRecountPanel />
      </QueryClientProvider>
    </I18nProvider>,
  );
}

let fetchMock: ReturnType<typeof vi.fn>;
let last: LastStockRecountDto;
let runAnswer: (() => Response) | null;

beforeEach(() => {
  last = { last_run_day: null, drifts: [] };
  runAnswer = null;
  fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
    const url = String(input);
    if (init?.method === "POST" && url.endsWith("/stock/recount")) {
      if (runAnswer !== null) return Promise.resolve(runAnswer());
      const report: StockRecountDto = {
        day: "2026-09-10",
        products_checked: 42,
        drifts: [drift],
      };
      last = { last_run_day: report.day, drifts: report.drifts };
      return Promise.resolve(json(200, report));
    }
    if (url.endsWith("/stock/recount")) return Promise.resolve(json(200, last));
    return Promise.resolve(json(404, { error: { code: "no_route", message: "no such route" } }));
  });
  vi.stubGlobal("fetch", fetchMock);
});

describe("the stock recount panel", () => {
  test("a shop that has never recounted says so and lists nothing", async () => {
    mount();
    expect(await screen.findByTestId("stock-recount-day")).toHaveTextContent(
      fr.stock_recount_never,
    );
    expect(screen.queryByTestId("stock-drift-row")).toBeNull();
    // Nothing was out is a different sentence from never having looked, and
    // a shop that has never run one must not be told its stock is right.
    expect(screen.queryByTestId("stock-recount-clean")).toBeNull();
  });

  test("a run that found nothing shows the day and says nothing was out", async () => {
    last = { last_run_day: "2026-09-09", drifts: [] };
    mount();
    expect(await screen.findByTestId("stock-recount-day")).toHaveTextContent("2026-09-09");
    expect(screen.getByTestId("stock-recount-clean")).toHaveTextContent(fr.stock_recount_none);
    expect(screen.queryByTestId("stock-drift-row")).toBeNull();
  });

  test("the last run's drifts read as quantities with the correction said once", async () => {
    last = { last_run_day: "2026-09-10", drifts: [drift] };
    mount();
    const row = await screen.findByTestId("stock-drift-row");
    expect(row).toHaveTextContent("Sucre 1kg");
    // 99000 and 24000 thousandths are 99 and 24 on the screen, and the
    // difference keeps its sign: this one took stock off the fiche.
    expect(row).toHaveTextContent("99");
    expect(row).toHaveTextContent("24");
    expect(screen.getByTestId("stock-drift-difference")).toHaveTextContent("-75");
    expect(screen.getByText(fr.stock_recount_corrected)).toBeInTheDocument();
  });

  test("a product corrected twice on one day is two rows, not one", async () => {
    // Two runs on a day read as one list, so the same product can appear
    // twice. Keyed by id alone, React dropped the second row and the panel
    // said one correction had happened where the log holds two.
    last = {
      last_run_day: "2026-09-10",
      drifts: [
        drift,
        { ...drift, cached_milli: 50_000, ledger_milli: 24_000, difference_milli: -26_000 },
      ],
    };
    // React renders both rows even when their keys clash, and only says so
    // on the console; the reconciliation goes wrong on the next render, which
    // is a refetch away. The warning is therefore the assertion.
    const complaints: unknown[][] = [];
    const consoleError = vi.spyOn(console, "error").mockImplementation((...args: unknown[]) => {
      complaints.push(args);
    });
    try {
      mount();
      expect(await screen.findAllByTestId("stock-drift-row")).toHaveLength(2);
      const differences = screen
        .getAllByTestId("stock-drift-difference")
        .map((node) => node.textContent);
      expect(differences[0]).toContain("-75");
      expect(differences[1]).toContain("-26");
      expect(complaints.map((args) => args.map(String).join(" ")).join("\n")).not.toContain(
        "same key",
      );
    } finally {
      consoleError.mockRestore();
    }
  });

  test("the button posts once and the panel then shows what the run checked", async () => {
    mount();
    await screen.findByTestId("stock-recount-day");
    await userEvent.click(screen.getByRole("button", { name: fr.action_recount_now }));

    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent(fr.stock_recount_done));
    expect(posts()).toEqual([expect.stringContaining("/stock/recount")]);
    expect(screen.getByTestId("stock-recount-checked")).toHaveTextContent("42");
    // The list is refetched from the server rather than filled in from the
    // answer: the panel shows what the file holds, not what a mutation said.
    await waitFor(() => expect(screen.getByTestId("stock-drift-row")).toBeInTheDocument());
    expect(screen.getByTestId("stock-recount-day")).toHaveTextContent("2026-09-10");
  });

  test("a refused run says so in the shop's language and keeps the last day", async () => {
    last = { last_run_day: "2026-09-09", drifts: [] };
    runAnswer = () => json(500, { error: { code: "storage", message: "the file is locked" } });
    mount();
    await screen.findByTestId("stock-recount-day");
    await userEvent.click(screen.getByRole("button", { name: fr.action_recount_now }));

    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent(fr.error_storage));
    expect(screen.queryByRole("status")).toBeNull();
    expect(screen.getByTestId("stock-recount-day")).toHaveTextContent("2026-09-09");
  });
});
