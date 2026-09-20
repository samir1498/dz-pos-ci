// The settings rooms are checked for rendering and wiring: what each shows
// from the API's answer and what it sends. The rules (a blank name, a bad
// day) are the API crate's tests.
//
// Settings is a layout with a rail and one room open beside it, so `mount`
// takes the address to open at and builds the same parent/child shape the
// generated route tree has. A test that wants the régime says so; the shop
// block is the default because `/settings` redirects there.

import { afterEach, beforeAll, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  RouterProvider,
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
} from "@tanstack/react-router";
import type { ReactNode } from "react";
import type { DatedRegimeDto, RegimeDto, SettingsDto, StoreDto } from "@dzpos/shared";
import { I18nProvider, type Lang } from "@/i18n";
import fr from "@/i18n/fr.json";
import ar from "@/i18n/ar.json";
import { ThemeProvider } from "@/lib/theme";
import { SessionProvider } from "@/lib/session";
import { ME_CASHIER, ME_OWNER } from "@/test/session";
import { SettingsLayout } from "./settings";
import { BackupsRoom } from "./settings.backups";
import { DataRoom } from "./settings.data";
import { RegimeRoom } from "./settings.regime";
import { ShopRoom } from "./settings.shop";
import { PrintingRoom } from "./settings.printing";
import { ThemePanel } from "@/components/settings/ThemePanel";

/** The régime's "valid from" field is three boxes now, not one native date
 *  input, so a test reaches it by the day/month/year test ids DateField
 *  gives its segments rather than by typing a whole ISO string at once. */
function dateFieldSegments(testId: string) {
  return {
    day: screen.getByTestId(`${testId}-day`),
    month: screen.getByTestId(`${testId}-month`),
    year: screen.getByTestId(`${testId}-year`),
  };
}

async function typeIsoDate(user: ReturnType<typeof userEvent.setup>, testId: string, iso: string) {
  const [year, month, day] = iso.split("-");
  const segments = dateFieldSegments(testId);
  await user.clear(segments.day);
  await user.type(segments.day, day ?? "");
  await user.clear(segments.month);
  await user.type(segments.month, month ?? "");
  await user.clear(segments.year);
  await user.type(segments.year, year ?? "");
}

function expectIsoDate(testId: string, iso: string) {
  const [year, month, day] = iso.split("-");
  const segments = dateFieldSegments(testId);
  expect(segments.day).toHaveValue(day);
  expect(segments.month).toHaveValue(month);
  expect(segments.year).toHaveValue(year);
}

const store: StoreDto = {
  name: "Mon magasin",
  rc: null,
  nif: null,
  nis: null,
  ai: null,
  address: null,
  phone: null,
};

/** What the server says the day is. Deliberately a day the machine is not
 * on: the shop's calendar is Algeria's, and a screen that read `new Date()`
 * would fail here. */
const SHOP_TODAY = "2027-03-04";

const seeded: SettingsDto = {
  store,
  regime: { regime: "reel", valid_from: "2026-01-01" },
  regime_planned: null,
  theme: null,
  facture_layout: "standard",
  facture_layouts: ["standard", "compact", "half_sheet"],
  discount_threshold_bps: 0,
};

/**
 * The régime is chosen through the kit's select, which is Radix's, and Radix
 * calls two DOM methods jsdom does not implement. They are stubbed rather
 * than avoided: what these tests assert is the body that leaves the screen,
 * and the browser path of the same control is proved in
 * `e2e/settings.spec.ts`, which drives a real Chromium.
 */
beforeAll(() => {
  Element.prototype.scrollIntoView = vi.fn();
  Element.prototype.hasPointerCapture = vi.fn(() => false);
  Element.prototype.setPointerCapture = vi.fn();
  Element.prototype.releasePointerCapture = vi.fn();
});

/** Opens the régime select of `form` and picks the option reading `label`. */
async function chooseRegime(
  user: ReturnType<typeof userEvent.setup>,
  form: HTMLElement,
  label: string,
): Promise<void> {
  await user.click(within(form).getByRole("combobox", { name: fr.field_regime }));
  await user.click(await screen.findByRole("option", { name: label }));
}

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function isInit(value: unknown): value is RequestInit {
  return typeof value === "object" && value !== null;
}

/** The URL and JSON body of the first request made with `method`. */
function sent(method: string): { url: string; body: Record<string, unknown> } {
  for (const call of fetchMock.mock.calls) {
    const init: unknown = call[1];
    if (isInit(init) && init.method === method) {
      if (typeof init.body !== "string") throw new Error(`the ${method} had no JSON body`);
      return { url: String(call[0]), body: JSON.parse(init.body) };
    }
  }
  throw new Error(`no ${method} was made`);
}

function countOf(method: string): number {
  return fetchMock.mock.calls.filter((call) => {
    const init: unknown = call[1];
    return isInit(init) && init.method === method;
  }).length;
}

function mount(lang: Lang = "fr", path = "/settings/shop") {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  // The rail builds a real `Link` per room, and a `Link` to an address the
  // router does not know throws. Every room is therefore a route here; the
  // three whose panels have tests of their own are stubbed, because what
  // this file checks is the rail and the four rooms below it.
  const rootRoute = createRootRoute();
  const settingsRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "/settings",
    component: SettingsLayout,
  });
  const room = (child: string, component: () => ReactNode) =>
    createRoute({ getParentRoute: () => settingsRoute, path: child, component });
  const rooms = [
    room("/shop", ShopRoom),
    room("/regime", RegimeRoom),
    room("/appearance", ThemePanel),
    room("/printing", PrintingRoom),
    room("/users", () => null),
    room("/phones", () => null),
    room("/backups", BackupsRoom),
    room("/data", DataRoom),
    room("/about", () => null),
  ];
  const router = createRouter({
    routeTree: rootRoute.addChildren([settingsRoute.addChildren(rooms)]),
    history: createMemoryHistory({ initialEntries: [path] }),
  });
  return render(
    <I18nProvider lang={lang}>
      <QueryClientProvider client={client}>
        {/* The appearance room's control reads the provider. It shares the
            settings query key with the shop and régime rooms, so the two
            are one fetch and the call counts below are unchanged. */}
        <ThemeProvider>
          {/* An owner by default: the staff and export/import rooms are what
              this file already tested before M4 T5 gated them on
              `manage_users` and `export_and_import`. `me` is set to
              `ME_CASHIER` first by the tests that care who is signed in. */}
          <SessionProvider>
            <RouterProvider router={router} />
          </SessionProvider>
        </ThemeProvider>
      </QueryClientProvider>
    </I18nProvider>,
  );
}

let fetchMock: ReturnType<typeof vi.fn>;
let current: SettingsDto;
let storeAnswer: (() => Response) | null;
let regimeAnswer: (() => Response) | null;
let clockAnswer: (() => Response | Promise<Response>) | null;
let me: typeof ME_OWNER | typeof ME_CASHIER;

beforeEach(() => {
  current = seeded;
  storeAnswer = null;
  regimeAnswer = null;
  clockAnswer = null;
  me = ME_OWNER;
  fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
    const url = String(input);
    if (url.endsWith("/auth/me")) return Promise.resolve(json(200, me));
    if (url.endsWith("/auth/idle")) return Promise.resolve(json(200, { idle_minutes: 30 }));
    if (init?.method === "PUT" && url.endsWith("/settings/store")) {
      if (storeAnswer !== null) return Promise.resolve(storeAnswer());
      const body: unknown = JSON.parse(String(init.body));
      if (typeof body !== "object" || body === null) throw new Error("no body");
      current = { ...current, store: { ...store, ...body } };
      return Promise.resolve(json(200, current.store));
    }
    if (init?.method === "PUT" && url.endsWith("/settings/facture-layout")) {
      const body: unknown = JSON.parse(String(init.body));
      if (typeof body !== "object" || body === null) throw new Error("no body");
      const asked = "facture_layout" in body ? body.facture_layout : undefined;
      // Narrowed against the fixture's own list rather than asserted: the
      // point of the round trip is that the screen reads back what the
      // server stored, so a value neither of them knows must not pass.
      const chosen = seeded.facture_layouts.find((layout) => layout === asked);
      current = { ...current, facture_layout: chosen ?? "standard" };
      return Promise.resolve(json(200, current));
    }
    if (init?.method === "POST" && url.endsWith("/settings/regime")) {
      if (regimeAnswer !== null) return Promise.resolve(regimeAnswer());
      const body: unknown = JSON.parse(String(init.body));
      if (typeof body !== "object" || body === null) throw new Error("no body");
      const regime: RegimeDto = "regime" in body && body.regime === "reel" ? "reel" : "ifu";
      const validFrom = "valid_from" in body ? String(body.valid_from) : "";
      // The stub applies the API's rule: a day past today is planned, and
      // "today" is the shop's, the same one the /clock branch answers.
      const today = SHOP_TODAY;
      const dated: DatedRegimeDto = { regime, valid_from: validFrom };
      current =
        validFrom > today
          ? { ...current, regime_planned: dated }
          : { ...current, regime: dated, regime_planned: null };
      return Promise.resolve(json(200, current));
    }
    // The page carries the backups block and the stock recount block too;
    // each asks for its own list as soon as the settings load, and an
    // unanswered call would leave a second alert on the screen these tests
    // read.
    if (url.endsWith("/backups"))
      return Promise.resolve(json(200, { backups: [], safety_copies: [], upgrade_copies: [] }));
    if (url.endsWith("/stock/recount"))
      return Promise.resolve(json(200, { last_run_day: null, drifts: [] }));
    // The paired phones block is on this page too and asks for its list the
    // moment the settings arrive. Same reason as the two above: a 404 here
    // puts a second alert on the screen and every assertion below reads
    // "found multiple elements with the role alert".
    if (url.endsWith("/pairing/devices")) return Promise.resolve(json(200, []));
    // The régime form dates its default from the shop's calendar, which the
    // server owns; a fixed day here so the field is assertable.
    if (url.endsWith("/clock")) {
      if (clockAnswer !== null) return Promise.resolve(clockAnswer());
      return Promise.resolve(json(200, { today: SHOP_TODAY }));
    }
    if (url.endsWith("/settings")) return Promise.resolve(json(200, current));
    return Promise.resolve(json(404, { error: { code: "not_found", message: "no" } }));
  });
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("the rail", () => {
  test("lists every room, and opening one leaves the others closed", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByLabelText(fr.field_name);
    expect(screen.getByTestId("settings-room-users")).toHaveAttribute("href", "/settings/users");
    expect(screen.getByTestId("settings-room-about")).toHaveAttribute("href", "/settings/about");
    expect(screen.getByTestId("settings-room-phones")).toHaveAttribute("href", "/settings/phones");

    await user.click(screen.getByTestId("settings-room-regime"));
    await screen.findByTestId("regime-current");
    // The room that was open is gone, which is the whole point of the
    // split: one panel at a time, not eight stacked.
    expect(screen.queryByLabelText(fr.field_name)).not.toBeInTheDocument();
  });

  test("hides the rooms a cashier would be refused at", async () => {
    me = ME_CASHIER;
    mount();
    await screen.findByLabelText(fr.field_name);
    // GET /users and the four exports already refuse a cashier
    // (`crates/api/src/gates.rs`, `ManageUsers` and `ExportAndImport`); this
    // is the hidden button, not the defence (M4 T5).
    expect(screen.queryByTestId("settings-room-users")).not.toBeInTheDocument();
    expect(screen.queryByTestId("settings-room-phones")).not.toBeInTheDocument();
    // The data room stays: the stock recount inside it is open to every
    // role, and only the export/import block is hidden.
    expect(screen.getByTestId("settings-room-data")).toBeInTheDocument();
  });

  test("the data room hides the export/import block from a cashier and keeps the recount", async () => {
    me = ME_CASHIER;
    mount("fr", "/settings/data");
    expect(await screen.findByText(fr.settings_stock_recount)).toBeInTheDocument();
    expect(screen.queryByText(fr.settings_export_import)).not.toBeInTheDocument();
  });
});

describe("the shop room", () => {
  test("shows the store block the API answers", async () => {
    current = {
      ...seeded,
      store: { ...store, name: "Superette El Baraka", rc: "16/00-1234567 B 20" },
    };
    mount();
    expect(await screen.findByLabelText(fr.field_name)).toHaveValue("Superette El Baraka");
    expect(screen.getByLabelText(fr.field_rc)).toHaveValue("16/00-1234567 B 20");
    expect(screen.getByLabelText(fr.field_nif)).toHaveValue("");
  });

  test("a server refusal on load is shown translated", async () => {
    fetchMock.mockImplementation(() =>
      Promise.resolve(json(401, { error: { code: "unauthorized", message: "no" } })),
    );
    mount();
    expect(await screen.findByRole("alert")).toHaveTextContent(fr.error_unauthorized);
  });
});

describe("the store form", () => {
  test("puts the whole block, blanks as null, and says it saved", async () => {
    const user = userEvent.setup();
    mount();
    const name = await screen.findByLabelText(fr.field_name);
    await user.clear(name);
    await user.type(name, "  Superette El Baraka ");
    await user.type(screen.getByLabelText(fr.field_rc), "16/00-1234567 B 20");
    await user.type(screen.getByLabelText(fr.field_nif), "000016001234567");
    await user.type(screen.getByLabelText(fr.field_phone), "0555 12 34 56");
    const storeForm = screen.getByRole("form", { name: fr.settings_store });
    await user.click(within(storeForm).getByRole("button", { name: fr.action_save }));

    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent(fr.settings_saved));
    const put = sent("PUT");
    expect(put.url).toMatch(/\/settings\/store$/);
    expect(put.body).toEqual({
      name: "Superette El Baraka",
      rc: "16/00-1234567 B 20",
      nif: "000016001234567",
      nis: null,
      ai: null,
      address: null,
      phone: "0555 12 34 56",
    });
  });

  test("a blank name never leaves the screen", async () => {
    const user = userEvent.setup();
    mount();
    const name = await screen.findByLabelText(fr.field_name);
    await user.clear(name);
    const storeForm = screen.getByRole("form", { name: fr.settings_store });
    await user.click(within(storeForm).getByRole("button", { name: fr.action_save }));
    expect(await screen.findByRole("alert")).toHaveTextContent(fr.error_name_required);
    expect(countOf("PUT")).toBe(0);
  });

  test("a refusal from the API is shown translated and nothing says saved", async () => {
    storeAnswer = () => json(422, { error: { code: "validation", message: "address is invalid" } });
    const user = userEvent.setup();
    mount();
    await screen.findByLabelText(fr.field_name);
    const storeForm = screen.getByRole("form", { name: fr.settings_store });
    await user.click(within(storeForm).getByRole("button", { name: fr.action_save }));
    expect(await within(storeForm).findByRole("alert")).toHaveTextContent(fr.error_validation);
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });
});

describe("the régime room", () => {
  test("shows the régime in force with its date, and no planned line when there is none", async () => {
    mount("fr", "/settings/regime");
    expect(await screen.findByTestId("regime-current")).toHaveTextContent(
      `${fr.regime_reel} · ${fr.regime_since} 2026-01-01`,
    );
    expect(screen.queryByTestId("regime-planned")).not.toBeInTheDocument();
  });

  test("shows a planned change when the API reports one", async () => {
    current = { ...seeded, regime_planned: { regime: "ifu", valid_from: "2027-01-01" } };
    mount("fr", "/settings/regime");
    expect(await screen.findByTestId("regime-planned")).toHaveTextContent(
      `${fr.regime_ifu} · ${fr.regime_from} 2027-01-01`,
    );
  });

  test("dates its default from the shop's calendar and not the machine's", async () => {
    // The one thing this asserts is where the day came from. The stub
    // answers a fixed day for `/clock`; a screen reading `new Date()` would
    // fill in whatever the box running the suite happens to be on, which is
    // the fork this replaced (the core dates documents on Algeria's
    // calendar, UTC+1, and a browser reads the machine's zone).
    mount("fr", "/settings/regime");
    await screen.findByRole("form", { name: fr.settings_regime });
    expectIsoDate("regime-valid-from", SHOP_TODAY);
  });

  test("says it is reading the shop's day while it waits, not that products are loading", async () => {
    // Its own line: the settings screen has no product list on it, and a
    // panel that borrowed the products' string would say something the
    // screen cannot back up.
    clockAnswer = () => new Promise<Response>(() => undefined);
    mount("fr", "/settings/regime");

    expect(await screen.findByText(fr.regime_loading)).toBeInTheDocument();
    expect(screen.queryByRole("form", { name: fr.settings_regime })).not.toBeInTheDocument();
  });

  test("a clock the server will not answer is an error line with a retry, not a wait", async () => {
    // The day is asked of the server, so it can fail like any other call. A
    // panel that read the answer as "not here yet" would sit on its loading
    // line for as long as the window stayed open, with no way back and
    // nothing on the screen saying why.
    clockAnswer = () => json(500, { error: { code: "storage", message: "no" } });
    const user = userEvent.setup();
    mount("fr", "/settings/regime");

    expect(await screen.findByRole("alert")).toHaveTextContent(fr.error_storage);
    expect(screen.queryByRole("form", { name: fr.settings_regime })).not.toBeInTheDocument();

    clockAnswer = null;
    await user.click(screen.getByRole("button", { name: fr.action_retry }));

    await screen.findByRole("form", { name: fr.settings_regime });
    expectIsoDate("regime-valid-from", SHOP_TODAY);
  });

  test("posts the régime and the day, and shows the planned line the API answers", async () => {
    const user = userEvent.setup();
    mount("fr", "/settings/regime");
    const regimeForm = await screen.findByRole("form", { name: fr.settings_regime });
    await chooseRegime(user, regimeForm, fr.regime_ifu);
    await typeIsoDate(user, "regime-valid-from", "2099-01-01");
    await user.click(within(regimeForm).getByRole("button", { name: fr.action_apply }));

    expect(await screen.findByTestId("regime-planned")).toHaveTextContent(
      `${fr.regime_ifu} · ${fr.regime_from} 2099-01-01`,
    );
    expect(screen.getByTestId("regime-current")).toHaveTextContent(fr.regime_reel);
    const post = sent("POST");
    expect(post.url).toMatch(/\/settings\/regime$/);
    expect(post.body).toEqual({ regime: "ifu", valid_from: "2099-01-01" });
  });

  test("a change dated in the past becomes the régime in force", async () => {
    const user = userEvent.setup();
    mount("fr", "/settings/regime");
    const regimeForm = await screen.findByRole("form", { name: fr.settings_regime });
    await chooseRegime(user, regimeForm, fr.regime_ifu);
    await typeIsoDate(user, "regime-valid-from", "2026-06-01");
    await user.click(within(regimeForm).getByRole("button", { name: fr.action_apply }));
    await waitFor(() =>
      expect(screen.getByTestId("regime-current")).toHaveTextContent(
        `${fr.regime_ifu} · ${fr.regime_since} 2026-06-01`,
      ),
    );
    expect(screen.queryByTestId("regime-planned")).not.toBeInTheDocument();
  });

  test("an empty day is refused on the screen and nothing is posted", async () => {
    const user = userEvent.setup();
    mount("fr", "/settings/regime");
    const regimeForm = await screen.findByRole("form", { name: fr.settings_regime });
    await chooseRegime(user, regimeForm, fr.regime_ifu);
    const segments = dateFieldSegments("regime-valid-from");
    await user.clear(segments.day);
    await user.clear(segments.month);
    await user.clear(segments.year);
    await user.click(within(regimeForm).getByRole("button", { name: fr.action_apply }));
    expect(await within(regimeForm).findByRole("alert")).toHaveTextContent(fr.error_day_invalid);
    expect(countOf("POST")).toBe(0);
  });

  test("a refusal from the API is shown translated", async () => {
    regimeAnswer = () => json(422, { error: { code: "validation", message: "valid_from" } });
    const user = userEvent.setup();
    mount("fr", "/settings/regime");
    const regimeForm = await screen.findByRole("form", { name: fr.settings_regime });
    await chooseRegime(user, regimeForm, fr.regime_ifu);
    await user.click(within(regimeForm).getByRole("button", { name: fr.action_apply }));
    expect(await within(regimeForm).findByRole("alert")).toHaveTextContent(fr.error_validation);
  });

  test("apply is off while the régime chosen is the one in force, on again once it differs", async () => {
    const user = userEvent.setup();
    mount("fr", "/settings/regime");
    const regimeForm = await screen.findByRole("form", { name: fr.settings_regime });
    const apply = within(regimeForm).getByRole("button", { name: fr.action_apply });
    expect(apply).toBeDisabled();
    await user.click(apply);
    expect(countOf("POST")).toBe(0);
    await chooseRegime(user, regimeForm, fr.regime_ifu);
    expect(apply).toBeEnabled();
  });

  test("with a change planned, re-applying the régime in force stays possible: it cancels the plan", async () => {
    current = { ...seeded, regime_planned: { regime: "ifu", valid_from: "2099-01-01" } };
    mount("fr", "/settings/regime");
    await screen.findByTestId("regime-planned");
    const regimeForm = screen.getByRole("form", { name: fr.settings_regime });
    expect(within(regimeForm).getByRole("button", { name: fr.action_apply })).toBeEnabled();
  });
});

describe("in Arabic", () => {
  test("the fiscal identifiers and the phone stay left to right, the address does not", async () => {
    current = {
      ...seeded,
      store: { ...store, name: "Superette El Baraka", rc: "16/00-1234567 B 20" },
    };
    mount("ar");
    expect(await screen.findByLabelText(ar.field_rc)).toHaveAttribute("dir", "ltr");
    expect(screen.getByLabelText(ar.field_nif)).toHaveAttribute("dir", "ltr");
    expect(screen.getByLabelText(ar.field_nis)).toHaveAttribute("dir", "ltr");
    expect(screen.getByLabelText(ar.field_ai)).toHaveAttribute("dir", "ltr");
    expect(screen.getByLabelText(ar.field_phone)).toHaveAttribute("dir", "ltr");
    expect(screen.getByLabelText(ar.field_address)).not.toHaveAttribute("dir");
  });

  test("the régime dates stay left to right inside the RTL panel", async () => {
    current = { ...seeded, regime_planned: { regime: "ifu", valid_from: "2027-01-01" } };
    mount("ar", "/settings/regime");
    const current_ = await screen.findByTestId("regime-current");
    expect(within(current_).getByText("2026-01-01")).toHaveAttribute("dir", "ltr");
    const planned = screen.getByTestId("regime-planned");
    expect(within(planned).getByText("2027-01-01")).toHaveAttribute("dir", "ltr");
  });
});

describe("the printing room", () => {
  test("shows the layout the shop is on and what it means", async () => {
    mount("fr", "/settings/printing");
    const picker = await screen.findByRole("combobox", { name: fr.settings_facture_layout });
    expect(picker).toHaveTextContent(fr.facture_layout_standard);
    expect(screen.getByText(fr.facture_layout_standard_hint)).toBeInTheDocument();
  });

  test("sends the layout that was picked and shows the answer", async () => {
    const user = userEvent.setup();
    mount("fr", "/settings/printing");
    const picker = await screen.findByRole("combobox", { name: fr.settings_facture_layout });
    await user.click(picker);
    await user.click(await screen.findByRole("option", { name: fr.facture_layout_compact }));
    await waitFor(() => expect(sent("PUT").url).toContain("/settings/facture-layout"));
    expect(sent("PUT").body).toEqual({ facture_layout: "compact" });
    // The answer is the whole page, so the screen reads the new layout back
    // rather than trusting the click that sent it.
    await waitFor(() => expect(picker).toHaveTextContent(fr.facture_layout_compact));
    expect(screen.getByText(fr.facture_layout_compact_hint)).toBeInTheDocument();
  });

  /** Every layout the server names is on the screen under its own wording.
   * Driven by the fixture's own list, so a layout added in Rust and given
   * its three translations is covered here without this test being edited,
   * and one added without wording fails rather than appearing as a code
   * name. */
  test("names every layout the server offers", async () => {
    const user = userEvent.setup();
    mount("fr", "/settings/printing");
    const picker = await screen.findByRole("combobox", { name: fr.settings_facture_layout });
    await user.click(picker);
    for (const layout of seeded.facture_layouts) {
      const label = fr[`facture_layout_${layout}` as keyof typeof fr];
      expect(label).toBeTruthy();
      expect(await screen.findByRole("option", { name: label })).toBeInTheDocument();
    }
  });

  test("sends the half sheet and says it prints on A5", async () => {
    const user = userEvent.setup();
    mount("fr", "/settings/printing");
    const picker = await screen.findByRole("combobox", { name: fr.settings_facture_layout });
    await user.click(picker);
    await user.click(await screen.findByRole("option", { name: fr.facture_layout_half_sheet }));
    await waitFor(() => expect(sent("PUT").url).toContain("/settings/facture-layout"));
    expect(sent("PUT").body).toEqual({ facture_layout: "half_sheet" });
    await waitFor(() => expect(picker).toHaveTextContent(fr.facture_layout_half_sheet));
    expect(screen.getByText(fr.facture_layout_half_sheet_hint)).toBeInTheDocument();
  });

  /** The list is the server's. A build that offered every layout it has
   * heard of would show a shop an option its own server cannot print. */
  test("offers only the layouts the server named", async () => {
    const user = userEvent.setup();
    current = { ...seeded, facture_layouts: ["standard"] };
    mount("fr", "/settings/printing");
    const picker = await screen.findByRole("combobox", { name: fr.settings_facture_layout });
    await user.click(picker);
    expect(await screen.findByRole("option", { name: fr.facture_layout_standard })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: fr.facture_layout_compact })).not.toBeInTheDocument();
  });
});
