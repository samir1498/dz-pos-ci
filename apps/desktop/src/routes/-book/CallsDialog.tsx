// Tomorrow's calls (C6b): the desk rings each of tomorrow's bookings the
// day before and records what came of it on the row. The list is the
// book's own day read for tomorrow, in time order, with the number to ring
// beside each name. What follows a call is the desk's choice (Samir,
// 2026-09-23 19:34): nothing here cancels or moves a booking.

import { useQuery } from "@tanstack/react-query";

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Skeleton } from "@/components/ui/skeleton";
import { api, appointmentsQueryKey } from "@/api";
import { useTranslation } from "@/i18n";
import { errorKey } from "@/lib/fields";

import { CallActions } from "./CallActions";

export function CallsDialog({
  day,
  open,
  onOpenChange,
}: {
  /** Tomorrow on the shop's clock, `YYYY-MM-DD`; `null` until it is known. */
  day: string | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { t } = useTranslation();
  const range = { day: day ?? "" };
  const calls = useQuery({
    queryKey: appointmentsQueryKey(range),
    queryFn: () => api.listAppointments(range),
    enabled: open && day !== null,
  });

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent data-testid="calls-dialog">
        <DialogHeader>
          <DialogTitle>{t("book_calls_button")}</DialogTitle>
          <DialogDescription>
            <span dir="ltr" className="font-numeric">
              {day ?? ""}
            </span>
          </DialogDescription>
        </DialogHeader>

        {calls.isPending && open ? <Skeleton className="h-24 w-full" /> : null}
        {calls.isError ? (
          <p role="alert" className="text-sm text-fg-danger">
            {t(errorKey(calls.error))}
          </p>
        ) : null}
        {calls.isSuccess && calls.data.appointments.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t("book_calls_empty")}</p>
        ) : null}
        {calls.isSuccess && calls.data.appointments.length > 0 ? (
          <ul className="flex max-h-96 flex-col gap-3 overflow-y-auto" data-testid="calls-list">
            {calls.data.appointments.map((appointment) => (
              <li
                key={appointment.id}
                data-testid={`calls-row-${appointment.id}`}
                className="flex flex-col gap-1 border-b border-border pb-2"
              >
                <div className="flex items-center gap-2 text-sm">
                  <span dir="ltr" className="font-numeric text-muted-foreground">
                    {appointment.starts_at.slice(11, 16)}
                  </span>
                  <span className="font-medium text-foreground">
                    {appointment.first_name} {appointment.last_name}
                  </span>
                  <span dir="ltr" className="ms-auto font-numeric text-foreground">
                    {appointment.phone ?? t("book_phone_none")}
                  </span>
                </div>
                <CallActions appointment={appointment} />
              </li>
            ))}
          </ul>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}
