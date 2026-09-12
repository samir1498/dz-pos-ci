// The owner's audit log (M4 T7, features.md §5): every sensitive action the
// shop's other screens have already written a row for — a price change, a
// credit override, a settings or régime change, a recount drift, a supplier
// or purchase correction — filterable by day, by user and by kind, with the
// before and after of each change. Nothing here edits or deletes a row:
// `services::audit::record` is the only way one is ever written, from
// inside the transaction of the change it records.
//
// `action` and `entity` are shown exactly as the writing service spelled
// them ("product.update", "debt.pay"): they are data the log holds, the way
// a supplier's own document number is, not UI chrome, so this screen does
// not invent a translated taxonomy on top of what a handful of services
// actually wrote.

import { createFileRoute } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { ChevronLeft, ChevronRight, History } from "lucide-react";
import { useState } from "react";
import { formatCentimes } from "@dzpos/shared";

import { DataTable, type Column } from "@/components/DataTable";
import { EmptyState } from "@/components/EmptyState";
import { FormField } from "@/components/FormField";
import { Icon } from "@/components/Icon";
import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { api, auditLogQueryKey } from "@/api";
import { useTranslation } from "@/i18n";
import { errorKey } from "@/lib/fields";
import type { AuditEntryDto } from "@dzpos/shared";

// No `beforeLoad` guard on this route: a manager who types `/audit` by hand
// still mounts the screen, and only learns they may not see it once
// `GET /audit-log` answers 403 and the error alert below shows in its
// place. Deliberate — the nav entry hiding the link is the sole point where
// this screen keeps a manager out, and the API refusing the request is the
// actual control (`crates/api/src/gates.rs`, `Permission::SeeAuditLog`).
export const Route = createFileRoute("/audit")({ component: AuditScreen });

export { AuditScreen };

/** The word a filter uses for "no filter at all" (the purchases screen's own
 *  reason: Radix refuses an item whose value is the empty string). */
const ANY = "any";

function AuditScreen() {
  const { t } = useTranslation();
  const [userId, setUserId] = useState("");
  const [action, setAction] = useState("");
  const [day, setDay] = useState("");
  const [page, setPage] = useState(1);

  const filters = {
    userId: userId === "" ? undefined : Number(userId),
    action: action === "" ? undefined : action,
    day: day === "" ? undefined : day,
    page,
  };
  const auditLog = useQuery({
    queryKey: [...auditLogQueryKey, filters.userId, filters.action, filters.day, filters.page],
    queryFn: () => api.listAuditLog(filters),
  });

  // A filter changed under the reader's feet: the page they were on may not
  // exist for the new, narrower set, and asking for page 3 of an empty
  // filter is a blank screen with no way back. Only the three filters reset
  // it; the pager buttons set `page` themselves.
  function setFilter(next: () => void) {
    next();
    setPage(1);
  }

  return (
    <section className="flex flex-col gap-4">
      <PageHeader title={t("audit_title")} />

      <div className="flex flex-wrap items-end gap-4">
        <FormField label={t("audit_filter_user")} className="min-w-48">
          {(parts) => (
            <Select
              value={userId === "" ? ANY : userId}
              onValueChange={(next) => setFilter(() => setUserId(next === ANY ? "" : next))}
            >
              <SelectTrigger id={parts.id} className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={ANY}>{t("audit_filter_any")}</SelectItem>
                {(auditLog.data?.users ?? []).map((u) => (
                  <SelectItem key={u.id} value={String(u.id)}>
                    {u.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          )}
        </FormField>
        <FormField label={t("audit_filter_kind")} className="min-w-48">
          {(parts) => (
            <Select
              value={action === "" ? ANY : action}
              onValueChange={(next) => setFilter(() => setAction(next === ANY ? "" : next))}
            >
              <SelectTrigger id={parts.id} className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={ANY}>{t("audit_filter_any")}</SelectItem>
                {(auditLog.data?.actions ?? []).map((a) => (
                  <SelectItem key={a} value={a}>
                    {a}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          )}
        </FormField>
        <FormField label={t("audit_filter_day")} className="min-w-48">
          {(parts) => (
            <Input
              {...parts}
              type="date"
              dir="ltr"
              data-testid="audit-filter-day"
              className="font-numeric tabular-nums"
              value={day}
              onChange={(event) => setFilter(() => setDay(event.target.value))}
            />
          )}
        </FormField>
      </div>

      {auditLog.isPending ? (
        <div className="flex flex-col gap-2">
          <span className="sr-only">{t("audit_loading")}</span>
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
        </div>
      ) : null}
      {auditLog.isError ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(auditLog.error))}
        </p>
      ) : null}
      {auditLog.isSuccess ? (
        <>
          <AuditTable rows={auditLog.data.rows} />
          <div className="flex items-center justify-end gap-2">
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={page <= 1}
              onClick={() => setPage((p) => Math.max(1, p - 1))}
            >
              <Icon as={ChevronLeft} size={18} flip />
              {t("audit_page_prev")}
            </Button>
            <Button
              type="button"
              variant="outline"
              size="sm"
              disabled={!auditLog.data.has_more}
              onClick={() => setPage((p) => p + 1)}
            >
              {t("audit_page_next")}
              <Icon as={ChevronRight} size={18} flip />
            </Button>
          </div>
        </>
      ) : null}
    </section>
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** One JSON document, or `null` when the field carried none or was not an
 *  object: the log never guessed a shape (`services::audit::record`'s own
 *  doc comment), so this screen refuses to guess one either. */
function parsed(text: string | null): Record<string, unknown> | null {
  if (text === null) return null;
  try {
    const value: unknown = JSON.parse(text);
    return isRecord(value) ? value : null;
  } catch {
    return null;
  }
}

function shown(key: string, value: unknown): string {
  if (value === undefined || value === null) return "—";
  if (typeof value === "number" && key.endsWith("_centimes")) return formatCentimes(value);
  if (typeof value === "boolean") return value ? "true" : "false";
  return String(value);
}

/** Only the keys that actually moved: `before` and `after` often carry a
 *  whole fiche, and a row's own reader wants what changed, not the fields
 *  that did not. */
function changedKeys(before: Record<string, unknown> | null, after: Record<string, unknown> | null) {
  const keys = [...new Set([...Object.keys(before ?? {}), ...Object.keys(after ?? {})])].sort();
  return keys.filter((key) => JSON.stringify(before?.[key]) !== JSON.stringify(after?.[key]));
}

function ChangeCell({ row }: { row: AuditEntryDto }) {
  const before = parsed(row.before);
  const after = parsed(row.after);
  const keys = changedKeys(before, after);
  if (keys.length === 0) return <span className="text-muted-foreground">—</span>;
  return (
    <dl dir="ltr" className="flex flex-col gap-0.5 text-xs">
      {keys.map((key) => (
        <div key={key} className="flex items-baseline gap-1">
          <dt className="text-muted-foreground">{key}</dt>
          <dd className="font-numeric tabular-nums">
            {shown(key, before?.[key])} → {shown(key, after?.[key])}
          </dd>
        </div>
      ))}
    </dl>
  );
}

function AuditTable({ rows }: { rows: readonly AuditEntryDto[] }) {
  const { t } = useTranslation();

  const columns: readonly Column<AuditEntryDto>[] = [
    {
      id: "date",
      header: t("col_date"),
      cell: (r) => (
        <span dir="ltr" className="font-numeric tabular-nums">
          {r.created_at}
        </span>
      ),
    },
    { id: "user", header: t("col_user"), cell: (r) => r.user_name },
    {
      id: "kind",
      header: t("col_kind"),
      cell: (r) => (
        <span dir="ltr" className="font-numeric text-xs">
          {r.action}
        </span>
      ),
    },
    {
      id: "entity",
      header: t("col_entity"),
      cell: (r) => (
        <span dir="ltr" className="font-numeric text-xs">
          {r.entity}
          {r.entity_id === null ? "" : ` #${r.entity_id}`}
        </span>
      ),
    },
    {
      id: "change",
      header: t("col_change"),
      cell: (r) => <ChangeCell row={r} />,
    },
  ];

  return (
    <DataTable
      columns={columns}
      rows={rows}
      rowKey={(r) => r.id}
      rowTestId={(r) => `audit-row-${r.id}`}
      caption={t("audit_title")}
      empty={<EmptyState icon={History} title={t("audit_empty")} description={t("audit_empty_hint")} />}
    />
  );
}
