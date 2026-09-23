// The book screen wraps FullCalendar, which jsdom cannot lay out (no real
// grid, no real hit-testing), so `@fullcalendar/react` is replaced with a
// stub that renders the props BookScreen handed it as plain buttons and
// text: one button plays a slot click, one button per appointment plays an
// event click, and the props FullCalendar would have read (`firstDay`,
// `hiddenDays`, `direction`) are printed so a test can read them back
// without ever asking a browser to lay out a week grid.

import { afterEach, beforeEach, describe, expect, test, vi, type Mock } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  RouterProvider,
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
} from "@tanstack/react-router";
import type {
  AppointmentDto,
  MeDto,
  PatientDto,
  VisitTypeDto,
  WorkingHoursDto,
} from "@dzpos/shared";

import type { ReactNode } from "react";

import { I18nProvider, type Lang } from "@/i18n";
import fr from "@/i18n/fr.json";
import { SessionProvider } from "@/lib/session";
import { ME_CASHIER, ME_OWNER } from "@/test/session";

interface StubCalendarProps {
  firstDay?: number;
  hiddenDays?: readonly number[];
  direction?: string;
  locale?: string;
  events?: readonly {
    id: string;
    title: string;
    start: Date;
    display?: string;
    extendedProps: { appointment?: AppointmentDto };
  }[];
  dateClick?: (info: { date: Date }) => void;
  eventClick?: (info: { event: { id: string; start: Date; extendedProps: unknown } }) => void;
  eventDrop?: (info: {
    event: { id: string; start: Date; extendedProps: unknown };
    revert: () => void;
  }) => void;
  datesSet?: (info: { start: Date; end: Date }) => void;
  eventContent?: (info: {
    event: { title: string; extendedProps: { appointment?: AppointmentDto } };
  }) => ReactNode;
}

let lastCalendarProps: StubCalendarProps | null = null;
/** The drag button below hands this to `eventDrop` on every click: a test
 *  that wants to know whether a refusal reverted the drag asserts on this
 *  directly, rather than trusting jsdom to have actually moved anything
 *  (it never did) and inferring a revert from that. */
let dragRevert: Mock<() => void>;

vi.mock("@fullcalendar/react", () => {
  return {
    default: (props: StubCalendarProps) => {
      lastCalendarProps = props;
      const visible = (props.events ?? []).filter((event) => event.display !== "background");
      return (
        <div data-testid="fc-stub">
          <span data-testid="fc-firstDay">{String(props.firstDay)}</span>
          <span data-testid="fc-hiddenDays">{JSON.stringify(props.hiddenDays ?? [])}</span>
          <span data-testid="fc-direction">{props.direction}</span>
          <span data-testid="fc-locale">{props.locale}</span>
          <button
            type="button"
            data-testid="fc-slot"
            onClick={() => props.dateClick?.({ date: new Date(2026, 8, 23, 9, 0, 0) })}
          >
            slot
          </button>
          {visible.map((event) => (
            <span key={event.id}>
              <button
                type="button"
                data-testid={`fc-event-${event.id}`}
                onClick={() =>
                  props.eventClick?.({
                    event: { id: event.id, start: event.start, extendedProps: event.extendedProps },
                  })
                }
              >
                {event.title}
              </button>
              <button
                type="button"
                data-testid={`fc-drag-${event.id}`}
                onClick={() =>
                  props.eventDrop?.({
                    event: {
                      id: event.id,
                      start: new Date(2026, 8, 23, 11, 0, 0),
                      extendedProps: event.extendedProps,
                    },
                    revert: dragRevert,
                  })
                }
              >
                drag
              </button>
            </span>
          ))}
        </div>
      );
    },
  };
});

vi.mock("@fullcalendar/daygrid", () => ({ default: {} }));
vi.mock("@fullcalendar/timegrid", () => ({ default: {} }));
vi.mock("@fullcalendar/interaction", () => ({ default: {} }));

import { BookScreen } from "../../../src/routes/-book/BookScreen";

const patient: PatientDto = {
  id: "p1",
  first_name: "Amina",
  last_name: "Benali",
  sex: null,
  date_of_birth: null,
  phone: "0555000000",
  archived_at: null,
  created_at: "2026-09-23 08:00:00",
  updated_at: "2026-09-23 08:00:00",
};

const appointment: AppointmentDto = {
  id: "a1",
  patient_id: "p1",
  first_name: "Amina",
  last_name: "Benali",
  starts_at: "2026-09-23 10:00:00",
  slot_minutes: 15,
  note: null,
  cancelled_at: null,
  no_show_at: null,
  phone: null,
  call_outcome: null,
  call_at: null,
};

const otherAppointment: AppointmentDto = {
  id: "a2",
  patient_id: "p2",
  first_name: "Karim",
  last_name: "Ferhat",
  starts_at: "2026-09-23 11:00:00",
  slot_minutes: 15,
  note: null,
  cancelled_at: null,
  no_show_at: null,
  phone: null,
  call_outcome: null,
  call_at: null,
};

const closedFridaySaturday: WorkingHoursDto = {
  days: [
    [{ opens: "08:00", closes: "16:00" }], // Sunday
    [{ opens: "08:00", closes: "16:00" }], // Monday
    [{ opens: "08:00", closes: "16:00" }], // Tuesday
    [{ opens: "08:00", closes: "16:00" }], // Wednesday
    [{ opens: "08:00", closes: "16:00" }], // Thursday
    [], // Friday
    [], // Saturday
  ],
};

const visitTypes: VisitTypeDto[] = [{ id: "v1", name: "Consultation", minutes: 15 }];

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function Stub() {
  return <p>stub</p>;
}

/** A memory router, the shape `settings.about.test.tsx` uses: the screen
 *  builds real `Link`s (to `/patients`, `/settings/book`) and a `Link`
 *  without a router is a screen that cannot render. */
function mount(lang: Lang = "fr", me: MeDto = ME_OWNER) {
  sessionMe = me;
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  const rootRoute = createRootRoute();
  const bookRoute = createRoute({ getParentRoute: () => rootRoute, path: "/book", component: BookScreen });
  const patientsRoute = createRoute({ getParentRoute: () => rootRoute, path: "/patients", component: Stub });
  const settingsBookRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/settings/book",
    component: Stub,
  });
  const router = createRouter({
    routeTree: rootRoute.addChildren([bookRoute, patientsRoute, settingsBookRoute]),
    history: createMemoryHistory({ initialEntries: ["/book"] }),
  });
  return render(
    <I18nProvider lang={lang}>
      <QueryClientProvider client={client}>
        <SessionProvider>
          <RouterProvider router={router} />
        </SessionProvider>
      </QueryClientProvider>
    </I18nProvider>,
  );
}

let fetchMock: ReturnType<typeof vi.fn>;
let appointmentsResponse: AppointmentDto[];
let bookResponse: { status: number; body: unknown } | null;
let patients: PatientDto[];
let sessionMe: MeDto = ME_OWNER;
let nextFreeResponse: { first_day: string; last_day: string; slot_minutes: number; starts_at: string | null };
/** The shop's `/clock` today; a test may move it far from the run date. */
let shopToday: string;

beforeEach(() => {
  shopToday = "2026-09-23";
  lastCalendarProps = null;
  appointmentsResponse = [];
  patients = [patient];
  bookResponse = null;
  sessionMe = ME_OWNER;
  dragRevert = vi.fn();
  nextFreeResponse = {
    first_day: "2026-09-23",
    last_day: "2026-11-21",
    slot_minutes: 15,
    starts_at: null,
  };

  fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
    const url = String(input);
    if (url.endsWith("/auth/me")) return Promise.resolve(json(200, sessionMe));
    if (url.endsWith("/auth/idle")) return Promise.resolve(json(200, { idle_minutes: 30 }));
    if (url.endsWith("/clock")) return Promise.resolve(json(200, { today: shopToday }));
    if (url.endsWith("/settings/working-hours")) return Promise.resolve(json(200, closedFridaySaturday));
    if (url.endsWith("/absence-blocks")) return Promise.resolve(json(200, { blocks: [] }));
    if (url.endsWith("/settings/slot-minutes")) return Promise.resolve(json(200, { slot_minutes: 15 }));
    if (url.endsWith("/settings/visit-types")) {
      return Promise.resolve(json(200, { visit_types: visitTypes }));
    }
    if (url.includes("/day-list")) {
      return Promise.resolve(json(200, { day: "2026-09-23", appointments: [], walk_ins: [] }));
    }
    if (url.includes("/appointments/next-free")) {
      return Promise.resolve(json(200, nextFreeResponse));
    }
    if (init?.method === "POST" && url.endsWith("/appointments")) {
      if (bookResponse !== null) return Promise.resolve(json(bookResponse.status, bookResponse.body));
      const sent: Record<string, unknown> = JSON.parse(String(init.body));
      const made: AppointmentDto = {
        id: "a-new",
        patient_id: String(sent.patient_id),
        first_name: patient.first_name,
        last_name: patient.last_name,
        starts_at: String(sent.starts_at),
        slot_minutes: 15,
        note: (sent.note as string | null | undefined) ?? null,
        cancelled_at: null,
        no_show_at: null,
        phone: null,
        call_outcome: null,
        call_at: null,
      };
      return Promise.resolve(json(201, made));
    }
    if (url.includes("/patients")) return Promise.resolve(json(200, patients));
    if (url.includes("/appointments")) {
      return Promise.resolve(
        json(200, { from: "2026-09-20", to: "2026-09-26", appointments: appointmentsResponse }),
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

function requestsFor(method: string): [unknown, RequestInit][] {
  return fetchMock.mock.calls.filter(
    (call): call is [unknown, RequestInit] =>
      typeof call[1] === "object" && call[1] !== null && call[1].method === method,
  );
}

describe("the book screen: the week starts on Sunday", () => {
  test("firstDay is 0 and Friday/Saturday are hidden from the working hours", async () => {
    mount();
    await waitFor(() => expect(lastCalendarProps).not.toBeNull());
    expect(await screen.findByTestId("fc-firstDay")).toHaveTextContent("0");
    await waitFor(() =>
      expect(screen.getByTestId("fc-hiddenDays")).toHaveTextContent(JSON.stringify([5, 6])),
    );
  });

  test("Arabic sets the grid to rtl", async () => {
    mount("ar");
    await waitFor(() => expect(screen.getByTestId("fc-direction")).toHaveTextContent("rtl"));
  });

  test("French carries its own calendar locale, not the English default", async () => {
    mount("fr");
    await waitFor(() => expect(screen.getByTestId("fc-locale")).toHaveTextContent("fr"));
  });
});

describe("the book screen: booking through the click flow", () => {
  test("clicking an empty slot and picking a patient books at that exact slot", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByTestId("fc-slot");
    await user.click(screen.getByTestId("fc-slot"));

    await screen.findByTestId("book-dialog");
    await user.type(await screen.findByTestId("book-search"), "a");
    await user.click(await screen.findByTestId(`book-pick-${patient.id}`));
    // Picking a visit type is optional (the plan itself says so); this test
    // asks for the one case that proves the pick actually reaches the
    // server, not the always-passing case where it is left at "none" and a
    // dropped `visit_type_id` would go unnoticed.
    await user.click(screen.getByRole("combobox", { name: fr.book_visit_type }));
    await user.click(await screen.findByRole("option", { name: visitTypes[0].name }));
    await user.click(await screen.findByTestId("book-submit"));

    await waitFor(() => expect(requestsFor("POST")).not.toHaveLength(0));
    const posted = requestsFor("POST").find(([url]) => String(url).endsWith("/appointments"));
    expect(posted).toBeDefined();
    const body: Record<string, unknown> = posted === undefined ? {} : JSON.parse(String(posted[1].body));
    expect(body.patient_id).toBe(patient.id);
    expect(body.starts_at).toBe("2026-09-23 09:00:00");
    expect(body.note).toBeNull();
    expect(body.visit_type_id).toBe(visitTypes[0].id);
  });

  test("a 409 (the slot was taken meanwhile) shows the plain refusal, not a silent failure", async () => {
    bookResponse = {
      status: 409,
      body: { error: { code: "conflict", message: "already taken" } },
    };
    const user = userEvent.setup();
    mount();
    await user.click(await screen.findByTestId("fc-slot"));
    await user.click(await screen.findByTestId(`book-pick-${patient.id}`));
    await user.click(await screen.findByTestId("book-submit"));

    expect(await screen.findByText(fr.error_conflict)).toBeInTheDocument();
  });

  test("a 422 (a start off the grid) shows the plain refusal", async () => {
    bookResponse = {
      status: 422,
      body: { error: { code: "validation", message: "not a slot boundary" } },
    };
    const user = userEvent.setup();
    mount();
    await user.click(await screen.findByTestId("fc-slot"));
    await user.click(await screen.findByTestId(`book-pick-${patient.id}`));
    await user.click(await screen.findByTestId("book-submit"));

    expect(await screen.findByText(fr.error_validation)).toBeInTheDocument();
  });

  test("a 403 (the session cannot book) shows the plain refusal, not a silent failure", async () => {
    bookResponse = {
      status: 403,
      body: { error: { code: "forbidden", message: "missing edit_patients" } },
    };
    const user = userEvent.setup();
    mount();
    await user.click(await screen.findByTestId("fc-slot"));
    await user.click(await screen.findByTestId(`book-pick-${patient.id}`));
    await user.click(await screen.findByTestId("book-submit"));

    expect(await screen.findByText(fr.error_forbidden)).toBeInTheDocument();
  });
});

describe("the book screen: next free slot, N days out", () => {
  test("the offset typed reaches the server, and the slot found is preselected", async () => {
    nextFreeResponse = {
      first_day: "2026-10-08",
      last_day: "2026-12-06",
      slot_minutes: 15,
      starts_at: "2026-10-08 09:00:00",
    };
    const user = userEvent.setup();
    mount();

    await user.click(await screen.findByTestId("book-next-free"));
    const offset = await screen.findByTestId("next-free-offset");
    await user.clear(offset);
    await user.type(offset, "15");
    await user.click(screen.getByTestId("next-free-search"));

    await waitFor(() => {
      const call = fetchMock.mock.calls.find((args) =>
        String(args[0]).includes("/appointments/next-free"),
      );
      expect(call).toBeDefined();
      const url = call === undefined ? "" : String(call[0]);
      expect(new URL(url, "http://x").searchParams.get("offset_days")).toBe("15");
    });

    // The dialog that opens next is the booking one, on the slot found.
    const dialog = await screen.findByTestId("book-dialog");
    expect(dialog).toHaveTextContent("2026-10-08 09:00:00");
  });
});

describe("the book screen: opening a booked appointment", () => {
  test("clicking it opens the appointment dialog with its own patient", async () => {
    appointmentsResponse = [appointment];
    const user = userEvent.setup();
    mount();
    await user.click(await screen.findByTestId(`fc-event-${appointment.id}`));
    const dialog = await screen.findByTestId("appointment-dialog");
    expect(dialog).toHaveTextContent("Amina Benali");
  });

  test("dragging it to a slot the server refuses (409) shows the plain message and reverts", async () => {
    appointmentsResponse = [appointment];
    fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
      const url = String(input);
      if (url.endsWith("/auth/me")) return Promise.resolve(json(200, ME_OWNER));
      if (url.endsWith("/auth/idle")) return Promise.resolve(json(200, { idle_minutes: 30 }));
      if (url.endsWith("/clock")) return Promise.resolve(json(200, { today: "2026-09-23" }));
      if (url.endsWith("/settings/working-hours")) return Promise.resolve(json(200, closedFridaySaturday));
      if (url.endsWith("/absence-blocks")) return Promise.resolve(json(200, { blocks: [] }));
      if (url.endsWith("/settings/slot-minutes")) return Promise.resolve(json(200, { slot_minutes: 15 }));
      if (url.endsWith("/settings/visit-types")) {
        return Promise.resolve(json(200, { visit_types: visitTypes }));
      }
      if (url.includes("/day-list")) {
        return Promise.resolve(json(200, { day: "2026-09-23", appointments: [], walk_ins: [] }));
      }
      if (init?.method === "POST" && url.endsWith(`/appointments/${appointment.id}/move`)) {
        return Promise.resolve(
          json(409, { error: { code: "conflict", message: "already taken" } }),
        );
      }
      if (url.includes("/appointments")) {
        return Promise.resolve(
          json(200, { from: "2026-09-20", to: "2026-09-26", appointments: appointmentsResponse }),
        );
      }
      return Promise.resolve(json(200, {}));
    });
    vi.stubGlobal("fetch", fetchMock);

    const user = userEvent.setup();
    mount();
    await user.click(await screen.findByTestId(`fc-drag-${appointment.id}`));

    expect(await screen.findByText(fr.error_conflict)).toBeInTheDocument();
    await waitFor(() => expect(dragRevert).toHaveBeenCalledTimes(1));
  });

  test("cancel calls the appointment's own cancel route", async () => {
    appointmentsResponse = [appointment];
    const user = userEvent.setup();
    mount();
    await user.click(await screen.findByTestId(`fc-event-${appointment.id}`));
    await user.click(await screen.findByTestId("appointment-cancel"));
    await waitFor(() =>
      expect(requestsFor("POST").some(([url]) => String(url).endsWith(`/appointments/${appointment.id}/cancel`))).toBe(
        true,
      ),
    );
  });

  test("arrived calls the appointment's own arrive route (C6b)", async () => {
    appointmentsResponse = [appointment];
    const user = userEvent.setup();
    mount();
    await user.click(await screen.findByTestId(`fc-event-${appointment.id}`));
    await user.click(await screen.findByTestId("appointment-arrive"));
    await waitFor(() =>
      expect(requestsFor("POST").some(([url]) => String(url).endsWith(`/appointments/${appointment.id}/arrive`))).toBe(
        true,
      ),
    );
  });

  test("closing one appointment and opening another starts its move field blank, not the last one typed", async () => {
    appointmentsResponse = [appointment, otherAppointment];
    const user = userEvent.setup();
    mount();

    await user.click(await screen.findByTestId(`fc-event-${appointment.id}`));
    const moveInput = await screen.findByTestId("appointment-move-input");
    fireEvent.change(moveInput, { target: { value: "2026-09-24T09:00" } });
    expect(moveInput).toHaveValue("2026-09-24T09:00");
    await user.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByTestId("appointment-dialog")).not.toBeInTheDocument());

    await user.click(await screen.findByTestId(`fc-event-${otherAppointment.id}`));
    const reopened = await screen.findByTestId("appointment-move-input");
    expect(reopened).toHaveValue("2026-09-23T11:00");
  });
});

describe("the book screen: the confirmation call (C6b)", () => {
  test("a booking with a recorded call carries the call's mark on the grid, one without carries none", async () => {
    appointmentsResponse = [appointment];
    mount();
    await screen.findByTestId(`fc-event-${appointment.id}`);
    const content = lastCalendarProps?.eventContent;
    expect(content).toBeDefined();
    const withCall = { ...appointment, call_outcome: "no_answer" as const, call_at: "2026-09-22 18:00:00" };
    const marked = render(
      <I18nProvider lang="fr">
        {content?.({ event: { title: "Amina Benali", extendedProps: { appointment: withCall } } })}
      </I18nProvider>,
    );
    expect(marked.getByRole("img", { name: fr.book_call_no_answer })).toBeInTheDocument();
    marked.unmount();
    const plain = render(
      <I18nProvider lang="fr">
        {content?.({ event: { title: "Amina Benali", extendedProps: { appointment } } })}
      </I18nProvider>,
    );
    expect(plain.queryByRole("img")).not.toBeInTheDocument();
  });

  test("tomorrow's calls reads the day after the shop's today and records a call on its own row", async () => {
    // Far from any run date, so tomorrow off the machine's clock shows.
    shopToday = "2027-03-10";
    appointmentsResponse = [{ ...appointment, phone: "0555000000" }];
    const user = userEvent.setup();
    mount();
    await user.click(await screen.findByTestId("book-calls"));
    const dialog = await screen.findByTestId("calls-dialog");
    expect(dialog).toHaveTextContent("2027-03-11");
    await waitFor(() =>
      expect(fetchMock.mock.calls.some(([url]) => String(url).endsWith("/appointments?day=2027-03-11"))).toBe(true),
    );
    const row = await screen.findByTestId(`calls-row-${appointment.id}`);
    expect(row).toHaveTextContent("0555000000");
    await user.click(screen.getByTestId(`call-confirmed-${appointment.id}`));
    await waitFor(() => {
      const posted = requestsFor("POST").find(([url]) =>
        String(url).endsWith(`/appointments/${appointment.id}/call-outcome`),
      );
      expect(posted).toBeDefined();
      expect(JSON.parse(String(posted?.[1].body))).toEqual({ outcome: "confirmed" });
    });
  });
});

describe("the book screen: a cashier reaches the absence-block tool from /book", () => {
  test("the toolbar's Absence button opens the block form for a cashier session", async () => {
    const user = userEvent.setup();
    mount("fr", ME_CASHIER);
    await user.click(await screen.findByTestId("book-absence"));
    const dialog = await screen.findByTestId("absence-dialog");
    expect(dialog).toContainElement(screen.getByTestId("absence-blocks-form"));
  });
});
