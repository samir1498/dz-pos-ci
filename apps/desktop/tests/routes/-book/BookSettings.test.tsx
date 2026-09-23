// The book's settings room: working hours and visit types, each checked
// for the one thing that matters here, the exact payload its own save
// button sends, since the refusal path (a week with no open range, a type
// below the slot length) is the clinic crate's own test, not this screen's.
// Absence blocks moved off this room in the review round of 2026-09-23
// (`AbsenceBlocksForm.test.tsx` covers it on its own now).

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { VisitTypesDto, WorkingHoursDto } from "@dzpos/shared";

import { I18nProvider } from "@/i18n";
import { SessionProvider } from "@/lib/session";
import { ME_OWNER } from "@/test/session";

import { BookSettings } from "../../../src/routes/-book/BookSettings";

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
          <BookSettings />
        </SessionProvider>
      </QueryClientProvider>
    </I18nProvider>,
  );
}

let fetchMock: ReturnType<typeof vi.fn>;
let workingHours: WorkingHoursDto;
let visitTypes: VisitTypesDto;

function lastRequest(method: string, pathSuffix: string): Record<string, unknown> {
  const call = fetchMock.mock.calls.find(
    (c): c is [unknown, RequestInit] =>
      typeof c[1] === "object" &&
      c[1] !== null &&
      c[1].method === method &&
      String(c[0]).endsWith(pathSuffix),
  );
  if (call === undefined) throw new Error(`no ${method} to ${pathSuffix}`);
  return JSON.parse(String(call[1].body));
}

beforeEach(() => {
  workingHours = { days: [[], [{ opens: "08:00", closes: "16:00" }], [], [], [], [], []] };
  visitTypes = { visit_types: [] };

  fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
    const url = String(input);
    if (url.endsWith("/auth/me")) return Promise.resolve(json(200, ME_OWNER));
    if (url.endsWith("/auth/idle")) return Promise.resolve(json(200, { idle_minutes: 30 }));
    if (init?.method === "PUT" && url.endsWith("/settings/working-hours")) {
      return Promise.resolve(json(200, JSON.parse(String(init.body))));
    }
    if (url.endsWith("/settings/working-hours")) return Promise.resolve(json(200, workingHours));
    if (init?.method === "POST" && url.endsWith("/settings/visit-types")) {
      const sent = JSON.parse(String(init.body));
      return Promise.resolve(json(201, { id: "v-new", ...sent }));
    }
    if (url.endsWith("/settings/visit-types")) return Promise.resolve(json(200, visitTypes));
    return Promise.resolve(json(200, {}));
  });
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("working hours: save sends the whole week, Sunday first", () => {
  test("adding a range to a closed day and saving sends it in the write", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByTestId("working-hours-form");
    // Sunday (index 0) starts closed in the fixture above.
    await user.click(await screen.findByTestId("hours-0-add"));
    await user.click(screen.getByTestId("hours-save"));

    await waitFor(() => {
      const body = lastRequest("PUT", "/settings/working-hours");
      expect((body.days as unknown[][])[0]).toHaveLength(1);
    });
  });

  test("the whole week is sent as it stands, weekday by weekday, not just the day touched", async () => {
    // Two different days, one carrying a lunch break as two ranges: a test
    // that only checked the touched day's own length (as the test above
    // does) would pass even if a save quietly rewrote every other day to
    // match it, or dropped a day's second range.
    workingHours = {
      days: [
        [],
        [{ opens: "08:00", closes: "12:00" }, { opens: "14:00", closes: "18:00" }],
        [{ opens: "09:00", closes: "17:00" }],
        [],
        [],
        [],
        [],
      ],
    };
    const user = userEvent.setup();
    mount();
    await screen.findByTestId("working-hours-form");
    await user.click(screen.getByTestId("hours-save"));

    await waitFor(() => {
      const body = lastRequest("PUT", "/settings/working-hours");
      expect(body).toEqual({
        days: [
          [],
          [{ opens: "08:00", closes: "12:00" }, { opens: "14:00", closes: "18:00" }],
          [{ opens: "09:00", closes: "17:00" }],
          [],
          [],
          [],
          [],
        ],
      });
    });
  });
});

describe("visit types: create sends the name and the minutes typed", () => {
  test("posts exactly what was typed, nothing guessed", async () => {
    const user = userEvent.setup();
    mount();
    await user.type(await screen.findByTestId("visit-type-name"), "Suivi");
    const minutes = screen.getByTestId("visit-type-minutes");
    await user.clear(minutes);
    await user.type(minutes, "20");
    await user.click(screen.getByTestId("visit-type-create"));

    await waitFor(() => {
      const body = lastRequest("POST", "/settings/visit-types");
      expect(body).toEqual({ name: "Suivi", minutes: 20 });
    });
  });
});
