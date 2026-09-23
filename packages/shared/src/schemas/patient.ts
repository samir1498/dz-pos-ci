// The patient file (C3, C3b of
// `the-first-clinic-module-patients-queue-appointments`). `notes` is the one
// field that is not always there: the server drops the key for a caller
// without `ViewPatientNotes` rather than sending an empty answer, so the
// screen can tell "not mine to read" from "the doctor left this blank"
// (`crates/api/src/dto/patients.rs`).

import { z } from "zod";

import type { PatientDto } from "../generated/PatientDto";
import type { PatientWriteDto } from "../generated/PatientWriteDto";
import type { SexDto } from "../generated/SexDto";
import type { Assert, Matches } from "./drift";

export const sexSchema = z.enum(["female", "male"]) satisfies z.ZodType<SexDto>;
type _Sex = Assert<Matches<SexDto, typeof sexSchema>>;

export const patientSchema = z.object({
  id: z.string(),
  first_name: z.string(),
  last_name: z.string(),
  sex: sexSchema.nullable(),
  date_of_birth: z.string().nullable(),
  phone: z.string().nullable(),
  notes: z.string().nullable().optional(),
  archived_at: z.string().nullable(),
  created_at: z.string(),
  updated_at: z.string(),
}) satisfies z.ZodType<PatientDto>;
type _Patient = Assert<Matches<PatientDto, typeof patientSchema>>;

/** The whole file, written on a create and an update alike. `notes` always
 *  travels: `null` clears it or, from a caller without `ViewPatientNotes`,
 *  changes nothing at all (the service keeps the stored value for that
 *  caller); a non-null value from that same caller is refused with 403
 *  before anything is written. The client never sends an empty string. */
export const patientWriteSchema = z.object({
  first_name: z.string(),
  last_name: z.string(),
  sex: sexSchema.nullable(),
  date_of_birth: z.string().nullable(),
  phone: z.string().nullable(),
  notes: z.string().nullable(),
}) satisfies z.ZodType<PatientWriteDto>;
type _PatientWrite = Assert<Matches<PatientWriteDto, typeof patientWriteSchema>>;
