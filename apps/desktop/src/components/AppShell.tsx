// The frame every screen renders inside: a sidebar of sections on the
// reading side, a topbar carrying the page's name and the two switches, and
// the screen itself in the rest.
//
// One `NAV` list drives both halves. The sidebar reads it for its items and
// the topbar reads it for the title, so a route added in one place cannot
// arrive with a heading that says something else, and the screens themselves
// stay untouched: none of them knows it is inside a shell.
//
// Direction. Radix reads its own direction context and defaults to `ltr`
// whatever the document says, so a Select or a Tabs list in Arabic would take
// the arrow keys the wrong way round without the provider below. The sidebar
// and the sheet it becomes on a narrow window keep a physical `side`, which
// the shell computes from the page direction: the panel's edge, its border
// and the direction it slides in from all have to agree, and a logical
// position with a physical animation is a panel that flies in from the wrong
// half of the screen.
//
// The day in the topbar is the shop's day, asked of the server
// (`lib/clock.ts`), never the machine's: a laptop carried across a border, or
// set wrong, is another day entirely and the shop's ledger is not on it.

import { useQuery } from "@tanstack/react-query";
import { Link, useRouterState } from "@tanstack/react-router";
import {
  Banknote,
  CalendarDays,
  ClipboardList,
  FileText,
  History,
  LayoutDashboard,
  Package,
  Receipt,
  Settings,
  ShoppingCart,
  Smartphone,
  Store,
  Truck,
  Users,
  type LucideProps,
} from "lucide-react";
import { Direction } from "radix-ui";
import type { ComponentType, ReactNode } from "react";
import type { PermissionDto } from "@dzpos/shared";

import { Icon } from "@/components/Icon";
import { Wordmark } from "@/components/Wordmark";
import { ThemeSwitcher } from "@/components/ThemeSwitcher";
import { UserMenu } from "@/components/UserMenu";
import { Toaster } from "@/components/ui/sonner";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarInset,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
  SidebarTrigger,
} from "@/components/ui/sidebar";
import { api, settingsQueryKey } from "@/api";
import { useTranslation, type Key } from "@/i18n";
import { LanguageSwitcher } from "@/i18n/LanguageSwitcher";
import { useShopToday } from "@/lib/clock";
import { hasPermission, useSession } from "@/lib/session";

/** The three groups the sidebar is divided into, in the order it shows them. */
const SECTIONS = ["sales", "purchases", "manage"] as const;
type Section = (typeof SECTIONS)[number];

const SECTION_LABEL: Readonly<Record<Section, Key>> = {
  sales: "nav_section_sales",
  purchases: "nav_section_purchases",
  manage: "nav_section_manage",
};

interface NavItem {
  /** The route this item goes to, and the prefix the title matches on. */
  readonly to: string;
  readonly label: Key;
  readonly icon: ComponentType<LucideProps>;
  readonly section: Section;
  /** Absent for a route open to every signed-in role. Where present, the
   *  sidebar shows the item only once `hasPermission` says the session
   *  holds it — the same permission the route itself is gated by
   *  server-side, not a client-only opinion (`crates/api/src/gates/`).
   *  The settings entry stays visible for everyone: its rail lists only
   *  the rooms the session may open and `/settings` itself redirects to the
   *  shop block, which every role may read. Dashboard and purchases carry a permission here because
   *  M4 T5's review found the server refuses the route outright, so a
   *  cashier reaching either by a stale link or a typed URL should not see
   *  a link into it in the first place. Suppliers and expenses carry one
   *  for the same reason since the closing review: what the shop owes its
   *  suppliers and what it spends were both readable by a cashier while
   *  the dashboard that sums them was not. */
  readonly permission?: PermissionDto;
}

/**
 * Every screen the shell knows, in sidebar order. The till leads: it is the
 * screen a counter spends its day on. A route missing from this list still
 * renders, it simply shows the app's name in the topbar instead of its own,
 * which is what a route with no place in the navigation should look like.
 */
export const NAV: readonly NavItem[] = [
  {
    to: "/dashboard",
    label: "nav_dashboard",
    icon: LayoutDashboard,
    section: "sales",
    permission: "see_reports",
  },
  { to: "/till", label: "nav_till", icon: ShoppingCart, section: "sales" },
  {
    to: "/till/shifts",
    label: "nav_till_shifts",
    icon: Banknote,
    section: "sales",
    permission: "see_reports",
  },
  { to: "/customers", label: "nav_customers", icon: Users, section: "sales" },
  { to: "/documents", label: "nav_documents", icon: FileText, section: "sales" },
  {
    to: "/suppliers",
    label: "nav_suppliers",
    icon: Truck,
    section: "purchases",
    permission: "see_cost_and_margin",
  },
  {
    to: "/purchases",
    label: "nav_purchases",
    icon: ClipboardList,
    section: "purchases",
    permission: "see_cost_and_margin",
  },
  {
    to: "/expenses",
    label: "nav_expenses",
    icon: Receipt,
    section: "purchases",
    permission: "see_reports",
  },
  { to: "/products", label: "nav_products", icon: Package, section: "manage" },
  {
    to: "/audit",
    label: "nav_audit_log",
    icon: History,
    section: "manage",
    permission: "see_audit_log",
  },
  // Pairing a phone is why this entry exists. It is a room inside
  // `/settings`, but it was unreachable in practice: a shop looking for it
  // scrolled past five other blocks first, and half the time gave up. A
  // second door into one room is cheaper than the hunt. `EditSettings` is
  // the permission `gates/` names on all three pairing routes.
  {
    to: "/settings/phones",
    label: "nav_phones",
    icon: Smartphone,
    section: "manage",
    permission: "edit_settings",
  },
  { to: "/settings", label: "nav_settings", icon: Settings, section: "manage" },
];

/**
 * The longest matching prefix wins, so `/customers/7` is still "Clients"
 * while `/purchases/new` does not answer for `/products`. `/` is the till,
 * which is the route the index redirects to.
 */
export function activeItem(pathname: string): NavItem | undefined {
  return NAV.filter(
    (item) => pathname === item.to || pathname.startsWith(`${item.to}/`),
  ).sort((a, b) => b.to.length - a.to.length)[0];
}

function SidebarNav() {
  const { t } = useTranslation();
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const active = activeItem(pathname);
  // Read once for the whole sidebar: `useHasPermission` is a hook and
  // cannot be called once per item inside `.filter` below, and `me` is
  // already the one copy of it the window holds (`lib/session.tsx`).
  // `hasPermission` is the plain function built for exactly this.
  const { me } = useSession();
  const visible = (item: NavItem) => item.permission === undefined || hasPermission(me, item.permission);
  return (
    <>
      {SECTIONS.map((section) => (
        <SidebarGroup key={section}>
          <SidebarGroupLabel>{t(SECTION_LABEL[section])}</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {NAV.filter((item) => item.section === section && visible(item)).map((item) => (
                <SidebarMenuItem key={item.to}>
                  <SidebarMenuButton asChild isActive={active?.to === item.to}>
                    <Link to={item.to} data-testid={`nav-${item.to.slice(1)}`}>
                      <Icon as={item.icon} size={18} />
                      <span>{t(item.label)}</span>
                    </Link>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      ))}
    </>
  );
}

/**
 * The shop at the foot of the sidebar. The brief that added sign-in
 * (M4 T4) put the signed-in user in the topbar instead (`UserMenu`, in the
 * `ms-auto` group below) rather than here: this reads the store's own name
 * from the same cache the theme provider fills, and stays the sidebar's
 * identity regardless of who is signed in.
 */
function ShopFooter() {
  const { t } = useTranslation();
  const settings = useQuery({ queryKey: settingsQueryKey, queryFn: () => api.getSettings() });
  const name = settings.data?.store.name;
  return (
    <div
      data-testid="shell-shop"
      className="flex items-center gap-2 rounded-md px-2 py-2 text-sm text-sidebar-foreground"
    >
      <Icon as={Store} size={18} className="text-sidebar-muted" />
      <span className="truncate">{name === undefined || name === "" ? t("app_name") : name}</span>
    </div>
  );
}

/** The shop's day, or nothing at all while the server has not said it. */
function ShopDay() {
  const { today } = useShopToday();
  if (today === undefined) return null;
  return (
    <span
      data-testid="shell-day"
      className="hidden items-center gap-1.5 text-sm text-muted-foreground sm:flex"
    >
      <Icon as={CalendarDays} size={18} />
      <span dir="ltr" className="font-numeric tabular-nums">
        {today}
      </span>
    </span>
  );
}

export function AppShell({ children }: { children: ReactNode }) {
  const { t, dir } = useTranslation();
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const active = activeItem(pathname);
  const title = active === undefined ? t("app_name") : t(active.label);

  return (
    <Direction.Provider dir={dir}>
      {/* 240 px, the width the design asks for. It is a literal because the
          scale in packages/design carries control heights and no layout
          widths yet; the day it gains one, this becomes that token. */}
      <SidebarProvider style={{ "--sidebar-width": "15rem" }}>
        <Sidebar side={dir === "rtl" ? "right" : "left"} collapsible="icon">
          <SidebarHeader className="px-2 py-3">
            <Wordmark className="px-1 text-sidebar-foreground" />
          </SidebarHeader>
          <SidebarContent>
            <SidebarNav />
          </SidebarContent>
          <SidebarFooter>
            <ShopFooter />
          </SidebarFooter>
        </Sidebar>
        <SidebarInset>
          <header
            data-testid="shell-topbar"
            className="sticky top-0 z-10 flex h-14 shrink-0 items-center gap-3 border-b border-border bg-background px-4"
          >
            <SidebarTrigger data-testid="sidebar-trigger" className="-ms-1" />
            {/* The page's `h1`. It is here rather than on each screen because
                it is the one heading every screen has, and a screen that
                headed itself would give a document with two of them.
                `PageHeader` starts at `h2` for that reason. */}
            <h1 data-testid="shell-title" className="truncate text-md font-semibold">
              {title}
            </h1>
            <div className="ms-auto flex items-center gap-3">
              <ShopDay />
              <LanguageSwitcher />
              <ThemeSwitcher />
              <UserMenu />
            </div>
          </header>
          <main className="min-w-0 flex-1 p-4">{children}</main>
        </SidebarInset>
        <Toaster position={dir === "rtl" ? "bottom-left" : "bottom-right"} />
      </SidebarProvider>
    </Direction.Provider>
  );
}
