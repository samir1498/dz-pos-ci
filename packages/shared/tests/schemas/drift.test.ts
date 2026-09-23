// Contract: the drift check refuses a schema that no longer answers the
// generated DTO, in all three directions. The `@ts-expect-error` lines are the
// test: `tsc --noEmit` runs as part of `pnpm test`, so if a direction stops
// firing the suppression becomes unused and the gate fails. A runtime
// assertion cannot see any of this, which is why the mechanism is pinned here
// rather than left to a reviewer.

import { describe, expect, test } from "vitest";
import { z } from "zod";

import type { ApiErrorPayloadDto } from "../../src/generated/ApiErrorPayloadDto";
import type { StoreDto } from "../../src/generated/StoreDto";
import type { Assert, Covers, Matches } from "../../src/schemas/drift";
import { apiErrorPayloadSchema } from "../../src/schemas/error";
import { permissionSchema } from "../../src/schemas/session";
import { storeSchema } from "../../src/schemas/settings";

const store: StoreDto = {
  name: "Alimentation El Baraka",
  rc: "16/00-1234567B25",
  nif: "000216001234567",
  nis: null,
  ai: null,
  address: "12 rue Didouche Mourad, Alger",
  phone: "0550112233",
};

// A schema that lost a field the Rust struct has: `satisfies` refuses it,
// because its answer no longer covers StoreDto.
// @ts-expect-error the schema is short of every field but the name
const short = z.object({ name: z.string() }) satisfies z.ZodType<StoreDto>;

// A schema carrying a field the Rust struct does not have: `satisfies` takes
// it, since the extra field only makes the answer more specific. `Covers` is
// what refuses it.
const wide = z.object({
  name: z.string(),
  rc: z.string().nullable(),
  nif: z.string().nullable(),
  nis: z.string().nullable(),
  ai: z.string().nullable(),
  address: z.string().nullable(),
  phone: z.string().nullable(),
  // The field the Rust struct never had.
  fax: z.string(),
}) satisfies z.ZodType<StoreDto>;
// @ts-expect-error StoreDto is not one of `wide`'s answers: it has no fax
type _Wide = Assert<Covers<StoreDto, typeof wide>>;

// The third direction, and the one neither check above can see. This schema
// is short of `party_side`, which the Rust payload declares optional: an
// answer without an optional key is still assignable, so `satisfies` takes it
// and so does `Covers`. Only the key comparison refuses it, and it has to,
// because z.object strips a figure it has no key for and the till's refusal
// panel would then never show the field the server sent.
const missingOptional = z.object({
  code: z.string(),
  message: z.string(),
  balance_after_centimes: z.int().optional(),
  credit_limit_centimes: z.int().optional(),
  field: z.string().optional(),
  outstanding_centimes: z.int().optional(),
  missing_ids: z.array(z.string()).optional(),
  retry_after_seconds: z.int().optional(),
  permission: permissionSchema.optional(),
}) satisfies z.ZodType<ApiErrorPayloadDto>;
type _StillCovers = Assert<Covers<ApiErrorPayloadDto, typeof missingOptional>>;
// @ts-expect-error party_side is a key of the payload and not of this schema
type _MissingOptional = Assert<Matches<ApiErrorPayloadDto, typeof missingOptional>>;

// The real schemas pass all three.
type _Store = Assert<Matches<StoreDto, typeof storeSchema>>;
type _Payload = Assert<Matches<ApiErrorPayloadDto, typeof apiErrorPayloadSchema>>;

describe("the schema drift check", () => {
  test("the schema it guards accepts the block the API sends", () => {
    expect(storeSchema.safeParse(store).success).toBe(true);
  });

  test("the wide schema refuses the API's block, the short one takes it and drops the rest", () => {
    // Why neither direction can be left to a runtime check. A schema short of
    // a field still parses the block, keeping only the fields it names; a
    // schema with a field the block has not is the only one of the two that
    // refuses anything. `satisfies` and `Covers` above are what catch both.
    expect(short.safeParse(store).success).toBe(true);
    expect(wide.safeParse(store).success).toBe(false);
  });

  test("a schema short of an optional key drops the figure the server sent", () => {
    // What the compile-time check above is protecting: the payload parses,
    // and the amount the till would have shown is gone from the answer.
    const refusal = {
      code: "party_ids",
      message: "the buyer block of a facture is missing rc",
      party_side: "buyer",
    };
    const parsed = missingOptional.parse(refusal);
    expect("party_side" in parsed).toBe(false);
    expect(apiErrorPayloadSchema.parse(refusal)).toMatchObject({ party_side: "buyer" });
  });
});
