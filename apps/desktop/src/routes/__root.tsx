import { createRootRoute, Link, Outlet } from "@tanstack/react-router";
import { useTranslation } from "@/i18n";

export const Route = createRootRoute({ component: RootLayout });

function RootLayout() {
  const { t, dir } = useTranslation();
  return (
    <div dir={dir} className="min-h-screen">
      <header className="flex items-center gap-4 border-b p-4">
        <span className="font-semibold">{t("app_name")}</span>
        <nav className="flex gap-4">
          <Link to="/products" className="underline">
            {t("nav_products")}
          </Link>
          <Link to="/settings" className="underline">
            {t("nav_settings")}
          </Link>
        </nav>
      </header>
      <main className="p-4">
        <Outlet />
      </main>
    </div>
  );
}
