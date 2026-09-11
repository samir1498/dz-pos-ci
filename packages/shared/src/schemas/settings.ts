// The shop itself: what it is called, which régime it is on, and the copies
// of its file the server keeps.

import { z } from "zod";

import type { BackupDto } from "../generated/BackupDto";
import type { BackupsDto } from "../generated/BackupsDto";
import type { ClockDto } from "../generated/ClockDto";
import type { DatedRegimeDto } from "../generated/DatedRegimeDto";
import type { HealthDto } from "../generated/HealthDto";
import type { RegimeDto } from "../generated/RegimeDto";
import type { RestoreDto } from "../generated/RestoreDto";
import type { SettingsDto } from "../generated/SettingsDto";
import type { StoreDto } from "../generated/StoreDto";
import type { ThemeDto } from "../generated/ThemeDto";
import { day, exactInteger } from "./common";
import type { Assert, Matches } from "./drift";

export const healthSchema = z.object({
  status: z.string(),
  shop_id: z.number(),
}) satisfies z.ZodType<HealthDto>;
type _Health = Assert<Matches<HealthDto, typeof healthSchema>>;

export const clockSchema = z.object({ today: day }) satisfies z.ZodType<ClockDto>;
type _Clock = Assert<Matches<ClockDto, typeof clockSchema>>;

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

export const settingsSchema = z.object({
  store: storeSchema,
  regime: datedRegimeSchema,
  regime_planned: datedRegimeSchema.nullable(),
  /** `null` is the shop following the machine, not a missing answer. */
  theme: themeSchema.nullable(),
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
}) satisfies z.ZodType<BackupsDto>;
type _Backups = Assert<Matches<BackupsDto, typeof backupsSchema>>;

export const restoreSchema = z.object({
  restored_from: z.string(),
  safety_copy: z.string(),
  products: exactInteger,
  documents: exactInteger.nullable(),
}) satisfies z.ZodType<RestoreDto>;
type _Restore = Assert<Matches<RestoreDto, typeof restoreSchema>>;
