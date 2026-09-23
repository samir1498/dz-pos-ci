// The clinic's waiting queue (C6 of
// `the-first-clinic-module-patients-queue-appointments`). Today's arrivals,
// in the order the desk wrote them down, almost no rules on top: add a
// patient who walked in, call the next one, mark them seen or gone.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { PatientDto, QueueEntryDto } from "@dzpos/shared";
import { PhoneCall, Search, UserPlus, Users } from "lucide-react";
import { useState } from "react";

import { DataTable, type Column } from "@/components/DataTable";
import { EmptyState } from "@/components/EmptyState";
import { FormField } from "@/components/FormField";
import { Icon } from "@/components/Icon";
import { PageHeader } from "@/components/PageHeader";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { api, patientsQueryKey, queueQueryKey } from "@/api";
import { useTranslation, type Key } from "@/i18n";
import { errorKey } from "@/lib/fields";

type QueueStatus = "waiting" | "called" | "seen" | "left";

const STATUS_KEY: Readonly<Record<QueueStatus, Key>> = {
  waiting: "queue_status_waiting",
  called: "queue_status_called",
  seen: "queue_status_seen",
  left: "queue_status_left",
};

/** Derived from the three stamps rather than stored on its own: the server
 *  already refuses the combinations that cannot happen (seen before called,
 *  called twice), so the screen only ever has to read which of the three
 *  came last. */
export function queueStatus(entry: QueueEntryDto): QueueStatus {
  if (entry.left_at !== null) return "left";
  if (entry.seen_at !== null) return "seen";
  if (entry.called_at !== null) return "called";
  return "waiting";
}

export function QueueScreen() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [addOpen, setAddOpen] = useState(false);

  const queue = useQuery({ queryKey: queueQueryKey, queryFn: () => api.listQueue() });

  const invalidateQueue = () => queryClient.invalidateQueries({ queryKey: queueQueryKey });

  const callNext = useMutation({ mutationFn: () => api.callNextInQueue(), onSuccess: invalidateQueue });
  const call = useMutation({ mutationFn: (id: string) => api.callInQueue(id), onSuccess: invalidateQueue });
  const seen = useMutation({ mutationFn: (id: string) => api.markSeenInQueue(id), onSuccess: invalidateQueue });
  const left = useMutation({ mutationFn: (id: string) => api.markLeftInQueue(id), onSuccess: invalidateQueue });

  const waiting = queue.data?.some((entry) => queueStatus(entry) === "waiting") ?? false;

  const actions = (
    <div className="flex items-center gap-2">
      <Button variant="outline" onClick={() => setAddOpen(true)}>
        <Icon as={UserPlus} size={18} />
        {t("queue_add")}
      </Button>
      <Button disabled={!waiting || callNext.isPending} onClick={() => callNext.mutate()}>
        <Icon as={PhoneCall} size={18} />
        {t("queue_call_next")}
      </Button>
    </div>
  );

  const columns: readonly Column<QueueEntryDto>[] = [
    {
      id: "name",
      header: t("field_name"),
      cell: (entry) => (
        <span className="font-medium text-foreground">
          {entry.first_name} {entry.last_name}
        </span>
      ),
    },
    {
      id: "arrived_at",
      header: t("col_arrived_at"),
      cell: (entry) => (
        <span dir="ltr" className="font-numeric text-muted-foreground">
          {entry.arrived_at}
        </span>
      ),
    },
    {
      id: "status",
      header: t("col_status"),
      cell: (entry) => {
        const status = queueStatus(entry);
        return (
          <Badge variant={status === "waiting" ? "outline" : "secondary"}>
            {t(STATUS_KEY[status])}
          </Badge>
        );
      },
    },
  ];

  return (
    <section className="flex flex-col gap-4">
      <PageHeader title={t("queue_title")} description={t("queue_subtitle")} actions={actions} />

      {queue.isPending ? (
        <div className="flex flex-col gap-2">
          <span className="sr-only">{t("queue_loading")}</span>
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
        </div>
      ) : null}
      {queue.isError ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(queue.error))}
        </p>
      ) : null}
      {queue.isSuccess ? (
        <DataTable
          columns={columns}
          rows={queue.data}
          rowKey={(entry) => entry.id}
          caption={t("queue_title")}
          empty={
            <EmptyState icon={Users} title={t("queue_empty")} description={t("queue_empty_hint")} />
          }
          actions={(entry) => {
            const status = queueStatus(entry);
            if (status === "waiting") {
              return (
                <Button
                  variant="ghost"
                  size="sm"
                  data-testid={`queue-call-${entry.id}`}
                  disabled={call.isPending}
                  onClick={() => call.mutate(entry.id)}
                >
                  {t("queue_call")}
                </Button>
              );
            }
            if (status === "called") {
              return (
                <div className="flex items-center justify-end gap-1">
                  <Button
                    variant="ghost"
                    size="sm"
                    data-testid={`queue-seen-${entry.id}`}
                    disabled={seen.isPending}
                    onClick={() => seen.mutate(entry.id)}
                  >
                    {t("queue_mark_seen")}
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    data-testid={`queue-left-${entry.id}`}
                    disabled={left.isPending}
                    onClick={() => left.mutate(entry.id)}
                  >
                    {t("queue_mark_left")}
                  </Button>
                </div>
              );
            }
            return null;
          }}
        />
      ) : null}

      <AddToQueueDialog
        open={addOpen}
        onOpenChange={setAddOpen}
        onAdded={() => {
          setAddOpen(false);
          void invalidateQueue();
        }}
      />
    </section>
  );
}

/** Search first, then add: the desk already knows the name or the phone the
 *  patient just gave at the door, the way the till's own customer search
 *  works (`-till/customer.tsx`). */
function AddToQueueDialog({
  open,
  onOpenChange,
  onAdded,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onAdded: () => void;
}) {
  const { t } = useTranslation();
  const [search, setSearch] = useState("");

  const results = useQuery({
    queryKey: [...patientsQueryKey, "for-queue", search.trim()],
    queryFn: () => api.listPatients(search),
    enabled: open,
  });

  const add = useMutation({
    mutationFn: (patientId: string) => api.addToQueue(patientId),
    onSuccess: onAdded,
  });

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) setSearch("");
        onOpenChange(next);
      }}
    >
      <DialogContent data-testid="queue-add-dialog">
        <DialogHeader>
          <DialogTitle>{t("queue_add")}</DialogTitle>
          <DialogDescription>{t("queue_add_search_hint")}</DialogDescription>
        </DialogHeader>

        <FormField label={t("queue_add_search")}>
          {(parts) => (
            <div className="relative">
              <Icon
                as={Search}
                size={18}
                className="pointer-events-none absolute inset-y-0 start-3 my-auto text-faint"
              />
              <Input
                {...parts}
                type="search"
                className="ps-9"
                data-testid="queue-add-search"
                placeholder={t("queue_add_search_hint")}
                value={search}
                onChange={(event) => setSearch(event.target.value)}
              />
            </div>
          )}
        </FormField>

        <div className="max-h-64 overflow-y-auto">
          <div role="group" aria-label={t("queue_add_search")} className="flex flex-col gap-1">
            {(results.data ?? []).map((patient: PatientDto) => (
              <div
                key={patient.id}
                className="flex items-center justify-between gap-2 rounded-md px-2 py-1.5"
              >
                <span className="truncate">
                  {patient.first_name} {patient.last_name}
                </span>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  data-testid={`queue-add-pick-${patient.id}`}
                  disabled={add.isPending}
                  onClick={() => add.mutate(patient.id)}
                >
                  {t("queue_add_button")}
                </Button>
              </div>
            ))}
          </div>
        </div>

        {add.isError ? (
          <p role="alert" className="text-sm text-fg-danger">
            {t(errorKey(add.error))}
          </p>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}
