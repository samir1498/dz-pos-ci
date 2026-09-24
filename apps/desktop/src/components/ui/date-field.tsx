// Three number boxes instead of the browser's own date picker. A native
// `<input type="date">` reads in the machine's language and the machine's
// day order, which is the bug this file exists to fix: an Algerian shop on
// a French Windows saw "11/09/2026" and had no way to tell September from
// November. Day, then month, then year, always in that order and always
// left to right, because the order is the app's choice and not the
// machine's.
//
// The value in and out is the same ISO string the native control used
// (`YYYY-MM-DD`, or `""` for no date), so a call site changes an element
// and nothing else. An incomplete or impossible date, 31 February
// included, reports as `""`: every screen that used the native control
// already treats that as "no date".
//
// Beside the boxes, a calendar button (T35): a month to pick a day from, and
// a row of presets (Aujourd'hui, Hier) when the caller hands over the shop's
// day. Typing still works and stays the fast way; the calendar is for the
// day somebody knows by its place in the week rather than by its number.
// The presets move from the shop's day, never the machine's, which is why
// the caller passes it in rather than this file reading a clock.
//
// Weeks start on Saturday, in every language: the calendar is the shop's,
// and an Algerian week runs Saturday to Friday on the calendars a shop has
// on its wall. An assumption, one prop to change (`WEEK_STARTS_ON`).

import { useEffect, useState } from "react";
import { CalendarDays } from "lucide-react";
import { arDZ, enUS, fr } from "react-day-picker/locale";

import { Icon } from "@/components/Icon";
import { useTranslation, type Key } from "@/i18n";
import {
  clampDayToMonth,
  clampSegment,
  clampYear,
  parseIsoDate,
  shiftIsoDate,
  toIsoDate,
} from "@/lib/date-segments";
import { cn } from "@/lib/utils";

import { Button } from "./button";
import { Calendar } from "./calendar";
import { Input } from "./input";
import { Popover, PopoverContent, PopoverTrigger } from "./popover";

/** Saturday, as react-day-picker counts the week (0 is Sunday). */
const WEEK_STARTS_ON = 6;

const LOCALES = { fr, en: enUS, ar: arDZ } as const;

/** A day named by where it sits from the shop's today. */
export type DatePreset = "today" | "yesterday" | "in_7_days" | "in_30_days";

const PRESETS: Record<DatePreset, { key: Key; days: number }> = {
  today: { key: "date_preset_today", days: 0 },
  yesterday: { key: "date_preset_yesterday", days: -1 },
  in_7_days: { key: "date_preset_in_7_days", days: 7 },
  in_30_days: { key: "date_preset_in_30_days", days: 30 },
};

/** The calendar's own Date for an ISO day, at local midnight, which is what
 *  react-day-picker builds its days as; `undefined` for no day. */
function dayOf(iso: string): Date | undefined {
  const { day, month, year } = parseIsoDate(iso);
  if (toIsoDate(day, month, year) === "") return undefined;
  return new Date(Number(year), Number(month) - 1, Number(day));
}

/** The ISO day the calendar handed back, read off its own local fields. */
function isoOf(date: Date): string {
  return toIsoDate(
    String(date.getDate()),
    String(date.getMonth() + 1),
    String(date.getFullYear()).padStart(4, "0"),
  );
}

export interface DateFieldProps {
  /** `YYYY-MM-DD`, or `""` for no date. */
  value: string;
  onChange: (value: string) => void;
  /** Fires once focus leaves all three boxes, not on the way between them. */
  onBlur?: () => void;
  id?: string;
  className?: string;
  disabled?: boolean;
  "data-testid"?: string;
  "aria-invalid"?: boolean;
  "aria-describedby"?: string;
  /** The field's label, named on the group: three boxes cannot be named by
   *  one `htmlFor`, and an `aria-label` on a box beats the label and hides
   *  it. */
  "aria-labelledby"?: string;
  /** The shop's day, `YYYY-MM-DD` (`useShopToday`). The presets are shown
   *  only when it is known, and the calendar opens on it when the field is
   *  empty. */
  today?: string;
  /** Which presets the calendar offers. A due date looks forward, a
   *  statement's range looks back. */
  presets?: readonly DatePreset[];
}

export function DateField({
  value,
  onChange,
  onBlur,
  id,
  className,
  disabled,
  "data-testid": testId,
  "aria-invalid": invalid,
  "aria-describedby": describedBy,
  "aria-labelledby": labelledBy,
  today,
  presets = ["today", "yesterday"],
}: DateFieldProps) {
  const { t, lang, dir } = useTranslation();
  const [open, setOpen] = useState(false);
  const initial = parseIsoDate(value);
  const [day, setDay] = useState(initial.day);
  const [month, setMonth] = useState(initial.month);
  const [year, setYear] = useState(initial.year);

  // Only reacts to a value that did not come from typing here: while the
  // three segments already build the value the caller just handed back, a
  // resync would fight the segment still being filled in (a year cleared
  // to retype it is briefly "" too, and that must not blank the day and
  // month sitting beside it).
  useEffect(() => {
    if (toIsoDate(day, month, year) === value) return;
    const next = parseIsoDate(value);
    setDay(next.day);
    setMonth(next.month);
    setYear(next.year);
  }, [value]);

  function commit(typedDay: string, nextMonth: string, nextYear: string) {
    const nextDay = clampDayToMonth(typedDay, nextMonth, nextYear);
    setDay(nextDay);
    setMonth(nextMonth);
    setYear(nextYear);
    onChange(toIsoDate(nextDay, nextMonth, nextYear));
  }

  return (
    <div
      dir="ltr"
      role="group"
      aria-labelledby={labelledBy}
      aria-describedby={describedBy}
      className={cn("flex min-w-0 max-w-full items-center gap-1", className)}
      data-testid={testId}
      onBlur={(event) => {
        if (onBlur !== undefined && !event.currentTarget.contains(event.relatedTarget)) onBlur();
      }}
    >
      <Input
        id={id}
        inputMode="numeric"
        autoComplete="off"
        disabled={disabled}
        aria-label={t("date_segment_day")}
        aria-invalid={invalid}
        data-testid={testId === undefined ? undefined : `${testId}-day`}
        className="w-11 text-center font-numeric tabular-nums"
        maxLength={2}
        value={day}
        onChange={(event) => commit(clampSegment(event.target.value, 31, 2), month, year)}
      />
      <span aria-hidden="true" className="text-muted-foreground">
        /
      </span>
      <Input
        inputMode="numeric"
        autoComplete="off"
        disabled={disabled}
        aria-label={t("date_segment_month")}
        aria-invalid={invalid}
        data-testid={testId === undefined ? undefined : `${testId}-month`}
        className="w-11 text-center font-numeric tabular-nums"
        maxLength={2}
        value={month}
        onChange={(event) => commit(day, clampSegment(event.target.value, 12, 2), year)}
      />
      <span aria-hidden="true" className="text-muted-foreground">
        /
      </span>
      <Input
        inputMode="numeric"
        autoComplete="off"
        disabled={disabled}
        aria-label={t("date_segment_year")}
        aria-invalid={invalid}
        data-testid={testId === undefined ? undefined : `${testId}-year`}
        className="w-16 text-center font-numeric tabular-nums"
        maxLength={4}
        value={year}
        onChange={(event) => commit(day, month, clampYear(event.target.value))}
      />
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            disabled={disabled}
            aria-label={t("date_open_calendar")}
            data-testid={testId === undefined ? undefined : `${testId}-calendar`}
          >
            <Icon as={CalendarDays} size={18} />
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-auto p-2" align="start" dir={dir}>
          {today === undefined ? null : (
            <div className="mb-2 flex flex-wrap gap-1">
              {presets.map((preset) => (
                <Button
                  key={preset}
                  type="button"
                  variant="outline"
                  size="xs"
                  onClick={() => {
                    const next = parseIsoDate(shiftIsoDate(today, PRESETS[preset].days));
                    commit(next.day, next.month, next.year);
                    setOpen(false);
                  }}
                >
                  {t(PRESETS[preset].key)}
                </Button>
              ))}
            </div>
          )}
          <Calendar
            mode="single"
            dir={dir}
            locale={LOCALES[lang]}
            weekStartsOn={WEEK_STARTS_ON}
            selected={dayOf(value)}
            defaultMonth={dayOf(value) ?? (today === undefined ? undefined : dayOf(today))}
            onSelect={(picked) => {
              if (picked === undefined) return;
              const next = parseIsoDate(isoOf(picked));
              commit(next.day, next.month, next.year);
              setOpen(false);
            }}
          />
        </PopoverContent>
      </Popover>
    </div>
  );
}
