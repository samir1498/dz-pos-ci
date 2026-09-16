// The store block a ticket prints as the seller.
//
// Seven fields and one save: the seller block is what the fiscal templates
// read, and a half-saved seller is a facture that names the shop wrongly.
// Four of the fields are identifiers read left to right with Western digits
// whatever the screen's language, which is the same decision the amounts and
// the barcodes carry.

import { useForm } from "@tanstack/react-form";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import type { SettingsDto, StoreDto } from "@dzpos/shared";
import { api, settingsQueryKey } from "@/api";
import { FormField } from "@/components/FormField";
import { messageOf, PanelHeading } from "@/components/settings/PanelHeading";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardFooter, CardHeader } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { useTranslation, type Key } from "@/i18n";
import { errorKey } from "@/lib/fields";

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

export function StoreForm({
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
