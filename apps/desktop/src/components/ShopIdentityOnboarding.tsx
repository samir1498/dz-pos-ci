// The step after `FirstSetupScreen` that asks for the shop's own identity
// (T24/T38): name, address, phone, RC, NIF, NIS, AI. The owner just typed a
// name and a password, nothing about the shop itself, and the first ticket
// used to print "Mon magasin" with no address or tax id. Optional and
// skippable (Samir's decision, T38): the till keeps selling without it, and
// the same seven fields stay reachable later from Paramètres → Magasin
// (`routes/settings.shop.tsx`), which is why this reuses its form rather
// than forking one. Shown instead of the shell, not behind it: mounting the
// shell here would raise the till's "Ouverture de la caisse" popup under a
// screen the owner has not asked to skip yet.

import { SettingsLoad } from "@/components/settings/SettingsLoad";
import { StoreForm } from "@/components/settings/StoreForm";
import { Button } from "@/components/ui/button";
import { Card, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Wordmark } from "@/components/Wordmark";
import { useTranslation } from "@/i18n";
import { useSession } from "@/lib/session";

export function ShopIdentityOnboarding() {
  const { t } = useTranslation();
  const { dismissOnboarding } = useSession();

  return (
    <div
      data-testid="onboarding-shop-screen"
      className="flex min-h-screen items-center justify-center bg-background p-4"
    >
      <div className="flex w-full max-w-lg flex-col gap-4">
        <Card>
          <CardHeader className="items-center text-center">
            <Wordmark className="mb-2" />
            <CardTitle>{t("onboarding_shop_title")}</CardTitle>
            <CardDescription>{t("onboarding_shop_hint")}</CardDescription>
          </CardHeader>
        </Card>
        <SettingsLoad>
          {(settings) => (
            <StoreForm
              key={JSON.stringify(settings.store)}
              initial={settings.store}
              saved={false}
              onSaved={(saved) => {
                if (saved) dismissOnboarding();
              }}
            />
          )}
        </SettingsLoad>
        <Button
          type="button"
          variant="ghost"
          data-testid="onboarding-shop-skip"
          className="w-full"
          onClick={dismissOnboarding}
        >
          {t("onboarding_shop_skip")}
        </Button>
      </div>
    </div>
  );
}
