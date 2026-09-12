// Two number boxes and a name, instead of the browser's own month picker.
// The boxes are what `type="month"` was for the value; the name is what it
// was never for and is the whole reason this file exists: a native month
// picker prints its month word in the machine's language, so an Arabic shop
// on a French Windows read "septembre" on its own expenses screen. The
// caption here is read from the dictionary instead, so it follows the
// shop's language, not the operating system's.
//
// The value in and out is the same `YYYY-MM` string `type="month"` used, or
// `""` for no month; an incomplete or impossible one (month 13, an empty
// year) reports as `""` the same way a bad day does in DateField.

import { useEffect, useState } from "react";

import { useTranslation } from "@/i18n";
import { clampSegment, clampYear, MONTH_KEYS, parseIsoMonth, toIsoMonth } from "@/lib/date-segments";
import { cn } from "@/lib/utils";

import { Input } from "./input";

export interface MonthFieldProps {
  /** `YYYY-MM`, or `""` for no month. */
  value: string;
  onChange: (value: string) => void;
  id?: string;
  className?: string;
  disabled?: boolean;
  "data-testid"?: string;
  "aria-invalid"?: boolean;
  "aria-describedby"?: string;
  /** The field's label, named on the group, for the reason DateField gives. */
  "aria-labelledby"?: string;
}

export function MonthField({
  value,
  onChange,
  id,
  className,
  disabled,
  "data-testid": testId,
  "aria-invalid": invalid,
  "aria-describedby": describedBy,
  "aria-labelledby": labelledBy,
}: MonthFieldProps) {
  const { t } = useTranslation();
  const initial = parseIsoMonth(value);
  const [month, setMonth] = useState(initial.month);
  const [year, setYear] = useState(initial.year);

  // Same reasoning as DateField: only an external value resyncs the boxes,
  // so clearing the year to retype it does not blank the month sitting
  // beside it.
  useEffect(() => {
    if (toIsoMonth(month, year) === value) return;
    const next = parseIsoMonth(value);
    setMonth(next.month);
    setYear(next.year);
  }, [value]);

  function commit(nextMonth: string, nextYear: string) {
    setMonth(nextMonth);
    setYear(nextYear);
    onChange(toIsoMonth(nextMonth, nextYear));
  }

  const monthNumber = Number(month);
  const monthName =
    month !== "" && monthNumber >= 1 && monthNumber <= 12 ? t(MONTH_KEYS[monthNumber - 1] ?? "month_01") : "";

  return (
    <div
      dir="ltr"
      role="group"
      aria-labelledby={labelledBy}
      aria-describedby={describedBy}
      className={cn("flex items-center gap-2", className)}
      data-testid={testId}
    >
      <div className="flex items-center gap-1">
        <Input
          id={id}
          inputMode="numeric"
          autoComplete="off"
          disabled={disabled}
          aria-label={t("date_segment_month")}
          aria-invalid={invalid}
          data-testid={testId === undefined ? undefined : `${testId}-month`}
          className="w-11 text-center font-numeric tabular-nums"
          maxLength={2}
          value={month}
          onChange={(event) => commit(clampSegment(event.target.value, 12, 2), year)}
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
          onChange={(event) => commit(month, clampYear(event.target.value))}
        />
      </div>
      {monthName === "" ? null : (
        <span data-testid={testId === undefined ? undefined : `${testId}-name`} className="text-sm text-muted-foreground">
          {monthName}
        </span>
      )}
    </div>
  );
}
