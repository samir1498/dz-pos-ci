// The day's bookings beside the walk-in queue (C5b, C6): the two lists are
// two different fetches through one `DayListDto`, so a fixture that only
// carries appointments would let a broken walk-ins render pass unnoticed.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { DayListDto } from "@dzpos/shared";

import { I18nProvider } from "@/i18n";
import { SessionProvider } from "@/lib/session";
import { ME_OWNER } from "@/test/session";

import { DayListPanel } from "../../../src/routes/-book/DayListPanel";

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function mount() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <I18nProvider lang="fr">
      <QueryClientProvider client={client}>
        <SessionProvider>
          <DayListPanel day="2026-09-23" onDayChange={() => {}} />
        </SessionProvider>
      </QueryClientProvider>
    </I18nProvider>,
  );
}

const dayList: DayListDto = {
  day: "2026-09-23",
  appointments: [
    {
      id: "a1",
      patient_id: "p1",
      first_name: "Amina",
      last_name: "Benali",
      starts_at: "2026-09-23 09:00:00",
      slot_minutes: 15,
      note: null,
      cancelled_at: null,
      no_show_at: null,
    },
  ],
  walk_ins: [
    {
      id: "q1",
      patient_id: "p2",
      first_name: "Rachid",
      last_name: "Amrani",
      day: "2026-09-23",
      arrived_at: "2026-09-23 08:45:00",
      called_at: null,
      seen_at: null,
      left_at: null,
    },
  ],
};

let fetchMock: ReturnType<typeof vi.fn>;

beforeEach(() => {
  fetchMock = vi.fn((input: unknown) => {
    const url = String(input);
    if (url.endsWith("/auth/me")) return Promise.resolve(json(200, ME_OWNER));
    if (url.endsWith("/auth/idle")) return Promise.resolve(json(200, { idle_minutes: 30 }));
    if (url.includes("/day-list")) return Promise.resolve(json(200, dayList));
    return Promise.resolve(json(200, {}));
  });
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("DayListPanel: bookings and walk-ins side by side", () => {
  test("both lists render from the one fetch, walk-ins included", async () => {
    mount();
    const appointments = await screen.findByTestId("day-list-appointments");
    expect(appointments).toHaveTextContent("Amina Benali");

    const walkIns = screen.getByTestId("day-list-walk-ins");
    expect(walkIns).toHaveTextContent("Rachid Amrani");
  });
});
