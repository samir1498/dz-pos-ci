// The book's own reading and writing of the shop-clock strings the server
// takes and gives back (`YYYY-MM-DD HH:MM:SS`, `crates/api/src/dto/
// appointments.rs`'s `parse_starts_at`). Every function here reads a JS
// `Date`'s *local* getters and never `toISOString`/`getUTCHours`: FullCalendar
// (pinned to the plain-`Date` v6 line, no Temporal) hands the screen a `Date`
// in the browser's own zone, and the shop's clock is that zone by
// construction on a machine at the till. Going through UTC would slide every
// booking by the machine's offset from Algeria's UTC+1.

function pad(n: number): string {
  return n < 10 ? `0${n}` : String(n);
}

/** A `Date` FullCalendar handed the screen (a slot click, a drag target) to
 *  the exact string a booking or a move sends. */
export function formatStartsAt(date: Date): string {
  const y = date.getFullYear();
  const mo = pad(date.getMonth() + 1);
  const d = pad(date.getDate());
  const h = pad(date.getHours());
  const mi = pad(date.getMinutes());
  const s = pad(date.getSeconds());
  return `${y}-${mo}-${d} ${h}:${mi}:${s}`;
}

/** The same `Date`, day only, for the `day=` and `week=` query parameters. */
export function formatDay(date: Date): string {
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

/** A slot's end, from its own start and length: the grid draws an
 *  appointment's block this many minutes tall. Algeria keeps no daylight
 *  saving (`crates/core/src/services/clock.rs`), so plain millisecond
 *  arithmetic never crosses a clock change. */
export function addMinutes(date: Date, minutes: number): Date {
  return new Date(date.getTime() + minutes * 60_000);
}

const STARTS_AT_RE = /^(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2}):(\d{2})$/;

/** The reverse of `formatStartsAt`, for an appointment the server sent: a
 *  `Date` FullCalendar can place on the grid. `null` on anything that is not
 *  exactly the server's own shape, rather than a `Date` silently built out of
 *  whatever `Date.parse` guesses (`Date.parse("2026-02-30 10:00:00")` does
 *  not throw, it rolls over into March). */
export function parseStartsAt(text: string): Date | null {
  const match = STARTS_AT_RE.exec(text);
  if (match === null) return null;
  const [, y, mo, d, h, mi, s] = match;
  const date = new Date(Number(y), Number(mo) - 1, Number(d), Number(h), Number(mi), Number(s));
  // A day that does not exist (30 February) rolls into the next month;
  // reading the fields back is how that corrupt input is told from a real
  // one rather than trusted because the regexp matched its shape.
  if (formatStartsAt(date) !== text) return null;
  return date;
}

const DAY_RE = /^(\d{4})-(\d{2})-(\d{2})$/;

/** The day after a `YYYY-MM-DD` day, the same shape back: the day the
 *  confirmation calls are about (C6b). Through a local `Date` at noon, so
 *  the month and year roll over and no clock change can land on midnight.
 *  `null` on anything that is not a real day. */
export function nextDay(day: string): string | null {
  const match = DAY_RE.exec(day);
  if (match === null) return null;
  const [, y, mo, d] = match;
  const date = new Date(Number(y), Number(mo) - 1, Number(d), 12, 0, 0);
  if (formatDay(date) !== day) return null;
  date.setDate(date.getDate() + 1);
  return formatDay(date);
}
