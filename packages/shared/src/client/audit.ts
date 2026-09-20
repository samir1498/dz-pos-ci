// The owner's audit log (M4 T7).

import type { AuditLogDto } from "../generated/AuditLogDto";
import { narrow } from "../client-response";
import type { Transport } from "../client";
import { auditLogSchema } from "../schemas/audit";

export function auditClient({ send }: Transport) {
  return {
    /** The owner's audit log (M4 T7): one page, newest first, narrowed to a
     * user, an action or a day when the screen asks for one, and the two
     * dropdowns' own options riding along on every page. Answers 403 for
     * anyone who is not the owner; the caller decides what that looks like. */
    async listAuditLog(filters?: {
      userId?: number;
      action?: string;
      day?: string;
      page?: number;
    }): Promise<AuditLogDto> {
      const query = new URLSearchParams();
      if (filters?.userId !== undefined) query.set("user_id", String(filters.userId));
      if (filters?.action !== undefined) query.set("action", filters.action);
      if (filters?.day !== undefined) query.set("day", filters.day);
      if (filters?.page !== undefined) query.set("page", String(filters.page));
      const suffix = query.toString() === "" ? "" : `?${query.toString()}`;
      return narrow(await send(`/audit-log${suffix}`), auditLogSchema, "audit log");
    },
  };
}
