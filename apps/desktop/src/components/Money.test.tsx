// An amount on screen. The formatter itself is tested in packages/shared;
// what is tested here is the three things the component adds, because each of
// them is invisible when it breaks and wrong on an Arabic screen.

import { render, screen } from "@testing-library/react";
import { describe, expect, test } from "vitest";

import { Money } from "./Money";

describe("Money", () => {
  test("shows the amount the shared formatter produces", () => {
    render(<Money centimes={327240} data-testid="m" />);
    expect(screen.getByTestId("m")).toHaveTextContent("3 272,40");
  });

  /**
   * An amount reads left to right with Western digits whatever the screen's
   * language, the same decision the fiscal identifiers and the barcodes take.
   * Without it the minus sign of a negative balance moves to the far end of
   * the number on an Arabic screen, which reads as a different figure.
   */
  test("stays left to right so a sign keeps its end", () => {
    render(<Money centimes={-1250} data-testid="m" />);
    const el = screen.getByTestId("m");
    expect(el).toHaveAttribute("dir", "ltr");
    expect(el).toHaveTextContent("-12,50");
  });

  test("wears the figure font with tabular numerals", () => {
    render(<Money centimes={0} data-testid="m" />);
    expect(screen.getByTestId("m")).toHaveClass("font-numeric", "tabular-nums");
  });

  /** A caller's class wins over the component's own on the same property. */
  test("lets a caller add classes without losing the figure font", () => {
    render(<Money centimes={0} className="text-lg" data-testid="m" />);
    const el = screen.getByTestId("m");
    expect(el).toHaveClass("font-numeric");
    expect(el).toHaveClass("text-lg");
  });

  /** Centimes are integers. A float here means an amount was computed in JS. */
  test("refuses an amount that is not a whole number of centimes", () => {
    expect(() => render(<Money centimes={12.5} />)).toThrow(RangeError);
  });
});
