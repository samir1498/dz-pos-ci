// The settings screen: the store block a ticket prints as the seller and
// the dated régime fiscal. Two forms, two routes, the same rules as every
// screen: the API decides, this file shows and translates.

import { createFileRoute } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useForm } from "@tanstack/react-form";
import { useState } from "react";
import { ApiError } from "@dzpos/shared";
import type { DatedRegimeDto, RegimeDto, SettingsDto, StoreDto } from "@dzpos/shared";
import { api, settingsQueryKey } from "@/api";
import { BackupsPanel } from "@/components/BackupsPanel";
import { useShopToday } from "@/lib/clock";
import { isKey, useTranslation, type Key } from "@/i18n";

export const Route = createFileRoute("/settings")({ component: SettingsScreen });

const REGIMES: readonly RegimeDto[] = ["ifu", "reel"];

const REGIME_KEY: Record<RegimeDto, Key> = {
  ifu: "regime_ifu",
  reel: "regime_reel",
};

const ERROR_KEY: Record<string, Key> = {
  validation: "error_validation",
  not_found: "error_not_found",
  storage: "error_storage",
  restart_needed: "error_restart_needed",
  bad_request: "error_bad_request",
  bad_response: "error_bad_response",
  unauthorized: "error_unauthorized",
  unreachable: "error_unreachable",
};

function errorKey(error: unknown): Key {
  if (error instanceof ApiError) return ERROR_KEY[error.code] ?? "error_unknown";
  return "error_unknown";
}

/** `YYYY-MM-DD`, the only shape the API takes a day in. */
const DAY = /^\d{4}-\d{2}-\d{2}$/;

/** A fiscal value is never guessed: an unknown option blocks the submit. */
function toRegime(value: string): RegimeDto | undefined {
  return REGIMES.find((r) => r === value);
}

export function SettingsScreen() {
  const { t } = useTranslation();
  const settings = useQuery({ queryKey: settingsQueryKey, queryFn: () => api.getSettings() });
  // The day a régime change defaults to belongs to the shop's calendar, not
  // to the machine's: the core reads a dated setting on Algeria's (§2, "One
  // clock"), so the form waits for the server to say which day it is.
  const clock = useShopToday();
  // Lives here, not in the form: a save refetches the page and the form is
  // remounted on the fresh block (its key), which would drop the message.
  const [storeSaved, setStoreSaved] = useState(false);

  return (
    <section className="flex flex-col gap-6">
      <h1 className="text-xl font-semibold">{t("settings_title")}</h1>
      {settings.isPending ? <p>{t("settings_loading")}</p> : null}
      {settings.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(settings.error))}
        </p>
      ) : null}
      {settings.isSuccess ? (
        <>
          <StoreForm
            key={JSON.stringify(settings.data.store)}
            initial={settings.data.store}
            saved={storeSaved}
            onSaved={setStoreSaved}
          />
          {clock.error !== null ? (
            // The day is a call like any other and it can be refused. Said
            // here rather than swallowed into the wait above: a panel that
            // showed "loading" for a refusal would never come back on its
            // own and would never say why.
            <div className="flex flex-col items-start gap-2">
              <p role="alert" className="text-red-700">
                {t(errorKey(clock.error))}
              </p>
              <button type="button" className="rounded border px-3 py-1.5" onClick={clock.retry}>
                {t("action_retry")}
              </button>
            </div>
          ) : clock.today === undefined ? (
            <p>{t("regime_loading")}</p>
          ) : (
            <RegimePanel
              current={settings.data.regime}
              planned={settings.data.regime_planned}
              today={clock.today}
            />
          )}
          <BackupsPanel />
        </>
      ) : null}
    </section>
  );
}

/**
 * The seven fields of the seller block, sent whole; a blank one is null.
 * `ltr` marks the four that are fiscal identifiers or a phone number,
 * read left to right with Western digits regardless of the screen's
 * language, the same decision as the amounts and barcodes on the
 * products screen. `address` is free text and stays with the page's own
 * direction.
 */
const STORE_FIELDS: readonly { name: keyof Omit<StoreDto, "name">; label: Key; ltr?: true }[] = [
  { name: "rc", label: "field_rc", ltr: true },
  { name: "nif", label: "field_nif", ltr: true },
  { name: "nis", label: "field_nis", ltr: true },
  { name: "ai", label: "field_ai", ltr: true },
  { name: "address", label: "field_address" },
  { name: "phone", label: "field_phone", ltr: true },
];

function StoreForm({
  initial,
  saved,
  onSaved,
}: {
  initial: StoreDto;
  saved: boolean;
  onSaved: (saved: boolean) => void;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<Key | null>(null);

  const save = useMutation({
    mutationFn: (input: StoreDto) => api.updateStore(input),
    onSuccess: async (store) => {
      setServerError(null);
      onSaved(true);
      queryClient.setQueryData<SettingsDto>(settingsQueryKey, (old) =>
        old === undefined ? old : { ...old, store },
      );
      await queryClient.invalidateQueries({ queryKey: settingsQueryKey });
    },
    onError: (error: unknown) => {
      onSaved(false);
      setServerError(errorKey(error));
    },
  });

  const form = useForm({
    defaultValues: {
      name: initial.name,
      rc: initial.rc ?? "",
      nif: initial.nif ?? "",
      nis: initial.nis ?? "",
      ai: initial.ai ?? "",
      address: initial.address ?? "",
      phone: initial.phone ?? "",
    },
    onSubmit: async ({ value }) => {
      const blank = (text: string) => (text.trim() === "" ? null : text.trim());
      await save
        .mutateAsync({
          name: value.name.trim(),
          rc: blank(value.rc),
          nif: blank(value.nif),
          nis: blank(value.nis),
          ai: blank(value.ai),
          address: blank(value.address),
          phone: blank(value.phone),
        })
        .catch(() => undefined);
    },
  });

  return (
    <form
      noValidate
      aria-labelledby="settings-store"
      className="flex flex-col gap-3 rounded border p-4"
      onSubmit={(e) => {
        e.preventDefault();
        onSaved(false);
        void form.handleSubmit();
      }}
    >
      <h2 id="settings-store" className="font-semibold">
        {t("settings_store")}
      </h2>
      <p className="text-sm opacity-80">{t("settings_store_hint")}</p>

      <form.Field
        name="name"
        validators={{
          onSubmit: ({ value }) => (value.trim() === "" ? "error_name_required" : undefined),
        }}
      >
        {(field) => (
          <label className="flex flex-col gap-1">
            <span>{t("field_name")}</span>
            <input
              className="rounded border px-2 py-1"
              value={field.state.value}
              onChange={(e) => field.handleChange(e.target.value)}
              onBlur={field.handleBlur}
            />
            <FieldError messages={field.state.meta.errors} />
          </label>
        )}
      </form.Field>

      <div className="grid gap-3 sm:grid-cols-2">
        {STORE_FIELDS.map((spec) => (
          <form.Field key={spec.name} name={spec.name}>
            {(field) => (
              <label className="flex flex-col gap-1">
                <span>{t(spec.label)}</span>
                <input
                  dir={spec.ltr === true ? "ltr" : undefined}
                  className="rounded border px-2 py-1"
                  value={field.state.value}
                  onChange={(e) => field.handleChange(e.target.value)}
                  onBlur={field.handleBlur}
                />
              </label>
            )}
          </form.Field>
        ))}
      </div>

      {serverError !== null ? (
        <p role="alert" className="text-red-700">
          {t(serverError)}
        </p>
      ) : null}
      {saved && serverError === null ? <p role="status">{t("settings_saved")}</p> : null}

      <div>
        <button
          type="submit"
          className="rounded border px-3 py-1.5"
          disabled={save.isPending}
        >
          {save.isPending ? t("action_saving") : t("action_save")}
        </button>
      </div>
    </form>
  );
}

function RegimePanel({
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
      await change
        .mutateAsync({ regime, valid_from: value.validFrom })
        .catch(() => undefined);
    },
  });

  return (
    <form
      noValidate
      aria-labelledby="settings-regime"
      className="flex flex-col gap-3 rounded border p-4"
      onSubmit={(e) => {
        e.preventDefault();
        void form.handleSubmit();
      }}
    >
      <h2 id="settings-regime" className="font-semibold">
        {t("settings_regime")}
      </h2>
      <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1">
        <dt>{t("regime_current_label")}</dt>
        <dd data-testid="regime-current">
          {t(REGIME_KEY[current.regime])} · {t("regime_since")}{" "}
          <span dir="ltr">{current.valid_from}</span>
        </dd>
        {planned !== null ? (
          <>
            <dt>{t("regime_planned_label")}</dt>
            <dd data-testid="regime-planned">
              {t(REGIME_KEY[planned.regime])} · {t("regime_from")}{" "}
              <span dir="ltr">{planned.valid_from}</span>
            </dd>
          </>
        ) : null}
      </dl>
      <p className="text-sm opacity-80">{t("regime_change_hint")}</p>

      <div className="grid gap-3 sm:grid-cols-2">
        <form.Field name="regime">
          {(field) => (
            <label className="flex flex-col gap-1">
              <span>{t("field_regime")}</span>
              <select
                className="rounded border px-2 py-1"
                value={field.state.value}
                onChange={(e) => field.handleChange(e.target.value)}
              >
                {REGIMES.map((r) => (
                  <option key={r} value={r}>
                    {t(REGIME_KEY[r])}
                  </option>
                ))}
              </select>
            </label>
          )}
        </form.Field>
        <form.Field
          name="validFrom"
          validators={{
            onSubmit: ({ value }) => (DAY.test(value) ? undefined : "error_day_invalid"),
          }}
        >
          {(field) => (
            <label className="flex flex-col gap-1">
              <span>{t("field_valid_from")}</span>
              <input
                type="date"
                className="rounded border px-2 py-1"
                value={field.state.value}
                onChange={(e) => field.handleChange(e.target.value)}
                onBlur={field.handleBlur}
              />
              <FieldError messages={field.state.meta.errors} />
            </label>
          )}
        </form.Field>
      </div>

      {serverError !== null ? (
        <p role="alert" className="text-red-700">
          {t(serverError)}
        </p>
      ) : null}

      <form.Subscribe selector={(state) => state.values.regime}>
        {(regime) => (
          <div>
            <button
              type="submit"
              className="rounded border px-3 py-1.5 disabled:opacity-50"
              // Applying the régime already in force would only move its
              // "since" date to today (the API refuses it too). With a
              // change planned ahead, re-applying the current régime is
              // how that plan is cancelled, so the button stays live.
              disabled={change.isPending || (regime === current.regime && planned === null)}
            >
              {change.isPending ? t("action_saving") : t("action_apply")}
            </button>
          </div>
        )}
      </form.Subscribe>
    </form>
  );
}

/** Field validators return translation keys, never sentences. */
function FieldError({ messages }: { messages: unknown[] }) {
  const { t } = useTranslation();
  const key = messages.find((m): m is string => typeof m === "string");
  if (key === undefined) return null;
  return (
    <span role="alert" className="text-sm text-red-700">
      {t(isKey(key) ? key : "error_unknown")}
    </span>
  );
}
