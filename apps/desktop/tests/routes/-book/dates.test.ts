// `dates.ts` reads and writes the shop-clock string
// (`YYYY-MM-DD HH:MM:SS`) FullCalendar's `Date`s go through on the way to
// and from the server. Every case here checks the *local* getters are used,
// never `toISOString`/UTC, because that is the one mistake that would book
// the wrong slot without ever failing an assertion that used the same UTC
// math to check itself.

import { describe, expect, test } from "vitest";

import {
  addMinutes,
  formatDay,
  formatStartsAt,
  parseStartsAt,
} from "../../../src/routes/-book/dates";

describe("formatStartsAt: a Date to the server's own string", () => {
  test("a plain morning slot", () => {
    expect(formatStartsAt(new Date(2026, 8, 23, 9, 30, 0))).toBe("2026-09-23 09:30:00");
  });

  test("zero-pads a single-digit month, day, hour, minute and second", () => {
    expect(formatStartsAt(new Date(2026, 0, 5, 8, 5, 3))).toBe("2026-01-05 08:05:03");
  });

  test("midnight, the boundary a lunch break and a closed day both sit on", () => {
    expect(formatStartsAt(new Date(2026, 8, 23, 0, 0, 0))).toBe("2026-09-23 00:00:00");
  });
});

describe("formatDay: the day half alone, for day= and week=", () => {
  test("drops the time of day", () => {
    expect(formatDay(new Date(2026, 8, 23, 14, 45, 0))).toBe("2026-09-23");
  });
});

describe("parseStartsAt: the server's string back to a Date", () => {
  test("round-trips a slot formatStartsAt produced", () => {
    const text = "2026-09-23 09:30:00";
    const date = parseStartsAt(text);
    expect(date).not.toBeNull();
    expect(date === null ? null : formatStartsAt(date)).toBe(text);
  });

  test("empty string is not a moment", () => {
    expect(parseStartsAt("")).toBeNull();
  });

  test("a date with a T instead of a space is refused, the same shape the server refuses", () => {
    expect(parseStartsAt("2026-09-23T09:30:00")).toBeNull();
  });

  test("a day that does not exist (30 February) is refused rather than rolled into March", () => {
    expect(parseStartsAt("2026-02-30 09:00:00")).toBeNull();
  });
});

describe("addMinutes: a slot's end from its own start and length", () => {
  test("a 30-minute slot", () => {
    const end = addMinutes(new Date(2026, 8, 23, 9, 0, 0), 30);
    expect(formatStartsAt(end)).toBe("2026-09-23 09:30:00");
  });

  test("crosses midnight into the next day", () => {
    const end = addMinutes(new Date(2026, 8, 23, 23, 45, 0), 30);
    expect(formatStartsAt(end)).toBe("2026-09-24 00:15:00");
  });

  test("zero minutes is the same moment", () => {
    const start = new Date(2026, 8, 23, 9, 0, 0);
    expect(formatStartsAt(addMinutes(start, 0))).toBe(formatStartsAt(start));
  });
});
