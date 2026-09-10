// The three ways a schema can drift from the Rust DTO it checks.
//
// `satisfies z.ZodType<Generated>` on the declaration catches a field the
// Rust struct gained: the schema's answer stops covering the generated type.
// It says nothing about a field the struct dropped, because an answer with
// extra fields is still assignable to a type without them, and nothing about
// an optional field either way, because an answer without an optional key is
// assignable too. `Matches` is the other two directions, so the pair pins a
// schema to `src/generated` whole and a drifted schema fails
// `just types-check` instead of a shop's screen.
//
// The optional direction is not a detail. `z.object` strips a key the schema
// has no field for, so an optional figure the API gained would be parsed
// away in silence and the panel that shows it would show nothing.

import type { z } from "zod";

/** True when the generated type is one of the schema's answers. A union
 *  distributes here, so a member the schema does not cover turns the whole
 *  thing into `boolean` and `Assert` refuses it. */
export type Covers<Generated, Schema extends z.ZodType> = Generated extends z.output<Schema>
  ? true
  : false;

/** True when two types are the same one, optional keys and all. The pair of
 *  identical conditional types is the trick that makes TypeScript compare
 *  them rather than ask whether one is assignable to the other. */
export type Equal<Left, Right> =
  (<T>() => T extends Left ? 1 : 2) extends <T>() => T extends Right ? 1 : 2 ? true : false;

/** True when the schema's answer is keyed exactly like the generated type. A
 *  required key it is short of is already a `Covers` failure; this is what
 *  catches an optional one, which nothing else can see. */
export type SameKeys<Generated, Schema extends z.ZodType> = Equal<
  keyof Generated,
  keyof z.output<Schema>
>;

/** The whole check: the generated type is one of the schema's answers, and
 *  the two are keyed alike. Declared beside every schema, under the
 *  `satisfies` that carries the third direction. */
export type Matches<Generated, Schema extends z.ZodType> =
  Covers<Generated, Schema> extends true ? SameKeys<Generated, Schema> : false;

/** Compiles for `true` and for nothing else. */
export type Assert<Covered extends true> = Covered;
