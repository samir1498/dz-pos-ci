// The till's shifts: the drawer a cashier opens with a float at the start of
// the day and counts again at close (features.md §1, the cash position; plan
// till-shifts-a-float-and-a-count).

import { z } from "zod";

import type { ShiftDto } from "../generated/ShiftDto";
import type { ShiftReportDto } from "../generated/ShiftReportDto";
import { exactInteger } from "./common";
import type { Assert, Matches } from "./drift";
import { takingsSchema } from "./expense";

/** The four close columns travel together or not at all, the way the core's
 *  `models::shift::ShiftClose` holds them: a row carrying a count and no
 *  moment never reaches this schema, because the core never writes one. */
export const shiftSchema = z.object({
  id: z.number(),
  opened_by: z.number(),
  opened_at: z.string(),
  opening_cash_centimes: exactInteger,
  closed_at: z.string().nullable(),
  closed_by: z.number().nullable(),
  counted_centimes: exactInteger.nullable(),
  expected_at_close_centimes: exactInteger.nullable(),
  difference_centimes: exactInteger.nullable(),
  note: z.string().nullable(),
}) satisfies z.ZodType<ShiftDto>;
type _Shift = Assert<Matches<ShiftDto, typeof shiftSchema>>;

/** A shift with the figures a screen puts beside it: this person's takings
 *  over the window, read live, and the expected figure to check the count
 *  against. `expected_centimes` is the stored snapshot once the shift is
 *  closed and a fresh sum while it is open (`ShiftReportDto`'s own doc), and
 *  this schema does not tell the two apart — the screen reads `shift.closed_at`
 *  for that, the way the core's own report does.
 *
 *  `refunds_centimes` is the cash this person handed back over the window,
 *  which the expected figure has already been lowered by. It travels beside
 *  it so a close screen can say why the drawer is lighter than the takings. */
export const shiftReportSchema = z.object({
  shift: shiftSchema,
  takings: takingsSchema,
  refunds_centimes: exactInteger,
  expected_centimes: exactInteger,
  difference_centimes: exactInteger.nullable(),
  until: z.string(),
}) satisfies z.ZodType<ShiftReportDto>;
type _ShiftReport = Assert<Matches<ShiftReportDto, typeof shiftReportSchema>>;
