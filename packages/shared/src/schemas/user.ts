// A fiche on the users screen (M4 T8). No hash and no failure counter ever
// cross the wire: `has_pin` and `has_password` are the honest answer to
// "can this person sign in", and the two booleans are all a reset needs to
// know it worked.

import { z } from "zod";

import type { UserDto } from "../generated/UserDto";
import { exactInteger } from "./common";
import type { Assert, Matches } from "./drift";
import { roleSchema } from "./session";

export const userSchema = z.object({
  id: exactInteger,
  shop_id: exactInteger,
  name: z.string(),
  role: roleSchema,
  has_pin: z.boolean(),
  has_password: z.boolean(),
  active: z.boolean(),
}) satisfies z.ZodType<UserDto>;
type _User = Assert<Matches<UserDto, typeof userSchema>>;
