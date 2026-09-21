// The shift list a manager runs the floor off (plan
// till-shifts-a-float-and-a-count T7): every shift opened over a day window,
// newest first, with the four close columns and the difference beside them.
//
// Row only, never the report: `GET /till/shifts` answers `ShiftDto`, not
// `ShiftReportDto` — a report per row is a query per row, and this screen's
// own job is the list, not the drawer's live takings.
//
// `till_.shifts.tsx`, not `till.shifts.tsx`: the router's flat file routing
// nests a dotted name under its first segment (`settings.users.tsx` ->
// `/settings/users` under `/settings/index.tsx`'s own outlet, which is what
// `settings.tsx` provides), and `till.tsx` renders no `<Outlet />` and is
// pinned at 904 lines besides. The trailing `_` before the dot is the
// router's own escape out of that nesting, the same one `customers_.$id.tsx`
// and `purchases_.$id.tsx` already use, so this file's route id is
// `/till_/shifts` while the URL it answers is the plain `/till/shifts`
// below. Gated on `see_reports` the same way `/audit` is gated on
// `see_audit_log`: no `beforeLoad` guard, so a manager who types the address
// by hand still mounts the screen and the 403 the server answers is what
// actually turns them away, in the wording below rather than a blank table.
//
// "Who" reads a name off `GET /auth/staff` (`staffQueryKey`, the sign-in
// picker's own list, ungated because it answers before a session exists) and
// falls back to the raw `opened_by` id for a fiche the list left out —
// deactivated between the shift and this read, the one case the picker's own
// "active fiches only" filter (`StaffDto`'s own doc) leaves this screen
// nothing better to show. `ShiftDto` itself carries no name (unlike the audit
// log's own joined `user_name`), and adding one would move inside
// `ShiftReportDto` too, which the plan keeps off this task's file list.

import { createFileRoute } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { Banknote } from "lucide-react";
import { useState } from "react";

import { DataTable, type Column } from "@/components/DataTable";
import { EmptyState } from "@/components/EmptyState";
import { FormField } from "@/components/FormField";
import { Money } from "@/components/Money";
import { PageHeader } from "@/components/PageHeader";
import { DateField } from "@/components/ui/date-field";
import { Skeleton } from "@/components/ui/skeleton";
import { api, staffQueryKey, tillShiftsQueryKey } from "@/api";
import { useTranslation } from "@/i18n";
import { errorKey } from "@/lib/fields";
import type { ShiftDto } from "@dzpos/shared";

export const Route = createFileRoute("/till_/shifts")({ component: TillShiftsScreen });

export { TillShiftsScreen };

function TillShiftsScreen() {
  const { t } = useTranslation();
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");

  const filters = { from: from === "" ? undefined : from, to: to === "" ? undefined : to };
  const shifts = useQuery({
    queryKey: [...tillShiftsQueryKey, filters.from, filters.to],
    queryFn: () => api.listShifts(filters),
  });
  const staff = useQuery({ queryKey: staffQueryKey, queryFn: () => api.listStaff() });
  const names = new Map((staff.data ?? []).map((s) => [s.id, s.name]));

  return (
    <section className="flex flex-col gap-4">
      <PageHeader title={t("till_shifts_title")} />

      <div className="flex flex-wrap items-end gap-4">
        <FormField label={t("till_shifts_filter_from")} className="min-w-48">
          {(parts) => (
            <DateField
              {...parts}
              data-testid="till-shifts-filter-from"
              value={from}
              onChange={setFrom}
            />
          )}
        </FormField>
        <FormField label={t("till_shifts_filter_to")} className="min-w-48">
          {(parts) => (
            <DateField {...parts} data-testid="till-shifts-filter-to" value={to} onChange={setTo} />
          )}
        </FormField>
      </div>

      {shifts.isPending ? (
        <div className="flex flex-col gap-2">
          <span className="sr-only">{t("till_shifts_loading")}</span>
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
        </div>
      ) : null}
      {shifts.isError ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(shifts.error))}
        </p>
      ) : null}
      {shifts.isSuccess ? <ShiftsTable rows={shifts.data} names={names} /> : null}
    </section>
  );
}

/** `null` reads as "the shift is still open": the four close columns of
 *  `ShiftDto` travel together or not at all (the DTO's own doc), so a row
 *  carrying one of them always carries the rest. */
function ShiftsTable({
  rows,
  names,
}: {
  rows: readonly ShiftDto[];
  names: ReadonlyMap<number, string>;
}) {
  const { t } = useTranslation();
  const open = t("till_shifts_still_open");

  const columns: readonly Column<ShiftDto>[] = [
    {
      id: "who",
      header: t("col_user"),
      cell: (r) => names.get(r.opened_by) ?? `#${r.opened_by}`,
    },
    {
      id: "opened_at",
      header: t("col_shift_opened_at"),
      cell: (r) => (
        <span dir="ltr" className="font-numeric tabular-nums">
          {r.opened_at}
        </span>
      ),
    },
    {
      id: "closed_at",
      header: t("col_shift_closed_at"),
      cell: (r) =>
        r.closed_at === null ? (
          <span className="text-muted-foreground">{open}</span>
        ) : (
          <span dir="ltr" className="font-numeric tabular-nums">
            {r.closed_at}
          </span>
        ),
    },
    {
      id: "opening_cash",
      header: t("till_shift_bar_opening_cash"),
      money: true,
      cell: (r) => <Money centimes={r.opening_cash_centimes} />,
    },
    {
      id: "counted",
      header: t("col_shift_counted"),
      money: true,
      cell: (r) =>
        r.counted_centimes === null ? (
          <span className="text-muted-foreground">{open}</span>
        ) : (
          <Money centimes={r.counted_centimes} />
        ),
    },
    {
      id: "difference",
      header: t("till_shift_difference"),
      money: true,
      cell: (r) =>
        r.difference_centimes === null ? (
          <span className="text-muted-foreground">{open}</span>
        ) : (
          <Money centimes={r.difference_centimes} data-testid={`till-shifts-difference-${r.id}`} />
        ),
    },
    {
      id: "note",
      header: t("field_shift_note"),
      cell: (r) => r.note ?? <span className="text-muted-foreground">—</span>,
    },
  ];

  return (
    <DataTable
      columns={columns}
      rows={rows}
      rowKey={(r) => r.id}
      rowTestId={(r) => `till-shifts-row-${r.id}`}
      caption={t("till_shifts_title")}
      empty={
        <EmptyState
          icon={Banknote}
          title={t("till_shifts_empty")}
          description={t("till_shifts_empty_hint")}
        />
      }
    />
  );
}
