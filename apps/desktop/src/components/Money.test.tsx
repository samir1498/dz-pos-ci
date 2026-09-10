// An amount is the one thing on the screen that must not be re-typed by a
// component. These check the three promises the component makes and nothing
// about colour: the figure font, the direction, and that the digits are the
// shared formatter's own output rather than a second spelling of it.

import { render, screen } from "@testing-library/react";
import { formatCentimes } from "@dzpos/shared";
import { describe, expect, test } from "vitest";

import { Money } from "./Money";

describe("Money", () => {
  test("renders the shared formatter's spelling, not its own", () => {
    render(<Money centimes={123456} data-testid="amount" />);
    const amount = screen.getByTestId("amount");
    // textContent rather than toHaveTextContent: the matcher collapses
    // whitespace, and the group separator is U+202F, a narrow no-break
    // space. Collapsed to a plain space, a component that grouped with an
    // ordinary space would pass, which is the drift this guards.
    expect(amount.textContent).toBe(formatCentimes(123456));
    // Spelled out once as well, so a change to the grouping shows up here
    // and not only in the shared package's own tests. The gap below the
    // thousands is that same U+202F and not a space bar press.
    expect(amount.textContent).toBe("1 234,56");
  });

  test("keeps a negative amount's sign in front of the digits", () => {
    render(<Money centimes={-500} data-testid="amount" />);
    const amount = screen.getByTestId("amount");
    expect(amount.textContent).toBe("-5,00");
    // The reason for dir="ltr": in an RTL paragraph the bidi algorithm moves
    // a trailing sign, and a debt would read as a credit.
    expect(amount).toHaveAttribute("dir", "ltr");
  });

  test("wears the numeric face with tabular figures", () => {
    render(<Money centimes={0} data-testid="amount" />);
    const amount = screen.getByTestId("amount");
    expect(amount.className).toContain("font-numeric");
    expect(amount.className).toContain("tabular-nums");
  });

  test("keeps a caller's classes beside its own", () => {
    render(<Money centimes={0} className="text-lg" data-testid="amount" />);
    expect(screen.getByTestId("amount").className).toContain("text-lg");
  });
});
