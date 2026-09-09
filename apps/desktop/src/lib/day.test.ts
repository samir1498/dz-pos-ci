import { describe, expect, it } from "vitest";

import { todayAsDay } from "./day";

describe("todayAsDay", () => {
  it("reads the machine's calendar and not UTC", () => {
    // Ten past one in the morning where the machine sits.
    const local = new Date(2026, 8, 10, 1, 10);
    expect(todayAsDay(local)).toBe("2026-09-10");
    // East of Greenwich that is a day UTC has not reached yet, which is the
    // whole reason this function exists.
    if (local.getTimezoneOffset() < 0) {
      expect(local.toISOString().slice(0, 10)).toBe("2026-09-09");
    }
  });

  it("pads the month and the day", () => {
    expect(todayAsDay(new Date(2026, 0, 3, 12, 0))).toBe("2026-01-03");
  });
});
