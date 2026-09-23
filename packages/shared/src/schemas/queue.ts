// Today's waiting queue (C4 of
// `the-first-clinic-module-patients-queue-appointments`).

import { z } from "zod";

import type { QueueAddDto } from "../generated/QueueAddDto";
import type { QueueEntryDto } from "../generated/QueueEntryDto";
import type { QueueOrderDto } from "../generated/QueueOrderDto";
import type { Assert, Matches } from "./drift";

export const queueEntrySchema = z.object({
  id: z.string(),
  patient_id: z.string(),
  first_name: z.string(),
  last_name: z.string(),
  day: z.string(),
  arrived_at: z.string(),
  called_at: z.string().nullable(),
  seen_at: z.string().nullable(),
  left_at: z.string().nullable(),
  appointment_id: z.string().nullable(),
  appointment_starts_at: z.string().nullable(),
  position: z.number(),
}) satisfies z.ZodType<QueueEntryDto>;
type _QueueEntry = Assert<Matches<QueueEntryDto, typeof queueEntrySchema>>;

export const queueAddSchema = z.object({
  patient_id: z.string(),
}) satisfies z.ZodType<QueueAddDto>;
type _QueueAdd = Assert<Matches<QueueAddDto, typeof queueAddSchema>>;

export const queueOrderSchema = z.object({
  ids: z.array(z.string()),
}) satisfies z.ZodType<QueueOrderDto>;
type _QueueOrder = Assert<Matches<QueueOrderDto, typeof queueOrderSchema>>;
