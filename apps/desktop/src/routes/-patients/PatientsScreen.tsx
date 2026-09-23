// The clinic's patient file (C6 of
// `the-first-clinic-module-patients-queue-appointments`). The list is the
// whole screen: search by name or phone, open a row to edit its fiche, and
// a blank one behind the header's button. Archiving takes a patient out of
// the search without deleting the file, the way a customer's fiche is
// closed rather than removed (features.md §2's reasoning applies here too:
// the queue and the book both point at a patient by id).

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { PatientDto } from "@dzpos/shared";
import { Inbox, SquarePen, Users } from "lucide-react";
import { useState } from "react";

import { ConfirmDialog } from "@/components/ConfirmDialog";
import { DataTable, type Column } from "@/components/DataTable";
import { EmptyState } from "@/components/EmptyState";
import { FormField } from "@/components/FormField";
import { Icon } from "@/components/Icon";
import { PageHeader } from "@/components/PageHeader";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { api, patientsQueryKey } from "@/api";
import { useTranslation } from "@/i18n";
import { errorKey } from "@/lib/fields";

import { PatientFicheSheet } from "./fiche";
import { SEX_KEY } from "./parts";

export function PatientsScreen() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [search, setSearch] = useState("");
  const [showArchived, setShowArchived] = useState(false);
  // One panel, two jobs, the way the customers screen's is: "new" is the
  // blank fiche, a patient is that patient's own.
  const [open, setOpen] = useState<"new" | PatientDto | null>(null);
  const [toArchive, setToArchive] = useState<PatientDto | null>(null);

  const patients = useQuery({
    queryKey: [...patientsQueryKey, search.trim(), showArchived],
    queryFn: () => api.listPatients(search, showArchived),
  });

  const opened =
    open === null || open === "new"
      ? open
      : (patients.data?.find((p) => p.id === open.id) ?? open);

  const archive = useMutation({
    mutationFn: (id: string) => api.archivePatient(id),
    onSuccess: async () => {
      setToArchive(null);
      await queryClient.invalidateQueries({ queryKey: patientsQueryKey });
    },
  });

  const add = (
    <Button onClick={() => setOpen("new")}>
      <Icon as={Users} size={18} />
      {t("patients_add")}
    </Button>
  );

  const columns: readonly Column<PatientDto>[] = [
    {
      id: "name",
      header: t("field_name"),
      cell: (patient) => (
        <div className="flex flex-wrap items-center gap-2">
          <span className="font-medium text-foreground">
            {patient.first_name} {patient.last_name}
          </span>
          {patient.archived_at === null ? null : (
            <Badge variant="secondary">{t("patients_archived_badge")}</Badge>
          )}
        </div>
      ),
    },
    {
      id: "sex",
      header: t("field_sex"),
      cell: (patient) => (patient.sex === null ? null : t(SEX_KEY[patient.sex])),
    },
    {
      id: "phone",
      header: t("col_phone"),
      cell: (patient) =>
        patient.phone === null ? null : (
          <span dir="ltr" className="font-numeric text-muted-foreground">
            {patient.phone}
          </span>
        ),
    },
    {
      id: "date_of_birth",
      header: t("field_date_of_birth"),
      cell: (patient) =>
        patient.date_of_birth === null ? null : (
          <span dir="ltr" className="font-numeric text-muted-foreground">
            {patient.date_of_birth}
          </span>
        ),
    },
  ];

  return (
    <section className="flex flex-col gap-4">
      <PageHeader
        title={t("patients_title")}
        description={t("patients_subtitle")}
        actions={add}
      />

      <div className="flex flex-wrap items-end gap-4">
        <FormField label={t("patients_search")} className="max-w-sm">
          {(parts) => (
            <Input
              {...parts}
              type="search"
              placeholder={t("patients_search_hint")}
              value={search}
              onChange={(event) => setSearch(event.target.value)}
            />
          )}
        </FormField>

        <div className="flex items-center gap-2 pb-2">
          <Checkbox
            id="patients-show-archived"
            checked={showArchived}
            onCheckedChange={(next) => setShowArchived(next === true)}
          />
          <Label htmlFor="patients-show-archived">{t("patients_show_archived")}</Label>
        </div>
      </div>

      {patients.isPending ? (
        <div className="flex flex-col gap-2">
          <span className="sr-only">{t("patients_loading")}</span>
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
        </div>
      ) : null}
      {patients.isError ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(patients.error))}
        </p>
      ) : null}
      {patients.isSuccess ? (
        <DataTable
          columns={columns}
          rows={patients.data}
          rowKey={(patient) => patient.id}
          caption={t("patients_title")}
          empty={
            <EmptyState
              icon={Inbox}
              title={t("patients_empty")}
              description={t("patients_empty_hint")}
            />
          }
          actions={(patient) => (
            <div className="flex items-center justify-end gap-1">
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label={`${t("patients_edit")} ${patient.first_name} ${patient.last_name}`}
                onClick={() => setOpen(patient)}
              >
                <Icon as={SquarePen} size={18} />
              </Button>
              {patient.archived_at === null ? (
                <Button
                  variant="ghost"
                  size="sm"
                  data-testid={`patient-archive-${patient.id}`}
                  onClick={() => setToArchive(patient)}
                >
                  {t("patients_archive")}
                </Button>
              ) : null}
            </div>
          )}
        />
      ) : null}

      <PatientFicheSheet
        open={opened !== null}
        initial={opened === "new" || opened === null ? null : opened}
        onOpenChange={(next) => {
          if (!next) setOpen(null);
        }}
      />

      <ConfirmDialog
        open={toArchive !== null}
        onCancel={() => setToArchive(null)}
        onConfirm={() => {
          if (toArchive !== null) archive.mutate(toArchive.id);
        }}
        title="patients_archive_confirm_title"
        question="patients_archive_confirm_question"
        confirm="patients_archive"
        pending={archive.isPending}
        destructive
        data-testid="patient-archive-dialog"
      >
        {toArchive === null ? null : (
          <p className="text-sm text-foreground">
            {toArchive.first_name} {toArchive.last_name}
          </p>
        )}
      </ConfirmDialog>
    </section>
  );
}
