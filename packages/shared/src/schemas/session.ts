// Who is signed in (M4 T2). The role and the permission list a screen reads
// once, never a comparison a screen makes: the permission table lives in the
// Rust core and this is the answer it gave, not a second copy of it.

import { z } from "zod";

import type { MeDto } from "../generated/MeDto";
import type { PermissionDto } from "../generated/PermissionDto";
import type { RoleDto } from "../generated/RoleDto";
import type { SessionDto } from "../generated/SessionDto";
import type { SessionIdleDto } from "../generated/SessionIdleDto";
import { exactInteger } from "./common";
import type { Assert, Matches } from "./drift";

export const roleSchema = z.enum(["owner", "manager", "cashier"]) satisfies z.ZodType<RoleDto>;
type _Role = Assert<Matches<RoleDto, typeof roleSchema>>;

/// Every permission the core's table names, in the order it names them.
export const permissionSchema = z.enum([
  "sell",
  "discount_above_threshold",
  "override_credit_block",
  "see_cost_and_margin",
  "edit_fiches",
  "edit_settings",
  "see_reports",
  "manage_users",
  "commit_money",
  "correct_ledger",
  "export_and_import",
  "change_price_at_the_till",
  "see_audit_log",
  "open_and_close_till",
  "close_another_persons_till",
]) satisfies z.ZodType<PermissionDto>;
type _Permission = Assert<Matches<PermissionDto, typeof permissionSchema>>;

export const meSchema = z.object({
  user_id: exactInteger,
  name: z.string(),
  role: roleSchema,
  permissions: z.array(permissionSchema),
}) satisfies z.ZodType<MeDto>;
type _Me = Assert<Matches<MeDto, typeof meSchema>>;

export const sessionSchema = z.object({
  me: meSchema,
  token: z.string(),
  idle_minutes: exactInteger,
}) satisfies z.ZodType<SessionDto>;
type _Session = Assert<Matches<SessionDto, typeof sessionSchema>>;

export const sessionIdleSchema = z.object({
  idle_minutes: exactInteger,
}) satisfies z.ZodType<SessionIdleDto>;
type _SessionIdle = Assert<Matches<SessionIdleDto, typeof sessionIdleSchema>>;
