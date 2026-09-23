// The confirmation call's three actions (C6b): each button sends its own
// request to its own route. The "confirmed" click is covered through the
// book screen's calls list in `BookScreen.test.tsx`; the other two here.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { AppointmentDto } from "@dzpos/shared";

import { I18nProvider } from "@/i18n";

import { CallActions } from "../../../src/routes/-book/CallActions";

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

const appointment: AppointmentDto = {
  id: "a-call",
  patient_id: "p-call",
  first_name: "Amina",
  last_name: "Benali",
  starts_at: "2027-03-11 09:00:00",
  slot_minutes: 15,
  note: null,
  cancelled_at: null,
  no_show_at: null,
  phone: "0555000000",
  call_outcome: "confirmed",
  call_at: "2027-03-10 18:00:00",
};

let fetchMock: ReturnType<typeof vi.fn>;

beforeEach(() => {
  fetchMock = vi.fn(() => Promise.resolve(json(200, appointment)));
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

function mount() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <I18nProvider lang="fr">
      <QueryClientProvider client={client}>
        <CallActions appointment={appointment} />
      </QueryClientProvider>
    </I18nProvider>,
  );
}

/** Every request sent, as `METHOD path` and its body, or `null` without one. */
function sent(): [string, unknown][] {
  return fetchMock.mock.calls.map((call) => {
    const init: RequestInit | undefined = call[1];
    return [
      `${init?.method ?? "GET"} ${new URL(String(call[0]), "http://x").pathname}`,
      init?.body === undefined ? null : JSON.parse(String(init.body)),
    ];
  });
}

describe("the call actions send each outcome to its own route", () => {
  test("no answer posts that outcome to the booking's call route", async () => {
    const user = userEvent.setup();
    mount();
    await user.click(screen.getByTestId(`call-no-answer-${appointment.id}`));
    await waitFor(() =>
      expect(sent()).toEqual([
        [`POST /appointments/${appointment.id}/call-outcome`, { outcome: "no_answer" }],
      ]),
    );
  });

  test("clear posts to the clear route with no body", async () => {
    const user = userEvent.setup();
    mount();
    await user.click(screen.getByTestId(`call-clear-${appointment.id}`));
    await waitFor(() =>
      expect(sent()).toEqual([[`POST /appointments/${appointment.id}/call-outcome/clear`, null]]),
    );
  });
});
