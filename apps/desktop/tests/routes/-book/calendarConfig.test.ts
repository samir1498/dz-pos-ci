// Turning working hours and absence blocks into the plain objects
// FullCalendar's `hiddenDays`, `businessHours` and background `events`
// take, checked with no calendar mounted at all (jsdom cannot lay one out).

import { describe, expect, test } from "vitest";

import {
  absenceBackgroundEvents,
  businessHoursOf,
  hiddenWeekdays,
  slotDurationOf,
} from "../../../src/routes/-book/calendarConfig";
import { formatStartsAt } from "../../../src/routes/-book/dates";

const CLOSED: readonly [] = [];
const MORNING = { opens: "08:00", closes: "12:00" };
const AFTERNOON = { opens: "14:00", closes: "18:00" };

describe("hiddenWeekdays: which columns the grid drops", () => {
  test("never set (null) hides nothing", () => {
    expect(hiddenWeekdays(null)).toEqual([]);
  });

  test("the Algerian week: Friday (5) and Saturday (6) closed, Sunday (0) first", () => {
    const week = [
      [MORNING, AFTERNOON], // Sunday
      [MORNING, AFTERNOON], // Monday
      [MORNING, AFTERNOON], // Tuesday
      [MORNING, AFTERNOON], // Wednesday
      [MORNING, AFTERNOON], // Thursday
      CLOSED, // Friday
      CLOSED, // Saturday
    ];
    expect(hiddenWeekdays(week)).toEqual([5, 6]);
  });

  test("every day open hides none", () => {
    const week = Array.from({ length: 7 }, () => [MORNING]);
    expect(hiddenWeekdays(week)).toEqual([]);
  });

  test("does not mutate the week it was handed", () => {
    const week = [CLOSED, [MORNING], CLOSED, [MORNING], CLOSED, [MORNING], CLOSED];
    const before = JSON.stringify(week);
    hiddenWeekdays(week);
    expect(JSON.stringify(week)).toBe(before);
  });
});

describe("businessHoursOf: one entry per open range, a lunch break as two", () => {
  test("null carries no range at all", () => {
    expect(businessHoursOf(null)).toEqual([]);
  });

  test("a lunch break is the two ranges either side of it, not one long one", () => {
    const week = [CLOSED, [MORNING, AFTERNOON], CLOSED, CLOSED, CLOSED, CLOSED, CLOSED];
    expect(businessHoursOf(week)).toEqual([
      { daysOfWeek: [1], startTime: "08:00", endTime: "12:00" },
      { daysOfWeek: [1], startTime: "14:00", endTime: "18:00" },
    ]);
  });

  test("a closed day contributes no entry", () => {
    const week = [CLOSED, CLOSED, CLOSED, CLOSED, CLOSED, CLOSED, CLOSED];
    expect(businessHoursOf(week)).toEqual([]);
  });
});

describe("slotDurationOf: minutes to FullCalendar's HH:MM:00", () => {
  test("no setting falls back to the server's own default, 15 minutes", () => {
    expect(slotDurationOf(undefined)).toBe("00:15:00");
  });

  test("a plain 30-minute grid", () => {
    expect(slotDurationOf(30)).toBe("00:30:00");
  });

  test("an hour and a half rolls into the hours place", () => {
    expect(slotDurationOf(90)).toBe("01:30:00");
  });
});

describe("absenceBackgroundEvents: blocks as background events", () => {
  test("one block, greyed", () => {
    const events = absenceBackgroundEvents([
      { id: "b1", starts_at: "2026-09-24 08:00:00", ends_at: "2026-09-24 12:00:00" },
    ]);
    expect(events).toHaveLength(1);
    expect(events[0]).toMatchObject({ id: "absence-b1", display: "background" });
    expect(events[0].classNames).toContain("book-absence");
    // The block's own start and end, not swapped: a block painted back to
    // front would grey the wrong hours of the day.
    expect(formatStartsAt(events[0].start)).toBe("2026-09-24 08:00:00");
    expect(formatStartsAt(events[0].end)).toBe("2026-09-24 12:00:00");
  });

  test("no blocks makes no events", () => {
    expect(absenceBackgroundEvents([])).toEqual([]);
  });

  test("a block with an unparsable stamp is dropped, not thrown on", () => {
    const events = absenceBackgroundEvents([
      { id: "bad", starts_at: "not-a-date", ends_at: "2026-09-24 12:00:00" },
      { id: "b2", starts_at: "2026-09-25 08:00:00", ends_at: "2026-09-25 09:00:00" },
    ]);
    expect(events.map((e) => e.id)).toEqual(["absence-b2"]);
  });

  test("does not mutate the blocks array it was handed", () => {
    const blocks = [{ id: "b1", starts_at: "2026-09-24 08:00:00", ends_at: "2026-09-24 12:00:00" }];
    const before = JSON.stringify(blocks);
    absenceBackgroundEvents(blocks);
    expect(JSON.stringify(blocks)).toBe(before);
  });
});
