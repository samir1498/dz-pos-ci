// One day's bookings beside the walk-in queue (C5b's `DayListDto`, C6's
// print). A plain `<style>` tag rather than a new stylesheet file: the app
// has none per screen today (`styles.css` imports only the generated
// tokens), and `@media print` cannot be expressed through an inline `style`
// prop, so this is the smallest thing that is still one file the panel
// owns rather than a global rule nothing else needed.

import { useQuery } from "@tanstack/react-query";
import { Printer } from "lucide-react";

import { Icon } from "@/components/Icon";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { api, dayListQueryKey } from "@/api";
import { useTranslation } from "@/i18n";
import { errorKey } from "@/lib/fields";

const PRINT_STYLE = `
@media print {
  body * { visibility: hidden; }
  #book-day-list, #book-day-list * { visibility: visible; }
  #book-day-list { position: absolute; inset: 0; }
  .book-no-print { display: none !important; }
}
`;

export function DayListPanel({
  day,
  onDayChange,
}: {
  day: string;
  onDayChange: (day: string) => void;
}) {
  const { t } = useTranslation();
  const dayList = useQuery({ queryKey: dayListQueryKey(day), queryFn: () => api.dayList(day) });

  return (
    <aside
      id="book-day-list"
      className="flex w-full max-w-xs flex-col gap-3 rounded-lg border border-border p-4"
    >
      <style>{PRINT_STYLE}</style>

      <div className="book-no-print flex items-center justify-between gap-2">
        <Input
          type="date"
          data-testid="day-list-date"
          value={day}
          onChange={(event) => onDayChange(event.target.value)}
        />
        <Button type="button" variant="outline" size="sm" onClick={() => window.print()}>
          <Icon as={Printer} size={18} />
          {t("book_print")}
        </Button>
      </div>

      <h3 className="text-sm font-semibold text-foreground">{t("book_day_list_title")}</h3>

      {dayList.isPending ? <Skeleton className="h-24 w-full" /> : null}
      {dayList.isError ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(dayList.error))}
        </p>
      ) : null}
      {dayList.isSuccess ? (
        <>
          <ul className="flex flex-col gap-1 text-sm" data-testid="day-list-appointments">
            {dayList.data.appointments.map((appointment) => (
              <li key={appointment.id} className="flex items-center gap-2">
                <span dir="ltr" className="font-numeric text-muted-foreground">
                  {appointment.starts_at.slice(11, 16)}
                </span>
                <span className="text-foreground">
                  {appointment.first_name} {appointment.last_name}
                </span>
                {appointment.no_show_at !== null ? (
                  <span className="text-fg-danger">{t("book_no_show_badge")}</span>
                ) : null}
                {appointment.cancelled_at !== null ? (
                  <span className="text-muted-foreground">{t("book_cancelled_badge")}</span>
                ) : null}
              </li>
            ))}
          </ul>

          <h4 className="text-sm font-semibold text-foreground">{t("book_walk_ins_title")}</h4>
          <ul className="flex flex-col gap-1 text-sm" data-testid="day-list-walk-ins">
            {dayList.data.walk_ins.map((entry) => (
              <li key={entry.id} className="text-foreground">
                {entry.first_name} {entry.last_name}
              </li>
            ))}
          </ul>
        </>
      ) : null}
    </aside>
  );
}
