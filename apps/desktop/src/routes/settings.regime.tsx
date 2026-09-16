// The régime fiscal, and a change dated ahead.

import { createFileRoute } from "@tanstack/react-router";

import { RegimePanel } from "@/components/settings/RegimePanel";
import { SettingsLoad } from "@/components/settings/SettingsLoad";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { useTranslation } from "@/i18n";
import { useShopToday } from "@/lib/clock";
import { errorKey } from "@/lib/fields";

export const Route = createFileRoute("/settings/regime")({ component: RegimeRoom });

export function RegimeRoom() {
  const { t } = useTranslation();
  // The day a régime change defaults to belongs to the shop's calendar, not
  // to the machine's: the core reads a dated setting on Algeria's (§2, "One
  // clock"), so the form waits for the server to say which day it is.
  const clock = useShopToday();

  // The day is a call like any other and it can be refused. Said here rather
  // than swallowed into the wait below: a panel that showed "loading" for a
  // refusal would never come back on its own and would never say why.
  if (clock.error !== null) {
    return (
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
    );
  }
  if (clock.today === undefined) {
    return <p className="text-sm text-muted-foreground">{t("regime_loading")}</p>;
  }

  const today = clock.today;
  return (
    <SettingsLoad>
      {(settings) => (
        <RegimePanel
          current={settings.regime}
          planned={settings.regime_planned}
          today={today}
        />
      )}
    </SettingsLoad>
  );
}
