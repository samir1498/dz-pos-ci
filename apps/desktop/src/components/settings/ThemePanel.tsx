// The theme. The control is the same component the topbar carries, so the
// two cannot drift.

import { PanelHeading } from "@/components/settings/PanelHeading";
import { ThemeSwitcher } from "@/components/ThemeSwitcher";
import { Card, CardContent, CardDescription, CardHeader } from "@/components/ui/card";
import { useTranslation } from "@/i18n";

export function ThemePanel() {
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
