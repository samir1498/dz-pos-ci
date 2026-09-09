// A day as the API takes it, read off the machine's own calendar rather than
// off UTC. A shop open at half past midnight is already on tomorrow's date
// while UTC is still on yesterday's, and a statement asked for "to today"
// would then end before every movement of that evening and close on nothing.

/** Today as `YYYY-MM-DD` on the machine's own calendar. */
export function todayAsDay(now: Date = new Date()): string {
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${now.getFullYear()}-${month}-${day}`;
}
