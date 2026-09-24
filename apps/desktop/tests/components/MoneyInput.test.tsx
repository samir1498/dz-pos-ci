// T14 (context/plans/20260924-shop-manual-test-findings.md): a money box
// used to type right-aligned while every other field on the same form typed
// left, which read as disorienting rather than as a currency convention.
// This proves the default is gone and that a caller asking for the old
// behaviour (a table column where digits stack) still gets it.

import { render, screen } from "@testing-library/react";
import { expect, test } from "vitest";

import { MoneyInput } from "../../src/components/MoneyInput";

test("types start-aligned like every other field by default", () => {
  render(<MoneyInput value={null} onChange={() => {}} data-testid="amount" />);
  const input = screen.getByTestId("amount");
  expect(input).not.toHaveClass("text-end");
  // Digits still read left to right on the Arabic screen too.
  expect(input).toHaveAttribute("dir", "ltr");
});

test("a table or totals column can still ask for end alignment", () => {
  render(
    <MoneyInput value={null} onChange={() => {}} data-testid="amount" className="text-end" />,
  );
  expect(screen.getByTestId("amount")).toHaveClass("text-end");
});
