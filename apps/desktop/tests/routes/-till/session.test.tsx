// The till shift bar: the popup at sign-in, the close modal, and the two
// figures the wire hands it rather than working either out itself
// (`session.tsx`'s own doc; plan till-shifts-a-float-and-a-count T5).
//
// No router: `TillShiftBar` carries no `Link`, so a plain
// `SessionProvider` + `QueryClientProvider` is the whole harness. The fake
// server below keeps just enough state to answer `GET /till/shifts/open`
// the way the real one would after the POST this screen just sent — a
// shift opened, then read back open; a shift closed, then read back gone —
// because the popup's own gating (`asked`, ruling 2b) depends on that
// round trip and not on the screen inventing the state itself.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ShiftDto, ShiftReportDto } from "@dzpos/shared";
import { I18nProvider, type Lang } from "@/i18n";
import fr from "@/i18n/fr.json";
import { SessionProvider } from "@/lib/session";
import { ME_CASHIER as ME } from "@/test/session";
import { TillShiftBar } from "../../../src/routes/-till/session";

function shiftDto(overrides: Partial<ShiftDto> = {}): ShiftDto {
  return {
    id: 10,
    opened_by: ME.user_id,
    opened_at: "2026-09-21 08:00:00",
    opening_cash_centimes: 500_000,
    closed_at: null,
    closed_by: null,
    counted_centimes: null,
    expected_at_close_centimes: null,
    difference_centimes: null,
    note: null,
    ...overrides,
  };
}

/** `takingsTotalCentimes` defaults to the figure that makes
 * `opening_cash + takings.total === expected` (every call site before
 * finding 4 relied on that identity), but a caller may pass a takings total
 * that disagrees with `expectedCentimes` — an annulled ticket rung after the
 * shift opened is the realistic case — so a test can tell the server's own
 * expected figure apart from `opening_cash + takings` worked out again on
 * the screen (ruling 4, forbidden). */
function reportOf(
  shift: ShiftDto,
  expectedCentimes: number,
  takingsTotalCentimes: number = expectedCentimes - shift.opening_cash_centimes,
  rungOutsideShift = 0,
): ShiftReportDto {
  return {
    shift,
    takings: {
      sales_centimes: takingsTotalCentimes,
      stamp_centimes: 0,
      customer_payments_centimes: 0,
      total_centimes: takingsTotalCentimes,
    },
    // No reversal was handed back in any of these fixtures, so the expected
    // figure below is the takings whole. The screen that shows this figure
    // is the second half of T6 and not here.
    refunds_centimes: 0,
    expected_centimes: expectedCentimes,
    difference_centimes: null,
    until: "2026-09-21 12:00:00",
    // Zero unless a case says otherwise: the drawer was open the whole time
    // in every fixture but the one that names this.
    rung_outside_shift: rungOutsideShift,
  };
}

/** U+202F, the narrow no-break space `formatCentimes` groups thousands
 * with — the same constant `money.test.ts` and `ProductTile.test.tsx` carry
 * under this name. `toHaveValue` reads an input's raw value and does not
 * normalise it the way `toHaveTextContent` does, so a plain space here would
 * silently fail to match. */
const NBSP = " ";

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function refusal(status: number, code: string, message: string, field?: string): Response {
  return json(status, { error: { code, message, field } });
}

/** What `GET /till/shifts/open` answers right now. The two POSTs below
 * update it, the way the real service's writes are what the next read
 * sees. */
let openState: ShiftReportDto | null;
/** Set by a test that wants the close call refused instead of the default
 * "close whatever is open" behaviour. */
let closeRefusal: Response | null;
/** Set by a test that wants the open call refused instead of the default
 * "open at what was posted" behaviour — the clock-backward guard
 * (`services::shifts::open`) firing on the auto-open path is the realistic
 * case (finding 2). */
let openRefusal: Response | null;
/** A test that wants `GET /till/shifts/open` to hang, to prove nothing
 * clickable renders before that call answers either way. */
let openHangs: boolean;
let fetchMock: ReturnType<typeof vi.fn>;

function requestBody(init: RequestInit | undefined): unknown {
  return init?.body === undefined ? undefined : JSON.parse(String(init.body));
}

beforeEach(() => {
  window.localStorage.clear();
  openState = null;
  closeRefusal = null;
  openRefusal = null;
  openHangs = false;
  fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
    const url = String(input);
    const method = init?.method ?? "GET";
    if (url.endsWith("/health")) {
      return Promise.resolve(json(200, { status: "ok", shop_id: 1, needs_first_setup: false }));
    }
    if (url.endsWith("/auth/me")) return Promise.resolve(json(200, ME));
    if (url.endsWith("/auth/idle")) return Promise.resolve(json(200, { idle_minutes: 30 }));
    if (method === "GET" && url.endsWith("/till/shifts/open")) {
      if (openHangs) return new Promise<Response>(() => undefined);
      return Promise.resolve(json(200, openState));
    }
    if (method === "POST" && url.endsWith("/till/shifts")) {
      if (openRefusal !== null) return Promise.resolve(openRefusal);
      const posted = requestBody(init);
      const amount =
        typeof posted === "object" && posted !== null && "opening_cash_centimes" in posted
          ? Number(Reflect.get(posted, "opening_cash_centimes"))
          : 0;
      const shift = shiftDto({ opening_cash_centimes: amount });
      openState = reportOf(shift, amount);
      return Promise.resolve(json(201, shift));
    }
    if (method === "POST" && /\/till\/shifts\/\d+\/close$/.test(url)) {
      if (closeRefusal !== null) return Promise.resolve(closeRefusal);
      const posted = requestBody(init);
      const countedCentimes =
        typeof posted === "object" && posted !== null && "counted_centimes" in posted
          ? Number(Reflect.get(posted, "counted_centimes"))
          : 0;
      const note =
        typeof posted === "object" && posted !== null && "note" in posted
          ? Reflect.get(posted, "note")
          : null;
      const expected = openState?.expected_centimes ?? 0;
      const opened = openState?.shift ?? shiftDto();
      const closed: ShiftDto = {
        ...opened,
        closed_at: "2026-09-21 19:00:00",
        closed_by: ME.user_id,
        counted_centimes: countedCentimes,
        expected_at_close_centimes: expected,
        difference_centimes: countedCentimes - expected,
        note: typeof note === "string" ? note : null,
      };
      openState = null;
      return Promise.resolve(json(200, closed));
    }
    return Promise.resolve(json(404, { error: { code: "not_found", message: "no route" } }));
  });
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

function mount(lang: Lang = "fr") {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <I18nProvider lang={lang}>
      <QueryClientProvider client={client}>
        <SessionProvider>
          <TillShiftBar />
        </SessionProvider>
      </QueryClientProvider>
    </I18nProvider>,
  );
}

/** Every POST this fake server saw at the given path, oldest first. */
function posted(path: string): unknown[] {
  return fetchMock.mock.calls
    .filter(([input, init]) => String(input).endsWith(path) && init?.method === "POST")
    .map(([, init]) => requestBody(init));
}

describe("opening the till", () => {
  test("shows neither a popup nor a manual button while the read is still on the wire", async () => {
    openHangs = true;
    mount();
    // Waiting for the mock to have been asked at all is what gives the bar
    // its chance to have drawn something, without racing a `findBy` on an
    // element this test expects never to appear.
    await waitFor(() =>
      expect(fetchMock.mock.calls.some(([input]) => String(input).endsWith("/till/shifts/open"))).toBe(
        true,
      ),
    );
    expect(screen.queryByTestId("till-open-dialog")).not.toBeInTheDocument();
    expect(screen.queryByTestId("till-open-trigger")).not.toBeInTheDocument();
    expect(screen.queryByTestId("till-shift-badge")).not.toBeInTheDocument();
  });

  test("raises the popup with nothing to remember, and posts exactly what was typed", async () => {
    const user = userEvent.setup();
    mount();
    const dialog = await screen.findByTestId("till-open-dialog");
    const amountBox = within(dialog).getByLabelText(fr.field_opening_cash);
    expect(amountBox).toHaveValue("");
    await user.type(amountBox, "5000");
    await user.click(within(dialog).getByTestId("till-open-submit"));
    await waitFor(() => expect(screen.queryByTestId("till-open-dialog")).not.toBeInTheDocument());
    expect(posted("/till/shifts")).toEqual([{ opening_cash_centimes: 500_000 }]);
    expect(await screen.findByTestId("till-shift-badge")).toBeInTheDocument();
  });

  test("pre-fills the popup with the last close's counted figure", async () => {
    window.localStorage.setItem(
      `dzpos:till-open:${ME.user_id}`,
      JSON.stringify({ autoOpen: false, lastCountedCentimes: 720_000 }),
    );
    mount();
    const dialog = await screen.findByTestId("till-open-dialog");
    expect(within(dialog).getByLabelText(fr.field_opening_cash)).toHaveValue("7 200,00");
  });

  test("do not ask again opens silently at the remembered amount on the next sign-in", async () => {
    const user = userEvent.setup();
    const first = mount();
    const dialog = await screen.findByTestId("till-open-dialog");
    await user.type(within(dialog).getByLabelText(fr.field_opening_cash), "6000");
    await user.click(within(dialog).getByTestId("till-open-remember"));
    await user.click(within(dialog).getByTestId("till-open-submit"));
    await screen.findByTestId("till-shift-badge");
    // The shift closes (a full day passing), so the next sign-in finds
    // none open and the preference is what has to answer the question the
    // popup would otherwise ask again.
    openState = null;
    first.unmount();

    mount();
    // No popup at all: the bar goes straight to a shift open at the
    // remembered figure, with nothing to click through and no `till-open-*`
    // element ever appearing in the document.
    await screen.findByTestId("till-shift-badge");
    expect(screen.queryByTestId("till-open-dialog")).not.toBeInTheDocument();
    expect(posted("/till/shifts")).toEqual([
      { opening_cash_centimes: 600_000 },
      { opening_cash_centimes: 600_000 },
    ]);
  });

  test("shows the auto-open's own refusal rather than swallowing it (finding 2)", async () => {
    // A shop PC waking with a drifted clock: `services::shifts::open`'s
    // clock-backward guard refuses because `opened_at` lands at or before
    // this person's own last `closed_at`. Ordinary here, not exceptional —
    // and with nothing rendering the refusal the cashier would sell the
    // whole day with no shift open at all.
    window.localStorage.setItem(
      `dzpos:till-open:${ME.user_id}`,
      JSON.stringify({ autoOpen: true, lastCountedCentimes: 600_000 }),
    );
    openRefusal = refusal(422, "validation", "opened_at must be after your last close", "opened_at");
    mount();
    const dialog = await screen.findByTestId("till-open-dialog");
    expect(await within(dialog).findByText(fr.error_validation)).toBeInTheDocument();
    expect(within(dialog).getByLabelText(fr.field_opening_cash)).toHaveValue(`6${NBSP}000,00`);
    expect(posted("/till/shifts")).toEqual([{ opening_cash_centimes: 600_000 }]);
  });

  test("a negative remembered amount falls back to no preference (finding 3)", async () => {
    window.localStorage.setItem(
      `dzpos:till-open:${ME.user_id}`,
      JSON.stringify({ autoOpen: true, lastCountedCentimes: -1 }),
    );
    mount();
    const dialog = await screen.findByTestId("till-open-dialog");
    expect(within(dialog).getByLabelText(fr.field_opening_cash)).toHaveValue("");
    expect(posted("/till/shifts")).toEqual([]);
  });
});

describe("closing the till", () => {
  async function openTheTill(
    user: ReturnType<typeof userEvent.setup>,
    expectedCentimes: number,
    takingsTotalCentimes?: number,
  ) {
    const shift = shiftDto({ opening_cash_centimes: 500_000 });
    openState =
      takingsTotalCentimes === undefined
        ? reportOf(shift, expectedCentimes)
        : reportOf(shift, expectedCentimes, takingsTotalCentimes);
    mount();
    await screen.findByTestId("till-shift-badge");
    await user.click(screen.getByTestId("till-close-trigger"));
    return screen.findByTestId("till-close-dialog");
  }

  test("shows the expected figure before anything is typed", async () => {
    const user = userEvent.setup();
    const dialog = await openTheTill(user, 800_000);
    expect(within(dialog).getByTestId("till-expected")).toHaveTextContent(/^8 000,00$/);
  });

  test("says how many sales were rung before the drawer was opened, and stays silent when none were", async () => {
    // Those sales' cash is physically in the drawer, but they fell outside
    // the window the expected figure is summed over, so the count is a
    // sentence explaining a drawer that reads over and never a term in the
    // arithmetic: the expected figure below it does not move.
    const user = userEvent.setup();
    const shift = shiftDto({ opening_cash_centimes: 500_000 });
    openState = reportOf(shift, 800_000, 300_000, 3);
    mount();
    await screen.findByTestId("till-shift-badge");
    await user.click(screen.getByTestId("till-close-trigger"));
    const dialog = await screen.findByTestId("till-close-dialog");

    expect(within(dialog).getByTestId("till-rung-outside").textContent).toBe(
      fr.till_shift_rung_outside.replace("{count}", "3"),
    );
    expect(within(dialog).getByTestId("till-expected")).toHaveTextContent(/^8 000,00$/);
  });

  test("a drawer that was open the whole time shows no such line at all", async () => {
    // A line reading "0" would train a cashier to skip the block on the
    // evenings it matters.
    const user = userEvent.setup();
    const dialog = await openTheTill(user, 800_000);
    expect(within(dialog).queryByTestId("till-rung-outside")).not.toBeInTheDocument();
  });

  test("an exact count needs no note and closes straight away", async () => {
    const user = userEvent.setup();
    const dialog = await openTheTill(user, 800_000);
    await user.type(within(dialog).getByLabelText(fr.field_counted_cash), "8000");
    expect(within(dialog).queryByLabelText(fr.field_shift_note)).not.toBeInTheDocument();
    await user.click(within(dialog).getByTestId("till-close-submit"));
    await waitFor(() => expect(screen.queryByTestId("till-close-dialog")).not.toBeInTheDocument());
    expect(posted("/close")).toEqual([{ counted_centimes: 800_000, note: null }]);
  });

  test("a short count is shown negative, and the note field blocks the close until it is filled", async () => {
    const user = userEvent.setup();
    const dialog = await openTheTill(user, 800_000);
    await user.type(within(dialog).getByLabelText(fr.field_counted_cash), "7500");

    expect(within(dialog).getByTestId("till-difference")).toHaveTextContent(/^-500,00$/);
    const submit = within(dialog).getByTestId("till-close-submit");
    expect(submit).toBeDisabled();

    // The refusal the client gate mirrors: the core's own migration CHECK
    // and `services::shifts::close` both refuse a difference with no note
    // (`crates/core/src/services/shifts.rs`), so this button staying
    // enabled would only mean a round trip to be told the same thing.
    await user.type(within(dialog).getByLabelText(fr.field_shift_note), "Tirelire prise par erreur");
    expect(submit).toBeEnabled();
    await user.click(submit);
    await waitFor(() => expect(screen.queryByTestId("till-close-dialog")).not.toBeInTheDocument());
    expect(posted("/close")).toEqual([
      { counted_centimes: 750_000, note: "Tirelire prise par erreur" },
    ]);
    expect(screen.getByTestId("till-closed-difference")).toHaveTextContent(/^-500,00$/);
  });

  test("prints the server's own refusal rather than composing one", async () => {
    const user = userEvent.setup();
    const dialog = await openTheTill(user, 800_000);
    closeRefusal = refusal(
      403,
      "forbidden",
      "this role does not have the close_another_persons_till permission",
    );
    await user.type(within(dialog).getByLabelText(fr.field_counted_cash), "8000");
    await user.click(within(dialog).getByTestId("till-close-submit"));
    expect(
      await screen.findByText("this role does not have the close_another_persons_till permission"),
    ).toBeInTheDocument();
    // Not the generic sentence the same code would draw for any other
    // caller: `fr.error_forbidden` reads "Vous n'êtes pas autorisé..." and
    // never appears here.
    expect(screen.queryByText(fr.error_forbidden)).not.toBeInTheDocument();
  });

  test("reveals the note box on the server's own note refusal, even on a count this screen read as exact", async () => {
    const user = userEvent.setup();
    // The expected figure moved between the refetch this modal opened with
    // and the submit — a sale rung by the same cashier while the dialog was
    // up — so the count typed against the figure on screen looks exact here
    // and `services::shifts::close` is the one that catches the mismatch.
    const dialog = await openTheTill(user, 800_000);
    closeRefusal = refusal(
      422,
      "validation",
      "a drawer that does not match what was expected needs a reason",
      "note",
    );
    await user.type(within(dialog).getByLabelText(fr.field_counted_cash), "8000");
    expect(within(dialog).queryByLabelText(fr.field_shift_note)).not.toBeInTheDocument();
    await user.click(within(dialog).getByTestId("till-close-submit"));
    // The generic sentence, not the server's own words: only the
    // `close_another_persons_till` refusal is printed verbatim (the test
    // above). What this test pins is that the note box appears anyway.
    await screen.findByText(fr.error_validation);

    const noteBox = within(dialog).getByLabelText(fr.field_shift_note);
    expect(within(dialog).getByTestId("till-close-submit")).toBeDisabled();
    closeRefusal = null;
    await user.type(noteBox, "Vente en cours pendant le comptage");
    await user.click(within(dialog).getByTestId("till-close-submit"));
    await waitFor(() => expect(screen.queryByTestId("till-close-dialog")).not.toBeInTheDocument());
    expect(posted("/close")).toEqual([
      { counted_centimes: 800_000, note: null },
      { counted_centimes: 800_000, note: "Vente en cours pendant le comptage" },
    ]);
  });

  test("refetches the expected figure on a note refusal, rather than leaving the stale one on screen (finding 1)", async () => {
    const user = userEvent.setup();
    const dialog = await openTheTill(user, 800_000);
    if (openState === null) throw new Error("the till was just opened above");
    // A sale rings after the dialog's own refetch (fired when the close
    // trigger was clicked, before this point) but before the close POST
    // below — the exact gap `services::shifts::close`'s own guard exists
    // for. The screen must not go on showing the figure it fetched a moment
    // too early.
    openState = reportOf(openState.shift, 850_000);
    closeRefusal = refusal(
      422,
      "validation",
      "a drawer that does not match what was expected needs a reason",
      "note",
    );
    await user.type(within(dialog).getByLabelText(fr.field_counted_cash), "8000");
    await user.click(within(dialog).getByTestId("till-close-submit"));
    await screen.findByText(fr.error_validation);
    await waitFor(() =>
      expect(within(dialog).getByTestId("till-expected")).toHaveTextContent(/^8 500,00$/),
    );
    expect(within(dialog).getByTestId("till-difference")).toHaveTextContent(/^-500,00$/);
  });

  test("shows the server's own expected figure even when opening cash plus takings would total something else (finding 4)", async () => {
    const user = userEvent.setup();
    // An annulled ticket rung after the shift opened: the takings total and
    // the server's own expected figure disagree, so a regression to
    // `opening_cash + takings` worked out again on this screen (forbidden by
    // ruling 4, `session.tsx:27-29`) would print a different figure than
    // this assertion pins.
    const dialog = await openTheTill(user, 800_000, 250_000);
    expect(within(dialog).getByTestId("till-expected")).toHaveTextContent(/^8 000,00$/);
  });

  test("remembers what was counted at close, not what the shift opened at (finding 6)", async () => {
    const user = userEvent.setup();
    const first = mount();
    // Opened manually at 500 000 without ticking "do not ask again": the
    // open-side save still records this figure, which is exactly what would
    // leak through to the next sign-in if the close-side save (this
    // finding) were missing.
    const openDialog = await screen.findByTestId("till-open-dialog");
    await user.type(within(openDialog).getByLabelText(fr.field_opening_cash), "5000");
    await user.click(within(openDialog).getByTestId("till-open-submit"));
    await screen.findByTestId("till-shift-badge");

    // Closed at a different, exact count so the close needs no note.
    openState = reportOf(shiftDto({ opening_cash_centimes: 500_000 }), 900_000);
    await user.click(screen.getByTestId("till-close-trigger"));
    const closeDialog = await screen.findByTestId("till-close-dialog");
    await user.type(within(closeDialog).getByLabelText(fr.field_counted_cash), "9000");
    await user.click(within(closeDialog).getByTestId("till-close-submit"));
    await waitFor(() => expect(screen.queryByTestId("till-close-dialog")).not.toBeInTheDocument());

    openState = null;
    first.unmount();

    mount();
    const dialog = await screen.findByTestId("till-open-dialog");
    expect(within(dialog).getByLabelText(fr.field_opening_cash)).toHaveValue(`9${NBSP}000,00`);
  });
});
