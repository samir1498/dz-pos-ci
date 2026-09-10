// The two decisions the wrapper exists to make: an icon is decoration until
// a caller names it, and only an icon that points somewhere mirrors in
// Arabic. Both are easy to get wrong per call site, which is why they live
// in one component and are checked here rather than reviewed by eye.

import { render, screen } from "@testing-library/react";
import { ArrowRight, Coins } from "lucide-react";
import { describe, expect, test } from "vitest";

import { Icon } from "./Icon";

describe("Icon", () => {
  test("is hidden from the reader when it sits beside a label", () => {
    const { container } = render(<Icon as={Coins} />);
    const glyph = container.querySelector("svg");
    expect(glyph).not.toBeNull();
    expect(glyph).toHaveAttribute("aria-hidden", "true");
    expect(glyph).not.toHaveAttribute("role", "img");
  });

  test("becomes an image with a name when a caller gives it one", () => {
    render(<Icon as={Coins} label="espèces" />);
    const glyph = screen.getByRole("img", { name: "espèces" });
    expect(glyph).not.toHaveAttribute("aria-hidden", "true");
  });

  test("mirrors only when asked", () => {
    const { container: plain } = render(<Icon as={Coins} />);
    expect(plain.querySelector("svg")?.getAttribute("class")).not.toContain("rtl:-scale-x-100");
    const { container: pointing } = render(<Icon as={ArrowRight} flip />);
    expect(pointing.querySelector("svg")?.getAttribute("class")).toContain("rtl:-scale-x-100");
  });

  test("takes one of the three sizes, and 20 by default", () => {
    const { container: standard } = render(<Icon as={Coins} />);
    expect(standard.querySelector("svg")).toHaveAttribute("width", "20");
    const { container: heading } = render(<Icon as={Coins} size={24} />);
    expect(heading.querySelector("svg")).toHaveAttribute("width", "24");
  });
});
