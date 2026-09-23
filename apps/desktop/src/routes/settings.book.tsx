// The book's settings room (C5b, C6): working hours, visit types and
// absence blocks. Thin for the same reason every settings room is
// (`settings.regime.tsx`): the rail already wrote the page's own heading,
// so this passes `level={3}`.

import { createFileRoute } from "@tanstack/react-router";

import { PageHeader } from "@/components/PageHeader";
import { useTranslation } from "@/i18n";

import { BookSettings } from "./-book/BookSettings";

export const Route = createFileRoute("/settings/book")({ component: BookRoom });

function BookRoom() {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col gap-4">
      <PageHeader level={3} title={t("settings_nav_book")} description={t("book_settings_subtitle")} />
      <BookSettings />
    </div>
  );
}
