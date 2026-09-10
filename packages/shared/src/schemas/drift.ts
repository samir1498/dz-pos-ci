// The two ways a schema can drift from the Rust DTO it checks.
//
// `satisfies z.ZodType<Generated>` on the declaration catches a field the
// Rust struct gained: the schema's answer stops covering the generated type.
// It says nothing about a field the struct dropped, because an answer with
// extra fields is still assignable to a type without them. `Covers` is that
// other direction, so the pair pins a schema to `src/generated` both ways and
// a drifted schema fails `just types-check` instead of a shop's screen.

import type { z } from "zod";

/** True when the generated type is one of the schema's answers. A union
 *  distributes here, so a member the schema does not cover turns the whole
 *  thing into `boolean` and `Assert` refuses it. */
export type Covers<Generated, Schema extends z.ZodType> = Generated extends z.output<Schema>
  ? true
  : false;

/** Compiles for `true` and for nothing else. */
export type Assert<Covered extends true> = Covered;
