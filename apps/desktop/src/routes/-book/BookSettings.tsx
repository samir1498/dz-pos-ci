// The book's own settings room (C5b, C6): working hours and visit types,
// both gated on `edit_settings`. Absence blocks moved to the book screen
// itself (`-book/AbsenceBlocksForm.tsx`): a cashier holds `edit_patients`
// and the server lets her use that tool, but never `edit_settings`, so this
// room is not where it can live. Every write here answers with the row as
// it now stands; the screen redraws from the answer rather than assuming
// the write went through as asked.

import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { OpenRangeDto, VisitTypeDto } from "@dzpos/shared";

import { FormField } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { api, visitTypesQueryKey, workingHoursQueryKey } from "@/api";
import { useTranslation, type Key } from "@/i18n";
import { errorKey } from "@/lib/fields";

const WEEKDAY_KEYS: readonly Key[] = [
  "book_weekday_0",
  "book_weekday_1",
  "book_weekday_2",
  "book_weekday_3",
  "book_weekday_4",
  "book_weekday_5",
  "book_weekday_6",
];

const BLANK_WEEK: readonly OpenRangeDto[][] = [[], [], [], [], [], [], []];

export function BookSettings() {
  return (
    <div className="flex flex-col gap-8">
      <WorkingHoursForm />
      <VisitTypesForm />
    </div>
  );
}

function WorkingHoursForm() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const workingHours = useQuery({
    queryKey: workingHoursQueryKey,
    queryFn: () => api.getWorkingHours(),
  });
  const [days, setDays] = useState<OpenRangeDto[][]>([...BLANK_WEEK.map((day) => [...day])]);
  const [loadedOnce, setLoadedOnce] = useState(false);

  useEffect(() => {
    if (workingHours.data !== undefined && !loadedOnce) {
      setDays((workingHours.data.days ?? BLANK_WEEK).map((day) => [...day]));
      setLoadedOnce(true);
    }
  }, [workingHours.data, loadedOnce]);

  const save = useMutation({
    mutationFn: () => api.setWorkingHours({ days }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: workingHoursQueryKey });
    },
  });

  const setRange = (weekday: number, index: number, field: "opens" | "closes", value: string) => {
    setDays((current) =>
      current.map((ranges, i) =>
        i !== weekday
          ? ranges
          : ranges.map((range, j) => (j !== index ? range : { ...range, [field]: value })),
      ),
    );
  };
  const addRange = (weekday: number) => {
    setDays((current) =>
      current.map((ranges, i) => (i !== weekday ? ranges : [...ranges, { opens: "08:00", closes: "12:00" }])),
    );
  };
  const removeRange = (weekday: number, index: number) => {
    setDays((current) =>
      current.map((ranges, i) => (i !== weekday ? ranges : ranges.filter((_, j) => j !== index))),
    );
  };

  return (
    <section className="flex flex-col gap-3" data-testid="working-hours-form">
      <h3 className="text-sm font-semibold text-foreground">{t("book_settings_hours_title")}</h3>
      {workingHours.isError ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(workingHours.error))}
        </p>
      ) : null}
      <div className="flex flex-col gap-2">
        {days.map((ranges, weekday) => (
          <div key={weekday} className="flex flex-wrap items-center gap-2">
            <span className="w-24 shrink-0 text-sm text-foreground">{t(WEEKDAY_KEYS[weekday])}</span>
            <div className="flex flex-wrap items-center gap-2">
              {ranges.map((range, index) => (
                <div key={index} className="flex items-center gap-1">
                  <Input
                    type="time"
                    data-testid={`hours-${weekday}-${index}-opens`}
                    className="w-28"
                    value={range.opens}
                    onChange={(event) => setRange(weekday, index, "opens", event.target.value)}
                  />
                  <span aria-hidden="true">-</span>
                  <Input
                    type="time"
                    data-testid={`hours-${weekday}-${index}-closes`}
                    className="w-28"
                    value={range.closes}
                    onChange={(event) => setRange(weekday, index, "closes", event.target.value)}
                  />
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => removeRange(weekday, index)}
                  >
                    {t("book_settings_remove_range")}
                  </Button>
                </div>
              ))}
              <Button
                type="button"
                variant="outline"
                size="sm"
                data-testid={`hours-${weekday}-add`}
                onClick={() => addRange(weekday)}
              >
                {t("book_settings_add_range")}
              </Button>
            </div>
          </div>
        ))}
      </div>
      {save.isError ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(save.error))}
        </p>
      ) : null}
      <div>
        <Button
          type="button"
          data-testid="hours-save"
          disabled={save.isPending}
          onClick={() => save.mutate()}
        >
          {t("book_settings_save")}
        </Button>
      </div>
    </section>
  );
}

function VisitTypesForm() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [name, setName] = useState("");
  const [minutes, setMinutes] = useState("15");

  const visitTypes = useQuery({ queryKey: visitTypesQueryKey, queryFn: () => api.listVisitTypes() });

  const invalidate = () => queryClient.invalidateQueries({ queryKey: visitTypesQueryKey });

  const create = useMutation({
    mutationFn: () => api.createVisitType({ name, minutes: Number(minutes) || 0 }),
    onSuccess: async () => {
      setName("");
      setMinutes("15");
      await invalidate();
    },
  });
  const remove = useMutation({
    mutationFn: (id: string) => api.removeVisitType(id),
    onSuccess: invalidate,
  });

  return (
    <section className="flex flex-col gap-3" data-testid="visit-types-form">
      <h3 className="text-sm font-semibold text-foreground">{t("book_settings_visit_types_title")}</h3>
      {visitTypes.isError ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(visitTypes.error))}
        </p>
      ) : null}
      <ul className="flex flex-col gap-1">
        {(visitTypes.data?.visit_types ?? []).map((type: VisitTypeDto) => (
          <li key={type.id} className="flex items-center justify-between gap-2 text-sm">
            <span>
              {type.name} — {type.minutes} {t("book_minutes_short")}
            </span>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              data-testid={`visit-type-remove-${type.id}`}
              disabled={remove.isPending}
              onClick={() => remove.mutate(type.id)}
            >
              {t("book_settings_remove")}
            </Button>
          </li>
        ))}
      </ul>

      <div className="flex flex-wrap items-end gap-2">
        <FormField label={t("book_settings_visit_type_name")}>
          {(parts) => (
            <Input
              {...parts}
              data-testid="visit-type-name"
              value={name}
              onChange={(event) => setName(event.target.value)}
            />
          )}
        </FormField>
        <FormField label={t("book_settings_visit_type_minutes")}>
          {(parts) => (
            <Input
              {...parts}
              type="number"
              data-testid="visit-type-minutes"
              value={minutes}
              onChange={(event) => setMinutes(event.target.value)}
            />
          )}
        </FormField>
        {create.isError ? (
          <p role="alert" className="text-sm text-fg-danger">
            {t(errorKey(create.error))}
          </p>
        ) : null}
        <Button
          type="button"
          data-testid="visit-type-create"
          disabled={create.isPending || name.trim() === ""}
          onClick={() => create.mutate()}
        >
          {t("book_settings_add")}
        </Button>
      </div>
    </section>
  );
}
