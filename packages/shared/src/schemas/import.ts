// What a product import file would do, and what it did. The counts are
// exact integers because the screen prints them and never adds them up
// against anything else.

import { z } from "zod";

import type { ImportAppliedDto } from "../generated/ImportAppliedDto";
import type { ImportDryRunDto } from "../generated/ImportDryRunDto";
import type { ImportOutcomeDto } from "../generated/ImportOutcomeDto";
import type { ImportRowDto } from "../generated/ImportRowDto";
import type { LabelSheetDto } from "../generated/LabelSheetDto";
import { exactInteger } from "./common";
import type { Assert, Matches } from "./drift";

export const importOutcomeSchema = z.enum([
  "created",
  "updated",
  "refused",
]) satisfies z.ZodType<ImportOutcomeDto>;
type _Outcome = Assert<Matches<ImportOutcomeDto, typeof importOutcomeSchema>>;

/** `field` and `reason` are the core's stable keys, translated by the screen,
 *  and null on a row that stands. */
export const importRowSchema = z.object({
  row: exactInteger,
  name: z.string(),
  outcome: importOutcomeSchema,
  field: z.string().nullable(),
  reason: z.string().nullable(),
}) satisfies z.ZodType<ImportRowDto>;
type _Row = Assert<Matches<ImportRowDto, typeof importRowSchema>>;

export const importDryRunSchema = z.object({
  rows: z.array(importRowSchema),
  accepted: exactInteger,
  refused: exactInteger,
}) satisfies z.ZodType<ImportDryRunDto>;
type _DryRun = Assert<Matches<ImportDryRunDto, typeof importDryRunSchema>>;

export const importAppliedSchema = z.object({
  created: exactInteger,
  updated: exactInteger,
  categories_created: exactInteger,
}) satisfies z.ZodType<ImportAppliedDto>;
type _Applied = Assert<Matches<ImportAppliedDto, typeof importAppliedSchema>>;

/** The most labels one sheet is asked for, the same number the API caps at
 * (`crates/api/src/dto.rs`): eighteen fit on an A4 page, so two hundred is
 * eleven pages. Checked here as well as there so a selection nobody could
 * print is caught before the call, not after two hundred queries. */
export const LABEL_SHEET_MAX = 200;

export const labelSheetSchema = z.object({
  ids: z.array(exactInteger).min(1).max(LABEL_SHEET_MAX),
}) satisfies z.ZodType<LabelSheetDto>;
type _LabelSheet = Assert<Matches<LabelSheetDto, typeof labelSheetSchema>>;
