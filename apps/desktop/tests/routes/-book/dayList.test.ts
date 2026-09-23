// What the day list reads off one `DayListDto` (C6b). Arrived is a queue
// entry naming the booking, so these cases are about which entries count.

import { describe, expect, test } from "vitest";
import type { AppointmentDto, DayListDto, QueueEntryDto } from "@dzpos/shared";

import { arrivedAppointmentIds, unarrivedBookings } from "../../../src/routes/-book/dayList";

const entry: QueueEntryDto = {
  id: "q1",
  patient_id: "p1",
  first_name: "Amina",
  last_name: "Benali",
  day: "2026-09-23",
  arrived_at: "2026-09-23 08:50:00",
  called_at: null,
  seen_at: null,
  left_at: null,
  appointment_id: null,
  appointment_starts_at: null,
  position: 1,
};

function day(walkIns: QueueEntryDto[]): DayListDto {
  return { day: "2026-09-23", appointments: [], walk_ins: walkIns };
}

describe("arrivedAppointmentIds", () => {
  test("names the bookings a queue entry points at, seen or not, and no walk-in", () => {
    const ids = arrivedAppointmentIds(
      day([
        entry,
        { ...entry, id: "q2", appointment_id: "a1", appointment_starts_at: "2026-09-23 09:00:00" },
        {
          ...entry,
          id: "q3",
          appointment_id: "a2",
          appointment_starts_at: "2026-09-23 09:15:00",
          called_at: "2026-09-23 09:10:00",
          seen_at: "2026-09-23 09:20:00",
        },
      ]),
    );
    expect([...ids].sort()).toEqual(["a1", "a2"]);
  });

  test("an empty waiting room has nobody arrived", () => {
    expect(arrivedAppointmentIds(day([])).size).toBe(0);
  });
});

const booking: AppointmentDto = {
  id: "a1",
  patient_id: "p1",
  first_name: "Amina",
  last_name: "Benali",
  starts_at: "2026-09-23 09:00:00",
  slot_minutes: 15,
  note: null,
  cancelled_at: null,
  no_show_at: null,
  phone: null,
  call_outcome: null,
  call_at: null,
};

describe("unarrivedBookings", () => {
  test("keeps, in order, only the bookings not arrived, not cancelled and not already marked", () => {
    const list: DayListDto = {
      day: "2026-09-23",
      appointments: [
        booking,
        { ...booking, id: "a2", starts_at: "2026-09-23 09:15:00" },
        { ...booking, id: "a3", starts_at: "2026-09-23 09:30:00", no_show_at: "2026-09-23 09:40:00" },
        { ...booking, id: "a4", starts_at: "2026-09-23 09:45:00", cancelled_at: "2026-09-22 10:00:00" },
        { ...booking, id: "a5", starts_at: "2026-09-23 10:00:00", call_outcome: "no_answer", call_at: "x" },
      ],
      walk_ins: [{ ...entry, id: "q9", appointment_id: "a2", appointment_starts_at: "2026-09-23 09:15:00" }],
    };
    expect(unarrivedBookings(list).map((a) => a.id)).toEqual(["a1", "a5"]);
  });
});
