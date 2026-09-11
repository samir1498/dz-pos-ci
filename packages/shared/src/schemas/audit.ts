// The owner's audit log (M4 T7, features.md §5). `before` and `after` stay
// strings here, the same JSON text `services::audit::record` was handed:
// this layer checks the envelope's shape, not what a service chose to put
// inside its own change, and a screen that wants those two fields parses
// them itself (see `apps/desktop/src/routes/audit.tsx`).

import { z } from "zod";

import type { AuditEntryDto } from "../generated/AuditEntryDto";
import type { AuditLogDto } from "../generated/AuditLogDto";
import type { AuditUserDto } from "../generated/AuditUserDto";
import { exactInteger } from "./common";
import type { Assert, Matches } from "./drift";

export const auditUserSchema = z.object({
  id: z.number(),
  name: z.string(),
}) satisfies z.ZodType<AuditUserDto>;
type _AuditUser = Assert<Matches<AuditUserDto, typeof auditUserSchema>>;

export const auditEntrySchema = z.object({
  id: z.number(),
  user_id: z.number(),
  user_name: z.string(),
  action: z.string(),
  entity: z.string(),
  entity_id: z.number().nullable(),
  before: z.string().nullable(),
  after: z.string().nullable(),
  created_at: z.string(),
}) satisfies z.ZodType<AuditEntryDto>;
type _AuditEntry = Assert<Matches<AuditEntryDto, typeof auditEntrySchema>>;

export const auditLogSchema = z.object({
  rows: z.array(auditEntrySchema),
  page: exactInteger,
  has_more: z.boolean(),
  users: z.array(auditUserSchema),
  actions: z.array(z.string()),
}) satisfies z.ZodType<AuditLogDto>;
type _AuditLog = Assert<Matches<AuditLogDto, typeof auditLogSchema>>;
