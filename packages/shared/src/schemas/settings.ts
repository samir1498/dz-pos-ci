// The shop itself: what it is called, which régime it is on, and the copies
// of its file the server keeps.

import { z } from "zod";

import type { BackupDto } from "../generated/BackupDto";
import type { BackupsDto } from "../generated/BackupsDto";
import type { BuildInfoDto } from "../generated/BuildInfoDto";
import type { ClockDto } from "../generated/ClockDto";
import type { DatedRegimeDto } from "../generated/DatedRegimeDto";
import type { HealthDto } from "../generated/HealthDto";
import type { RegimeDto } from "../generated/RegimeDto";
import type { RestoreDto } from "../generated/RestoreDto";
import type { SettingsDto } from "../generated/SettingsDto";
import type { StoreDto } from "../generated/StoreDto";
import type { FactureLayoutDto } from "../generated/FactureLayoutDto";
import type { ThemeDto } from "../generated/ThemeDto";
import { day, exactInteger } from "./common";
import type { Assert, Matches } from "./drift";

export const healthSchema = z.object({
  status: z.string(),
  shop_id: z.number(),
  needs_first_setup: z.boolean(),
}) satisfies z.ZodType<HealthDto>;
type _Health = Assert<Matches<HealthDto, typeof healthSchema>>;

export const clockSchema = z.object({ today: day }) satisfies z.ZodType<ClockDto>;
type _Clock = Assert<Matches<ClockDto, typeof clockSchema>>;

/** The About screen's whole source: the version, the git short hash and the
 *  build date `crates/core::build_info` baked in, plus whether this is a
 *  debug build. No field here is ever read from a second, hand-typed
 *  constant (`apps/desktop/src/routes/settings_.about.test.tsx` checks it). */
export const buildInfoSchema = z.object({
  version: z.string(),
  git_hash: z.string(),
  build_date: z.string(),
  debug: z.boolean(),
}) satisfies z.ZodType<BuildInfoDto>;
type _BuildInfo = Assert<Matches<BuildInfoDto, typeof buildInfoSchema>>;

export const regimeSchema = z.enum(["ifu", "reel"]) satisfies z.ZodType<RegimeDto>;
type _Regime = Assert<Matches<RegimeDto, typeof regimeSchema>>;

export const datedRegimeSchema = z.object({
  regime: regimeSchema,
  valid_from: day,
}) satisfies z.ZodType<DatedRegimeDto>;
type _DatedRegime = Assert<Matches<DatedRegimeDto, typeof datedRegimeSchema>>;

export const storeSchema = z.object({
  name: z.string(),
  rc: z.string().nullable(),
  nif: z.string().nullable(),
  nis: z.string().nullable(),
  ai: z.string().nullable(),
  address: z.string().nullable(),
  phone: z.string().nullable(),
}) satisfies z.ZodType<StoreDto>;
type _Store = Assert<Matches<StoreDto, typeof storeSchema>>;

/** The four `[data-theme]` blocks packages/design emits, by their own names. */
export const themeSchema = z.enum([
  "comptoir",
  "registre",
  "observe",
  "observe-dark",
]) satisfies z.ZodType<ThemeDto>;
type _Theme = Assert<Matches<ThemeDto, typeof themeSchema>>;

/** Which layout a facture is drawn in. Not the paper: the sheet is named on
 *  each print, because the till knows which tray the cashier reached for. */
export const factureLayoutSchema = z.enum(["standard", "compact"]) satisfies z.ZodType<FactureLayoutDto>;
type _FactureLayout = Assert<Matches<FactureLayoutDto, typeof factureLayoutSchema>>;

export const settingsSchema = z.object({
  store: storeSchema,
  regime: datedRegimeSchema,
  regime_planned: datedRegimeSchema.nullable(),
  /** `null` is the shop following the machine, not a missing answer. */
  theme: themeSchema.nullable(),
  /** Never null: a shop that has never chosen prints on `standard`. */
  facture_layout: factureLayoutSchema,
  /** Every layout the shop may pick, from the server, so the screen carries
   *  no copy of the list to go stale when one is added. */
  facture_layouts: z.array(factureLayoutSchema),
  /** Basis points of the basket, so a whole number: 250 is 2,5 %. Zero on
   *  a shop that has never set one, which refuses a cashier every
   *  discount. */
  discount_threshold_bps: exactInteger,
}) satisfies z.ZodType<SettingsDto>;
type _Settings = Assert<Matches<SettingsDto, typeof settingsSchema>>;

/** `bytes` is a file size, so an integer: a fractional byte count means the
 *  server is not the one this client was generated against. */
export const backupSchema = z.object({
  name: z.string(),
  taken_at: z.string(),
  bytes: exactInteger,
}) satisfies z.ZodType<BackupDto>;
type _Backup = Assert<Matches<BackupDto, typeof backupSchema>>;

export const backupsSchema = z.object({
  backups: z.array(backupSchema),
  safety_copies: z.array(backupSchema),
  upgrade_copies: z.array(backupSchema),
}) satisfies z.ZodType<BackupsDto>;
type _Backups = Assert<Matches<BackupsDto, typeof backupsSchema>>;

export const restoreSchema = z.object({
  restored_from: z.string(),
  safety_copy: z.string(),
  products: exactInteger,
  documents: exactInteger.nullable(),
}) satisfies z.ZodType<RestoreDto>;
type _Restore = Assert<Matches<RestoreDto, typeof restoreSchema>>;
