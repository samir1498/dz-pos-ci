import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import type { ReactNode } from "react";

import { I18nProvider } from "@/i18n";
import { SessionProvider } from "@/lib/session";

import { FirstSetupScreen } from "../../src/components/FirstSetupScreen";

const json = (status: number, body: unknown): Response =>
  new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });

beforeEach(() => {
  vi.stubGlobal(
    "fetch",
    vi.fn((input: unknown) => {
      const url = String(input);
      if (url.endsWith("/health")) {
        return Promise.resolve(json(200, { status: "ok", shop_id: 1, needs_first_setup: true }));
      }
      return Promise.resolve(json(404, { error: { code: "not_found", message: "no" } }));
    }),
  );
});

afterEach(() => {
  vi.unstubAllGlobals();
});

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
  render(<FirstSetupScreen />, { wrapper: Wrapper });
}

const OWNER_SESSION = {
  me: { user_id: 1, name: "Anouar", role: "owner", permissions: ["manage_users"] },
  token: "owner-token",
  idle_minutes: 15,
};

function stubFetch(handler: (url: string, init: RequestInit | undefined) => Response): void {
  vi.stubGlobal(
    "fetch",
    vi.fn((input: unknown, init?: RequestInit) => Promise.resolve(handler(String(input), init))),
  );
}

describe("FirstSetupScreen", () => {
  test("matching passwords post the trimmed name and show no error", async () => {
    const posted: unknown[] = [];
    stubFetch((url, init) => {
      if (url.endsWith("/auth/me")) {
        return json(401, { error: { code: "session_required", message: "no" } });
      }
      if (url.endsWith("/auth/first-setup")) {
        posted.push(JSON.parse(String(init?.body)));
        return json(200, OWNER_SESSION);
      }
      return json(404, { error: { code: "not_found", message: "no" } });
    });
    const user = userEvent.setup();
    mount();
    await screen.findByTestId("setup-screen");

    await user.type(screen.getByTestId("setup-name"), "  Anouar  ");
    await user.type(screen.getByTestId("setup-password"), "huit caracteres");
    await user.type(screen.getByTestId("setup-confirm"), "huit caracteres");
    await user.click(screen.getByTestId("setup-submit"));

    await waitFor(() => expect(posted).toHaveLength(1));
    expect(posted[0]).toEqual({ name: "Anouar", password: "huit caracteres" });
    expect(screen.queryByTestId("setup-error")).toBeNull();
  });

  test("a refused claim shows the server sentence", async () => {
    stubFetch((url) => {
      if (url.endsWith("/auth/me")) {
        return json(401, { error: { code: "session_required", message: "no" } });
      }
      if (url.endsWith("/auth/first-setup")) {
        return json(422, { error: { code: "validation", message: "no" } });
      }
      return json(404, { error: { code: "not_found", message: "no" } });
    });
    const user = userEvent.setup();
    mount();
    await screen.findByTestId("setup-screen");

    await user.type(screen.getByTestId("setup-name"), "Anouar");
    await user.type(screen.getByTestId("setup-password"), "huit caracteres");
    await user.type(screen.getByTestId("setup-confirm"), "huit caracteres");
    await user.click(screen.getByTestId("setup-submit"));

    expect(await screen.findByTestId("setup-error")).toHaveTextContent(
      "Vérifiez les champs saisis.",
    );
  });

  test("shows no error under the name field before it is touched or submitted (T1)", async () => {
    mount();
    await screen.findByTestId("setup-screen");
    // The bug: "Obligatoire." rendered under "Votre nom" the instant the
    // screen opened, with nothing typed and nothing submitted yet.
    expect(screen.queryByText("Obligatoire.")).toBeNull();
    expect(screen.getByTestId("setup-name")).toHaveAttribute("aria-invalid", "false");
  });

  test("shows the error under the name field once it is blurred empty", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByTestId("setup-screen");
    await user.click(screen.getByTestId("setup-name"));
    await user.tab();
    expect(screen.getByText("Obligatoire.")).toBeInTheDocument();
  });

  test("the submit stays enabled and an empty form is refused with an error", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByTestId("setup-screen");
    expect(screen.getByTestId("setup-submit")).toBeEnabled();
    await user.click(screen.getByTestId("setup-submit"));
    expect(screen.getByTestId("setup-error")).toHaveTextContent("Obligatoire.");
  });

  test("two different passwords are refused and the confirm box keeps its value", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByTestId("setup-screen");

    await user.type(screen.getByTestId("setup-name"), "Anouar");
    await user.type(screen.getByTestId("setup-password"), "huit caracteres");
    await user.type(screen.getByTestId("setup-confirm"), "autre mot de passe");
    await user.click(screen.getByTestId("setup-submit"));
    expect(screen.getByTestId("setup-error")).toHaveTextContent(
      "Les deux mots de passe ne sont pas les mêmes.",
    );
    expect(screen.getByTestId("setup-confirm")).toHaveValue("autre mot de passe");
  });
});
