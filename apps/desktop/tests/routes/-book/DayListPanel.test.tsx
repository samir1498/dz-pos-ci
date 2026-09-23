// The day's bookings beside the day's waiting room (C5b, C6, C6b): the two
// lists come through one `DayListDto`, so a fixture that only carries
// appointments would let a broken waiting-room render pass unnoticed. C6b
// adds the arrived action and marks, read off the waiting room's links.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
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
      phone: null,
      call_outcome: null,
      call_at: null,
    },
    {
      id: "a2",
      patient_id: "p3",
      first_name: "Karim",
      last_name: "Haddad",
      starts_at: "2026-09-23 10:30:00",
      slot_minutes: 15,
      note: null,
      cancelled_at: null,
      no_show_at: null,
      phone: null,
      call_outcome: null,
      call_at: null,
    },
    {
      id: "a3",
      patient_id: "p4",
      first_name: "Sami",
      last_name: "Ait",
      starts_at: "2026-09-23 11:00:00",
      slot_minutes: 15,
      note: null,
      cancelled_at: null,
      no_show_at: null,
      phone: null,
      call_outcome: null,
      call_at: null,
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
      appointment_id: null,
      appointment_starts_at: null,
      position: 1,
    },
    {
      id: "q2",
      patient_id: "p3",
      first_name: "Karim",
      last_name: "Haddad",
      day: "2026-09-23",
      arrived_at: "2026-09-23 10:10:00",
      called_at: null,
      seen_at: null,
      left_at: null,
      appointment_id: "a2",
      appointment_starts_at: "2026-09-23 10:30:00",
      position: 2,
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

describe("DayListPanel: a booked patient comes in (C6b)", () => {
  test("a booking not yet arrived offers the action, and it posts that booking's arrive", async () => {
    const user = userEvent.setup();
    mount();
    await user.click(await screen.findByTestId("day-list-arrive-a1"));
    await waitFor(() =>
      expect(
        fetchMock.mock.calls.some(
          ([url, init]) =>
            String(url).endsWith("/appointments/a1/arrive") &&
            typeof init === "object" &&
            init !== null &&
            "method" in init &&
            init.method === "POST",
        ),
      ).toBe(true),
    );
  });

  test("an arrived booking carries the mark instead, and its patient waits with the booking's time", async () => {
    mount();
    await screen.findByTestId("day-list-arrive-a1");
    expect(screen.queryByTestId("day-list-arrive-a2")).not.toBeInTheDocument();
    const appointments = screen.getByTestId("day-list-appointments");
    expect(appointments).toHaveTextContent("Arrivé");
    const waiting = screen.getByTestId("day-list-walk-ins");
    expect(waiting).toHaveTextContent("Karim Haddad");
    expect(waiting).toHaveTextContent("RDV 10:30");
    expect(waiting).not.toHaveTextContent("RDV 08:45");
  });
});

describe("DayListPanel: the end-of-day no-shows (C6b)", () => {
  test("lists the bookings nobody marked arrived, all ticked, and marks only those still ticked", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByTestId("day-list-arrive-a1");
    await user.click(screen.getByTestId("day-list-end-of-day"));
    const dialog = await screen.findByTestId("end-of-day-dialog");
    expect(dialog).toHaveTextContent("Amina Benali");
    expect(dialog).toHaveTextContent("Sami Ait");
    expect(dialog).not.toHaveTextContent("Karim Haddad");
    expect(screen.getByTestId("end-of-day-a1")).toHaveAttribute("aria-checked", "true");
    expect(screen.getByTestId("end-of-day-a3")).toHaveAttribute("aria-checked", "true");
    // Nothing is written by opening it.
    const posts = () =>
      fetchMock.mock.calls
        .filter(([, init]) => typeof init === "object" && init !== null && "method" in init && init.method === "POST")
        .map(([url]) => String(url));
    expect(posts().filter((url) => url.includes("/no-show"))).toEqual([]);

    await user.click(screen.getByTestId("end-of-day-a3"));
    await user.click(screen.getByTestId("end-of-day-mark"));
    await waitFor(() => expect(posts().filter((url) => url.includes("/no-show"))).toHaveLength(1));
    expect(posts().filter((url) => url.includes("/no-show"))[0]).toMatch(/\/appointments\/a1\/no-show$/);
  });
});
