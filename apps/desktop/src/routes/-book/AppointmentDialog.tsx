// An appointment the desk clicked (C6). Every action the plan asks for
// (arrived, move, cancel, mark no-show, clear it, the confirmation call,
// open the patient file) sits here, always offered: Samir's ruling of 2026-09-23 19:34 is that the
// desk decides which one applies and the software does not, so nothing here
// is disabled by the row's own state. A refusal (a cancelled row moved again,
// a slot already taken) comes back from the server and is shown as a plain
// message, never guessed at first.

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import type { AppointmentDto } from "@dzpos/shared";
import { useEffect, useState } from "react";

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
import { api, appointmentsQueryKey, dayListQueryKey, queueQueryKey } from "@/api";
import { useTranslation } from "@/i18n";
import { errorKey } from "@/lib/fields";

import { CallActions } from "./CallActions";
import { formatDay, parseStartsAt } from "./dates";

/** `starts_at` (`YYYY-MM-DD HH:MM:SS`) to the value a `datetime-local`
 *  input takes, and back. The input never carries seconds (its default
 *  step is a minute), so the appointment's own is dropped going in and
 *  restored as `:00` coming out. */
/** A plain `string`, never the route literal: see the comment beside its
 *  one use below. */
const PATIENTS_PATH: string = "/patients";

function toLocalInputValue(startsAt: string): string {
  return startsAt.slice(0, 16).replace(" ", "T");
}
function fromLocalInputValue(value: string): string {
  return `${value.replace("T", " ")}:00`;
}

export function AppointmentDialog({
  appointment,
  invalidateRange,
  onOpenChange,
  onChanged,
}: {
  appointment: AppointmentDto | null;
  /** The book's own query key for the range on screen, invalidated
   *  alongside the day list on every write here. */
  invalidateRange: { day: string } | { week: string };
  onOpenChange: (open: boolean) => void;
  onChanged: () => void;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [moveTo, setMoveTo] = useState("");

  const open = appointment !== null;

  const invalidate = async () => {
    await queryClient.invalidateQueries({ queryKey: appointmentsQueryKey(invalidateRange) });
    const start = appointment === null ? null : parseStartsAt(appointment.starts_at);
    if (start !== null) {
      await queryClient.invalidateQueries({ queryKey: dayListQueryKey(formatDay(start)) });
    }
  };

  const cancel = useMutation({
    mutationFn: () => api.cancelAppointment(appointment?.id ?? ""),
    onSuccess: async () => {
      await invalidate();
      onChanged();
    },
  });
  const move = useMutation({
    mutationFn: (startsAt: string) =>
      api.moveAppointment(appointment?.id ?? "", { starts_at: startsAt }),
    onSuccess: async () => {
      await invalidate();
      onChanged();
    },
  });
  const markNoShow = useMutation({
    mutationFn: () => api.markAppointmentNoShow(appointment?.id ?? ""),
    onSuccess: async () => {
      await invalidate();
      onChanged();
    },
  });
  const clearNoShow = useMutation({
    mutationFn: () => api.clearAppointmentNoShow(appointment?.id ?? ""),
    onSuccess: async () => {
      await invalidate();
      onChanged();
    },
  });
  // C6b: the patient came in. Today's queue gets them, linked to this
  // booking; asked twice, the server answers the same entry.
  const arrive = useMutation({
    mutationFn: () => api.checkInAppointment(appointment?.id ?? ""),
    onSuccess: async () => {
      await invalidate();
      await queryClient.invalidateQueries({ queryKey: queueQueryKey });
      onChanged();
    },
  });

  // Each appointment gets a clean form: without this, closing A after typing
  // a move date and opening B leaves B's "Move" button enabled with A's own
  // date already filled in, and a refusal shown for A would still be on
  // screen for B.
  useEffect(() => {
    setMoveTo("");
    cancel.reset();
    move.reset();
    markNoShow.reset();
    clearNoShow.reset();
    arrive.reset();
    // Keyed on the appointment's own id only: the four `reset` functions
    // change identity every render, and this codebase carries no
    // `react-hooks/exhaustive-deps` rule to appease over that.
  }, [appointment?.id]);

  const pending =
    cancel.isPending ||
    move.isPending ||
    markNoShow.isPending ||
    clearNoShow.isPending ||
    arrive.isPending;
  const lastError =
    cancel.error ?? move.error ?? markNoShow.error ?? clearNoShow.error ?? arrive.error ?? null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent data-testid="appointment-dialog">
        <DialogHeader>
          <DialogTitle>
            {appointment === null ? "" : `${appointment.first_name} ${appointment.last_name}`}
          </DialogTitle>
          <DialogDescription>
            <span dir="ltr" className="font-numeric">
              {appointment?.starts_at ?? ""}
            </span>
            {appointment?.note === null || appointment?.note === undefined ? null : (
              <span className="ms-2">{appointment.note}</span>
            )}
          </DialogDescription>
        </DialogHeader>

        {appointment === null ? null : (
          <div className="flex flex-col gap-4">
            <div className="flex items-end gap-2">
              <div className="flex-1">
                <label className="flex flex-col gap-1.5 text-start text-sm">
                  {t("book_move_to")}
                  <Input
                    type="datetime-local"
                    data-testid="appointment-move-input"
                    value={moveTo === "" ? toLocalInputValue(appointment.starts_at) : moveTo}
                    onChange={(event) => setMoveTo(event.target.value)}
                  />
                </label>
              </div>
              <Button
                type="button"
                variant="outline"
                data-testid="appointment-move"
                disabled={pending || moveTo === ""}
                onClick={() => move.mutate(fromLocalInputValue(moveTo))}
              >
                {t("book_move")}
              </Button>
            </div>

            <div className="flex flex-wrap gap-2">
              <Button
                type="button"
                data-testid="appointment-arrive"
                disabled={pending}
                onClick={() => arrive.mutate()}
              >
                {t("book_mark_arrived")}
              </Button>
              <Button
                type="button"
                variant="outline"
                data-testid="appointment-cancel"
                disabled={pending}
                onClick={() => cancel.mutate()}
              >
                {t("book_cancel_appointment")}
              </Button>
              <Button
                type="button"
                variant="outline"
                data-testid="appointment-no-show"
                disabled={pending}
                onClick={() => markNoShow.mutate()}
              >
                {t("book_mark_no_show")}
              </Button>
              <Button
                type="button"
                variant="outline"
                data-testid="appointment-clear-no-show"
                disabled={pending}
                onClick={() => clearNoShow.mutate()}
              >
                {t("book_clear_no_show")}
              </Button>
              {/* No `search` param: `-patients/PatientsScreen.tsx` is built
                  without ever importing this route's registration (its own
                  header comment), so a jump straight to one patient's fiche
                  would need that coupling. The desk opens the list and
                  searches the name shown above instead.
                  `PATIENTS_PATH` (a plain `string`, not the literal here) is
                  what lets this file typecheck in a build without `clinic`
                  too: this screen sits in `-book/`, a folder `tsc` always
                  walks (`scripts/build.mjs`'s own comment), but `"/patients"`
                  spelled as a literal only exists in the router's registered
                  paths once a `clinic` build generates the route tree, the
                  same reason `AppShell.tsx`'s `NAV` items carry `to: string`
                  rather than a route literal. */}
              <Button type="button" variant="ghost" asChild>
                <Link to={PATIENTS_PATH}>{t("book_open_patient_file")}</Link>
              </Button>
            </div>

            <div className="flex flex-col gap-1">
              <span className="text-sm font-medium text-foreground">{t("book_call_title")}</span>
              <CallActions appointment={appointment} onChanged={() => onChanged()} />
            </div>

            {lastError === null ? null : (
              <p role="alert" className="text-sm text-fg-danger">
                {t(errorKey(lastError))}
              </p>
            )}
          </div>
        )}

        <DialogFooter>
          <Button type="button" variant="ghost" onClick={() => onOpenChange(false)}>
            {t("action_close")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
