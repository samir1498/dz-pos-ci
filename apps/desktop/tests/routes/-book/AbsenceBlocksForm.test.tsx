// Absence blocks (C6, moved off the settings room in the review round of
// 2026-09-23: a cashier holds `edit_patients`, never `edit_settings`, and
// the server already lets her use this tool). Mounted on its own, the way
// `BookSettings.test.tsx` mounts its two remaining forms, since this one no
// longer shares a screen with them.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { AbsenceBlocksDto, AppointmentDto } from "@dzpos/shared";

import { I18nProvider } from "@/i18n";
import fr from "@/i18n/fr.json";
import { SessionProvider } from "@/lib/session";
import { ME_OWNER } from "@/test/session";

import { AbsenceBlocksForm } from "../../../src/routes/-book/AbsenceBlocksForm";

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
          <AbsenceBlocksForm />
        </SessionProvider>
      </QueryClientProvider>
    </I18nProvider>,
  );
}

const hit: AppointmentDto = {
  id: "a-hit",
  patient_id: "p-hit",
  first_name: "Nadia",
  last_name: "Cherif",
  starts_at: "2026-10-05 09:00:00",
  slot_minutes: 15,
  note: null,
  cancelled_at: null,
  no_show_at: null,
};

let fetchMock: ReturnType<typeof vi.fn>;
let absenceBlocks: AbsenceBlocksDto;
let hitsOnNextBlock: AppointmentDto[];

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
  absenceBlocks = { blocks: [] };
  hitsOnNextBlock = [];

  fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
    const url = String(input);
    if (url.endsWith("/auth/me")) return Promise.resolve(json(200, ME_OWNER));
    if (url.endsWith("/auth/idle")) return Promise.resolve(json(200, { idle_minutes: 30 }));
    if (init?.method === "POST" && url.endsWith("/absence-blocks")) {
      const sent = JSON.parse(String(init.body));
      return Promise.resolve(json(201, { block: { id: "b-new", ...sent }, hits: hitsOnNextBlock }));
    }
    if (url.endsWith("/absence-blocks")) return Promise.resolve(json(200, absenceBlocks));
    if (url.includes("/appointments/next-free")) {
      return Promise.resolve(
        json(200, { first_day: "2026-10-01", last_day: "2026-11-30", slot_minutes: 15, starts_at: null }),
      );
    }
    return Promise.resolve(json(200, {}));
  });
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("absence blocks: create sends the two stamps and the label", () => {
  test("posts starts_at, ends_at and the label typed", async () => {
    const user = userEvent.setup();
    mount();
    const starts = await screen.findByTestId("block-starts");
    const ends = screen.getByTestId("block-ends");
    await user.type(starts, "2026-10-01T08:00");
    await user.type(ends, "2026-10-01T12:00");
    await user.type(screen.getByTestId("block-label"), "Congrès");
    await user.click(screen.getByTestId("block-create"));

    await waitFor(() => {
      const body = lastRequest("POST", "/absence-blocks");
      expect(body).toEqual({
        starts_at: "2026-10-01 08:00:00",
        ends_at: "2026-10-01 12:00:00",
        label: "Congrès",
      });
    });
  });
});

describe("absence blocks: a hit with nowhere to move shows the plain message", () => {
  test("the block's own next-free search coming back empty is shown, not swallowed", async () => {
    hitsOnNextBlock = [hit];
    const user = userEvent.setup();
    mount();
    const starts = await screen.findByTestId("block-starts");
    const ends = screen.getByTestId("block-ends");
    await user.type(starts, "2026-10-05T08:00");
    await user.type(ends, "2026-10-05T18:00");
    await user.click(screen.getByTestId("block-create"));

    await user.click(await screen.findByTestId(`hit-move-${hit.id}`));

    expect(await screen.findByText(fr.book_no_free_slot)).toBeInTheDocument();
  });
});
