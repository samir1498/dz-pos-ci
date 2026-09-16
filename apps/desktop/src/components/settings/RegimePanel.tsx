// The régime fiscal, and a change dated ahead.
//
// A régime is a yearly election and the day it takes effect is a calendar
// day on the shop's own clock, never the machine's — so the form waits for
// the server to say which day it is rather than reading the browser's.

import { useForm } from "@tanstack/react-form";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import type { DatedRegimeDto, RegimeDto, SettingsDto } from "@dzpos/shared";
import { api, settingsQueryKey } from "@/api";
import { FormField } from "@/components/FormField";
import { messageOf, PanelHeading } from "@/components/settings/PanelHeading";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardFooter, CardHeader } from "@/components/ui/card";
import { DateField } from "@/components/ui/date-field";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useTranslation, type Key } from "@/i18n";
import { errorKey } from "@/lib/fields";

const REGIMES: readonly RegimeDto[] = ["ifu", "reel"];

const REGIME_KEY: Record<RegimeDto, Key> = {
  ifu: "regime_ifu",
  reel: "regime_reel",
};

/** `YYYY-MM-DD`, the only shape the API takes a day in. */
const DAY = /^\d{4}-\d{2}-\d{2}$/;

/** A fiscal value is never guessed: an unknown option blocks the submit. */
function toRegime(value: string): RegimeDto | undefined {
  return REGIMES.find((r) => r === value);
}

export function RegimePanel({
  current,
  planned,
  today,
}: {
  current: DatedRegimeDto;
  planned: DatedRegimeDto | null;
  today: string;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<Key | null>(null);

  const change = useMutation({
    mutationFn: (input: { regime: RegimeDto; valid_from: string }) => api.changeRegime(input),
    onSuccess: async (settings) => {
      setServerError(null);
      // The answer is the whole page: current or planned depends on the
      // day, and the server said which.
      queryClient.setQueryData<SettingsDto>(settingsQueryKey, settings);
      await queryClient.invalidateQueries({ queryKey: settingsQueryKey });
    },
    onError: (error: unknown) => setServerError(errorKey(error)),
  });

  const form = useForm({
    defaultValues: { regime: String(current.regime), validFrom: today },
    onSubmit: async ({ value }) => {
      const regime = toRegime(value.regime);
      if (regime === undefined) {
        setServerError("error_validation");
        return;
      }
      await change.mutateAsync({ regime, valid_from: value.validFrom }).catch(() => undefined);
    },
  });

  return (
    <Card>
      <CardHeader>
        <PanelHeading id="settings-regime">{t("settings_regime")}</PanelHeading>
        <CardDescription>{t("regime_change_hint")}</CardDescription>
      </CardHeader>
      <form
        noValidate
        aria-labelledby="settings-regime"
        className="flex min-w-0 flex-col gap-6"
        onSubmit={(e) => {
          e.preventDefault();
          void form.handleSubmit();
        }}
      >
        <CardContent className="flex min-w-0 flex-col gap-4">
          <dl className="grid min-w-0 grid-cols-[auto_1fr] items-baseline gap-x-4 gap-y-1 rounded-lg bg-muted p-4">
            <dt className="text-sm text-muted-foreground">{t("regime_current_label")}</dt>
            <dd data-testid="regime-current" className="min-w-0 font-medium break-words text-foreground">
              {t(REGIME_KEY[current.regime])} · {t("regime_since")}{" "}
              <span dir="ltr" className="font-numeric tabular-nums">
                {current.valid_from}
              </span>
            </dd>
            {planned !== null ? (
              <>
                <dt className="text-sm text-muted-foreground">{t("regime_planned_label")}</dt>
                <dd data-testid="regime-planned" className="min-w-0 font-medium break-words text-foreground">
                  {t(REGIME_KEY[planned.regime])} · {t("regime_from")}{" "}
                  <span dir="ltr" className="font-numeric tabular-nums">
                    {planned.valid_from}
                  </span>
                </dd>
              </>
            ) : null}
          </dl>

          <div className="grid min-w-0 gap-4 sm:grid-cols-2">
            <form.Field name="regime">
              {(field) => (
                <FormField label={t("field_regime")}>
                  {(parts) => (
                    <Select
                      value={field.state.value}
                      onValueChange={(next) => field.handleChange(next)}
                    >
                      <SelectTrigger
                        id={parts.id}
                        aria-describedby={parts["aria-describedby"]}
                        className="w-full"
                      >
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {REGIMES.map((r) => (
                          <SelectItem key={r} value={r}>
                            {t(REGIME_KEY[r])}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  )}
                </FormField>
              )}
            </form.Field>
            <form.Field
              name="validFrom"
              validators={{
                onSubmit: ({ value }) => (DAY.test(value) ? undefined : "error_day_invalid"),
              }}
            >
              {(field) => (
                <FormField
                  label={t("field_valid_from")}
                  error={messageOf(field.state.meta.errors, t)}
                >
                  {(parts) => (
                    <DateField
                      {...parts}
                      data-testid="regime-valid-from"
                      value={field.state.value}
                      onChange={field.handleChange}
                      onBlur={field.handleBlur}
                    />
                  )}
                </FormField>
              )}
            </form.Field>
          </div>
        </CardContent>

        <CardFooter className="flex flex-wrap items-center gap-3">
          <form.Subscribe selector={(state) => state.values.regime}>
            {(regime) => (
              <Button
                type="submit"
                // Applying the régime already in force would only move its
                // "since" date to today (the API refuses it too). With a
                // change planned ahead, re-applying the current régime is
                // how that plan is cancelled, so the button stays live.
                disabled={change.isPending || (regime === current.regime && planned === null)}
              >
                {change.isPending ? t("action_saving") : t("action_apply")}
              </Button>
            )}
          </form.Subscribe>
          {serverError !== null ? (
            <p role="alert" className="text-sm text-fg-danger">
              {t(serverError)}
            </p>
          ) : null}
        </CardFooter>
      </form>
    </Card>
  );
}
