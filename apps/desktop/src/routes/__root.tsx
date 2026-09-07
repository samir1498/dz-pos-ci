import { createRootRoute, Outlet } from "@tanstack/react-router";
import { useTranslation } from "@/i18n";

export const Route = createRootRoute({ component: RootLayout });

function RootLayout() {
  const { t, dir } = useTranslation();
  return (
    <div dir={dir} className="min-h-screen">
      <header className="p-4 border-b">{t("app_name")}</header>
      <main className="p-4">
        <Outlet />
      </main>
    </div>
  );
}
