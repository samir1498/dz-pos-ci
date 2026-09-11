// What the audit screen is checked for: it shows the row the API answered
// (the before and after of a change included), it asks for another page or
// another filter rather than filtering in the browser, and it offers no
// control that edits or deletes anything. Whether the *row itself* was
// written by a real service, and who may reach the route at all, are the
// API crate's own tests (`crates/api/tests/audit_log.rs`,
// `crates/api/tests/route_gates.rs`); this file only checks the screen.

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
import type { AuditLogDto } from "@dzpos/shared";
import { I18nProvider, type Lang } from "@/i18n";
import fr from "@/i18n/fr.json";
import ar from "@/i18n/ar.json";

import { AuditScreen } from "./audit";

/** The kit's select is Radix, driven by pointer capture and a resize
 *  observer jsdom has neither of (`purchases.test.tsx`'s own reasoning). */
beforeAll(() => {
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

const page: AuditLogDto = {
  rows: [
    {
      id: 41,
      user_id: 2,
      user_name: "Yasmine",
      action: "update",
      entity: "product",
      entity_id: 9,
      before: JSON.stringify({ selling_centimes: 15_000, active: true }),
      after: JSON.stringify({ selling_centimes: 18_000, active: true }),
      created_at: "2026-09-11 10:15:00",
    },
  ],
  page: 1,
  has_more: false,
  users: [
    { id: 1, name: "Anouar" },
    { id: 2, name: "Yasmine" },
  ],
  actions: ["update", "debt.pay"],
};

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

let fetchMock: ReturnType<typeof vi.fn>;
let answer: AuditLogDto | null;

beforeEach(() => {
  answer = page;
  fetchMock = vi.fn((input: unknown) => {
    const url = String(input);
    if (url.includes("/audit-log")) return Promise.resolve(json(200, answer));
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
  const rootRoute = createRootRoute();
  const route = createRoute({
    getParentRoute: () => rootRoute,
    path: "/",
    component: () => <AuditScreen />,
  });
  const router = createRouter({
    routeTree: rootRoute.addChildren([route]),
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

describe("rows", () => {
  test("shows the row's user, its kind and the field that changed, before and after", async () => {
    mount();
    const table = await screen.findByRole("table", { name: fr.audit_title });
    const row = within(table).getAllByRole("row")[1];
    expect(within(row).getByText("Yasmine")).toBeInTheDocument();
    expect(within(row).getByText("update")).toBeInTheDocument();
    expect(within(row).getByText(/product #9/)).toBeInTheDocument();
    // The field that moved is readable as before → after; `active` did not
    // change and so is left out.
    expect(within(row).getByText("selling_centimes")).toBeInTheDocument();
    expect(within(row).getByText(/150,00.*180,00/)).toBeInTheDocument();
    expect(within(row).queryByText("active")).toBeNull();
  });

  test("an empty page says so", async () => {
    answer = { ...page, rows: [] };
    mount();
    expect(await screen.findByText(fr.audit_empty)).toBeInTheDocument();
  });

  test("a manager who types the address by hand is told, not shown a table", async () => {
    fetchMock = vi.fn(() =>
      Promise.resolve(
        json(403, { error: { code: "forbidden", message: "forbidden", permission: "see_audit_log" } }),
      ),
    );
    vi.stubGlobal("fetch", fetchMock);
    mount();
    expect(await screen.findByRole("alert")).toHaveTextContent(fr.error_forbidden);
    expect(screen.queryByRole("table")).toBeNull();
  });

  test("no control on the screen edits or deletes a row", async () => {
    mount();
    await screen.findByRole("table", { name: fr.audit_title });
    expect(screen.queryAllByRole("button", { name: /supprim|modifi|delete|edit/i })).toHaveLength(0);
  });

  test("the Arabic screen says the same things in Arabic", async () => {
    mount("ar");
    expect(await screen.findByRole("heading", { name: ar.audit_title })).toBeInTheDocument();
  });
});

describe("filters", () => {
  test("choosing a user asks the API for that user alone", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByRole("table", { name: fr.audit_title });
    await pick(user, fr.audit_filter_user, "Yasmine");
    await waitFor(() => {
      expect(asked().some((url) => url.includes("user_id=2"))).toBe(true);
    });
  });

  test("choosing a kind asks the API for that action alone", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByRole("table", { name: fr.audit_title });
    await pick(user, fr.audit_filter_kind, "debt.pay");
    await waitFor(() => {
      expect(asked().some((url) => url.includes("action=debt.pay"))).toBe(true);
    });
  });

  test("typing a day asks the API for that day alone", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByRole("table", { name: fr.audit_title });
    await user.type(screen.getByTestId("audit-filter-day"), "2026-09-11");
    await waitFor(() => {
      expect(asked().some((url) => url.includes("day=2026-09-11"))).toBe(true);
    });
  });
});

describe("paging", () => {
  test("the next page button asks for page 2, and there is no page before the first", async () => {
    answer = { ...page, has_more: true };
    const user = userEvent.setup();
    mount();
    await screen.findByRole("table", { name: fr.audit_title });
    expect(screen.getByRole("button", { name: fr.audit_page_prev })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: fr.audit_page_next }));
    await waitFor(() => {
      expect(asked().some((url) => url.includes("page=2"))).toBe(true);
    });
  });

  test("the next page button is disabled once the server says there is no more", async () => {
    mount();
    await screen.findByRole("table", { name: fr.audit_title });
    expect(screen.getByRole("button", { name: fr.audit_page_next })).toBeDisabled();
  });
});
