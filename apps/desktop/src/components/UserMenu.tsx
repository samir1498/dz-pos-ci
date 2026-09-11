// The signed-in user, in the topbar. A name, a role label and two actions:
// lock the till now, and sign out.
//
// The role label is a lookup keyed by `RoleDto`, the same shape
// `ThemeSwitcher`'s `THEME_LABEL` uses for a theme name: a fourth role
// added to the core would fail to compile here until it has a label, and
// nothing in this file branches on which one it is. That is what
// architecture.md rule 2 asks for, and `role.test.ts` holds every file
// under `src/` to it; this one already reads it.

import type { RoleDto } from "@dzpos/shared";
import { LogOut, Lock, User } from "lucide-react";

import { Icon } from "@/components/Icon";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useTranslation, type Key } from "@/i18n";
import { ROLE_LABEL } from "@/lib/roles";
import { useSession } from "@/lib/session";

export function UserMenu() {
  const { t } = useTranslation();
  const { me, signOut, lockNow } = useSession();
  if (me === null) return null;

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          data-testid="user-menu-trigger"
          className="gap-2"
        >
          <Icon as={User} size={18} className="text-muted-foreground" />
          <span className="max-w-32 truncate">{me.name}</span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" data-testid="user-menu">
        <DropdownMenuLabel className="font-normal text-muted-foreground">
          {t(ROLE_LABEL[me.role])}
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuItem data-testid="user-menu-lock" onSelect={() => lockNow()}>
          <Icon as={Lock} size={18} />
          {t("topbar_lock_now")}
        </DropdownMenuItem>
        <DropdownMenuItem data-testid="user-menu-signout" onSelect={() => void signOut()}>
          <Icon as={LogOut} size={18} />
          {t("topbar_sign_out")}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
