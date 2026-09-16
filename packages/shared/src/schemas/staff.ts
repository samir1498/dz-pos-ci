// One name on the sign-in picker (`GET /auth/staff`). Read before anyone is
// signed in, so it carries the least that lets a cashier tap their name and
// be shown the right box: no shop id, no `active` (the list holds only
// active fiches), and no hash — the rule `userSchema` keeps, kept here for
// a list that a stolen paired phone could read.

import { z } from "zod";

import type { StaffDto } from "../generated/StaffDto";
import { exactInteger } from "./common";
import type { Assert, Matches } from "./drift";
import { roleSchema } from "./session";

export const staffSchema = z.object({
  id: exactInteger,
  name: z.string(),
  role: roleSchema,
  has_pin: z.boolean(),
  has_password: z.boolean(),
}) satisfies z.ZodType<StaffDto>;
type _Staff = Assert<Matches<StaffDto, typeof staffSchema>>;
