// The appointment book and its tools (C5, C5b of
// `the-first-clinic-module-patients-queue-appointments`). One doctor, a
// slot grid, and the desk's own tools around it: working hours, absence
// blocks, visit types, the next free slot and the printed day list.

import { z } from "zod";

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
import type { OpenRangeDto } from "../generated/OpenRangeDto";
import type { SlotMinutesDto } from "../generated/SlotMinutesDto";
import type { VisitTypeDto } from "../generated/VisitTypeDto";
import type { VisitTypesDto } from "../generated/VisitTypesDto";
import type { VisitTypeWriteDto } from "../generated/VisitTypeWriteDto";
import type { WorkingHoursDto } from "../generated/WorkingHoursDto";
import type { WorkingHoursWriteDto } from "../generated/WorkingHoursWriteDto";
import type { Assert, Matches } from "./drift";
import { queueEntrySchema } from "./queue";

export const appointmentSchema = z.object({
  id: z.string(),
  patient_id: z.string(),
  first_name: z.string(),
  last_name: z.string(),
  starts_at: z.string(),
  slot_minutes: z.number(),
  note: z.string().nullable(),
  cancelled_at: z.string().nullable(),
  no_show_at: z.string().nullable(),
}) satisfies z.ZodType<AppointmentDto>;
type _Appointment = Assert<Matches<AppointmentDto, typeof appointmentSchema>>;

export const appointmentsSchema = z.object({
  from: z.string(),
  to: z.string(),
  appointments: z.array(appointmentSchema),
}) satisfies z.ZodType<AppointmentsDto>;
type _Appointments = Assert<Matches<AppointmentsDto, typeof appointmentsSchema>>;

export const appointmentBookSchema = z.object({
  patient_id: z.string(),
  starts_at: z.string(),
  note: z.string().nullable(),
  visit_type_id: z.string().optional(),
}) satisfies z.ZodType<AppointmentBookDto>;
type _AppointmentBook = Assert<Matches<AppointmentBookDto, typeof appointmentBookSchema>>;

export const appointmentMoveSchema = z.object({
  starts_at: z.string(),
}) satisfies z.ZodType<AppointmentMoveDto>;
type _AppointmentMove = Assert<Matches<AppointmentMoveDto, typeof appointmentMoveSchema>>;

export const slotMinutesSchema = z.object({
  slot_minutes: z.number(),
}) satisfies z.ZodType<SlotMinutesDto>;
type _SlotMinutes = Assert<Matches<SlotMinutesDto, typeof slotMinutesSchema>>;

export const openRangeSchema = z.object({
  opens: z.string(),
  closes: z.string(),
}) satisfies z.ZodType<OpenRangeDto>;
type _OpenRange = Assert<Matches<OpenRangeDto, typeof openRangeSchema>>;

export const workingHoursSchema = z.object({
  days: z.array(z.array(openRangeSchema)).nullable(),
}) satisfies z.ZodType<WorkingHoursDto>;
type _WorkingHours = Assert<Matches<WorkingHoursDto, typeof workingHoursSchema>>;

export const workingHoursWriteSchema = z.object({
  days: z.array(z.array(openRangeSchema)),
}) satisfies z.ZodType<WorkingHoursWriteDto>;
type _WorkingHoursWrite = Assert<Matches<WorkingHoursWriteDto, typeof workingHoursWriteSchema>>;

export const absenceBlockSchema = z.object({
  id: z.string(),
  starts_at: z.string(),
  ends_at: z.string(),
  label: z.string().nullable(),
}) satisfies z.ZodType<AbsenceBlockDto>;
type _AbsenceBlock = Assert<Matches<AbsenceBlockDto, typeof absenceBlockSchema>>;

export const absenceBlockWriteSchema = z.object({
  starts_at: z.string(),
  ends_at: z.string(),
  label: z.string().nullable(),
}) satisfies z.ZodType<AbsenceBlockWriteDto>;
type _AbsenceBlockWrite = Assert<Matches<AbsenceBlockWriteDto, typeof absenceBlockWriteSchema>>;

export const absenceBlocksSchema = z.object({
  blocks: z.array(absenceBlockSchema),
}) satisfies z.ZodType<AbsenceBlocksDto>;
type _AbsenceBlocks = Assert<Matches<AbsenceBlocksDto, typeof absenceBlocksSchema>>;

export const blockMadeSchema = z.object({
  block: absenceBlockSchema,
  hits: z.array(appointmentSchema),
}) satisfies z.ZodType<BlockMadeDto>;
type _BlockMade = Assert<Matches<BlockMadeDto, typeof blockMadeSchema>>;

export const visitTypeSchema = z.object({
  id: z.string(),
  name: z.string(),
  minutes: z.number(),
}) satisfies z.ZodType<VisitTypeDto>;
type _VisitType = Assert<Matches<VisitTypeDto, typeof visitTypeSchema>>;

export const visitTypeWriteSchema = z.object({
  name: z.string(),
  minutes: z.number(),
}) satisfies z.ZodType<VisitTypeWriteDto>;
type _VisitTypeWrite = Assert<Matches<VisitTypeWriteDto, typeof visitTypeWriteSchema>>;

export const visitTypesSchema = z.object({
  visit_types: z.array(visitTypeSchema),
}) satisfies z.ZodType<VisitTypesDto>;
type _VisitTypes = Assert<Matches<VisitTypesDto, typeof visitTypesSchema>>;

export const freeSlotSchema = z.object({
  first_day: z.string(),
  last_day: z.string(),
  slot_minutes: z.number(),
  starts_at: z.string().nullable(),
}) satisfies z.ZodType<FreeSlotDto>;
type _FreeSlot = Assert<Matches<FreeSlotDto, typeof freeSlotSchema>>;

export const dayListSchema = z.object({
  day: z.string(),
  appointments: z.array(appointmentSchema),
  walk_ins: z.array(queueEntrySchema),
}) satisfies z.ZodType<DayListDto>;
type _DayList = Assert<Matches<DayListDto, typeof dayListSchema>>;
