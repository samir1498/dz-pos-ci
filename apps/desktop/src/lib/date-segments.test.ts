// The calendar arithmetic behind DateField and MonthField, tested apart
// from the boxes: a segment can only ever hold a number a keystroke could
// clamp to, but the three segments together can still spell a date that
// does not exist (31 February is the standing example), and that is what
// `toIsoDate` has to catch.

import { describe, expect, test } from "vitest";

import { clampSegment, clampYear, parseIsoDate, parseIsoMonth, toIsoDate, toIsoMonth } from "./date-segments";

describe("toIsoDate: an incomplete or impossible date is \"\"", () => {
  test("an empty year is not a date yet", () => {
    expect(toIsoDate("15", "06", "")).toBe("");
    expect(toIsoDate("15", "06", "202")).toBe("");
  });

  test("a month past twelve does not exist", () => {
    expect(toIsoDate("15", "13", "2026")).toBe("");
  });

  test("the 31st of February does not exist, leap year or not", () => {
    expect(toIsoDate("31", "02", "2026")).toBe("");
    expect(toIsoDate("31", "02", "2024")).toBe("");
  });

  test("the 29th of February exists only on a leap year", () => {
    expect(toIsoDate("29", "02", "2024")).toBe("2024-02-29");
    expect(toIsoDate("29", "02", "2026")).toBe("");
  });

  test("a century year is a leap year only on a multiple of 400", () => {
    expect(toIsoDate("29", "02", "2000")).toBe("2000-02-29");
    expect(toIsoDate("29", "02", "1900")).toBe("");
  });

  test("day or month zero is not a date", () => {
    expect(toIsoDate("00", "06", "2026")).toBe("");
    expect(toIsoDate("15", "00", "2026")).toBe("");
  });
});

test("toIsoDate: a complete valid date pads to two digits", () => {
  expect(toIsoDate("5", "9", "2026")).toBe("2026-09-05");
});

test("parseIsoDate reads the three segments back out of the string", () => {
  expect(parseIsoDate("2026-09-05")).toEqual({ day: "05", month: "09", year: "2026" });
  expect(parseIsoDate("")).toEqual({ day: "", month: "", year: "" });
  expect(parseIsoDate("not a date")).toEqual({ day: "", month: "", year: "" });
});

describe("toIsoMonth", () => {
  test("an empty year is not a month yet", () => {
    expect(toIsoMonth("09", "")).toBe("");
  });

  test("a month past twelve does not exist", () => {
    expect(toIsoMonth("13", "2026")).toBe("");
  });

  test("a complete valid month pads to two digits", () => {
    expect(toIsoMonth("9", "2026")).toBe("2026-09");
  });
});

test("parseIsoMonth reads the two segments back out of the string", () => {
  expect(parseIsoMonth("2026-09")).toEqual({ month: "09", year: "2026" });
  expect(parseIsoMonth("")).toEqual({ month: "", year: "" });
});

describe("clampSegment: a keystroke cannot make a day or a month that cannot exist", () => {
  test("stays under the ceiling as-is", () => {
    expect(clampSegment("9", 31, 2)).toBe("9");
    expect(clampSegment("31", 31, 2)).toBe("31");
  });

  test("a number past the ceiling clamps down to it", () => {
    expect(clampSegment("99", 31, 2)).toBe("31");
    expect(clampSegment("13", 12, 2)).toBe("12");
  });

  test("anything that is not a digit is dropped", () => {
    expect(clampSegment("3a", 31, 2)).toBe("3");
    expect(clampSegment("ab", 31, 2)).toBe("");
  });

  test("never longer than the segment can hold", () => {
    // The length limit bites before the ceiling does: a paste of "123"
    // truncates to "12" the way `maxLength` would, rather than reading the
    // whole number and clamping that.
    expect(clampSegment("123", 31, 2)).toBe("12");
  });
});

test("clampYear keeps only digits, up to four of them", () => {
  expect(clampYear("2026")).toBe("2026");
  expect(clampYear("20267")).toBe("2026");
  expect(clampYear("2a0b26")).toBe("2026");
});
