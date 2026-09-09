// The settings screen is checked for rendering and wiring: what it shows
// from the API's answer and what it sends. The rules (a blank name, a bad
// day) are the API crate's tests.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { DatedRegimeDto, RegimeDto, SettingsDto, StoreDto } from "@dzpos/shared";
import { I18nProvider, type Lang } from "@/i18n";
import fr from "@/i18n/fr.json";
import ar from "@/i18n/ar.json";
import { SettingsScreen } from "./settings";

const store: StoreDto = {
  name: "Mon magasin",
  rc: null,
  nif: null,
  nis: null,
  ai: null,
  address: null,
  phone: null,
};

const seeded: SettingsDto = {
  store,
  regime: { regime: "reel", valid_from: "2026-01-01" },
  regime_planned: null,
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

function mount(lang: Lang = "fr") {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <I18nProvider lang={lang}>
      <QueryClientProvider client={client}>
        <SettingsScreen />
      </QueryClientProvider>
    </I18nProvider>,
  );
}

let fetchMock: ReturnType<typeof vi.fn>;
let current: SettingsDto;
let storeAnswer: (() => Response) | null;
let regimeAnswer: (() => Response) | null;

beforeEach(() => {
  current = seeded;
  storeAnswer = null;
  regimeAnswer = null;
  fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
    const url = String(input);
    if (init?.method === "PUT" && url.endsWith("/settings/store")) {
      if (storeAnswer !== null) return Promise.resolve(storeAnswer());
      const body: unknown = JSON.parse(String(init.body));
      if (typeof body !== "object" || body === null) throw new Error("no body");
      current = { ...current, store: { ...store, ...body } };
      return Promise.resolve(json(200, current.store));
    }
    if (init?.method === "POST" && url.endsWith("/settings/regime")) {
      if (regimeAnswer !== null) return Promise.resolve(regimeAnswer());
      const body: unknown = JSON.parse(String(init.body));
      if (typeof body !== "object" || body === null) throw new Error("no body");
      const regime: RegimeDto = "regime" in body && body.regime === "reel" ? "reel" : "ifu";
      const validFrom = "valid_from" in body ? String(body.valid_from) : "";
      // The stub applies the API's rule: a day past today is planned.
      const today = new Date().toISOString().slice(0, 10);
      const dated: DatedRegimeDto = { regime, valid_from: validFrom };
      current =
        validFrom > today
          ? { ...current, regime_planned: dated }
          : { ...current, regime: dated, regime_planned: null };
      return Promise.resolve(json(200, current));
    }
    // The page carries the backups block too; it asks for the list as soon
    // as the settings load, and an unanswered call would leave a second
    // alert on the screen these tests read.
    if (url.endsWith("/backups")) return Promise.resolve(json(200, []));
    if (url.endsWith("/settings")) return Promise.resolve(json(200, current));
    return Promise.resolve(json(404, { error: { code: "not_found", message: "no" } }));
  });
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("the page", () => {
  test("shows the store block and the régime in force with its date", async () => {
    current = {
      ...seeded,
      store: { ...store, name: "Superette El Baraka", rc: "16/00-1234567 B 20" },
    };
    mount();
    expect(await screen.findByLabelText(fr.field_name)).toHaveValue("Superette El Baraka");
    expect(screen.getByLabelText(fr.field_rc)).toHaveValue("16/00-1234567 B 20");
    expect(screen.getByLabelText(fr.field_nif)).toHaveValue("");
    expect(screen.getByTestId("regime-current")).toHaveTextContent(
      `${fr.regime_reel} · ${fr.regime_since} 2026-01-01`,
    );
    expect(screen.queryByTestId("regime-planned")).not.toBeInTheDocument();
  });

  test("shows a planned change when the API reports one", async () => {
    current = { ...seeded, regime_planned: { regime: "ifu", valid_from: "2027-01-01" } };
    mount();
    expect(await screen.findByTestId("regime-planned")).toHaveTextContent(
      `${fr.regime_ifu} · ${fr.regime_from} 2027-01-01`,
    );
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

describe("the régime form", () => {
  test("posts the régime and the day, and shows the planned line the API answers", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByLabelText(fr.field_name);
    const regimeForm = screen.getByRole("form", { name: fr.settings_regime });
    await user.selectOptions(within(regimeForm).getByLabelText(fr.field_regime), "ifu");
    const day = within(regimeForm).getByLabelText(fr.field_valid_from);
    await user.clear(day);
    await user.type(day, "2099-01-01");
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
    mount();
    await screen.findByLabelText(fr.field_name);
    const regimeForm = screen.getByRole("form", { name: fr.settings_regime });
    await user.selectOptions(within(regimeForm).getByLabelText(fr.field_regime), "ifu");
    const day = within(regimeForm).getByLabelText(fr.field_valid_from);
    await user.clear(day);
    await user.type(day, "2026-06-01");
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
    mount();
    await screen.findByLabelText(fr.field_name);
    const regimeForm = screen.getByRole("form", { name: fr.settings_regime });
    await user.selectOptions(within(regimeForm).getByLabelText(fr.field_regime), "ifu");
    await user.clear(within(regimeForm).getByLabelText(fr.field_valid_from));
    await user.click(within(regimeForm).getByRole("button", { name: fr.action_apply }));
    expect(await within(regimeForm).findByRole("alert")).toHaveTextContent(fr.error_day_invalid);
    expect(countOf("POST")).toBe(0);
  });

  test("a refusal from the API is shown translated", async () => {
    regimeAnswer = () => json(422, { error: { code: "validation", message: "valid_from" } });
    const user = userEvent.setup();
    mount();
    await screen.findByLabelText(fr.field_name);
    const regimeForm = screen.getByRole("form", { name: fr.settings_regime });
    await user.selectOptions(within(regimeForm).getByLabelText(fr.field_regime), "ifu");
    await user.click(within(regimeForm).getByRole("button", { name: fr.action_apply }));
    expect(await within(regimeForm).findByRole("alert")).toHaveTextContent(fr.error_validation);
  });

  test("apply is off while the régime chosen is the one in force, on again once it differs", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByLabelText(fr.field_name);
    const regimeForm = screen.getByRole("form", { name: fr.settings_regime });
    const apply = within(regimeForm).getByRole("button", { name: fr.action_apply });
    expect(apply).toBeDisabled();
    await user.click(apply);
    expect(countOf("POST")).toBe(0);
    await user.selectOptions(within(regimeForm).getByLabelText(fr.field_regime), "ifu");
    expect(apply).toBeEnabled();
  });

  test("with a change planned, re-applying the régime in force stays possible: it cancels the plan", async () => {
    current = { ...seeded, regime_planned: { regime: "ifu", valid_from: "2099-01-01" } };
    mount();
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
    mount("ar");
    const current_ = await screen.findByTestId("regime-current");
    expect(within(current_).getByText("2026-01-01")).toHaveAttribute("dir", "ltr");
    const planned = screen.getByTestId("regime-planned");
    expect(within(planned).getByText("2027-01-01")).toHaveAttribute("dir", "ltr");
  });
});
