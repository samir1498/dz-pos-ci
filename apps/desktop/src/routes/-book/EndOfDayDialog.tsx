// End of the day (C6b): the day's bookings nobody marked arrived, each
// ticked, for the desk to mark no-show in one go. A suggestion the desk
// confirms and never an automatic rule: unticking a row leaves it alone,
// and nothing is written until the button is pressed. Each tick is the
// book's own no-show mark, one request per booking, so one request that
// fails fails alone and the rest still go through. A booking the desk
// marked by hand meanwhile comes back as it stands, not refused.

import { useMutation, useQueryClient } from "@tanstack/react-query";
import type { AppointmentDto } from "@dzpos/shared";
import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { api, dayListQueryKey } from "@/api";
import { useTranslation } from "@/i18n";
import { errorKey } from "@/lib/fields";

export function EndOfDayDialog({
  day,
  candidates,
  open,
  onOpenChange,
}: {
  day: string;
  /** The day's bookings never marked arrived, in time order. */
  candidates: readonly AppointmentDto[];
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [ticked, setTicked] = useState<ReadonlySet<string>>(new Set());

  // Every row starts ticked each time the dialog opens: the suggestion is
  // all of them, and the desk takes out the ones it knows better about.
  useEffect(() => {
    if (open) setTicked(new Set(candidates.map((c) => c.id)));
    // Keyed on opening only: a refetch of the list while it is open must
    // not tick again a row the desk just unticked.
  }, [open]);

  const mark = useMutation({
    mutationFn: async (ids: readonly string[]) => {
      const refused: unknown[] = [];
      for (const id of ids) {
        try {
          await api.markAppointmentNoShow(id);
        } catch (error) {
          refused.push(error);
        }
      }
      if (refused.length > 0) throw refused[0];
    },
    onSettled: async () => {
      await queryClient.invalidateQueries({ queryKey: dayListQueryKey(day) });
      await queryClient.invalidateQueries({ queryKey: ["appointments"] });
    },
    onSuccess: () => onOpenChange(false),
  });

  const toggle = (id: string, on: boolean) =>
    setTicked((current) => {
      const next = new Set(current);
      if (on) next.add(id);
      else next.delete(id);
      return next;
    });
  const chosen = candidates.filter((c) => ticked.has(c.id)).map((c) => c.id);

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) mark.reset();
        onOpenChange(next);
      }}
    >
      <DialogContent data-testid="end-of-day-dialog">
        <DialogHeader>
          <DialogTitle>{t("book_end_of_day_title")}</DialogTitle>
          <DialogDescription>{t("book_end_of_day_hint")}</DialogDescription>
        </DialogHeader>

        {candidates.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t("book_end_of_day_none")}</p>
        ) : (
          <ul className="flex max-h-80 flex-col gap-2 overflow-y-auto">
            {candidates.map((appointment) => {
              const id = `end-of-day-${appointment.id}`;
              return (
                <li key={appointment.id} className="flex items-center gap-2 text-sm">
                  <Checkbox
                    id={id}
                    data-testid={id}
                    checked={ticked.has(appointment.id)}
                    onCheckedChange={(next) => toggle(appointment.id, next === true)}
                  />
                  <Label htmlFor={id} className="flex items-center gap-2 font-normal">
                    <span dir="ltr" className="font-numeric text-muted-foreground">
                      {appointment.starts_at.slice(11, 16)}
                    </span>
                    <span className="text-foreground">
                      {appointment.first_name} {appointment.last_name}
                    </span>
                  </Label>
                </li>
              );
            })}
          </ul>
        )}

        {mark.isError ? (
          <p role="alert" className="text-sm text-fg-danger">
            {t(errorKey(mark.error))}
          </p>
        ) : null}

        <DialogFooter>
          <Button type="button" variant="ghost" onClick={() => onOpenChange(false)}>
            {t("action_close")}
          </Button>
          <Button
            type="button"
            data-testid="end-of-day-mark"
            disabled={chosen.length === 0 || mark.isPending}
            onClick={() => mark.mutate(chosen)}
          >
            {t("book_end_of_day_mark").replace("{count}", String(chosen.length))}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
