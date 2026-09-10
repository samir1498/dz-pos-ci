// The icon wrapper. Two rules worth pinning: an icon is decoration unless a
// caller names it, and only an icon a caller marks as directional mirrors in
// Arabic.

import { render, screen } from "@testing-library/react";
import { ArrowRight, Printer } from "lucide-react";
import { describe, expect, test } from "vitest";

import { Icon } from "./Icon";

describe("Icon", () => {
  test("is hidden from a screen reader when it sits beside a label", () => {
    const { container } = render(<Icon as={Printer} />);
    const svg = container.querySelector("svg");
    expect(svg).toHaveAttribute("aria-hidden", "true");
  });

  test("is announced when it is the only thing saying what a control does", () => {
    render(<Icon as={Printer} label="Imprimer" />);
    expect(screen.getByRole("img", { name: "Imprimer" })).toBeInTheDocument();
  });

  /**
   * An arrow points at the next thing, so it mirrors with the direction. A
   * printer, a clock or a coin must not: mirroring those makes them wrong
   * rather than translated, which is why the flip is opt-in per call.
   */
  test("mirrors only what the caller marks as directional", () => {
    const { container: directional } = render(<Icon as={ArrowRight} flip />);
    expect(directional.querySelector("svg")).toHaveClass("rtl:-scale-x-100");

    const { container: fixed } = render(<Icon as={Printer} />);
    expect(fixed.querySelector("svg")).not.toHaveClass("rtl:-scale-x-100");
  });

  test("draws at the size asked for, and at 20 by default", () => {
    const { container } = render(
      <>
        <Icon as={Printer} />
        <Icon as={Printer} size={24} />
      </>,
    );
    const [first, second] = Array.from(container.querySelectorAll("svg"));
    expect(first).toHaveAttribute("width", "20");
    expect(second).toHaveAttribute("width", "24");
  });
});
