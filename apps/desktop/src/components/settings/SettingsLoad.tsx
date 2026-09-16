// The one wait and the one refusal the settings panels share.
//
// Two of the settings rooms (the shop block and the régime) read the same
// `GET /settings`, and each is its own route now. Written twice they would
// drift: one would grow a retry the other did not, or spell the refusal
// differently. TanStack Query dedupes the call itself, so this is only
// about the two lines of screen that surround it.

import { useQuery } from "@tanstack/react-query";
import type { ReactNode } from "react";
import type { SettingsDto } from "@dzpos/shared";

import { api, settingsQueryKey } from "@/api";
import { Skeleton } from "@/components/ui/skeleton";
import { useTranslation } from "@/i18n";
import { errorKey } from "@/lib/fields";

export function SettingsLoad({ children }: { children: (settings: SettingsDto) => ReactNode }) {
  const { t } = useTranslation();
  const settings = useQuery({ queryKey: settingsQueryKey, queryFn: () => api.getSettings() });

  if (settings.isPending) {
    return (
      <div className="flex flex-col gap-3" aria-busy="true">
        <p className="text-sm text-muted-foreground">{t("settings_loading")}</p>
        <Skeleton className="h-32 w-full" />
        <Skeleton className="h-32 w-full" />
      </div>
    );
  }
  if (settings.isError) {
    return (
      <p role="alert" className="text-sm text-fg-danger">
        {t(errorKey(settings.error))}
      </p>
    );
  }
  return <>{children(settings.data)}</>;
}
