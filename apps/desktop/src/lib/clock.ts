// The day the shop is on, asked of the server. The core dates every
// document on Algeria's calendar, UTC+1 with no daylight saving
// (crates/core/src/services/clock.rs); a browser reads the machine's zone,
// which on a laptop carried across a border, or set wrong, is another day
// entirely. A statement asked for "to today" would then end before the
// evening's movements, or the régime form would date a change to a day the
// ledger has not reached.

import { useQuery } from "@tanstack/react-query";

import { api, clockQueryKey } from "@/api";

/**
 * Today as `YYYY-MM-DD` on the shop's calendar, or `undefined` until the
 * server has said. A caller that needs it for a default value renders the
 * field only once it has one: a form pre-filled with the wrong day and
 * corrected a moment later is worse than a form that arrives complete.
 *
 * Never cached: a day is stale the moment it turns, and a screen left open
 * over midnight must not hand tomorrow's statement yesterday's date.
 */
export function useShopToday(): string | undefined {
  const clock = useQuery({
    queryKey: clockQueryKey,
    queryFn: () => api.clock(),
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
  });
  return clock.data?.today;
}
