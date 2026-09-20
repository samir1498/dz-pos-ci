// The language every fiscal paper prints in: the ticket, the facture, the
// customer statement and the debt slip alike (one setting, not a knob per
// document -- context/plans/20260920-a-print-language-the-shop-keeps.md).
//
// Independent of the screen's own language (`@/i18n`'s `Lang`). The six
// document routes read it: the shop's stored choice beats the language a
// screen happens to be open in, and a `print_lang` named on one call beats
// the stored choice in turn.
//
// `null` is not "nothing chosen and refused": it is the shop asking to
// follow the till, which is today's behaviour and stays the default. Radix's
// `Select` cannot carry `null` as an item value, so a sentinel stands in for
// it here and is translated back at the boundary.

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import type { PrintLangDto, SettingsDto } from "@dzpos/shared";

import { api, settingsQueryKey } from "@/api";
import { PanelHeading } from "@/components/settings/PanelHeading";
import { Card, CardContent, CardDescription, CardHeader } from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useTranslation, type Key } from "@/i18n";
import { errorKey } from "@/lib/fields";

/** Stands in for `null` on the wire: the item value Radix shows for "the
 *  shop has not chosen", which is not itself a `PrintLangDto`. */
const FOLLOW_TILL = "follow-till";

const CHOICES = [FOLLOW_TILL, "fr", "en", "ar"] as const;
type Choice = (typeof CHOICES)[number];

const LANG_LABEL: Record<PrintLangDto, Key> = {
  fr: "lang_name_fr",
  en: "lang_name_en",
  ar: "lang_name_ar",
};

function toChoice(lang: PrintLangDto | null): Choice {
  return lang ?? FOLLOW_TILL;
}

function toLang(choice: Choice): PrintLangDto | null {
  return choice === FOLLOW_TILL ? null : choice;
}

export function PrintLanguagePanel({ settings }: { settings: SettingsDto }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<Key | null>(null);

  const choose = useMutation({
    mutationFn: (lang: PrintLangDto | null) => api.setPrintLang(lang),
    onSuccess: async (next) => {
      setServerError(null);
      queryClient.setQueryData<SettingsDto>(settingsQueryKey, next);
      await queryClient.invalidateQueries({ queryKey: settingsQueryKey });
    },
    onError: (error: unknown) => setServerError(errorKey(error)),
  });

  return (
    <section aria-labelledby="settings-print-lang">
      <Card>
        <CardHeader>
          <PanelHeading id="settings-print-lang">{t("settings_print_lang")}</PanelHeading>
          <CardDescription>{t("settings_print_lang_hint")}</CardDescription>
        </CardHeader>
        <CardContent className="flex min-w-0 flex-col gap-3">
          <Select
            value={toChoice(settings.print_lang)}
            disabled={choose.isPending}
            onValueChange={(value) => {
              // Never guessed. A value this build did not offer is ignored
              // rather than sent on: what a facture is printed in is not
              // something to resolve by falling back (the same shape
              // `FactureLayoutPanel`'s own `onValueChange` uses).
              const picked = CHOICES.find((choice) => choice === value);
              if (picked === undefined) return;
              choose.mutate(toLang(picked));
            }}
          >
            <SelectTrigger aria-label={t("settings_print_lang")} className="sm:w-80">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={FOLLOW_TILL}>{t("print_lang_follow_till")}</SelectItem>
              {(["fr", "en", "ar"] as const).map((lang) => (
                <SelectItem key={lang} value={lang}>
                  {t(LANG_LABEL[lang])}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          {serverError !== null ? (
            <p role="alert" className="text-sm text-fg-danger">
              {t(serverError)}
            </p>
          ) : null}
        </CardContent>
      </Card>
    </section>
  );
}
