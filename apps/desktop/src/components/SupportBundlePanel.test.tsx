// The support bundle button on the settings screen. What the zip actually
// holds is the Rust suite's business (`crates/api/tests/support_bundle.rs`);
// this checks the one call the button makes and the name it saves under.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { I18nProvider } from "@/i18n";
import fr from "@/i18n/fr.json";
import { SupportBundlePanel } from "./SupportBundlePanel";

function zip(filename: string): Response {
  return new Response(new Blob([new Uint8Array([0x50, 0x4b, 0x03, 0x04])]), {
    status: 200,
    headers: {
      "content-type": "application/zip",
      "content-disposition": `attachment; filename="${filename}"`,
    },
  });
}

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function gets(): string[] {
  return fetchMock.mock.calls
    .filter((call) => {
      const init: unknown = call[1];
      return (
        init === undefined ||
        (typeof init === "object" && init !== null && Reflect.get(init, "method") === undefined)
      );
    })
    .map((call) => String(call[0]));
}

function mount() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <I18nProvider lang="fr">
      <QueryClientProvider client={client}>
        <SupportBundlePanel />
      </QueryClientProvider>
    </I18nProvider>,
  );
}

let fetchMock: ReturnType<typeof vi.fn>;
let refused: boolean;
let saved: { filename: string }[];

beforeEach(() => {
  refused = false;
  saved = [];
  // jsdom has no download: the anchor click is a no-op and the object URL
  // is not implemented, so both are stubbed and what the test reads is the
  // name the panel would have saved under.
  vi.stubGlobal(
    "URL",
    Object.assign(URL, { createObjectURL: () => "blob:x", revokeObjectURL() {} }),
  );
  vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (
    this: HTMLAnchorElement,
  ) {
    saved.push({ filename: this.download });
  });

  fetchMock = vi.fn((input: unknown) => {
    const url = String(input);
    if (url.includes("/support-bundle")) {
      if (refused) {
        return Promise.resolve(
          json(403, { error: { code: "forbidden", message: "no" } }),
        );
      }
      return Promise.resolve(zip("dzpos-support-2026-09-11.zip"));
    }
    return Promise.resolve(json(404, { error: { code: "not_found", message: "no" } }));
  });
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("the support bundle button", () => {
  test("asks the one route and saves under the name the server gave it", async () => {
    const user = userEvent.setup();
    mount();
    await user.click(screen.getByTestId("support-bundle-download"));
    await waitFor(() => expect(saved).toHaveLength(1));
    expect(gets().some((url) => url.includes("/support-bundle"))).toBe(true);
    expect(saved[0]?.filename).toBe("dzpos-support-2026-09-11.zip");
    expect(await screen.findByTestId("support-bundle-done")).toHaveTextContent(
      fr.support_bundle_done,
    );
  });

  test("a refusal is shown and nothing is saved", async () => {
    refused = true;
    const user = userEvent.setup();
    mount();
    await user.click(screen.getByTestId("support-bundle-download"));
    expect(await screen.findByRole("alert")).toBeInTheDocument();
    expect(saved).toHaveLength(0);
  });
});
