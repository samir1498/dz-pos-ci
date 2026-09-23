// The patients screen is checked for what it shows from the API's answer
// and for what it sends: the list, the search, the notes field's
// permission gate and the archive flow. The rules behind each (a blank
// name, the 403 on a notes write) are the clinic crate's own tests
// (architecture.md, consequence of rule 2); what these hold is the wiring.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { MeDto, PatientDto } from "@dzpos/shared";

import { I18nProvider, type Lang } from "@/i18n";
import fr from "@/i18n/fr.json";
import { SessionProvider } from "@/lib/session";
import { ME_CASHIER, ME_OWNER, ME_WITHOUT_PATIENT_NOTES } from "@/test/session";

import { PatientsScreen } from "../../../src/routes/-patients/PatientsScreen";

const amina: PatientDto = {
  id: "01912a2e-0000-7000-8000-000000000001",
  first_name: "Amina",
  last_name: "Benali",
  sex: "female",
  date_of_birth: "1990-05-17",
  phone: "0555123456",
  notes: "diabète de type 2",
  archived_at: null,
  created_at: "2026-09-23 09:00:00",
  updated_at: "2026-09-23 09:00:00",
};

/** A second patient, only for the tests that must prove an action reached
 *  the row it was clicked on and not the other one sitting beside it in
 *  the same table. */
const sofiane: PatientDto = {
  ...amina,
  id: "01912a2e-0000-7000-8000-000000000003",
  first_name: "Sofiane",
  last_name: "Cherif",
  notes: null,
};

/** The receptionist's own answer: no `notes` key at all, the way the
 *  server actually redacts it (`crates/api/src/dto/patients.rs`), never a
 *  key present with an empty string. */
const { notes: _aminaNotes, ...aminaNoNotes }: PatientDto = amina;

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

function postedBody(): Record<string, unknown> {
  for (const [url, init] of requestsFor("POST")) {
    if (String(url).endsWith("/patients")) {
      if (typeof init.body !== "string") throw new Error("no JSON body");
      return JSON.parse(init.body);
    }
  }
  throw new Error("the form never posted");
}

/** The most recent request of a given method, its target URL and its
 *  decoded JSON body: what the fiche's save needs checked (what was sent)
 *  and the archive confirm needs checked (which id was addressed). Archive
 *  carries no body at all (`packages/shared/src/client/patients.ts`), so
 *  `null` there is the request actually made. */
function lastRequest(method: string): { url: string; body: Record<string, unknown> | null } {
  const calls = requestsFor(method);
  const last = calls[calls.length - 1];
  if (last === undefined) throw new Error(`no ${method} request was made`);
  const [url, init] = last;
  const body = typeof init.body === "string" ? JSON.parse(init.body) : null;
  return { url: String(url), body };
}

/** `lastRequest`, for the requests that always carry one (the fiche's
 *  save): a `null` there is the test's own mistake, never a shape to
 *  branch on. */
function lastBody(method: string): Record<string, unknown> {
  const { body } = lastRequest(method);
  if (body === null) throw new Error(`the last ${method} request carried no body`);
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
          <PatientsScreen />
        </SessionProvider>
      </QueryClientProvider>
    </I18nProvider>,
  );
}

let fetchMock: ReturnType<typeof vi.fn>;
let list: PatientDto[];
let me: MeDto;

beforeEach(() => {
  list = [];
  me = ME_OWNER;
  fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
    const url = String(input);
    if (url.endsWith("/auth/me")) return Promise.resolve(json(200, me));
    if (url.endsWith("/auth/idle")) return Promise.resolve(json(200, { idle_minutes: 30 }));
    if (init?.method === "POST" && url.endsWith("/patients")) {
      const sent: Record<string, unknown> = JSON.parse(String(init.body));
      const made: PatientDto = {
        ...amina,
        id: "01912a2e-0000-7000-8000-000000000002",
        first_name: typeof sent.first_name === "string" ? sent.first_name : amina.first_name,
        last_name: typeof sent.last_name === "string" ? sent.last_name : amina.last_name,
        notes: typeof sent.notes === "string" ? sent.notes : null,
      };
      return Promise.resolve(json(201, made));
    }
    if (init?.method === "POST" && url.includes("/archive")) {
      const id = url.split("/patients/")[1]?.split("/")[0];
      list = list.map((p) => (p.id === id ? { ...p, archived_at: "2026-09-23 10:00:00" } : p));
      return Promise.resolve(json(200, list.find((p) => p.id === id)));
    }
    if (url.includes("/patients")) return Promise.resolve(json(200, list));
    return Promise.resolve(json(200, {}));
  });
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("the patients list", () => {
  test("renders what the API returned", async () => {
    list = [amina];
    mount();
    expect(await screen.findByText("Amina Benali")).toBeInTheDocument();
    expect(screen.getByText("0555123456")).toBeInTheDocument();
  });

  test("says so when there is nothing yet", async () => {
    mount();
    expect(await screen.findByText(fr.patients_empty)).toBeInTheDocument();
  });
});

describe("the notes field", () => {
  test("shows and saves notes for a session holding view_patient_notes", async () => {
    const user = userEvent.setup();
    mount();
    await user.click(await screen.findByRole("button", { name: fr.patients_add }));
    await user.type(screen.getByTestId("patient-first-name"), "Yacine");
    await user.type(screen.getByTestId("patient-last-name"), "Meziane");
    await user.type(screen.getByTestId("patient-notes"), "allergique à la pénicilline");
    await user.click(screen.getByRole("button", { name: fr.action_save }));

    await waitFor(() => expect(postedBody().notes).toBe("allergique à la pénicilline"));
  });

  // `ME_WITHOUT_PATIENT_NOTES`, not `ME_CASHIER`: a cashier is also missing
  // `see_reports`, `manage_users` and a dozen other permissions the notes
  // field never looks at, so a bug that gated the field on the wrong one
  // of those could still pass this test. The fixture below differs from an
  // owner by exactly the one permission being checked.
  test("never renders the field for a session without view_patient_notes, and sends null", async () => {
    me = ME_WITHOUT_PATIENT_NOTES;
    const user = userEvent.setup();
    mount();
    await user.click(await screen.findByRole("button", { name: fr.patients_add }));
    expect(screen.queryByTestId("patient-notes")).not.toBeInTheDocument();
    expect(screen.getByText(fr.patients_notes_hidden)).toBeInTheDocument();

    await user.type(screen.getByTestId("patient-first-name"), "Yacine");
    await user.type(screen.getByTestId("patient-last-name"), "Meziane");
    await user.click(screen.getByRole("button", { name: fr.action_save }));

    await waitFor(() => expect(postedBody().notes).toBeNull());
  });

  test("a receptionist's fiche opened on a file with no notes key still sends null, never the empty string", async () => {
    me = ME_CASHIER;
    list = [aminaNoNotes];
    const user = userEvent.setup();
    mount();
    await user.click(await screen.findByRole("button", { name: `${fr.patients_edit} Amina Benali` }));
    await user.click(screen.getByRole("button", { name: fr.action_save }));

    await waitFor(() => expect(lastBody("PUT").notes).toBeNull());
  });

  // The sheet does not unmount across a lock and an unlock: `__root.tsx`
  // marks the whole app `inert` while locked rather than removing it, so a
  // fiche a receptionist had open is still that exact component tree once
  // the doctor unlocks. `fiche.tsx` used to seed `defaultValues` once, from
  // whoever was signed in when the sheet opened: a receptionist opening
  // Amina's file saw the notes field seeded blank (she cannot read it), and
  // an owner unlocking the same till without ever closing the sheet would
  // still be looking at that blank field — a save from there would have
  // sent `cleared("")` over Amina's real notes.
  //
  // The receptionist edits the phone number first: `@tanstack/react-form`
  // re-seeds an *untouched* form's values from a changed `defaultValues` on
  // its own, which would otherwise paper over the bug for the one case
  // that never actually happens on a real till — nobody touches a fiche and
  // then walks away without changing a thing. The moment a field is
  // touched, that re-seeding stops, which is exactly the state a fiche
  // left open across a lock is in.
  //
  // The session's own "look again on focus" revalidation
  // (`lib/session.tsx`) is what actually changes `me` without unmounting
  // anything above the fiche; a real unlock takes the same shape
  // (`establish()` also just calls `setMe`), and this test does not need
  // `__root.tsx` or `LockScreen` mounted to exercise it.
  test("keeps the real notes across a permission change while the fiche stays open, touched or not", async () => {
    me = ME_WITHOUT_PATIENT_NOTES;
    list = [amina];
    const user = userEvent.setup();
    mount();
    await user.click(await screen.findByRole("button", { name: `${fr.patients_edit} Amina Benali` }));
    expect(screen.queryByTestId("patient-notes")).not.toBeInTheDocument();
    await user.type(screen.getByTestId("patient-phone"), "9");

    me = ME_OWNER;
    window.dispatchEvent(new Event("focus"));

    const notes = await screen.findByTestId("patient-notes");
    expect(notes).toHaveValue("diabète de type 2");

    await user.click(screen.getByRole("button", { name: fr.action_save }));
    await waitFor(() => expect(lastBody("PUT").notes).toBe("diabète de type 2"));
  });
});

describe("archiving", () => {
  test("asks for confirmation, then archives, never before the confirm click", async () => {
    list = [amina, sofiane];
    const user = userEvent.setup();
    mount();
    await screen.findByText("Amina Benali");
    await user.click(screen.getByTestId(`patient-archive-${sofiane.id}`));

    const dialog = await screen.findByTestId("patient-archive-dialog");
    expect(dialog).toHaveTextContent(fr.patients_archive_confirm_question);
    expect(requestsFor("POST").filter(([url]) => String(url).includes("/archive"))).toHaveLength(0);

    await user.click(screen.getByTestId("patient-archive-dialog-confirm"));

    // Two patients sit in the table; this is the one whose row the archive
    // button was on, and never the other, whatever order the table drew
    // them in.
    await waitFor(() => expect(lastRequest("POST").url).toMatch(new RegExp(`/patients/${sofiane.id}/archive$`)));
    expect(fetchMock.mock.calls.some((call) => String(call[0]).endsWith(`/patients/${amina.id}/archive`))).toBe(
      false,
    );
  });
});
