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
import en from "@/i18n/en.json";
import fr from "@/i18n/fr.json";

import { DataTable, type Column } from "../../src/components/DataTable";
import { EmptyState } from "../../src/components/EmptyState";
import { FormField } from "../../src/components/FormField";
import { Money } from "../../src/components/Money";
import { MoneyInput } from "../../src/components/MoneyInput";
import { PageHeader } from "../../src/components/PageHeader";
import { PayButton } from "../../src/components/PayButton";
import { StatusPill, type Status } from "../../src/components/StatusPill";
import { DateField } from "../../src/components/ui/date-field";
import { Input } from "../../src/components/ui/input";
import { MonthField } from "../../src/components/ui/month-field";
import { Tabs, TabsList, TabsTrigger } from "../../src/components/ui/tabs";

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
 *  an `ml-2`. Only a colon outside brackets separates a variant, so an
 *  arbitrary property keeps its own: `[grid-template-columns:repeat(...)]`,
 *  which the till uses, is one class and not a `repeat(...)` utility. */
function utility(className: string): string {
  let depth = 0;
  let start = 0;
  for (let i = 0; i < className.length; i += 1) {
    const character = className[i];
    if (character === "[") depth += 1;
    else if (character === "]") depth -= 1;
    else if (character === ":" && depth === 0) start = i + 1;
  }
  return className.slice(start);
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
    // A class that reads a side keeps a physical one on purpose. The sheet
    // and the sidebar: a panel's edge, its border and the half it slides in
    // from have to agree, and `AppShell` computes the side from the page
    // direction (`frontend-conventions`, "content-side physical
    // properties"). A Radix popper: it picks its own side from the space it
    // has and the direction it is in. The sidebar spells it both
    // `group-data-[side=left]` and `[[data-side=left]_&]`, so the test is on
    // `side=` rather than on either bracket.
    .filter((name) => name !== "" && !name.includes("side="))
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
  test("a money column is end-aligned; its cells take the figure face and its heading does not", () => {
    // A heading is words (T44): set in the digits' face it read as a
    // different kind of header from its neighbours.
    renderIn("fr", <DataTable columns={COLUMNS} rows={ROWS} rowKey={(row) => row.id} caption="Produits" />);
    const heading = screen.getByRole("columnheader", { name: "Total" });
    expect(heading).toHaveClass("text-end");
    expect(heading).not.toHaveClass("font-numeric");
    expect(screen.getByText("3 272,40").closest("td")).toHaveClass("text-end", "font-numeric", "tabular-nums");
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

// ---- DateField ----

function DateHarness({ start }: { start: string }) {
  const [value, setValue] = useState(start);
  return (
    <>
      <DateField value={value} onChange={setValue} data-testid="date" />
      <output data-testid="date-value">{value}</output>
    </>
  );
}

describe("DateField", () => {
  test("shows a complete date across its three boxes", () => {
    renderIn("fr", <DateHarness start="2026-09-05" />);
    expect(screen.getByTestId("date-day")).toHaveValue("05");
    expect(screen.getByTestId("date-month")).toHaveValue("09");
    expect(screen.getByTestId("date-year")).toHaveValue("2026");
  });

  test("editing one box hands back the whole date, the others untouched", async () => {
    const user = userEvent.setup();
    renderIn("fr", <DateHarness start="2026-09-05" />);
    await user.clear(screen.getByTestId("date-day"));
    await user.type(screen.getByTestId("date-day"), "10");
    expect(screen.getByTestId("date-value")).toHaveTextContent("2026-09-10");
    expect(screen.getByTestId("date-month")).toHaveValue("09");
    expect(screen.getByTestId("date-year")).toHaveValue("2026");
  });

  test("an empty year is not a date yet: it hands back \"\"", async () => {
    const user = userEvent.setup();
    renderIn("fr", <DateHarness start="" />);
    await user.type(screen.getByTestId("date-day"), "15");
    await user.type(screen.getByTestId("date-month"), "06");
    expect(screen.getByTestId("date-value")).toHaveTextContent("");
  });

  /** The standing example of a day and a month that cannot go together.
   *  Leaving 31/02 on the screen while handing back "" would show a shop a
   *  date and leave the screen under it filtering on nothing. */
  test("a day the month does not have is brought back to the last one it does", async () => {
    const user = userEvent.setup();
    renderIn("fr", <DateHarness start="" />);
    await user.type(screen.getByTestId("date-year"), "2026");
    await user.type(screen.getByTestId("date-month"), "02");
    await user.type(screen.getByTestId("date-day"), "31");
    expect(screen.getByTestId("date-day")).toHaveValue("28");
    expect(screen.getByTestId("date-value")).toHaveTextContent("2026-02-28");
  });

  /** The correction waits for the year, because February has a 29th in one
   *  year out of four and the year is the last box typed. */
  test("the 29th of February stands in a leap year and not in the year before", async () => {
    const user = userEvent.setup();
    renderIn("fr", <DateHarness start="" />);
    await user.type(screen.getByTestId("date-day"), "29");
    await user.type(screen.getByTestId("date-month"), "02");
    expect(screen.getByTestId("date-day")).toHaveValue("29");
    await user.type(screen.getByTestId("date-year"), "2024");
    expect(screen.getByTestId("date-value")).toHaveTextContent("2024-02-29");

    await user.clear(screen.getByTestId("date-year"));
    await user.type(screen.getByTestId("date-year"), "2023");
    expect(screen.getByTestId("date-day")).toHaveValue("28");
  });

  /** Three boxes cannot be named by one `htmlFor`, and the `aria-label` on
   *  each box beats the field's label rather than adding to it, so the name
   *  of the field would be announced nowhere. It sits on the group. */
  test("the field's own label names the whole control", () => {
    renderIn(
      "fr",
      <FormField label="Date d'échéance">
        {(parts) => <DateField {...parts} value="2026-09-05" onChange={() => {}} data-testid="due" />}
      </FormField>,
    );
    expect(screen.getByRole("group", { name: "Date d'échéance" })).toBeInTheDocument();
  });

  test("typing is clamped to what a day or a month can be", async () => {
    const user = userEvent.setup();
    renderIn("fr", <DateHarness start="" />);
    await user.type(screen.getByTestId("date-day"), "99");
    expect(screen.getByTestId("date-day")).toHaveValue("31");
    await user.type(screen.getByTestId("date-month"), "13");
    expect(screen.getByTestId("date-month")).toHaveValue("12");
  });

  test("reads day, month, year left to right even on an Arabic page", () => {
    renderIn("ar", <DateHarness start="2026-09-05" />);
    expect(screen.getAllByRole("textbox").map((box) => box.getAttribute("aria-label"))).toEqual([
      ar.date_segment_day,
      ar.date_segment_month,
      ar.date_segment_year,
    ]);
    expect(screen.getByTestId("date")).toHaveAttribute("dir", "ltr");
  });

  test("carries nothing left or right", () => {
    const { container } = renderIn("ar", <DateHarness start="2026-09-05" />);
    expect(noPhysicalSides(container)).toEqual([]);
  });
});

// ---- MonthField ----

function MonthHarness({ start }: { start: string }) {
  const [value, setValue] = useState(start);
  return (
    <>
      <MonthField value={value} onChange={setValue} data-testid="month" />
      <output data-testid="month-value">{value}</output>
    </>
  );
}

describe("MonthField", () => {
  test("shows a complete month across its two boxes", () => {
    renderIn("fr", <MonthHarness start="2026-09" />);
    expect(screen.getByTestId("month-month")).toHaveValue("09");
    expect(screen.getByTestId("month-year")).toHaveValue("2026");
  });

  test("editing one box hands back the whole month", async () => {
    const user = userEvent.setup();
    renderIn("fr", <MonthHarness start="2026-09" />);
    await user.clear(screen.getByTestId("month-month"));
    await user.type(screen.getByTestId("month-month"), "01");
    expect(screen.getByTestId("month-value")).toHaveTextContent("2026-01");
  });

  test("an empty year is not a month yet: it hands back \"\"", async () => {
    const user = userEvent.setup();
    renderIn("fr", <MonthHarness start="" />);
    await user.type(screen.getByTestId("month-month"), "06");
    expect(screen.getByTestId("month-value")).toHaveTextContent("");
  });

  test("typing is clamped to what a month can be", async () => {
    const user = userEvent.setup();
    renderIn("fr", <MonthHarness start="" />);
    await user.type(screen.getByTestId("month-month"), "13");
    expect(screen.getByTestId("month-month")).toHaveValue("12");
  });

  test("reads month, year left to right even on an Arabic page", () => {
    renderIn("ar", <MonthHarness start="2026-09" />);
    expect(screen.getAllByRole("textbox").map((box) => box.getAttribute("aria-label"))).toEqual([
      ar.date_segment_month,
      ar.date_segment_year,
    ]);
    expect(screen.getByTestId("month")).toHaveAttribute("dir", "ltr");
  });

  /** The whole point of this control: the caption follows the shop's
   *  language, not whatever the machine underneath it is set to. */
  test("names the month in the shop's own language", () => {
    const fr1 = renderIn("fr", <MonthHarness start="2026-09" />);
    expect(screen.getByTestId("month-name")).toHaveTextContent(fr.month_09);
    fr1.unmount();

    const en1 = renderIn("en", <MonthHarness start="2026-09" />);
    expect(screen.getByTestId("month-name")).toHaveTextContent(en.month_09);
    en1.unmount();

    const ar1 = renderIn("ar", <MonthHarness start="2026-09" />);
    expect(screen.getByTestId("month-name")).toHaveTextContent(ar.month_09);
    ar1.unmount();
  });

  test("carries nothing left or right", () => {
    const { container } = renderIn("ar", <MonthHarness start="2026-09" />);
    expect(noPhysicalSides(container)).toEqual([]);
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

// ---- Tabs ----
//
// Here rather than left to the screens that use tabs, because the trigger is
// where the active bar is drawn and the bar is drawn with an inset: the one
// shape in the kit whose side lives behind a variant prefix. The vertical
// list carried its bar on the physical right until 2026-09-12 and the check
// above read past it.

describe("Tabs", () => {
  test("shows the tab that is open and the ones that are not", () => {
    renderIn("fr", (
      <Tabs defaultValue="lines">
        <TabsList>
          <TabsTrigger value="lines">Lignes</TabsTrigger>
          <TabsTrigger value="payments">Règlements</TabsTrigger>
        </TabsList>
      </Tabs>
    ));
    expect(screen.getByRole("tab", { name: "Lignes" })).toHaveAttribute("data-state", "active");
    expect(screen.getByRole("tab", { name: "Règlements" })).toHaveAttribute("data-state", "inactive");
  });

  test("carries nothing left or right, standing up or lying down", () => {
    const { container } = renderIn("ar", (
      <Tabs defaultValue="lines" orientation="vertical">
        <TabsList>
          <TabsTrigger value="lines">السطور</TabsTrigger>
          <TabsTrigger value="payments">التسديدات</TabsTrigger>
        </TabsList>
      </Tabs>
    ));
    expect(noPhysicalSides(container)).toEqual([]);
  });
});
