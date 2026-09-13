// The About screen (M5 T1, docs/architecture.md § Release): the version,
// the git short hash and the build date, read from `GET /build-info`, which
// itself reads `crates/core::build_info::BUILD_INFO`, the one place all
// three come from. This file shows and translates; it holds no copy of any
// of the three itself.
//
// Same trailing-underscore file name `settings_.users.tsx` uses, and the
// same reason: `/settings` is its own whole page, not a layout this one
// nests inside.
//
// The update check (M5 T7) lives on this screen and nowhere else, and only
// ever runs because the button below was pressed: a shop on a phone
// connection did not agree to thirty megabytes moving on their own, and
// the check is never wired to a timer, a mount effect or an interval.
// `src/lib/updater.ts` is the only file that knows a Tauri command answers
// it; this file reads a plain three-answer value.

import { useMutation, useQuery } from "@tanstack/react-query";
import { Link, createFileRoute } from "@tanstack/react-router";
import { ArrowLeft } from "lucide-react";
import { useState } from "react";

import { api, buildInfoQueryKey } from "@/api";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { Icon } from "@/components/Icon";
import { PageHeader } from "@/components/PageHeader";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { Skeleton } from "@/components/ui/skeleton";
import { useTranslation } from "@/i18n";
import { errorKey } from "@/lib/fields";
import { checkForUpdate, formatUpdateSize, installUpdate } from "@/lib/updater";

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

/** The three answers named in docs/architecture.md § Release, plus the
 * install step the "newer" answer leads to. Rendered below the build-info
 * `<dl>` in the same card, but a separate component from `AboutScreen` so
 * that `<dl>` keeps reading only `GET /build-info`, the boundary
 * `settings_.about.test.tsx` already relies on. */
function UpdateCheckPanel() {
  const { t } = useTranslation();
  const [confirmOpen, setConfirmOpen] = useState(false);

  const check = useMutation({ mutationFn: checkForUpdate });
  const install = useMutation({
    mutationFn: installUpdate,
    onSuccess: () => setConfirmOpen(false),
  });

  const answer = check.data;

  return (
    <div className="flex flex-col gap-3">
      <Separator />

      <Button
        type="button"
        variant="outline"
        className="self-start"
        data-testid="about-update-check"
        disabled={check.isPending}
        onClick={() => {
          install.reset();
          check.mutate();
        }}
      >
        {check.isPending ? t("action_checking") : t("about_update_check")}
      </Button>

      {answer?.kind === "newest" ? (
        <p data-testid="about-update-newest" className="text-sm text-muted-foreground">
          {t("about_update_newest")}
        </p>
      ) : null}

      {answer?.kind === "unreachable" ? (
        <p role="alert" data-testid="about-update-unreachable" className="text-sm text-fg-danger">
          {t("about_update_unreachable")}
        </p>
      ) : null}

      {answer?.kind === "newer" ? (
        <div
          data-testid="about-update-available"
          className="flex flex-col items-start gap-3 rounded-lg bg-muted p-4"
        >
          <p className="text-sm text-foreground">{t("about_update_available")}</p>
          <dl className="grid grid-cols-[auto_1fr] items-baseline gap-x-4 gap-y-2">
            <dt className="text-sm text-muted-foreground">{t("field_version")}</dt>
            <dd data-testid="about-update-version" className="font-numeric tabular-nums text-foreground" dir="ltr">
              {answer.version}
            </dd>
            <dt className="text-sm text-muted-foreground">{t("about_update_size")}</dt>
            <dd data-testid="about-update-size" className="font-numeric tabular-nums text-foreground" dir="ltr">
              {answer.size === null ? t("about_update_size_unknown") : formatUpdateSize(answer.size)}
            </dd>
          </dl>
          <Button type="button" data-testid="about-update-install" onClick={() => setConfirmOpen(true)}>
            {t("about_update_install")}
          </Button>
        </div>
      ) : null}

      {install.isError ? (
        <p role="alert" data-testid="about-update-install-failed" className="text-sm text-fg-danger">
          {t("about_update_install_failed")}
        </p>
      ) : null}

      {answer?.kind === "newer" ? (
        <ConfirmDialog
          open={confirmOpen}
          onCancel={() => setConfirmOpen(false)}
          onConfirm={() => install.mutate()}
          title="about_update_available"
          question="about_update_confirm_question"
          confirm="about_update_install"
          pending={install.isPending}
          data-testid="about-update-install-dialog"
        >
          <dl className="grid grid-cols-[auto_1fr] items-baseline gap-x-4 gap-y-2">
            <dt className="text-sm text-muted-foreground">{t("field_version")}</dt>
            <dd className="font-numeric tabular-nums text-foreground" dir="ltr">
              {answer.version}
            </dd>
          </dl>
          {install.isPending ? (
            <p className="text-sm text-muted-foreground">{t("about_update_installing")}</p>
          ) : null}
        </ConfirmDialog>
      ) : null}
    </div>
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

            <UpdateCheckPanel />
          </CardContent>
        </Card>
      ) : null}
    </section>
  );
}
