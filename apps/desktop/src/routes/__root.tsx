import { createRootRoute, Link, Outlet } from "@tanstack/react-router";
import { useTranslation } from "@/i18n";
import { LanguageSwitcher } from "@/i18n/LanguageSwitcher";

export const Route = createRootRoute({ component: RootLayout });

function RootLayout() {
  const { t, dir } = useTranslation();
  return (
    <div dir={dir} className="min-h-screen">
      <header className="flex items-center gap-4 border-b p-4">
        <span className="font-semibold">{t("app_name")}</span>
        <nav className="flex gap-4">
          {/* The till comes first: it is the home and the screen the shop
              spends its day on. */}
          <Link to="/till" className="underline">
            {t("nav_till")}
          </Link>
          <Link to="/customers" className="underline">
            {t("nav_customers")}
          </Link>
          <Link to="/products" className="underline">
            {t("nav_products")}
          </Link>
          {/* Money out that is not stock, beside the screens the shop works
              in: the cash position it carries is read from the same day. */}
          <Link to="/expenses" className="underline">
            {t("nav_expenses")}
          </Link>
          {/* After the three screens a shop works in: a document is found
              again here, not made here. */}
          <Link to="/documents" className="underline">
            {t("nav_documents")}
          </Link>
          <Link to="/settings" className="underline">
            {t("nav_settings")}
          </Link>
        </nav>
        {/* ms-auto (logical, not ml-auto): sits at the end of the row in
            either direction, so it lands opposite the brand in fr/en and
            ar alike without a second rule. */}
        <LanguageSwitcher className="ms-auto" />
      </header>
      <main className="p-4">
        <Outlet />
      </main>
    </div>
  );
}
