// The settings screen: the store block a ticket prints as the seller, the
// dated régime fiscal, the appearance, and the two maintenance panels. Every
// block is a card, every control comes from the kit, and the API decides:
// this file shows and translates.

import { Link, createFileRoute } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useForm } from "@tanstack/react-form";
import { useState } from "react";
import { ApiError } from "@dzpos/shared";
import type { DatedRegimeDto, RegimeDto, SettingsDto, StoreDto } from "@dzpos/shared";
import { api, settingsQueryKey } from "@/api";
import { BackupsPanel } from "@/components/BackupsPanel";
import { ExportImportPanel } from "@/components/ExportImportPanel";
import { PageHeader } from "@/components/PageHeader";
import { StockRecountPanel } from "@/components/StockRecountPanel";
import { SupportBundlePanel } from "@/components/SupportBundlePanel";
import { ThemeSwitcher } from "@/components/ThemeSwitcher";
import { FormField } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { DateField } from "@/components/ui/date-field";
import { Card, CardContent, CardDescription, CardFooter, CardHeader } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { useShopToday } from "@/lib/clock";
import { useHasPermission } from "@/lib/session";
import { isKey, useTranslation, type Key } from "@/i18n";
import { errorKey } from "@/lib/fields";

export const Route = createFileRoute("/settings")({ component: SettingsScreen });

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
  // `GET /users`, and every route the staff panel's link leads to, already
  // name `ManageUsers` in gates.rs and refuse a cashier or a manager with a
  // 403 (services::permissions says the owner alone holds it). Same for
  // `ExportAndImport` on the four exports and the two import routes: the
  // panel below is only the hidden button; the server already refuses both.
  const manageUsers = useHasPermission("manage_users");
  const exportAndImport = useHasPermission("export_and_import");

  return (
    <section className="flex min-w-0 w-full max-w-full flex-col">
      <PageHeader title={t("settings_title")} description={t("settings_hint")} />
      {settings.isPending ? (
        <div className="flex flex-col gap-3" aria-busy="true">
          <p className="text-sm text-muted-foreground">{t("settings_loading")}</p>
          <Skeleton className="h-32 w-full" />
          <Skeleton className="h-32 w-full" />
        </div>
      ) : null}
      {settings.isError ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(settings.error))}
        </p>
      ) : null}
      {settings.isSuccess ? (
        <div className="flex min-w-0 w-full max-w-full flex-col gap-6">
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
            <Card>
              <CardContent className="flex flex-col items-start gap-3">
                <p role="alert" className="text-sm text-fg-danger">
                  {t(errorKey(clock.error))}
                </p>
                <Button type="button" variant="outline" onClick={clock.retry}>
                  {t("action_retry")}
                </Button>
              </CardContent>
            </Card>
          ) : clock.today === undefined ? (
            <p className="text-sm text-muted-foreground">{t("regime_loading")}</p>
          ) : (
            <RegimePanel
              current={settings.data.regime}
              planned={settings.data.regime_planned}
              today={clock.today}
            />
          )}
          <ThemePanel />
          {manageUsers ? <StaffPanel /> : null}
          <BackupsPanel />
          <SupportBundlePanel />
          {exportAndImport ? <ExportImportPanel /> : null}
          <StockRecountPanel />
          <AboutPanel />
        </div>
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

/**
 * The heading of a settings card. An `h3` because `PageHeader` owns the
 * page's `h2` and the shell owns the `h1`; the id is what the block's form
 * or section names itself by, so a test and a screen reader find the block
 * the same way.
 */
function PanelHeading({ id, children }: { id: string; children: string }) {
  return (
    <h3 id={id} className="font-semibold text-foreground">
      {children}
    </h3>
  );
}

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
    <Card className="min-w-0">
      <CardHeader>
        <PanelHeading id="settings-store">{t("settings_store")}</PanelHeading>
        <CardDescription>{t("settings_store_hint")}</CardDescription>
      </CardHeader>
      <form
        noValidate
        aria-labelledby="settings-store"
        className="flex min-w-0 flex-col gap-6"
        onSubmit={(e) => {
          e.preventDefault();
          onSaved(false);
          void form.handleSubmit();
        }}
      >
        <CardContent className="flex min-w-0 flex-col gap-4">
          <form.Field
            name="name"
            validators={{
              onSubmit: ({ value }) => (value.trim() === "" ? "error_name_required" : undefined),
            }}
          >
            {(field) => (
              <FormField
                label={t("field_name")}
                error={messageOf(field.state.meta.errors, t)}
                className="sm:max-w-md"
              >
                {(parts) => (
                  <Input
                    {...parts}
                    value={field.state.value}
                    onChange={(e) => field.handleChange(e.target.value)}
                    onBlur={field.handleBlur}
                  />
                )}
              </FormField>
            )}
          </form.Field>

          <div className="grid min-w-0 gap-4 sm:grid-cols-2">
            {STORE_FIELDS.map((spec) => (
              <form.Field key={spec.name} name={spec.name}>
                {(field) => (
                  <FormField label={t(spec.label)}>
                    {(parts) => (
                      <Input
                        {...parts}
                        dir={spec.ltr === true ? "ltr" : undefined}
                        value={field.state.value}
                        onChange={(e) => field.handleChange(e.target.value)}
                        onBlur={field.handleBlur}
                      />
                    )}
                  </FormField>
                )}
              </form.Field>
            ))}
          </div>
        </CardContent>

        <CardFooter className="flex flex-wrap items-center gap-3">
          <Button type="submit" disabled={save.isPending}>
            {save.isPending ? t("action_saving") : t("action_save")}
          </Button>
          {serverError !== null ? (
            <p role="alert" className="text-sm text-fg-danger">
              {t(serverError)}
            </p>
          ) : null}
          {saved && serverError === null ? (
            <p role="status" className="text-sm text-fg-success">
              {t("settings_saved")}
            </p>
          ) : null}
        </CardFooter>
      </form>
    </Card>
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

/**
 * The theme, beside the store block rather than in a preferences screen of
 * its own: there is one settings page and this is a setting. The control is
 * the same component the topbar carries, so the two cannot drift.
 */
function ThemePanel() {
  const { t } = useTranslation();
  return (
    <section aria-labelledby="settings-theme">
      <Card>
        <CardHeader>
          <PanelHeading id="settings-theme">{t("settings_theme")}</PanelHeading>
          <CardDescription>{t("settings_theme_hint")}</CardDescription>
        </CardHeader>
        <CardContent>
          <ThemeSwitcher />
        </CardContent>
      </Card>
    </section>
  );
}

/**
 * The link out to the users screen (M4 T8), not the list itself: the fiches
 * and their roles are their own screen because a table, an "add a user"
 * dialog and a PIN reset dialog are a page's worth, not a card's. `crate::
 * gates` names `ManageUsers` on every route behind that screen, so the
 * owner alone holds it; `SettingsScreen` hides this whole panel from a
 * manager and a cashier for that reason (M4 T5), and typing `/settings/users`
 * by hand still meets the server's own refusal there — the panel is the
 * hidden button, `gates.rs` is the defence.
 */
function StaffPanel() {
  const { t } = useTranslation();
  return (
    <section aria-labelledby="settings-users">
      <Card>
        <CardHeader>
          <PanelHeading id="settings-users">{t("settings_users")}</PanelHeading>
          <CardDescription>{t("settings_users_hint")}</CardDescription>
        </CardHeader>
        <CardFooter>
          <Button variant="outline" asChild>
            <Link to="/settings/users">{t("users_manage_link")}</Link>
          </Button>
        </CardFooter>
      </Card>
    </section>
  );
}

/**
 * The link out to the About screen (M5 T1), the same shape `StaffPanel`
 * uses for the users screen: the version, the git hash and the build date
 * are their own page because a screen reader announcing them belongs on a
 * heading of its own, not folded into a settings card's description.
 */
function AboutPanel() {
  const { t } = useTranslation();
  return (
    <section aria-labelledby="settings-about">
      <Card>
        <CardHeader>
          <PanelHeading id="settings-about">{t("settings_about")}</PanelHeading>
          <CardDescription>{t("settings_about_hint")}</CardDescription>
        </CardHeader>
        <CardFooter>
          <Button variant="outline" asChild>
            <Link to="/settings/about">{t("about_open_link")}</Link>
          </Button>
        </CardFooter>
      </Card>
    </section>
  );
}

/** Field validators return translation keys, never sentences. */
function messageOf(messages: unknown[], t: (key: Key) => string): string | undefined {
  const key = messages.find((m): m is string => typeof m === "string");
  if (key === undefined) return undefined;
  return t(isKey(key) ? key : "error_unknown");
}
