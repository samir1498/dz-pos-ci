// Who is signed in, and what that role may do (M4 T7's own small piece of
// the sign-in work T4 is landing beside this). `GET /auth/me` already
// answers `role` and `permissions`; nothing here decides anything, it only
// reads that answer the way `useShopToday` reads the clock.
//
// Before T4 there is no session at all, and the call answers a refusal
// (`session_required` today, a 401 either way once sign-in lands and nobody
// has signed in yet). A caller that gates a screen or a nav entry on a
// permission reads that the same way it reads "does not hold it": the entry
// hides, it does not throw up an error a person never asked to see.

import { useQuery } from "@tanstack/react-query";
import type { PermissionDto } from "@dzpos/shared";

import { api } from "@/api";

export const meQueryKey: readonly string[] = ["me"];

export function useMe() {
  return useQuery({
    queryKey: meQueryKey,
    queryFn: () => api.me(),
    retry: false,
  });
}

/** Whether the signed-in session holds `permission`. `false` for every
 * refusal `useMe` can answer, not only "role does not hold it": a screen
 * that cannot even tell who is signed in offers nothing that needs asking. */
export function useHasPermission(permission: PermissionDto): boolean {
  const me = useMe();
  return me.data?.permissions.includes(permission) ?? false;
}
