// T24/T38: the shop's identity is asked once, right after the owner claims
// the shop, skippable, and reachable again later from Paramètres → Magasin.
// Driven through the real `SessionProvider` (not a mocked hook), the way
// `FirstSetupScreen.test.tsx` drives the screen before this one: what is
// proved is the whole seam, `claimFirstOwner` raising the step and either
// path (skip, or a save that answers) lowering it again.

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import type { ReactNode } from "react";
import type { StoreDto } from "@dzpos/shared";

import { I18nProvider } from "@/i18n";
import fr from "@/i18n/fr.json";
import { SessionProvider, useSession } from "@/lib/session";

import { ShopIdentityOnboarding } from "../../src/components/ShopIdentityOnboarding";

const store: StoreDto = {
  name: "Mon magasin",
  rc: null,
  nif: null,
  nis: null,
  ai: null,
  address: null,
  phone: null,
};

const SEEDED_SETTINGS = {
  store,
  regime: { regime: "reel", valid_from: "2026-01-01" },
  regime_planned: null,
  theme: null,
  facture_layout: "standard",
  facture_layouts: ["standard", "compact", "half_sheet", "roll_80mm"],
  print_lang: null,
  thermal_mode: "text",
  discount_threshold_bps: 0,
  ticket_fiscal_ids: false,
};

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });
}

let fetchMock: ReturnType<typeof vi.fn>;
let settings: typeof SEEDED_SETTINGS;

beforeEach(() => {
  settings = { ...SEEDED_SETTINGS, store: { ...store } };
  fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
    const url = String(input);
    if (url.endsWith("/health")) {
      return Promise.resolve(json(200, { status: "ok", shop_id: 1, needs_first_setup: false }));
    }
    if (url.endsWith("/auth/me")) {
      return Promise.resolve(json(401, { error: { code: "session_required", message: "no" } }));
    }
    if (url.endsWith("/auth/first-setup")) {
      return Promise.resolve(
        json(200, {
          me: { user_id: 1, name: "Propriétaire", role: "owner", permissions: [] },
          token: "owner-token",
          idle_minutes: 15,
        }),
      );
    }
    if (url.endsWith("/settings") && (init?.method ?? "GET") === "GET") {
      return Promise.resolve(json(200, settings));
    }
    if (url.endsWith("/settings/store")) {
      const body: unknown = init?.body === undefined ? {} : JSON.parse(String(init.body));
      settings = { ...settings, store: { ...store, ...(body as Partial<StoreDto>) } };
      return Promise.resolve(json(200, settings.store));
    }
    return Promise.resolve(json(404, { error: { code: "not_found", message: "no" } }));
  });
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
});

/** Exposes `claimFirstOwner` through a button, since the screen under test
 *  never calls it itself — `FirstSetupScreen` does, one screen earlier. */
function Harness() {
  const { freshOwner, claimFirstOwner } = useSession();
  return (
    <div>
      <button
        type="button"
        data-testid="do-claim"
        onClick={() => void claimFirstOwner("Propriétaire", "developpement12")}
      >
        claim
      </button>
      {freshOwner ? <ShopIdentityOnboarding /> : <div data-testid="past-onboarding" />}
    </div>
  );
}

function mount(): void {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  function Wrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={client}>
        <I18nProvider lang="fr">
          <SessionProvider>{children}</SessionProvider>
        </I18nProvider>
      </QueryClientProvider>
    );
  }
  render(<Harness />, { wrapper: Wrapper });
}

describe("ShopIdentityOnboarding", () => {
  test("raised right after the shop is claimed, and skip lowers it again", async () => {
    const user = userEvent.setup();
    mount();
    await user.click(screen.getByTestId("do-claim"));

    const step = await screen.findByTestId("onboarding-shop-screen");
    expect(within(step).getByText(fr.onboarding_shop_title)).toBeInTheDocument();

    await user.click(screen.getByTestId("onboarding-shop-skip"));
    await waitFor(() => expect(screen.queryByTestId("onboarding-shop-screen")).not.toBeInTheDocument());
    expect(screen.getByTestId("past-onboarding")).toBeInTheDocument();
  });

  test("saving the shop's identity from the step lowers it too", async () => {
    const user = userEvent.setup();
    mount();
    await user.click(screen.getByTestId("do-claim"));
    await screen.findByTestId("onboarding-shop-screen");

    const nameField = screen.getByLabelText(fr.field_name);
    await user.clear(nameField);
    await user.type(nameField, "Superette El Baraka");
    await user.click(screen.getByRole("button", { name: fr.action_save }));

    await waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        expect.stringContaining("/settings/store"),
        expect.objectContaining({ method: "PUT" }),
      ),
    );
    await waitFor(() => expect(screen.queryByTestId("onboarding-shop-screen")).not.toBeInTheDocument());
    expect(screen.getByTestId("past-onboarding")).toBeInTheDocument();
  });
});
