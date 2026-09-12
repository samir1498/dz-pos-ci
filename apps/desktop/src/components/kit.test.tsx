// The kit's own tests. One file rather than one per component, because what
// is being checked is the same three things everywhere and reading them
// side by side is what shows a component that answers differently from its
// neighbours:
//
// 1. It renders what it was given.
// 2. It carries nothing physically left or right, so it mirrors in Arabic
//    without a second rule. `noPhysicalSides` is the check; a component wears
//    `ms-`, `pe-`, `text-start`, `start-0`, never their left/right twins.
// 3. Money goes in and comes out as integer centimes, spelled the way
//    packages/shared spells it.
//
// What is not here: opening a Radix overlay. A dropdown or a select needs
// pointer capture and `scrollIntoView`, neither of which jsdom has, and a
// popover that opens in a test but not in a browser proves nothing. Those
// are e2e's, in kit.spec.ts.

import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Package } from "lucide-react";
import { useState } from "react";
import { describe, expect, test } from "vitest";

import { I18nProvider, type Lang } from "@/i18n";
import ar from "@/i18n/ar.json";

import { DataTable, type Column } from "./DataTable";
import { EmptyState } from "./EmptyState";
import { FormField } from "./FormField";
import { Money } from "./Money";
import { MoneyInput } from "./MoneyInput";
import { PageHeader } from "./PageHeader";
import { PayButton } from "./PayButton";
import { StatusPill, type Status } from "./StatusPill";
import { Input } from "./ui/input";

/**
 * Tailwind's physical utilities. A logical one (`ms-2`, `pe-4`, `text-start`,
 * `end-0`, `border-s`) means the same thing in both directions; its physical
 * twin means the wrong thing in one of them, and the screen it is wrong on is
 * the Arabic one nobody on this team reads first.
 *
 * Matched against one class at a time with its variants already stripped, so
 * `after:-right-1` is caught as `-right-1`. The check read the whole class
 * string at once until 2026-09-12 and anchored on a space, which let every
 * physical utility behind a variant through: `tabs.tsx` carried the vertical
 * tabs' active bar on `after:-right-1` and no test said a word.
 *
 * `-?` so a negative margin is caught too, and the trailing boundary so
 * `border-ring` does not report as `border-r`.
 */
const PHYSICAL =
  /^-?(ml|mr|pl|pr|left|right|border-l|border-r|rounded-l|rounded-r|text-left|text-right)(-|$)/;

/** The variant prefixes off a class: `group-data-[state=open]:after:ml-2` is
 *  an `ml-2`. An arbitrary value can hold a colon of its own, which would
 *  make this read the wrong half; none in this app does, and a false hit is
 *  a failing test rather than a wrong screen. */
function utility(className: string): string {
  return className.slice(className.lastIndexOf(":") + 1);
}

/**
 * The thousands separator packages/shared groups with: U+202F, a narrow
 * no-break space, not the ordinary one. Written as an escape because the two
 * are indistinguishable in a diff, and a test that pasted the ordinary one
 * fails with two strings that look identical in its own output.
 */
const NARROW = "\u202f";

function noPhysicalSides(root: HTMLElement): string[] {
  return [root, ...root.querySelectorAll("*")]
    .filter((node): node is HTMLElement => node instanceof HTMLElement)
    .flatMap((node) => (typeof node.className === "string" ? node.className.split(/\s+/) : []))
    // The sheet and the sidebar keep a physical `side` on purpose: a panel's
    // edge, its border and the half it slides in from have to agree, and
    // `AppShell` computes the side from the page direction
    // (`frontend-conventions`, "content-side physical properties").
    .filter((name) => name !== "" && !name.includes("[side="))
    .filter((name) => PHYSICAL.test(utility(name)));
}

function renderIn(lang: Lang, node: React.ReactNode) {
  return render(<I18nProvider lang={lang}>{node}</I18nProvider>);
}

// ---- PageHeader ----

describe("PageHeader", () => {
  test("heads the page at level two, under the shell's own h1", () => {
    renderIn("fr", <PageHeader title="Clients" />);
    expect(screen.getByRole("heading", { level: 2, name: "Clients" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { level: 1 })).toBeNull();
  });

  test("shows the line and the actions when it is given them", () => {
    renderIn("fr", <PageHeader title="Clients" description="Qui doit quoi" actions={<button type="button">Nouveau</button>} />);
    expect(screen.getByText("Qui doit quoi")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Nouveau" })).toBeInTheDocument();
  });

  test("carries nothing left or right", () => {
    const { container } = renderIn("ar", <PageHeader title="العملاء" actions={<span>x</span>} />);
    expect(noPhysicalSides(container)).toEqual([]);
  });
});

// ---- FormField ----

describe("FormField", () => {
  test("points the label at the control it generated an id for", async () => {
    renderIn("fr", <FormField label="Nom">{(parts) => <Input {...parts} />}</FormField>);
    const control = screen.getByLabelText("Nom");
    expect(control).toBeInstanceOf(HTMLInputElement);
    expect(control.id).not.toBe("");
  });

  test("two fields with the same label do not share an id", () => {
    renderIn(
      "fr",
      <>
        <FormField label="Nom">{(parts) => <Input {...parts} data-testid="a" />}</FormField>
        <FormField label="Nom">{(parts) => <Input {...parts} data-testid="b" />}</FormField>
      </>,
    );
    expect(screen.getByTestId("a").id).not.toBe(screen.getByTestId("b").id);
  });

  test("a refusal is announced and described, not only coloured", () => {
    renderIn(
      "fr",
      <FormField label="Nom" error="Obligatoire">
        {(parts) => <Input {...parts} data-testid="c" />}
      </FormField>,
    );
    const control = screen.getByTestId("c");
    expect(control).toHaveAttribute("aria-invalid", "true");
    const alert = screen.getByRole("alert");
    expect(alert).toHaveTextContent("Obligatoire");
    expect(control.getAttribute("aria-describedby")).toContain(alert.id);
  });

  test("the error is described before the hint, because it is the newer fact", () => {
    renderIn(
      "fr",
      <FormField label="Nom" hint="Deux lettres au moins" error="Obligatoire">
        {(parts) => <Input {...parts} data-testid="d" />}
      </FormField>,
    );
    const ids = screen.getByTestId("d").getAttribute("aria-describedby")?.split(" ") ?? [];
    expect(ids).toHaveLength(2);
    expect(document.getElementById(ids[0] ?? "")).toHaveTextContent("Obligatoire");
  });

  test("with nothing to say it describes nothing at all", () => {
    renderIn("fr", <FormField label="Nom">{(parts) => <Input {...parts} data-testid="e" />}</FormField>);
    expect(screen.getByTestId("e")).not.toHaveAttribute("aria-describedby");
  });

  test("carries nothing left or right", () => {
    const { container } = renderIn(
      "ar",
      <FormField label="الاسم" hint="حرفان على الأقل" error="مطلوب" required>
        {(parts) => <Input {...parts} />}
      </FormField>,
    );
    expect(noPhysicalSides(container)).toEqual([]);
  });
});

// ---- StatusPill ----

describe("StatusPill", () => {
  const STATES: readonly Status[] = ["issued", "cancelled", "paid", "open", "low"];

  test.each(STATES)("%s says a word, not only a colour", (status) => {
    renderIn("fr", <StatusPill status={status} data-testid="pill" />);
    const pill = screen.getByTestId("pill");
    expect(pill.textContent?.trim()).not.toBe("");
    expect(pill).toHaveAttribute("data-status", status);
  });

  test("every state has its own tone, so two of them never read alike", () => {
    const tones = STATES.map((status) => {
      const { unmount } = renderIn("fr", <StatusPill status={status} data-testid="pill" />);
      const className = screen.getByTestId("pill").className;
      unmount();
      return className;
    });
    expect(new Set(tones).size).toBe(STATES.length);
  });

  test("the word follows the language", () => {
    renderIn("ar", <StatusPill status="paid" data-testid="pill" />);
    expect(screen.getByTestId("pill")).toHaveTextContent(ar.pill_paid);
  });
});

// ---- EmptyState ----

describe("EmptyState", () => {
  test("says what would fill the list and offers the one thing that does", () => {
    renderIn(
      "fr",
      <EmptyState
        icon={Package}
        title="Aucun produit"
        description="Importez un catalogue ou ajoutez le premier."
        action={<button type="button">Ajouter</button>}
      />,
    );
    expect(screen.getByText("Aucun produit")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Ajouter" })).toBeInTheDocument();
  });

  /** The title carries the meaning; the glyph is decoration beside it. */
  test("hides its icon from a screen reader", () => {
    const { container } = renderIn("fr", <EmptyState icon={Package} title="Aucun produit" />);
    const svg = container.querySelector("svg");
    expect(svg).toHaveAttribute("aria-hidden", "true");
  });

  test("carries nothing left or right", () => {
    const { container } = renderIn("ar", <EmptyState icon={Package} title="لا منتجات" description="ابدأ بواحد." />);
    expect(noPhysicalSides(container)).toEqual([]);
  });
});

// ---- DataTable ----

interface Row {
  readonly id: number;
  readonly name: string;
  readonly total: number;
}

const ROWS: readonly Row[] = [
  { id: 1, name: "Farine", total: 327240 },
  { id: 2, name: "Sucre", total: 4500 },
];

const COLUMNS: readonly Column<Row>[] = [
  { id: "name", header: "Produit", cell: (row) => row.name },
  { id: "total", header: "Total", money: true, cell: (row) => <Money centimes={row.total} /> },
];

describe("DataTable", () => {
  test("renders one row per row and one cell per column", () => {
    renderIn("fr", <DataTable columns={COLUMNS} rows={ROWS} rowKey={(row) => row.id} caption="Produits" />);
    const body = screen.getAllByRole("rowgroup")[1];
    expect(within(body ?? document.body).getAllByRole("row")).toHaveLength(2);
    expect(screen.getByText("Farine")).toBeInTheDocument();
  });

  /**
   * The reason the table exists. A column of amounts that is not end-aligned
   * in the figure face lines up on the glyph width instead of the digit, and
   * the shop reads a column it cannot scan.
   */
  test("a money column is end-aligned in the figure face", () => {
    renderIn("fr", <DataTable columns={COLUMNS} rows={ROWS} rowKey={(row) => row.id} caption="Produits" />);
    const heading = screen.getByRole("columnheader", { name: "Total" });
    expect(heading).toHaveClass("text-end", "font-numeric", "tabular-nums");
  });

  test("the amounts are the ones the shared formatter spells", () => {
    renderIn("fr", <DataTable columns={COLUMNS} rows={ROWS} rowKey={(row) => row.id} caption="Produits" />);
    expect(screen.getByText("3 272,40")).toBeInTheDocument();
    expect(screen.getByText("45,00")).toBeInTheDocument();
  });

  test("an empty list shows the caller's empty state, not an empty body", () => {
    renderIn(
      "fr",
      <DataTable
        columns={COLUMNS}
        rows={[]}
        rowKey={(row) => row.id}
        caption="Produits"
        empty={<EmptyState icon={Package} title="Aucun produit" />}
      />,
    );
    expect(screen.getByText("Aucun produit")).toBeInTheDocument();
    expect(screen.queryByRole("table")).toBeNull();
  });

  test("the actions column is named for a screen reader even with no heading text", () => {
    renderIn(
      "fr",
      <DataTable
        columns={COLUMNS}
        rows={ROWS}
        rowKey={(row) => row.id}
        caption="Produits"
        actions={() => <button type="button">Ouvrir</button>}
      />,
    );
    expect(screen.getByRole("columnheader", { name: "Actions" })).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: "Ouvrir" })).toHaveLength(2);
  });

  test("carries nothing left or right", () => {
    const { container } = renderIn(
      "ar",
      <DataTable columns={COLUMNS} rows={ROWS} rowKey={(row) => row.id} caption="المنتجات" actions={() => <span>x</span>} />,
    );
    expect(noPhysicalSides(container)).toEqual([]);
  });
});

// ---- MoneyInput ----

function Harness({ start }: { start: number | null }) {
  const [value, setValue] = useState<number | null>(start);
  return (
    <>
      <MoneyInput value={value} onChange={setValue} aria-label="Montant" data-testid="amount" />
      <output data-testid="centimes">{value === null ? "null" : String(value)}</output>
    </>
  );
}

describe("MoneyInput", () => {
  test("shows an amount the way the shared formatter spells it", () => {
    renderIn("fr", <Harness start={327240} />);
    expect(screen.getByTestId("amount")).toHaveValue(`3${NARROW}272,40`);
  });

  test("hands back integer centimes, never a float", async () => {
    const user = userEvent.setup();
    renderIn("fr", <Harness start={null} />);
    await user.type(screen.getByTestId("amount"), "1250,55");
    expect(screen.getByTestId("centimes")).toHaveTextContent("125055");
  });

  /** Blank is "no amount given", which is not the same answer as zero. */
  test("an empty field is null and not zero", async () => {
    const user = userEvent.setup();
    renderIn("fr", <Harness start={4500} />);
    await user.clear(screen.getByTestId("amount"));
    expect(screen.getByTestId("centimes")).toHaveTextContent("null");
  });

  test("writes the canonical spelling once the caret leaves", async () => {
    const user = userEvent.setup();
    renderIn("fr", <Harness start={null} />);
    const field = screen.getByTestId("amount");
    await user.type(field, "1000");
    await user.tab();
    expect(field).toHaveValue(`1${NARROW}000,00`);
  });

  test("puts the last understood amount back when what is left is unreadable", async () => {
    const user = userEvent.setup();
    renderIn("fr", <Harness start={4500} />);
    const field = screen.getByTestId("amount");
    // Typed onto the end of the amount already there, so the field is
    // unreadable while the value behind it is still 4500.
    await user.type(field, "abc");
    await user.tab();
    expect(field).toHaveValue("45,00");
    expect(screen.getByTestId("centimes")).toHaveTextContent("4500");
  });

  test("reads left to right so a sign keeps its end on an Arabic screen", () => {
    renderIn("ar", <Harness start={-1250} />);
    const field = screen.getByTestId("amount");
    expect(field).toHaveAttribute("dir", "ltr");
    expect(field).toHaveValue("-12,50");
  });
});

// ---- PayButton ----

describe("PayButton", () => {
  test("is the brass control, at the tall height", () => {
    renderIn("fr", <PayButton>Encaisser</PayButton>);
    const button = screen.getByTestId("pay-button");
    expect(button).toHaveClass("bg-money", "text-money-foreground");
    expect(button.className).toContain("h-(--control-h-lg)");
  });

  /** The till's payment panel is not a form; a stray submit reloads it. */
  test("does not submit unless it is asked to", () => {
    renderIn("fr", <PayButton>Encaisser</PayButton>);
    expect(screen.getByTestId("pay-button")).toHaveAttribute("type", "button");
  });

  test("carries nothing left or right", () => {
    const { container } = renderIn("ar", <PayButton>ادفع</PayButton>);
    expect(noPhysicalSides(container)).toEqual([]);
  });
});
