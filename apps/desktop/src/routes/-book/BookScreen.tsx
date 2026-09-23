// The appointment book (C6 of
// `the-first-clinic-module-patients-queue-appointments`): day and week
// views on FullCalendar's free MIT plugins
// (`context/research/20260923-a-calendar-for-the-clinics-book.md`), pinned
// to the v6 line so no `temporal-polyfill` rides along
// (`packages/shared`'s commit message says which version and why).
//
// The screen adds no rule of its own: a click books, drags move, and every
// refusal the server sends (a slot already taken, a start off the grid) is
// shown as the plain message `errorKey` already gives every other screen
// (Samir, 2026-09-23 19:34, "the desk decides, the software does not
// enforce").

import { useMemo, useRef, useState } from "react";
import type { AppointmentDto } from "@dzpos/shared";
import type { EventClickArg, EventContentArg, EventDropArg } from "@fullcalendar/core";
import arDzLocale from "@fullcalendar/core/locales/ar-dz";
import frLocale from "@fullcalendar/core/locales/fr";
import dayGridPlugin from "@fullcalendar/daygrid";
import interactionPlugin from "@fullcalendar/interaction";
import FullCalendar from "@fullcalendar/react";
import timeGridPlugin from "@fullcalendar/timegrid";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { CalendarOff, CalendarPlus, ChevronLeft, ChevronRight, PhoneCall } from "lucide-react";

import { Icon } from "@/components/Icon";
import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  absenceBlocksQueryKey,
  api,
  appointmentsQueryKey,
  slotMinutesQueryKey,
  workingHoursQueryKey,
} from "@/api";
import { useTranslation } from "@/i18n";
import { useShopToday } from "@/lib/clock";
import { errorKey } from "@/lib/fields";

import { AbsenceBlocksForm } from "./AbsenceBlocksForm";
import { AppointmentDialog } from "./AppointmentDialog";
import { BookDialog } from "./BookDialog";
import { CallMark } from "./CallActions";
import { CallsDialog } from "./CallsDialog";
import {
  absenceBackgroundEvents,
  businessHoursOf,
  hiddenWeekdays,
  slotDurationOf,
} from "./calendarConfig";
import { addMinutes, formatDay, formatStartsAt, nextDay, parseStartsAt } from "./dates";
import { DayListPanel } from "./DayListPanel";
import { NextFreeDialog } from "./NextFreeDialog";

type ViewKind = "timeGridDay" | "timeGridWeek";

/** A plain `string`, never the route literal: see the comment beside its
 *  one use below. */
const SETTINGS_BOOK_PATH: string = "/settings/book";

export function BookScreen() {
  const { t, dir, lang } = useTranslation();
  const queryClient = useQueryClient();
  const calendarRef = useRef<FullCalendar | null>(null);
  const today = useShopToday();

  const [view, setView] = useState<ViewKind>("timeGridWeek");
  const [range, setRange] = useState<{ start: Date } | null>(null);
  const [bookStartsAt, setBookStartsAt] = useState<string | null>(null);
  const [openAppointment, setOpenAppointment] = useState<AppointmentDto | null>(null);
  const [nextFreeOpen, setNextFreeOpen] = useState(false);
  const [dayListDay, setDayListDay] = useState<string | null>(null);
  const [absenceOpen, setAbsenceOpen] = useState(false);
  const [callsOpen, setCallsOpen] = useState(false);

  const workingHours = useQuery({
    queryKey: workingHoursQueryKey,
    queryFn: () => api.getWorkingHours(),
  });
  const absenceBlocks = useQuery({
    queryKey: absenceBlocksQueryKey,
    queryFn: () => api.listAbsenceBlocks(),
  });
  const slotMinutes = useQuery({ queryKey: slotMinutesQueryKey, queryFn: () => api.getSlotMinutes() });

  const bookRange: { day: string } | { week: string } =
    range === null
      ? { day: today.today ?? formatDay(new Date()) }
      : view === "timeGridWeek"
        ? { week: formatDay(range.start) }
        : { day: formatDay(range.start) };

  const appointments = useQuery({
    queryKey: appointmentsQueryKey(bookRange),
    queryFn: () => api.listAppointments(bookRange),
  });

  const effectiveDayListDay = dayListDay ?? today.today ?? formatDay(new Date());

  const workingDays = workingHours.data?.days ?? null;
  const hiddenDays = useMemo(() => [...hiddenWeekdays(workingDays)], [workingDays]);
  const businessHours = workingDays === null ? false : [...businessHoursOf(workingDays)];

  const events = useMemo(() => {
    const appointmentEvents = (appointments.data?.appointments ?? []).map((appointment) => {
      const start = parseStartsAt(appointment.starts_at);
      const color =
        appointment.cancelled_at !== null
          ? "var(--muted)"
          : appointment.no_show_at !== null
            ? "var(--destructive)"
            : "var(--primary)";
      return {
        id: appointment.id,
        title: `${appointment.first_name} ${appointment.last_name}`,
        start: start ?? new Date(),
        end: start === null ? new Date() : addMinutes(start, appointment.slot_minutes),
        backgroundColor: color,
        borderColor: color,
        extendedProps: { appointment },
      };
    });
    // `classNames` is spread into a fresh mutable array here rather than
    // widened on `AbsenceBackgroundEvent` itself: FullCalendar's own
    // `ClassNamesInput` wants a `string[]` it might sort or push to, and
    // every other field on that type stays `readonly` for the same reason
    // the rest of this codebase does.
    const absenceEvents = absenceBackgroundEvents(absenceBlocks.data?.blocks ?? []).map(
      (event) => ({ ...event, classNames: [...event.classNames] }),
    );
    return [...appointmentEvents, ...absenceEvents];
  }, [appointments.data, absenceBlocks.data]);

  const invalidateBook = async () => {
    await queryClient.invalidateQueries({ queryKey: appointmentsQueryKey(bookRange) });
    await queryClient.invalidateQueries({ queryKey: ["day-list"] });
  };

  const moveMutation = useMutation({
    mutationFn: ({ id, startsAt }: { id: string; startsAt: string }) =>
      api.moveAppointment(id, { starts_at: startsAt }),
    onSuccess: () => invalidateBook(),
  });

  const calendarApi = () => calendarRef.current?.getApi();
  const switchView = (next: ViewKind) => {
    setView(next);
    calendarApi()?.changeView(next);
  };

  const handleEventClick = (info: EventClickArg) => {
    const appointment = info.event.extendedProps.appointment;
    if (appointment !== undefined) setOpenAppointment(appointment);
  };

  // The event's own name, and the confirmation call's small mark once one
  // is recorded (C6b). Background events (absence blocks) keep their own
  // look: they carry no appointment.
  const renderEvent = (info: EventContentArg) => {
    const appointment = info.event.extendedProps.appointment;
    return (
      <span className="flex items-center gap-1 overflow-hidden">
        {appointment === undefined ? null : <CallMark outcome={appointment.call_outcome} />}
        <span className="truncate">{info.event.title}</span>
      </span>
    );
  };

  const handleMoveByDrag = (info: EventDropArg) => {
    const appointment = info.event.extendedProps.appointment;
    const start = info.event.start;
    if (appointment === undefined || start === null) {
      info.revert();
      return;
    }
    moveMutation.mutate(
      { id: appointment.id, startsAt: formatStartsAt(start) },
      { onError: () => info.revert() },
    );
  };

  const toolbar = (
    <div className="flex flex-wrap items-center gap-2">
      <Button
        type="button"
        variant="outline"
        size="sm"
        data-testid="book-prev"
        onClick={() => calendarApi()?.prev()}
      >
        <Icon as={ChevronLeft} size={18} flip />
      </Button>
      <Button
        type="button"
        variant="outline"
        size="sm"
        data-testid="book-today"
        onClick={() => calendarApi()?.today()}
      >
        {t("book_today")}
      </Button>
      <Button
        type="button"
        variant="outline"
        size="sm"
        data-testid="book-next"
        onClick={() => calendarApi()?.next()}
      >
        <Icon as={ChevronRight} size={18} flip />
      </Button>
      <div className="flex items-center gap-1 rounded-md border border-border p-0.5">
        <Button
          type="button"
          variant={view === "timeGridDay" ? "secondary" : "ghost"}
          size="sm"
          data-testid="book-view-day"
          onClick={() => switchView("timeGridDay")}
        >
          {t("book_view_day")}
        </Button>
        <Button
          type="button"
          variant={view === "timeGridWeek" ? "secondary" : "ghost"}
          size="sm"
          data-testid="book-view-week"
          onClick={() => switchView("timeGridWeek")}
        >
          {t("book_view_week")}
        </Button>
      </div>
      <Button
        type="button"
        variant="outline"
        size="sm"
        data-testid="book-next-free"
        onClick={() => setNextFreeOpen(true)}
      >
        <Icon as={CalendarPlus} size={18} />
        {t("book_next_free_button")}
      </Button>
      {/* Alongside the book's own write actions (book, move, cancel), all
          gated by the server on `edit_patients`, the same permission a
          cashier already holds: absence blocks moved here from the
          `edit_settings`-gated settings room for that reason (Samir's
          review round, 2026-09-23). */}
      <Button
        type="button"
        variant="outline"
        size="sm"
        data-testid="book-calls"
        onClick={() => setCallsOpen(true)}
      >
        <Icon as={PhoneCall} size={18} />
        {t("book_calls_button")}
      </Button>
      <Button
        type="button"
        variant="outline"
        size="sm"
        data-testid="book-absence"
        onClick={() => setAbsenceOpen(true)}
      >
        <Icon as={CalendarOff} size={18} />
        {t("book_absence_button")}
      </Button>
      {/* `SETTINGS_BOOK_PATH`, not the literal: this screen is walked by
          `tsc` in every build (`scripts/build.mjs`'s comment on `-folder`s),
          and `"/settings/book"` is only a registered path once a `clinic`
          build generates the route tree. */}
      <Button type="button" variant="ghost" size="sm" asChild>
        <Link to={SETTINGS_BOOK_PATH}>{t("book_settings_link")}</Link>
      </Button>
    </div>
  );

  const settingsError = workingHours.error ?? absenceBlocks.error ?? slotMinutes.error;

  return (
    <section className="flex flex-col gap-4">
      <PageHeader title={t("book_title")} description={t("book_subtitle")} actions={toolbar} />

      {settingsError !== null && settingsError !== undefined ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(settingsError))}
        </p>
      ) : null}
      {appointments.isError ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(appointments.error))}
        </p>
      ) : null}
      {moveMutation.isError ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(moveMutation.error))}
        </p>
      ) : null}

      <div className="flex flex-col gap-4 lg:flex-row">
        <div
          className="min-w-0 flex-1 rounded-lg border border-border p-2"
          style={{
            "--fc-border-color": "var(--border)",
            "--fc-page-bg-color": "var(--background)",
            "--fc-neutral-bg-color": "var(--muted)",
            "--fc-non-business-color": "var(--muted)",
            "--fc-today-bg-color": "var(--accent)",
            "--fc-event-text-color": "var(--primary-foreground)",
            "--fc-bg-event-color": "var(--muted)",
            "--fc-bg-event-opacity": "0.65",
          }}
        >
          <FullCalendar
            ref={calendarRef}
            plugins={[dayGridPlugin, timeGridPlugin, interactionPlugin]}
            initialView={view}
            headerToolbar={false}
            firstDay={0}
            hiddenDays={hiddenDays}
            businessHours={businessHours}
            slotDuration={slotDurationOf(slotMinutes.data?.slot_minutes)}
            height="auto"
            direction={dir}
            locales={[arDzLocale, frLocale]}
            locale={lang === "ar" ? "ar-dz" : lang}
            events={events}
            editable
            eventStartEditable
            eventDurationEditable={false}
            dateClick={(info) => setBookStartsAt(formatStartsAt(info.date))}
            eventClick={handleEventClick}
            eventContent={renderEvent}
            eventDrop={handleMoveByDrag}
            datesSet={(info) => setRange({ start: info.start })}
          />
        </div>

        <DayListPanel day={effectiveDayListDay} onDayChange={setDayListDay} />
      </div>

      <BookDialog
        startsAt={bookStartsAt}
        onOpenChange={(open) => {
          if (!open) setBookStartsAt(null);
        }}
        onBooked={async () => {
          setBookStartsAt(null);
          await invalidateBook();
        }}
      />

      <AppointmentDialog
        appointment={openAppointment}
        invalidateRange={bookRange}
        onOpenChange={(open) => {
          if (!open) setOpenAppointment(null);
        }}
        onChanged={() => setOpenAppointment(null)}
      />

      <NextFreeDialog
        open={nextFreeOpen}
        from={today.today ?? formatDay(new Date())}
        onOpenChange={setNextFreeOpen}
        onFound={(startsAt) => {
          setNextFreeOpen(false);
          const date = parseStartsAt(startsAt);
          if (date !== null) {
            calendarApi()?.gotoDate(date);
            setRange({ start: date });
          }
          setBookStartsAt(startsAt);
        }}
      />

      <CallsDialog
        day={today.today === undefined ? null : nextDay(today.today)}
        open={callsOpen}
        onOpenChange={setCallsOpen}
      />

      <Dialog open={absenceOpen} onOpenChange={setAbsenceOpen}>
        <DialogContent data-testid="absence-dialog">
          <DialogHeader>
            <DialogTitle>{t("book_absence_button")}</DialogTitle>
          </DialogHeader>
          <AbsenceBlocksForm />
        </DialogContent>
      </Dialog>
    </section>
  );
}
