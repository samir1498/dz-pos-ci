// The arithmetic behind DateField and MonthField. Kept apart from the
// components because both of them need it and neither owns it: a day box
// clamps against the month it is sitting next to, a month box clamps on its
// own, and both build back into the ISO string the rest of the app already
// speaks.
//
// No `Date` anywhere. `new Date(year, month - 1, day)` rolls a day-31-of-
// April into May 1st instead of saying the date does not exist, which is
// the one thing this file has to get right, and `Intl` is the machine's
// language again, the thing the two fields exist to stop leaking. So the
// calendar is written out by hand: it is twelve numbers and one rule about
// February.

import type { Key } from "@/i18n";

export const MONTH_KEYS: readonly Key[] = [
  "month_01",
  "month_02",
  "month_03",
  "month_04",
  "month_05",
  "month_06",
  "month_07",
  "month_08",
  "month_09",
  "month_10",
  "month_11",
  "month_12",
];

const DAYS_IN_MONTH: readonly number[] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

function isLeapYear(year: number): boolean {
  return year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
}

function daysInMonth(month: number, year: number): number {
  if (month === 2 && isLeapYear(year)) return 29;
  return DAYS_IN_MONTH[month - 1] ?? 31;
}

/** Digits only, and only as many as the segment can hold. */
function digits(text: string, maxLength: number): string {
  return text.replace(/\D/g, "").slice(0, maxLength);
}

/**
 * A segment cannot be typed past what a day or a month can ever be: the
 * clamp is on the number, not on the calendar (a day of 31 is still let
 * through here even in April, since which months have thirty is a
 * question for `toIsoDate`, not for the keystroke).
 */
export function clampSegment(text: string, max: number, maxLength: number): string {
  const clean = digits(text, maxLength);
  if (clean === "") return "";
  const value = Number(clean);
  return value > max ? String(max) : clean;
}

/**
 * The day, brought back to the last one the month has. Typing 31 in
 * February left the boxes reading 31/02/2026 while the caller was handed
 * `""`, so a shop saw a date on the screen and the screen below it behaved
 * as though no date had been entered at all. The correction waits for the
 * year, because 29 February is a real day in a year nobody has finished
 * typing yet.
 */
export function clampDayToMonth(day: string, month: string, year: string): string {
  if (day === "" || month === "" || year.length !== 4) return day;
  const m = Number(month);
  if (m < 1 || m > 12) return day;
  const last = daysInMonth(m, Number(year));
  return Number(day) > last ? String(last) : day;
}

export function clampYear(text: string): string {
  return digits(text, 4);
}

function pad2(text: string): string {
  return text.padStart(2, "0");
}

export interface DateParts {
  readonly day: string;
  readonly month: string;
  readonly year: string;
}

export function parseIsoDate(value: string): DateParts {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
  if (!match) return { day: "", month: "", year: "" };
  const [, year, month, day] = match;
  return { day: day ?? "", month: month ?? "", year: year ?? "" };
}

/** `""` for anything short of a real calendar date: an empty segment, a
 *  year not fully typed, or a day that does not exist in that month
 *  (31 February included). Every call site already reads `""` as "no
 *  date", so an incomplete one is just left alone rather than guessed at. */
export function toIsoDate(day: string, month: string, year: string): string {
  if (day === "" || month === "" || year.length !== 4) return "";
  const d = Number(day);
  const m = Number(month);
  const y = Number(year);
  if (d < 1 || m < 1 || m > 12) return "";
  if (d > daysInMonth(m, y)) return "";
  return `${year}-${pad2(month)}-${pad2(day)}`;
}

export interface MonthParts {
  readonly month: string;
  readonly year: string;
}

export function parseIsoMonth(value: string): MonthParts {
  const match = /^(\d{4})-(\d{2})$/.exec(value);
  if (!match) return { month: "", year: "" };
  const [, year, month] = match;
  return { month: month ?? "", year: year ?? "" };
}

export function toIsoMonth(month: string, year: string): string {
  if (month === "" || year.length !== 4) return "";
  const m = Number(month);
  if (m < 1 || m > 12) return "";
  return `${year}-${pad2(month)}`;
}
