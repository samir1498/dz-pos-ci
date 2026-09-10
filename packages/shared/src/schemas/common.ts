// The two field shapes every DTO family repeats.

import { z } from "zod";

/** An amount, a quantity or a byte count: an integer JSON.parse did not have
 *  to round. Past 2^53 a centime figure comes back already rounded, so it is
 *  refused rather than shown to a shop. `z.int()` is zod's safe-integer
 *  format, the same bound `Number.isSafeInteger` draws. */
export const exactInteger = z.int();

/** `YYYY-MM-DD`, the only shape the API writes a calendar day in. A day the
 *  client cannot parse is a server it cannot date anything from. */
export const day = z.string().regex(/^\d{4}-\d{2}-\d{2}$/);
