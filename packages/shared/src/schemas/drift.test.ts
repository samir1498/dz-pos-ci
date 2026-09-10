// Contract: the drift check refuses a schema that no longer answers the
// generated DTO, in both directions. The two `@ts-expect-error` lines are the
// test: `tsc --noEmit` runs as part of `pnpm test`, so if either direction
// stops firing the suppression becomes unused and the gate fails. A runtime
// assertion cannot see any of this, which is why the mechanism is pinned here
// rather than left to a reviewer.

import { describe, expect, test } from "vitest";
import { z } from "zod";

import type { StoreDto } from "../generated/StoreDto";
import type { Assert, Covers } from "./drift";
import { storeSchema } from "./settings";

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

// The real schema passes both.
type _Store = Assert<Covers<StoreDto, typeof storeSchema>>;

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
});
