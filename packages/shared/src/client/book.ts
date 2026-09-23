// The appointment book and its tools (C5, C5b), for the same reason
// `patients.ts` beside it carries no switch of its own: a build that never
// mounts the book screen never calls any of these.

import type { AbsenceBlockDto } from "../generated/AbsenceBlockDto";
import type { AbsenceBlocksDto } from "../generated/AbsenceBlocksDto";
import type { AbsenceBlockWriteDto } from "../generated/AbsenceBlockWriteDto";
import type { AppointmentBookDto } from "../generated/AppointmentBookDto";
import type { AppointmentDto } from "../generated/AppointmentDto";
import type { AppointmentMoveDto } from "../generated/AppointmentMoveDto";
import type { AppointmentsDto } from "../generated/AppointmentsDto";
import type { BlockMadeDto } from "../generated/BlockMadeDto";
import type { DayListDto } from "../generated/DayListDto";
import type { FreeSlotDto } from "../generated/FreeSlotDto";
import type { SlotMinutesDto } from "../generated/SlotMinutesDto";
import type { VisitTypeDto } from "../generated/VisitTypeDto";
import type { VisitTypesDto } from "../generated/VisitTypesDto";
import type { VisitTypeWriteDto } from "../generated/VisitTypeWriteDto";
import type { WorkingHoursDto } from "../generated/WorkingHoursDto";
import type { WorkingHoursWriteDto } from "../generated/WorkingHoursWriteDto";
import { narrow } from "../client-response";
import type { Transport } from "../client";
import {
  absenceBlockSchema,
  absenceBlocksSchema,
  appointmentSchema,
  appointmentsSchema,
  blockMadeSchema,
  dayListSchema,
  freeSlotSchema,
  slotMinutesSchema,
  visitTypeSchema,
  visitTypesSchema,
  workingHoursSchema,
} from "../schemas/book";

const JSON_HEADERS = { "content-type": "application/json" };

export function bookClient({ send }: Transport) {
  return {
    /** One day of the book, or the Sunday-to-Saturday week holding a day.
     * Exactly one of `day` or `week` is sent; the server refuses both or
     * neither. */
    async listAppointments(range: { day: string } | { week: string }): Promise<AppointmentsDto> {
      const query = "day" in range ? `day=${range.day}` : `week=${range.week}`;
      return narrow(await send(`/appointments?${query}`), appointmentsSchema, "appointment book");
    },

    async getAppointment(id: string): Promise<AppointmentDto> {
      return narrow(await send(`/appointments/${id}`), appointmentSchema, "appointment");
    },

    async bookAppointment(input: AppointmentBookDto): Promise<AppointmentDto> {
      const body = await send("/appointments", {
        method: "POST",
        headers: JSON_HEADERS,
        body: JSON.stringify(input),
      });
      return narrow(body, appointmentSchema, "appointment");
    },

    /** Gives the slot back; the row stays, stamped. */
    async cancelAppointment(id: string): Promise<AppointmentDto> {
      const body = await send(`/appointments/${id}/cancel`, { method: "POST" });
      return narrow(body, appointmentSchema, "appointment");
    },

    /** The same row at a new start, on its own length. */
    async moveAppointment(id: string, input: AppointmentMoveDto): Promise<AppointmentDto> {
      const body = await send(`/appointments/${id}/move`, {
        method: "POST",
        headers: JSON_HEADERS,
        body: JSON.stringify(input),
      });
      return narrow(body, appointmentSchema, "appointment");
    },

    async markAppointmentNoShow(id: string): Promise<AppointmentDto> {
      const body = await send(`/appointments/${id}/no-show`, { method: "POST" });
      return narrow(body, appointmentSchema, "appointment");
    },

    async clearAppointmentNoShow(id: string): Promise<AppointmentDto> {
      const body = await send(`/appointments/${id}/no-show/clear`, { method: "POST" });
      return narrow(body, appointmentSchema, "appointment");
    },

    /** The book's slot length in minutes. Read by anyone signed in. */
    async getSlotMinutes(): Promise<SlotMinutesDto> {
      return narrow(await send("/settings/slot-minutes"), slotMinutesSchema, "slot length");
    },

    async setSlotMinutes(input: SlotMinutesDto): Promise<SlotMinutesDto> {
      const body = await send("/settings/slot-minutes", {
        method: "PUT",
        headers: JSON_HEADERS,
        body: JSON.stringify(input),
      });
      return narrow(body, slotMinutesSchema, "slot length");
    },

    /** The cabinet's week, Sunday first. `null` while never set. */
    async getWorkingHours(): Promise<WorkingHoursDto> {
      return narrow(await send("/settings/working-hours"), workingHoursSchema, "working hours");
    },

    async setWorkingHours(input: WorkingHoursWriteDto): Promise<WorkingHoursDto> {
      const body = await send("/settings/working-hours", {
        method: "PUT",
        headers: JSON_HEADERS,
        body: JSON.stringify(input),
      });
      return narrow(body, workingHoursSchema, "working hours");
    },

    /** The blocks not yet over, in time order. */
    async listAbsenceBlocks(): Promise<AbsenceBlocksDto> {
      return narrow(await send("/absence-blocks"), absenceBlocksSchema, "absence blocks");
    },

    /** A new block, and every live appointment it lands on: the desk moves
     * or cancels each one from the answer, the block itself changed
     * none. */
    async createAbsenceBlock(input: AbsenceBlockWriteDto): Promise<BlockMadeDto> {
      const body = await send("/absence-blocks", {
        method: "POST",
        headers: JSON_HEADERS,
        body: JSON.stringify(input),
      });
      return narrow(body, blockMadeSchema, "absence block");
    },

    async removeAbsenceBlock(id: string): Promise<AbsenceBlockDto> {
      const body = await send(`/absence-blocks/${id}/remove`, { method: "POST" });
      return narrow(body, absenceBlockSchema, "absence block");
    },

    /** Every visit type of the cabinet, by name. */
    async listVisitTypes(): Promise<VisitTypesDto> {
      return narrow(await send("/settings/visit-types"), visitTypesSchema, "visit types");
    },

    async createVisitType(input: VisitTypeWriteDto): Promise<VisitTypeDto> {
      const body = await send("/settings/visit-types", {
        method: "POST",
        headers: JSON_HEADERS,
        body: JSON.stringify(input),
      });
      return narrow(body, visitTypeSchema, "visit type");
    },

    async updateVisitType(id: string, input: VisitTypeWriteDto): Promise<VisitTypeDto> {
      const body = await send(`/settings/visit-types/${id}`, {
        method: "PUT",
        headers: JSON_HEADERS,
        body: JSON.stringify(input),
      });
      return narrow(body, visitTypeSchema, "visit type");
    },

    async removeVisitType(id: string): Promise<VisitTypeDto> {
      const body = await send(`/settings/visit-types/${id}/remove`, { method: "POST" });
      return narrow(body, visitTypeSchema, "visit type");
    },

    /** The earliest start a booking would be taken at, from a date and an
     * optional visit type; `offsetDays` is "see again in N days" counted
     * from `from` (or from today, server side, without one). */
    async nextFreeSlot(query: {
      from?: string;
      offsetDays?: number;
      visitTypeId?: string;
    }): Promise<FreeSlotDto> {
      const params = new URLSearchParams();
      if (query.from !== undefined) params.set("from", query.from);
      if (query.offsetDays !== undefined) params.set("offset_days", String(query.offsetDays));
      if (query.visitTypeId !== undefined) params.set("visit_type_id", query.visitTypeId);
      const qs = params.size === 0 ? "" : `?${params.toString()}`;
      return narrow(await send(`/appointments/next-free${qs}`), freeSlotSchema, "next free slot");
    },

    /** One day's appointments and walk-ins, printed by the screen. */
    async dayList(day: string): Promise<DayListDto> {
      return narrow(await send(`/day-list?day=${day}`), dayListSchema, "day list");
    },
  };
}
