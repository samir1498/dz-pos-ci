// The exports and the import block on the settings screen: what it asks
// for, what it posts, and what it refuses to post. The rules (which rows a
// file may carry, what a barcode already in the shop does) are the core's
// and the API crate's tests.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ImportDryRunDto } from "@dzpos/shared";
import { I18nProvider } from "@/i18n";
import fr from "@/i18n/fr.json";
import { ExportImportPanel } from "./ExportImportPanel";

/**
 * The rows of the dry-run report. The report is a `DataTable` now, so a row
 * is a `<tr>` inside it rather than an element carrying its own id; the
 * header row is dropped because it describes the columns and not the file.
 */
function importRows(): HTMLElement[] {
  const table = screen.queryByTestId("import-table");
  if (table === null) return [];
  return within(table).getAllByRole("row").slice(1);
}

/** The range boxes are DateField now: three segments each, so a test fills
 *  in the day, the month and the year rather than typing one ISO string. */
async function typeIsoDate(user: ReturnType<typeof userEvent.setup>, testId: string, iso: string) {
  const [year, month, day] = iso.split("-");
  await user.type(screen.getByTestId(`${testId}-day`), day ?? "");
  await user.type(screen.getByTestId(`${testId}-month`), month ?? "");
  await user.type(screen.getByTestId(`${testId}-year`), year ?? "");
}

/** A workbook, as far as this screen is concerned: bytes with the media type
 * and the name the server puts on them. */
function workbook(filename: string): Response {
  return new Response(new Blob([new Uint8Array([0x50, 0x4b, 0x03, 0x04])]), {
    status: 200,
    headers: {
      "content-type": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
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

const CLEAN: ImportDryRunDto = {
  rows: [{ row: 2, name: "Café moulu 250 g", outcome: "created", field: null, reason: null }],
  accepted: 1,
  refused: 0,
};

const DIRTY: ImportDryRunDto = {
  rows: [
    { row: 2, name: "Café moulu 250 g", outcome: "created", field: null, reason: null },
    { row: 3, name: "Sac de semoule", outcome: "refused", field: "unit", reason: "unknown_unit" },
  ],
  accepted: 1,
  refused: 1,
};

function urls(method: string): string[] {
  return fetchMock.mock.calls
    .filter((call) => {
      const init: unknown = call[1];
      return typeof init === "object" && init !== null && Reflect.get(init, "method") === method;
    })
    .map((call) => String(call[0]));
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
        <ExportImportPanel />
      </QueryClientProvider>
    </I18nProvider>,
  );
}

/** The file the picker hands over. Its bytes never leave this test: the
 * screen posts the `File` straight through, and what is asserted is the
 * route and the method. */
function xlsx(): File {
  return new File([new Uint8Array([0x50, 0x4b, 0x03, 0x04])], "produits.xlsx", {
    type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
  });
}

let fetchMock: ReturnType<typeof vi.fn>;
let dryRun: ImportDryRunDto;
let applyAnswer: (() => Response) | null;
let saved: { filename: string }[];

beforeEach(() => {
  dryRun = CLEAN;
  applyAnswer = null;
  saved = [];
  // jsdom has no download: the anchor click is a no-op and the object URL
  // is not implemented, so both are stubbed and what the test reads is the
  // name the panel would have saved under.
  vi.stubGlobal("URL", Object.assign(URL, { createObjectURL: () => "blob:x", revokeObjectURL() {} }));
  vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (this: HTMLAnchorElement) {
    saved.push({ filename: this.download });
  });

  fetchMock = vi.fn((input: unknown, init?: RequestInit) => {
    const url = String(input);
    if (init?.method === "POST" && url.includes("/import/products/dry-run")) {
      return Promise.resolve(json(200, dryRun));
    }
    if (init?.method === "POST" && url.includes("/import/products")) {
      if (applyAnswer !== null) return Promise.resolve(applyAnswer());
      return Promise.resolve(json(200, { created: 1, updated: 0, categories_created: 1 }));
    }
    if (url.includes("/import/products/template")) {
      return Promise.resolve(workbook("modele-produits-2026-09-10.xlsx"));
    }
    if (url.includes("/export/")) {
      const kind = url.split("/export/")[1]?.split("?")[0] ?? "x";
      return Promise.resolve(workbook(`${kind}-2026-09-10.xlsx`));
    }
    return Promise.resolve(json(404, { error: { code: "not_found", message: "no" } }));
  });
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("the four exports", () => {
  test("each asks its own route in the screen's language and saves under the server's name", async () => {
    const user = userEvent.setup();
    mount();
    await user.click(screen.getByTestId("export-products"));
    await waitFor(() => expect(saved).toHaveLength(1));
    expect(gets().some((url) => url.includes("/export/products?lang=fr"))).toBe(true);
    expect(saved[0]?.filename).toBe("products-2026-09-10.xlsx");

    await user.click(screen.getByTestId("export-customers"));
    await waitFor(() => expect(saved).toHaveLength(2));
    expect(gets().some((url) => url.includes("/export/customers?lang=fr"))).toBe(true);
  });

  test("the range rides on the sales export and on nothing else", async () => {
    const user = userEvent.setup();
    mount();
    await typeIsoDate(user, "export-range-from", "2026-01-01");
    await typeIsoDate(user, "export-range-to", "2026-12-31");

    await user.click(screen.getByTestId("export-sales"));
    await waitFor(() => expect(saved).toHaveLength(1));
    const sales = gets().find((url) => url.includes("/export/sales"));
    expect(sales).toContain("from=2026-01-01");
    expect(sales).toContain("to=2026-12-31");

    await user.click(screen.getByTestId("export-products"));
    await waitFor(() => expect(saved).toHaveLength(2));
    const products = gets().find((url) => url.includes("/export/products"));
    expect(products).not.toContain("from=");
  });

  test("a range the wrong way round is caught before the call", async () => {
    const user = userEvent.setup();
    mount();
    await typeIsoDate(user, "export-range-from", "2026-12-31");
    await typeIsoDate(user, "export-range-to", "2026-01-01");
    expect(await screen.findByRole("alert")).toHaveTextContent(fr.error_statement_range_invalid);
    expect(screen.getByTestId("export-sales")).toBeDisabled();
    // The others are unaffected: the range is the sales export's alone.
    expect(screen.getByTestId("export-products")).not.toBeDisabled();
  });
});

describe("the import", () => {
  test("the template downloads under the name the server gave it", async () => {
    const user = userEvent.setup();
    mount();
    await user.click(screen.getByTestId("import-template"));
    await waitFor(() => expect(saved).toHaveLength(1));
    expect(gets().some((url) => url.includes("/import/products/template?lang=fr"))).toBe(true);
    expect(saved[0]?.filename).toBe("modele-produits-2026-09-10.xlsx");
  });

  /**
   * A file input writes its own button and its own "no file chosen", in the
   * language the machine is in. An Arabic counter on a French Windows read
   * "Choisir un fichier" there. The input still does the work; what the shop
   * reads sits beside it and comes from the dictionary.
   */
  test("the picker says what was chosen in the shop's own words", async () => {
    const user = userEvent.setup();
    mount();

    expect(screen.getByTestId("import-file")).toHaveClass("sr-only");
    expect(screen.getByTestId("import-file")).toHaveAttribute("tabindex", "-1");
    expect(screen.getByTestId("import-file-name")).toHaveAttribute("dir", "ltr");
    expect(screen.getByTestId("import-pick")).toHaveTextContent(fr.import_pick_file);
    expect(screen.getByTestId("import-file-name")).toHaveTextContent(fr.import_no_file);

    const opened = vi.spyOn(HTMLInputElement.prototype, "click");
    await user.click(screen.getByTestId("import-pick"));
    expect(opened).toHaveBeenCalledTimes(1);
    opened.mockRestore();

    await user.upload(screen.getByTestId("import-file"), xlsx());

    expect(screen.getByTestId("import-file-name")).toHaveTextContent("produits.xlsx");
  });

  /**
   * The hidden input is not a second way in. A file picked while a check is
   * out lands its report under the new name, and the visible button already
   * closes for that; the input did not until 2026-09-12.
   */
  test("nothing can pick a second file while a check is out", async () => {
    const user = userEvent.setup();
    let answer: () => void = () => {};
    vi.stubGlobal(
      "fetch",
      vi.fn(
        () =>
          new Promise<Response>((resolve) => {
            answer = () => resolve(json(200, CLEAN));
          }),
      ),
    );
    mount();

    await user.upload(screen.getByTestId("import-file"), xlsx());
    await user.click(screen.getByTestId("import-dry-run"));

    await waitFor(() => expect(screen.getByTestId("import-pick")).toBeDisabled());
    expect(screen.getByTestId("import-file")).toBeDisabled();

    answer();
    await screen.findByTestId("import-counts");
  });

  test("a file is checked before it can be imported, and the check writes nothing", async () => {
    const user = userEvent.setup();
    mount();
    // Nothing to import until a file is picked and checked.
    expect(screen.getByTestId("import-dry-run")).toBeDisabled();
    expect(screen.queryByTestId("import-apply")).toBeNull();

    await user.upload(screen.getByTestId("import-file"), xlsx());
    await user.click(screen.getByTestId("import-dry-run"));

    expect(await screen.findByTestId("import-counts")).toHaveTextContent("1");
    expect(importRows()).toHaveLength(1);
    expect(importRows()[0]).toHaveTextContent(fr.import_outcome_created);
    // The dry run is the only call so far: nothing was written.
    expect(urls("POST")).toHaveLength(1);
    expect(urls("POST")[0]).toContain("/import/products/dry-run");
  });

  test("a file with one refusal cannot be applied and says why in the row", async () => {
    dryRun = DIRTY;
    const user = userEvent.setup();
    mount();
    await user.upload(screen.getByTestId("import-file"), xlsx());
    await user.click(screen.getByTestId("import-dry-run"));

    await screen.findByTestId("import-table");
    const rows = importRows();
    expect(rows).toHaveLength(2);
    expect(rows[1]).toHaveTextContent(fr.import_outcome_refused);
    // The core sends a key; the screen owns the sentence.
    expect(rows[1]).toHaveTextContent(fr.import_reason_unknown_unit);
    expect(rows[1]).not.toHaveTextContent("unknown_unit");

    expect(screen.getByTestId("import-apply")).toBeDisabled();
    expect(screen.getByText(fr.import_fix_first)).toBeInTheDocument();
  });

  test("an updated row says the stock is left alone, and a file of creates does not", async () => {
    dryRun = {
      rows: [{ row: 2, name: "Café moulu 250 g", outcome: "updated", field: null, reason: null }],
      accepted: 1,
      refused: 0,
    };
    const user = userEvent.setup();
    mount();
    await user.upload(screen.getByTestId("import-file"), xlsx());
    await user.click(screen.getByTestId("import-dry-run"));
    expect(await screen.findByTestId("import-keeps-stock")).toHaveTextContent(
      fr.import_update_keeps_stock,
    );
  });

  test("a file that only creates says nothing about stock", async () => {
    const user = userEvent.setup();
    mount();
    await user.upload(screen.getByTestId("import-file"), xlsx());
    await user.click(screen.getByTestId("import-dry-run"));
    await screen.findByTestId("import-counts");
    expect(screen.queryByTestId("import-keeps-stock")).toBeNull();
  });

  test("a clean file is applied as the same bytes and reports the counts", async () => {
    const user = userEvent.setup();
    mount();
    await user.upload(screen.getByTestId("import-file"), xlsx());
    await user.click(screen.getByTestId("import-dry-run"));
    await screen.findByTestId("import-counts");

    await user.click(screen.getByTestId("import-apply"));
    expect(await screen.findByTestId("import-done")).toHaveTextContent("1");
    const posted = urls("POST");
    expect(posted).toHaveLength(2);
    expect(posted[1]).toContain("/import/products");
    expect(posted[1]).not.toContain("dry-run");
    // The table is gone once the file landed: it described a file that has
    // been written, and leaving it up invites a second apply.
    expect(screen.queryByTestId("import-table")).toBeNull();
  });

  test("a server refusal on apply is shown and nothing is claimed done", async () => {
    applyAnswer = () =>
      json(422, { error: { code: "validation", field: "rows", message: "refused rows" } });
    const user = userEvent.setup();
    mount();
    await user.upload(screen.getByTestId("import-file"), xlsx());
    await user.click(screen.getByTestId("import-dry-run"));
    await screen.findByTestId("import-counts");

    await user.click(screen.getByTestId("import-apply"));
    expect(await screen.findByRole("alert")).toHaveTextContent(fr.error_validation);
    expect(screen.queryByTestId("import-done")).toBeNull();
  });
});
