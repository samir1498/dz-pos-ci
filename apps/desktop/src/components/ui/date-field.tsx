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

import { useEffect, useState } from "react";

import { useTranslation } from "@/i18n";
import {
  clampDayToMonth,
  clampSegment,
  clampYear,
  parseIsoDate,
  toIsoDate,
} from "@/lib/date-segments";
import { cn } from "@/lib/utils";

import { Input } from "./input";

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
}: DateFieldProps) {
  const { t } = useTranslation();
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
      className={cn("flex items-center gap-1", className)}
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
    </div>
  );
}
