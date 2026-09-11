// The About screen (M5 T1, docs/architecture.md § Release): the version,
// the git short hash and the build date, read from `GET /build-info`, which
// itself reads `crates/core::build_info::BUILD_INFO`, the one place all
// three come from. This file shows and translates; it holds no copy of any
// of the three itself.
//
// Same trailing-underscore file name `settings_.users.tsx` uses, and the
// same reason: `/settings` is its own whole page, not a layout this one
// nests inside.

import { useQuery } from "@tanstack/react-query";
import { Link, createFileRoute } from "@tanstack/react-router";
import { ArrowLeft } from "lucide-react";

import { api, buildInfoQueryKey } from "@/api";
import { Icon } from "@/components/Icon";
import { PageHeader } from "@/components/PageHeader";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { useTranslation } from "@/i18n";
import { errorKey } from "@/lib/fields";

export const Route = createFileRoute("/settings_/about")({ component: AboutScreen });

function BackToSettings() {
  const { t } = useTranslation();
  return (
    <Button variant="ghost" asChild>
      <Link to="/settings">
        <Icon as={ArrowLeft} size={18} flip />
        {t("action_back_to_settings")}
      </Link>
    </Button>
  );
}

export function AboutScreen() {
  const { t } = useTranslation();
  const info = useQuery({ queryKey: buildInfoQueryKey, queryFn: () => api.getBuildInfo() });

  return (
    <section className="flex flex-col gap-4">
      <PageHeader title={t("about_title")} description={t("about_hint")} actions={<BackToSettings />} />

      {info.isPending ? (
        <div className="flex flex-col gap-3" aria-busy="true">
          <p className="text-sm text-muted-foreground">{t("about_loading")}</p>
          <Skeleton className="h-32 w-full" />
        </div>
      ) : null}

      {info.isError ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(info.error))}
        </p>
      ) : null}

      {info.isSuccess ? (
        <Card>
          <CardContent className="flex flex-col gap-4">
            {info.data.debug ? (
              <Badge data-testid="about-debug-badge" variant="outline">
                {t("about_debug_badge")}
              </Badge>
            ) : null}
            <dl className="grid grid-cols-[auto_1fr] items-baseline gap-x-4 gap-y-2 rounded-lg bg-muted p-4">
              <dt className="text-sm text-muted-foreground">{t("field_version")}</dt>
              <dd data-testid="about-version" className="font-numeric tabular-nums text-foreground" dir="ltr">
                {info.data.version}
              </dd>
              <dt className="text-sm text-muted-foreground">{t("field_git_hash")}</dt>
              <dd data-testid="about-git-hash" className="font-numeric tabular-nums text-foreground" dir="ltr">
                {info.data.git_hash}
              </dd>
              <dt className="text-sm text-muted-foreground">{t("field_build_date")}</dt>
              <dd data-testid="about-build-date" className="font-numeric tabular-nums text-foreground" dir="ltr">
                {info.data.build_date}
              </dd>
            </dl>
          </CardContent>
        </Card>
      ) : null}
    </section>
  );
}
