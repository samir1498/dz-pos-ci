// Turning the cabinet's own settings (working hours, absence blocks) into
// the plain objects FullCalendar's `hiddenDays`, `businessHours` and
// `events` (display: "background") take. Kept apart from `BookScreen.tsx`
// so the mapping is unit-testable without mounting FullCalendar at all,
// which jsdom cannot lay out.

import { parseStartsAt } from "./dates";

export interface WorkingHoursRange {
  readonly opens: string;
  readonly closes: string;
}

/** `null` while the cabinet has never set its hours: the book refuses no
 *  time of any day (`WorkingHoursDto`'s own contract), so nothing is hidden
 *  and nothing is greyed either. */
export type WorkingWeek = ReadonlyArray<ReadonlyArray<WorkingHoursRange>> | null;

/** The weekday indices (0 = Sunday, matching `firstDay: 0`) whose working
 *  hours hold no open range at all: a whole day closed, hidden from the
 *  grid rather than shown empty. */
export function hiddenWeekdays(days: WorkingWeek): readonly number[] {
  if (days === null) return [];
  const hidden: number[] = [];
  days.forEach((ranges, weekday) => {
    if (ranges.length === 0) hidden.push(weekday);
  });
  return hidden;
}

export interface BusinessHoursRange {
  readonly daysOfWeek: readonly [number];
  readonly startTime: string;
  readonly endTime: string;
}

/** One entry per open range of every weekday, the shape FullCalendar's
 *  `businessHours` option takes; it also paints every hour outside these
 *  (a lunch break included, since a break is a gap between two ranges) with
 *  its own dimmed `.fc-non-business` background, which is where "closed
 *  hours ... show as grey background" comes from without a component of
 *  Dinar's own. */
export function businessHoursOf(days: WorkingWeek): readonly BusinessHoursRange[] {
  if (days === null) return [];
  return days.flatMap((ranges, weekday) =>
    ranges.map((range) => ({
      daysOfWeek: [weekday] as const,
      startTime: range.opens,
      endTime: range.closes,
    })),
  );
}

/** The slot length in minutes to the duration string FullCalendar's
 *  `slotDuration` takes (`"HH:MM:00"`), 15 minutes without an answer yet:
 *  `DEFAULT_SLOT_MINUTES` in `crates/clinic/src/services/slot_length.rs`,
 *  the same fallback the server itself gives a cabinet that never set one. */
export function slotDurationOf(minutes: number | undefined): string {
  const value = minutes ?? 15;
  const hours = Math.floor(value / 60);
  const rest = value % 60;
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(hours)}:${pad(rest)}:00`;
}

export interface AbsenceBlockLike {
  readonly id: string;
  readonly starts_at: string;
  readonly ends_at: string;
}

export interface AbsenceBackgroundEvent {
  readonly id: string;
  readonly start: Date;
  readonly end: Date;
  readonly display: "background";
  readonly classNames: readonly string[];
}

/** Absence blocks as FullCalendar background events, greyed the same way a
 *  closed hour is: `BookScreen.tsx` sets `--fc-bg-event-color` to the same
 *  `--muted` token as `--fc-non-business-color`, so a background event needs
 *  no CSS file of its own. `classNames` carries `book-absence` in case a
 *  later screen wants to target it, but today's greying comes from that
 *  custom property alone. A block whose stamps do not parse (never sent by
 *  this server, but not this mapping's job to trust) is left out rather
 *  than thrown on, so one bad row does not blank the whole grid. */
export function absenceBackgroundEvents(
  blocks: readonly AbsenceBlockLike[],
): readonly AbsenceBackgroundEvent[] {
  const events: AbsenceBackgroundEvent[] = [];
  for (const block of blocks) {
    const start = parseStartsAt(block.starts_at);
    const end = parseStartsAt(block.ends_at);
    if (start === null || end === null) continue;
    events.push({
      id: `absence-${block.id}`,
      start,
      end,
      display: "background",
      classNames: ["book-absence"],
    });
  }
  return events;
}
