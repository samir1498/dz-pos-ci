// The shift list screen: it shows the row the API answered, it asks for
// another window rather than filtering in the browser, an empty window says
// so, and a 403 is told rather than shown a table. Whether the *row itself*
// is right and who may reach the route at all are the API crate's own tests
// (`crates/api/tests/till_api.rs`, `crates/api/tests/route_gates.rs`); this
// file only checks the screen (plan till-shifts-a-float-and-a-count T7).

import { beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ShiftDto, StaffDto } from "@dzpos/shared";
import { I18nProvider, type Lang } from "@/i18n";
import fr from "@/i18n/fr.json";
import ar from "@/i18n/ar.json";

import { TillShiftsScreen } from "../../src/routes/till_.shifts";

const staff: StaffDto[] = [
  { id: 1, name: "Amina", role: "owner", has_pin: true, has_password: false },
  { id: 2, name: "Karim", role: "cashier", has_pin: true, has_password: false },
];

function shiftDto(overrides: Partial<ShiftDto> = {}): ShiftDto {
  return {
    id: 41,
    opened_by: 2,
    opened_at: "2026-09-21 08:00:00",
    opening_cash_centimes: 500_000,
    closed_at: "2026-09-21 19:00:00",
    closed_by: 2,
    counted_centimes: 480_000,
    expected_at_close_centimes: 500_000,
    difference_centimes: -20_000,
    note: "20 000 remis au patron",
    ...overrides,
  };
}

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

let fetchMock: ReturnType<typeof vi.fn>;
let answer: ShiftDto[];
let refusal: Response | null;

beforeEach(() => {
  answer = [shiftDto()];
  refusal = null;
  fetchMock = vi.fn((input: unknown) => {
    const url = String(input);
    if (url.includes("/auth/staff")) return Promise.resolve(json(200, staff));
    if (url.includes("/till/shifts")) {
      if (refusal !== null) return Promise.resolve(refusal);
      return Promise.resolve(json(200, answer));
    }
    throw new Error(`no stub for ${url}`);
  });
  vi.stubGlobal("fetch", fetchMock);
});

/** Every address the fetch stub was asked for, in order. */
function asked(): string[] {
  return fetchMock.mock.calls.map((call) => String(call[0]));
}

function mount(lang: Lang = "fr") {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <I18nProvider lang={lang}>
      <QueryClientProvider client={client}>
        <TillShiftsScreen />
      </QueryClientProvider>
    </I18nProvider>,
  );
}

describe("rows", () => {
  test("shows the row's opener by name, its close columns and the difference", async () => {
    mount();
    const table = await screen.findByRole("table", { name: fr.till_shifts_title });
    const row = within(table).getAllByRole("row")[1];
    expect(within(row).getByText("Karim")).toBeInTheDocument();
    expect(within(row).getByText("2026-09-21 08:00:00")).toBeInTheDocument();
    expect(within(row).getByText("2026-09-21 19:00:00")).toBeInTheDocument();
    expect(within(row).getByText("20 000 remis au patron")).toBeInTheDocument();
    // The figures themselves, not just their cells: 5 000,00 opened, 4 800,00
    // counted, and a drawer 200,00 short reads negative. A column that
    // printed the shortfall unsigned, or counted where the difference goes,
    // would still have every cell in place.
    expect(row).toHaveTextContent(/5.000,00/);
    expect(row).toHaveTextContent(/4.800,00/);
    expect(within(row).getByTestId("till-shifts-difference-41")).toHaveTextContent("-200,00");
  });

  test("a fiche the staff list left out falls back to the raw id", async () => {
    answer = [shiftDto({ opened_by: 99 })];
    mount();
    const table = await screen.findByRole("table", { name: fr.till_shifts_title });
    const row = within(table).getAllByRole("row")[1];
    expect(within(row).getByText("#99")).toBeInTheDocument();
  });

  test("a shift still open shows the wording, not a null cell", async () => {
    answer = [
      shiftDto({
        closed_at: null,
        closed_by: null,
        counted_centimes: null,
        expected_at_close_centimes: null,
        difference_centimes: null,
        note: null,
      }),
    ];
    mount();
    const table = await screen.findByRole("table", { name: fr.till_shifts_title });
    const row = within(table).getAllByRole("row")[1];
    // Three cells read the same wording: closed, counted and the difference
    // all travel together on `ShiftDto` (its own doc), so all three ask the
    // same question of the same `null`.
    expect(within(row).getAllByText(fr.till_shifts_still_open)).toHaveLength(3);
  });

  test("an empty window says so", async () => {
    answer = [];
    mount();
    expect(await screen.findByText(fr.till_shifts_empty)).toBeInTheDocument();
  });

  test("a cashier who reaches the address is told, not shown a table", async () => {
    refusal = json(403, {
      error: { code: "forbidden", message: "forbidden", permission: "see_reports" },
    });
    mount();
    expect(await screen.findByRole("alert")).toHaveTextContent(fr.error_forbidden);
    expect(screen.queryByRole("table")).toBeNull();
  });

  test("the Arabic screen says the same things in Arabic", async () => {
    mount("ar");
    expect(await screen.findByRole("heading", { name: ar.till_shifts_title })).toBeInTheDocument();
  });

  test("no control on the screen edits or deletes a row", async () => {
    mount();
    await screen.findByRole("table", { name: fr.till_shifts_title });
    expect(screen.queryAllByRole("button", { name: /supprim|modifi|delete|edit/i })).toHaveLength(0);
  });
});

describe("the day window", () => {
  test("typing a from day asks the API for it", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByRole("table", { name: fr.till_shifts_title });
    await user.type(screen.getByTestId("till-shifts-filter-from-day"), "05");
    await user.type(screen.getByTestId("till-shifts-filter-from-month"), "09");
    await user.type(screen.getByTestId("till-shifts-filter-from-year"), "2026");
    await waitFor(() => {
      expect(asked().some((url) => url.includes("from=2026-09-05"))).toBe(true);
    });
  });

  test("typing a to day asks the API for it", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByRole("table", { name: fr.till_shifts_title });
    await user.type(screen.getByTestId("till-shifts-filter-to-day"), "07");
    await user.type(screen.getByTestId("till-shifts-filter-to-month"), "09");
    await user.type(screen.getByTestId("till-shifts-filter-to-year"), "2026");
    await waitFor(() => {
      expect(asked().some((url) => url.includes("to=2026-09-07"))).toBe(true);
    });
  });
});
