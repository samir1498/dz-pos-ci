// Booking an empty slot the desk clicked (C6). Two steps in one dialog: find
// the patient the way the queue's own add dialog does (`-queue/QueueScreen.tsx`),
// then the optional visit type and a short reason. The server decides
// whether the slot still holds; this dialog only shows what it says.

import { useMutation, useQuery } from "@tanstack/react-query";
import type { PatientDto } from "@dzpos/shared";
import { Search } from "lucide-react";
import { useState } from "react";

import { FormField } from "@/components/FormField";
import { Icon } from "@/components/Icon";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { api, patientsQueryKey, visitTypesQueryKey } from "@/api";
import { useTranslation } from "@/i18n";
import { cleared, errorKey } from "@/lib/fields";

/** No visit type picked. Radix's `Select` reserves the empty string for
 *  "cleared", so the sentinel is a word rather than "". */
const NO_VISIT_TYPE = "none";

export function BookDialog({
  startsAt,
  onOpenChange,
  onBooked,
}: {
  /** `null` closes the dialog; a string is the slot clicked, already in the
   *  server's own `YYYY-MM-DD HH:MM:SS`. */
  startsAt: string | null;
  onOpenChange: (open: boolean) => void;
  onBooked: () => void;
}) {
  const { t } = useTranslation();
  const [search, setSearch] = useState("");
  const [patient, setPatient] = useState<PatientDto | null>(null);
  const [visitTypeId, setVisitTypeId] = useState(NO_VISIT_TYPE);
  const [note, setNote] = useState("");

  const open = startsAt !== null;

  const results = useQuery({
    queryKey: [...patientsQueryKey, "for-book", search.trim()],
    queryFn: () => api.listPatients(search),
    enabled: open && patient === null,
  });

  const visitTypes = useQuery({
    queryKey: visitTypesQueryKey,
    queryFn: () => api.listVisitTypes(),
    enabled: open,
  });

  const reset = () => {
    setSearch("");
    setPatient(null);
    setVisitTypeId(NO_VISIT_TYPE);
    setNote("");
  };

  const book = useMutation({
    mutationFn: () => {
      if (patient === null || startsAt === null) {
        return Promise.reject(new Error("no patient or slot chosen"));
      }
      return api.bookAppointment({
        patient_id: patient.id,
        starts_at: startsAt,
        note: cleared(note),
        visit_type_id: visitTypeId === NO_VISIT_TYPE ? undefined : visitTypeId,
      });
    },
    onSuccess: () => {
      reset();
      onBooked();
    },
  });

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) reset();
        onOpenChange(next);
      }}
    >
      <DialogContent data-testid="book-dialog">
        <DialogHeader>
          <DialogTitle>{t("book_new_title")}</DialogTitle>
          <DialogDescription>
            <span dir="ltr" className="font-numeric">
              {startsAt ?? ""}
            </span>
          </DialogDescription>
        </DialogHeader>

        {patient === null ? (
          <>
            <FormField label={t("book_search_patient")}>
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
                    data-testid="book-search"
                    placeholder={t("book_search_patient_hint")}
                    value={search}
                    onChange={(event) => setSearch(event.target.value)}
                  />
                </div>
              )}
            </FormField>
            <div className="max-h-64 overflow-y-auto">
              <div role="group" aria-label={t("book_search_patient")} className="flex flex-col gap-1">
                {(results.data ?? []).map((candidate) => (
                  <div
                    key={candidate.id}
                    className="flex items-center justify-between gap-2 rounded-md px-2 py-1.5"
                  >
                    <span className="truncate">
                      {candidate.first_name} {candidate.last_name}
                    </span>
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      data-testid={`book-pick-${candidate.id}`}
                      onClick={() => setPatient(candidate)}
                    >
                      {t("book_pick_patient")}
                    </Button>
                  </div>
                ))}
              </div>
            </div>
          </>
        ) : (
          <div className="flex flex-col gap-4">
            <div className="flex items-center justify-between gap-2">
              <span className="font-medium text-foreground">
                {patient.first_name} {patient.last_name}
              </span>
              <Button type="button" variant="ghost" size="sm" onClick={() => setPatient(null)}>
                {t("book_change_patient")}
              </Button>
            </div>

            <FormField label={t("book_visit_type")}>
              {(parts) => (
                <Select value={visitTypeId} onValueChange={setVisitTypeId}>
                  <SelectTrigger id={parts.id} className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value={NO_VISIT_TYPE}>{t("book_visit_type_none")}</SelectItem>
                    {(visitTypes.data?.visit_types ?? []).map((type) => (
                      <SelectItem key={type.id} value={type.id}>
                        {type.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}
            </FormField>

            <FormField label={t("book_note")} hint={t("book_note_hint")}>
              {(parts) => (
                <Textarea
                  {...parts}
                  data-testid="book-note"
                  value={note}
                  onChange={(event) => setNote(event.target.value)}
                />
              )}
            </FormField>

            {book.isError ? (
              <p role="alert" className="text-sm text-fg-danger">
                {t(errorKey(book.error))}
              </p>
            ) : null}

            <DialogFooter>
              <Button
                type="button"
                data-testid="book-submit"
                disabled={book.isPending}
                onClick={() => book.mutate()}
              >
                {t("book_submit")}
              </Button>
            </DialogFooter>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
