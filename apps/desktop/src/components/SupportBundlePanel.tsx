// The support bundle block on the settings screen (M5 T3). One button, one
// call, one file saved: `dzpos_core::services::support_bundle`'s own doc
// names everything the zip carries and everything it refuses to, and this
// file adds none of its own logic to that list. Not gated client-side, the
// same as the backups block beside it: `/support-bundle` is refused server
// side to anyone without `edit_settings` (`crates/api/src/gates.rs`), and a
// cashier or manager who cannot reach it sees the refusal `errorKey` reads
// rather than a hidden button.

import { useMutation } from "@tanstack/react-query";
import { api } from "@/api";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader } from "@/components/ui/card";
import { saveBlob } from "@/lib/download";
import { useTranslation } from "@/i18n";
import { errorKey } from "@/lib/fields";

export function SupportBundlePanel() {
  const { t } = useTranslation();

  const download = useMutation({
    mutationFn: () => api.supportBundle(),
    onSuccess: (got) => saveBlob(got.blob, got.filename),
  });

  return (
    <section aria-labelledby="settings-support-bundle">
      <Card>
        <CardHeader>
          <h3 id="settings-support-bundle" className="font-semibold text-foreground">
            {t("settings_support_bundle")}
          </h3>
          <CardDescription>{t("settings_support_bundle_hint")}</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          <div>
            <Button
              variant="outline"
              data-testid="support-bundle-download"
              disabled={download.isPending}
              onClick={() => download.mutate()}
            >
              {download.isPending
                ? t("action_support_bundle_pending")
                : t("action_support_bundle")}
            </Button>
          </div>
          {download.isError ? (
            <p role="alert" className="text-sm text-fg-danger">
              {t(errorKey(download.error))}
            </p>
          ) : null}
          {download.isSuccess ? (
            <p role="status" data-testid="support-bundle-done" className="text-sm text-fg-success">
              {t("support_bundle_done")}
            </p>
          ) : null}
        </CardContent>
      </Card>
    </section>
  );
}
