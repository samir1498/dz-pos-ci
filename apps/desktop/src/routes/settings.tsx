// Settings is a list of rooms, not one long scroll.
//
// Eight blocks stacked in one column meant that pairing a phone — a thing a
// shop does once and then again the day a phone is replaced — was found by
// scrolling past the seller block, the régime, the theme, the staff card and
// the backups. Each block is its own route now: the sections are listed on
// the reading side and one panel renders beside them, so an address is a
// place you can be sent to and come back to.
//
// Not a tab row. Eight tabs wrap at any window a shop actually uses and a
// wrapped tab row reads as broken; the rail is a column at `md` and up and
// becomes a two-or-three-across grid of the same links below that, which is
// the shape ObserveOne's settings settled on for the same reason.
//
// The rail hides what the session cannot hold, and hides it for the same
// reason the sidebar does: `crates/api/src/gates.rs` already refuses the
// routes behind those rooms, so this is the hidden button and never the
// defence. Typing `/settings/users` by hand still meets the server's 403.

import { Link, Outlet, createFileRoute } from "@tanstack/react-router";
import {
  Database,
  HardDriveDownload,
  Info,
  Landmark,
  Palette,
  Printer,
  Smartphone,
  Store,
  Users,
  type LucideProps,
} from "lucide-react";
import type { ComponentType } from "react";
import type { PermissionDto } from "@dzpos/shared";

import { Icon } from "@/components/Icon";
import { PageHeader } from "@/components/PageHeader";
import { useTranslation, type Key } from "@/i18n";
import { hasPermission, useSession } from "@/lib/session";
import { cn } from "@/lib/utils";

export const Route = createFileRoute("/settings")({ component: SettingsLayout });

interface SectionItem {
  readonly to: string;
  readonly label: Key;
  readonly icon: ComponentType<LucideProps>;
  /** Absent for a room every signed-in role may open. Where present, it is
   *  the same permission `gates.rs` names on the routes that room calls. */
  readonly permission?: PermissionDto;
}

/**
 * The rooms, in the order the rail lists them: what the shop is, how it
 * sells, how it looks, who works there, what sells for it, and then the
 * three that are about the file rather than the shop.
 */
export const SETTINGS_SECTIONS: readonly SectionItem[] = [
  { to: "/settings/shop", label: "settings_nav_shop", icon: Store },
  { to: "/settings/regime", label: "settings_nav_regime", icon: Landmark },
  { to: "/settings/appearance", label: "settings_nav_appearance", icon: Palette },
  {
    to: "/settings/printing",
    label: "settings_nav_printing",
    icon: Printer,
    permission: "edit_settings",
  },
  {
    to: "/settings/users",
    label: "settings_nav_users",
    icon: Users,
    permission: "manage_users",
  },
  {
    to: "/settings/phones",
    label: "settings_nav_phones",
    icon: Smartphone,
    permission: "edit_settings",
  },
  { to: "/settings/backups", label: "settings_nav_backups", icon: HardDriveDownload },
  // Ungated on purpose: the stock recount in this room is open to every
  // role, and only the export/import block inside it is `ExportAndImport`.
  // Hiding the whole room would take the recount from a cashier too.
  { to: "/settings/data", label: "settings_nav_data", icon: Database },
  { to: "/settings/about", label: "settings_nav_about", icon: Info },
];

const LINK = "flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors";
const ACTIVE = "border border-border bg-muted text-foreground";
const IDLE = "border border-transparent text-muted-foreground hover:bg-muted hover:text-foreground";

export function SettingsLayout() {
  const { t } = useTranslation();
  // Read once for the whole rail. `useHasPermission` is a hook and cannot be
  // called inside the `.filter` below; `hasPermission` is the plain function
  // built for exactly this, and `AppShell`'s sidebar reads it the same way.
  const { me } = useSession();
  const rooms = SETTINGS_SECTIONS.filter(
    (item) => item.permission === undefined || hasPermission(me, item.permission),
  );

  return (
    <section className="flex min-w-0 w-full max-w-full flex-col">
      <PageHeader title={t("settings_title")} description={t("settings_hint")} />

      {/* Below `md` the rail would eat the whole first screen, so the same
          links render as a grid of chips above the panel instead. One list,
          two renderings: a room added once appears in both. */}
      <nav
        aria-label={t("settings_sections")}
        className="mb-6 grid grid-cols-2 gap-2 sm:grid-cols-3 md:hidden"
      >
        {rooms.map((item) => (
          <Link
            key={item.to}
            to={item.to}
            data-testid={`settings-room-mobile-${item.to.slice("/settings/".length)}`}
            className={cn(LINK, "flex-col justify-center gap-1 py-3 text-center")}
            activeProps={{ className: ACTIVE }}
            inactiveProps={{ className: IDLE }}
          >
            <Icon as={item.icon} size={18} />
            <span className="text-xs">{t(item.label)}</span>
          </Link>
        ))}
      </nav>

      <div className="flex min-w-0 flex-col gap-6 md:flex-row">
        <nav
          aria-label={t("settings_sections")}
          className="hidden shrink-0 space-y-1 md:block md:w-44"
        >
          {rooms.map((item) => (
            <Link
              key={item.to}
              to={item.to}
              data-testid={`settings-room-${item.to.slice("/settings/".length)}`}
              className={LINK}
              activeProps={{ className: ACTIVE }}
              inactiveProps={{ className: IDLE }}
            >
              <Icon as={item.icon} size={18} />
              <span className="truncate">{t(item.label)}</span>
            </Link>
          ))}
        </nav>

        {/* `min-w-0` and no width of its own: a minimum width here is what
            made ObserveOne's settings scroll sideways at 1384px, because a
            table inside a room pushes this column instead of scrolling. */}
        <div className="flex min-w-0 flex-1 flex-col gap-6">
          <Outlet />
        </div>
      </div>
    </section>
  );
}
