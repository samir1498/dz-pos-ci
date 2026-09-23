import { describe, expect, test } from "vitest";

import { cn } from "../../src/lib/utils";

/**
 * `cn` merges Tailwind class lists for the kit. The behaviour worth pinning
 * is the one plain string concatenation does not have: when two utilities set
 * the same property, the later one wins and the earlier one is dropped from
 * the output. Without it a caller's override lands beside the component's own
 * class and the winner is decided by the order Tailwind emitted them in,
 * which is not the order they were written in.
 */
describe("cn", () => {
  test("keeps the last of two utilities on the same property", () => {
    expect(cn("p-2", "p-4")).toBe("p-4");
    expect(cn("bg-card", "bg-primary")).toBe("bg-primary");
  });

  test("keeps utilities that do not collide", () => {
    expect(cn("rounded-md", "text-muted-foreground")).toBe("rounded-md text-muted-foreground");
  });

  test("drops the falsy forms a conditional class produces", () => {
    expect(cn("p-2", false, undefined, null, "")).toBe("p-2");
    expect(cn(["p-2", { "font-numeric": true, hidden: false }])).toBe("p-2 font-numeric");
  });

  test("is empty rather than undefined when nothing is passed", () => {
    expect(cn()).toBe("");
  });
});
