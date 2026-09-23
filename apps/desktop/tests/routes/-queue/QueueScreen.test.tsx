// The waiting queue is checked for what it shows from `GET /queue` and for
// which route each action calls. The refusals (calling twice, seeing before
// called) are the clinic crate's own tests; this holds the wiring.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { PatientDto, QueueEntryDto } from "@dzpos/shared";

import { I18nProvider, type Lang } from "@/i18n";
import fr from "@/i18n/fr.json";
import { SessionProvider } from "@/lib/session";
import { ME_OWNER } from "@/test/session";

import { QueueScreen, queueStatus } from "../../../src/routes/-queue/QueueScreen";

const waiting: QueueEntryDto = {
  id: "q1",
  patient_id: "p1",
  first_name: "Amina",
  last_name: "Benali",
  day: "2026-09-23",
  arrived_at: "2026-09-23 09:00:00",
  called_at: null,
  seen_at: null,
  left_at: null,
};

const called: QueueEntryDto = {
  ...waiting,
  id: "q2",
  patient_id: "p2",
  first_name: "Yacine",
  last_name: "Meziane",
  called_at: "2026-09-23 09:05:00",
};

/** A second waiting arrival and a second called one, only for the tests
 *  that must prove an action reached the row it was clicked on and not the
 *  other candidate sitting beside it. */
const waiting2: QueueEntryDto = {
  ...waiting,
  id: "q3",
  patient_id: "p4",
  first_name: "Sami",
  last_name: "Ait",
  arrived_at: "2026-09-23 09:02:00",
};

const called2: QueueEntryDto = {
  ...called,
  id: "q4",
  patient_id: "p5",
  first_name: "Nawel",
  last_name: "Brahimi",
  called_at: "2026-09-23 09:06:00",
};

const patient: PatientDto = {
  id: "p3",
  first_name: "Farid",
  last_name: "Kaci",
  sex: null,
  date_of_birth: null,
  phone: "0555000000",
  archived_at: null,
  created_at: "2026-09-23 08:00:00",
  updated_at: "2026-09-23 08:00:00",
};

const patient2: PatientDto = {
  ...patient,
  id: "p6",
  first_name: "Ryad",
  last_name: "Hamdi",
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

function requestsFor(method: string): [unknown, RequestInit][] {
  return fetchMock.mock.calls.filter(
    (call): call is [unknown, RequestInit] => isInit(call[1]) && call[1].method === method,
  );
}

function postedTo(pathSuffix: string): boolean {
  return requestsFor("POST").some(([url]) => String(url).endsWith(pathSuffix));
}

/** The most recent POST and its target URL: what a row action needs
 *  checked, which id was addressed. `body` decodes what the add dialog
 *  sends; the row actions (call, seen, left, next) carry none at all
 *  (`packages/shared/src/client/queue.ts`), so `null` there is the request
 *  actually made and not a missing fixture. */
function lastPost(): { url: string; body: Record<string, unknown> | null } {
  const calls = requestsFor("POST");
  const last = calls[calls.length - 1];
  if (last === undefined) throw new Error("no POST request was made");
  const [url, init] = last;
  const body = typeof init.body === "string" ? JSON.parse(init.body) : null;
  return { url: String(url), body };
}

/** `lastPost`, for the one POST that always carries a body (adding to the
 *  queue): a `null` there is the test's own mistake. */
function lastPostBody(): Record<string, unknown> {
  const { body } = lastPost();
  if (body === null) throw new Error("the last POST request carried no body");
  return body;
}

function mount(lang: Lang = "fr") {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <I18nProvider lang={lang}>
      <QueryClientProvider client={client}>
        <SessionProvider>
          <QueueScreen />
        </SessionProvider>
      </QueryClientProvider>
    </I18nProvider>,
  );
}

let fetchMock: ReturnType<typeof vi.fn>;
let queue: QueueEntryDto[];
let patients: PatientDto[];

beforeEach(() => {
  queue = [];
  patients = [];
  fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
    const url = String(input);
    if (url.endsWith("/auth/me")) return Promise.resolve(json(200, ME_OWNER));
    if (url.endsWith("/auth/idle")) return Promise.resolve(json(200, { idle_minutes: 30 }));
    if (init?.method === "POST" && url.endsWith("/queue/next")) {
      const next = queue.find((entry) => queueStatus(entry) === "waiting");
      if (next === undefined) return Promise.resolve(json(409, { error: { code: "conflict" } }));
      const after = { ...next, called_at: "2026-09-23 09:10:00" };
      queue = queue.map((entry) => (entry.id === next.id ? after : entry));
      return Promise.resolve(json(200, after));
    }
    if (init?.method === "POST" && url.includes("/call")) {
      const id = url.split("/queue/")[1]?.split("/")[0];
      queue = queue.map((entry) =>
        entry.id === id ? { ...entry, called_at: "2026-09-23 09:10:00" } : entry,
      );
      return Promise.resolve(json(200, queue.find((entry) => entry.id === id)));
    }
    if (init?.method === "POST" && url.includes("/seen")) {
      const id = url.split("/queue/")[1]?.split("/")[0];
      queue = queue.map((entry) =>
        entry.id === id ? { ...entry, seen_at: "2026-09-23 09:15:00" } : entry,
      );
      return Promise.resolve(json(200, queue.find((entry) => entry.id === id)));
    }
    if (init?.method === "POST" && url.includes("/left")) {
      const id = url.split("/queue/")[1]?.split("/")[0];
      queue = queue.map((entry) =>
        entry.id === id ? { ...entry, left_at: "2026-09-23 09:15:00" } : entry,
      );
      return Promise.resolve(json(200, queue.find((entry) => entry.id === id)));
    }
    if (init?.method === "POST" && url.endsWith("/queue")) {
      const sent: Record<string, unknown> = JSON.parse(String(init.body));
      const added: QueueEntryDto = {
        id: "q-new",
        patient_id: String(sent.patient_id),
        first_name: patient.first_name,
        last_name: patient.last_name,
        day: "2026-09-23",
        arrived_at: "2026-09-23 09:20:00",
        called_at: null,
        seen_at: null,
        left_at: null,
      };
      queue = [...queue, added];
      return Promise.resolve(json(201, added));
    }
    if (url.endsWith("/queue")) return Promise.resolve(json(200, queue));
    if (url.includes("/patients")) return Promise.resolve(json(200, patients));
    return Promise.resolve(json(200, {}));
  });
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("queueStatus", () => {
  test("waiting, called, seen and left, read off the three stamps", () => {
    expect(queueStatus(waiting)).toBe("waiting");
    expect(queueStatus(called)).toBe("called");
    expect(queueStatus({ ...called, seen_at: "2026-09-23 09:20:00" })).toBe("seen");
    expect(queueStatus({ ...called, left_at: "2026-09-23 09:20:00" })).toBe("left");
  });
});

describe("the queue screen", () => {
  test("renders today's queue in arrival order, not sorted by name", async () => {
    // "Yacine" sorts before "Amina" were this a name sort, so the array
    // order (Amina arrived first) is the only thing that could produce
    // this order.
    queue = [waiting, called];
    mount();
    const rows = await screen.findAllByText(/^(Amina Benali|Yacine Meziane)$/);
    expect(rows.map((row) => row.textContent)).toEqual(["Amina Benali", "Yacine Meziane"]);
  });

  test("says so when there is nobody yet", async () => {
    mount();
    expect(await screen.findByText(fr.queue_empty)).toBeInTheDocument();
  });

  test("calls the next waiting patient, and only that one, not another still waiting", async () => {
    queue = [waiting, waiting2];
    const user = userEvent.setup();
    mount();
    await screen.findByText("Amina Benali");
    await user.click(screen.getByRole("button", { name: fr.queue_call_next }));
    await waitFor(() => expect(postedTo("/queue/next")).toBe(true));

    // The mock's own "next" rule ("first row still waiting") moved Amina
    // to "called"; Sami is still waiting. The screen's own row actions,
    // not the mock, are what this reads back.
    await waitFor(() => expect(screen.getByTestId(`queue-seen-${waiting.id}`)).toBeInTheDocument());
    expect(screen.getByTestId(`queue-call-${waiting2.id}`)).toBeInTheDocument();
  });

  test("the call-next button is disabled with nobody waiting", async () => {
    queue = [called];
    mount();
    await screen.findByText("Yacine Meziane");
    expect(screen.getByRole("button", { name: fr.queue_call_next })).toBeDisabled();
  });

  test("calls one particular waiting patient by their own row, not the next in line", async () => {
    queue = [waiting, waiting2];
    const user = userEvent.setup();
    mount();
    await screen.findByText("Sami Ait");
    await user.click(screen.getByTestId(`queue-call-${waiting2.id}`));
    await waitFor(() => expect(lastPost().url).toMatch(new RegExp(`/queue/${waiting2.id}/call$`)));
    expect(postedTo(`/queue/${waiting.id}/call`)).toBe(false);
  });

  test("marks the right called patient seen, not the other one waiting to be", async () => {
    queue = [called, called2];
    const user = userEvent.setup();
    mount();
    await screen.findByText("Nawel Brahimi");
    await user.click(screen.getByTestId(`queue-seen-${called2.id}`));
    await waitFor(() => expect(lastPost().url).toMatch(new RegExp(`/queue/${called2.id}/seen$`)));
    expect(postedTo(`/queue/${called.id}/seen`)).toBe(false);
  });

  test("marks the right called patient left, not the other one", async () => {
    queue = [called, called2];
    const user = userEvent.setup();
    mount();
    await screen.findByText("Nawel Brahimi");
    await user.click(screen.getByTestId(`queue-left-${called2.id}`));
    await waitFor(() => expect(lastPost().url).toMatch(new RegExp(`/queue/${called2.id}/left$`)));
    expect(postedTo(`/queue/${called.id}/left`)).toBe(false);
  });

  test("adds the patient picked, not the other candidate the search also found", async () => {
    patients = [patient, patient2];
    const user = userEvent.setup();
    mount();
    await screen.findByText(fr.queue_empty);
    await user.click(screen.getByRole("button", { name: fr.queue_add }));
    await user.type(await screen.findByTestId("queue-add-search"), "a");
    await user.click(await screen.findByTestId(`queue-add-pick-${patient2.id}`));
    await waitFor(() => expect(lastPostBody().patient_id).toBe(patient2.id));
  });
});
