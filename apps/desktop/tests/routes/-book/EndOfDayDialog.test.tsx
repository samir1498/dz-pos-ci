// End of the day (C6b): one refusal among the ticked bookings leaves the
// others marked and the refusal on screen.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { AppointmentDto } from "@dzpos/shared";

import { I18nProvider } from "@/i18n";
import fr from "@/i18n/fr.json";

import { EndOfDayDialog } from "../../../src/routes/-book/EndOfDayDialog";

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function booking(id: string, first: string, time: string): AppointmentDto {
  return {
    id,
    patient_id: `p-${id}`,
    first_name: first,
    last_name: "Test",
    starts_at: `2027-03-10 ${time}:00`,
    slot_minutes: 15,
    note: null,
    cancelled_at: null,
    no_show_at: null,
    phone: null,
    call_outcome: null,
    call_at: null,
  };
}

const amina = booking("a-amina", "Amina", "09:00");
const karim = booking("a-karim", "Karim", "10:00");

let fetchMock: ReturnType<typeof vi.fn>;

beforeEach(() => {
  fetchMock = vi.fn((input: unknown) => {
    const url = String(input);
    if (url.endsWith(`/appointments/${amina.id}/no-show`)) {
      return Promise.resolve(json(409, { error: { code: "conflict", message: "refused" } }));
    }
    if (url.endsWith(`/appointments/${karim.id}/no-show`)) {
      return Promise.resolve(json(200, { ...karim, no_show_at: "2027-03-10 18:00:00" }));
    }
    return Promise.resolve(json(200, { day: "2027-03-10", appointments: [], walk_ins: [] }));
  });
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("end of the day: a refused booking does not stop the others", () => {
  test("the first mark refused, the second is still posted and the refusal is shown", async () => {
    const onOpenChange = vi.fn();
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
    render(
      <I18nProvider lang="fr">
        <QueryClientProvider client={client}>
          <EndOfDayDialog day="2027-03-10" candidates={[amina, karim]} open onOpenChange={onOpenChange} />
        </QueryClientProvider>
      </I18nProvider>,
    );
    const user = userEvent.setup();
    await user.click(await screen.findByTestId("end-of-day-mark"));

    expect(await screen.findByText(fr.error_conflict)).toBeInTheDocument();
    const posted = fetchMock.mock.calls
      .filter((call) => {
        const init: RequestInit | undefined = call[1];
        return init?.method === "POST";
      })
      .map((call) => new URL(String(call[0]), "http://x").pathname);
    expect(posted).toEqual([`/appointments/${amina.id}/no-show`, `/appointments/${karim.id}/no-show`]);
    // A refusal keeps the dialog open for the desk to read it.
    expect(onOpenChange).not.toHaveBeenCalledWith(false);
  });
});
