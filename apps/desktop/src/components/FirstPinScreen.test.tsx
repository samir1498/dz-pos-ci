import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import type { ReactNode } from "react";

import { I18nProvider } from "@/i18n";
import { SessionProvider } from "@/lib/session";

import { FirstPinScreen } from "./FirstPinScreen";

const json = (status: number, body: unknown): Response =>
  new Response(JSON.stringify(body), { status, headers: { "content-type": "application/json" } });

beforeEach(() => {
  vi.stubGlobal(
    "fetch",
    vi.fn((input: unknown) => {
      const url = String(input);
      if (url.endsWith("/health")) {
        return Promise.resolve(json(200, { status: "ok", shop_id: 1, needs_first_pin: true }));
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
  render(<FirstPinScreen />, { wrapper: Wrapper });
}

describe("FirstPinScreen", () => {
  test("two different PINs are refused and the confirm box is emptied", async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByTestId("firstpin-screen");

    await user.keyboard("2580");
    await user.keyboard("{Enter}");
    expect(screen.getByText("Retapez-le")).toBeInTheDocument();

    await user.keyboard("2581");
    await user.keyboard("{Enter}");
    expect(screen.getByTestId("firstpin-error")).toHaveTextContent("Les deux codes ne sont pas les mêmes.");
    expect(screen.getByTestId("firstpin-display").textContent?.replace(/\s/g, "")).toBe("");
  });
});
