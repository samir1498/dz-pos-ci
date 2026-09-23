// What the day list reads off one `DayListDto` (C6b): which bookings have
// arrived, and which the desk might mark no-show at the end of the day.
// Arrived is not a column of the appointment; it is a queue entry naming
// it (`queue_entries.appointment_id`), so the day's queue is the one place
// to read it from.

import type { AppointmentDto, DayListDto } from "@dzpos/shared";

/** The ids of the day's appointments that have a queue entry naming them. */
export function arrivedAppointmentIds(dayList: DayListDto): ReadonlySet<string> {
  const ids = new Set<string>();
  for (const entry of dayList.walk_ins) {
    if (entry.appointment_id !== null) ids.add(entry.appointment_id);
  }
  return ids;
}

/** The end-of-day suggestion: the day's bookings never marked arrived, not
 *  cancelled and not already marked no-show, in time order. A suggestion
 *  only; the desk ticks which ones to mark, and nothing is marked without
 *  it. */
export function unarrivedBookings(dayList: DayListDto): AppointmentDto[] {
  const arrived = arrivedAppointmentIds(dayList);
  return dayList.appointments.filter(
    (appointment) =>
      !arrived.has(appointment.id) && appointment.cancelled_at === null && appointment.no_show_at === null,
  );
}
