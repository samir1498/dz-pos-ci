// The confirmation call (C6b): what came of it, as a small mark, and the
// desk's three actions on it. Samir, 2026-09-23 19:34: the desk records the
// outcome and decides what follows; nothing here cancels or moves a
// booking because of a call. Used by the appointment dialog and by the
// list of tomorrow's calls.

import { useMutation, useQueryClient } from "@tanstack/react-query";
import type { AppointmentDto, CallOutcomeDto } from "@dzpos/shared";
import { Check, PhoneMissed } from "lucide-react";

import { Icon } from "@/components/Icon";
import { Button } from "@/components/ui/button";
import { api } from "@/api";
import { useTranslation, type Key } from "@/i18n";
import { errorKey } from "@/lib/fields";

const OUTCOME_KEY: Readonly<Record<CallOutcomeDto, Key>> = {
  confirmed: "book_call_confirmed",
  no_answer: "book_call_no_answer",
};

/** The small mark a booking carries once a call is recorded: an icon the
 *  screen reader names, nothing before a call. */
export function CallMark({ outcome }: { outcome: CallOutcomeDto | null }) {
  const { t } = useTranslation();
  if (outcome === null) return null;
  return outcome === "confirmed" ? (
    <Icon as={Check} size={18} className="text-fg-success" label={t(OUTCOME_KEY[outcome])} />
  ) : (
    <Icon as={PhoneMissed} size={18} className="text-fg-danger" label={t(OUTCOME_KEY[outcome])} />
  );
}

export function CallActions({
  appointment,
  onChanged,
}: {
  appointment: AppointmentDto;
  onChanged?: (after: AppointmentDto) => void;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const done = async (after: AppointmentDto) => {
    await queryClient.invalidateQueries({ queryKey: ["appointments"] });
    await queryClient.invalidateQueries({ queryKey: ["day-list"] });
    onChanged?.(after);
  };
  const record = useMutation({
    mutationFn: (outcome: CallOutcomeDto) => api.recordAppointmentCall(appointment.id, { outcome }),
    onSuccess: done,
  });
  const clear = useMutation({
    mutationFn: () => api.clearAppointmentCall(appointment.id),
    onSuccess: done,
  });
  const pending = record.isPending || clear.isPending;
  const error = record.error ?? clear.error ?? null;

  return (
    <div className="flex flex-col gap-1">
      <div className="flex flex-wrap items-center gap-1">
        <span className="me-1 flex items-center gap-1 text-sm text-muted-foreground">
          <CallMark outcome={appointment.call_outcome} />
          {appointment.call_outcome === null ? t("book_call_none") : null}
          {appointment.call_at === null ? null : (
            <span dir="ltr" className="font-numeric">
              {appointment.call_at.slice(0, 16)}
            </span>
          )}
        </span>
        <Button
          type="button"
          variant="outline"
          size="sm"
          data-testid={`call-confirmed-${appointment.id}`}
          disabled={pending}
          onClick={() => record.mutate("confirmed")}
        >
          {t("book_call_confirmed")}
        </Button>
        <Button
          type="button"
          variant="outline"
          size="sm"
          data-testid={`call-no-answer-${appointment.id}`}
          disabled={pending}
          onClick={() => record.mutate("no_answer")}
        >
          {t("book_call_no_answer")}
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          data-testid={`call-clear-${appointment.id}`}
          disabled={pending}
          onClick={() => clear.mutate()}
        >
          {t("book_call_clear")}
        </Button>
      </div>
      {error === null ? null : (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(error))}
        </p>
      )}
    </div>
  );
}
