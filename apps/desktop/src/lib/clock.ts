// The day the shop is on, asked of the server. The core dates every
// document on Algeria's calendar, UTC+1 with no daylight saving
// (crates/core/src/services/clock.rs); a browser reads the machine's zone,
// which on a laptop carried across a border, or set wrong, is another day
// entirely. A statement asked for "to today" would then end before the
// evening's movements, or the régime form would date a change to a day the
// ledger has not reached.

import { useQuery } from "@tanstack/react-query";

import { api, clockQueryKey } from "@/api";

/** What the shop's day is, or why it is not on the screen yet. */
export interface ShopToday {
  /** The day as `YYYY-MM-DD`, once the server has said it. */
  today: string | undefined;
  /** The refusal, when the call was refused; `null` while it is in flight
   * or once it has landed. A caller shows it the way it shows any other
   * refusal, because a day that cannot be read is not a day that is late. */
  error: unknown;
  /** Asks again. What the error line's button is for. */
  retry: () => void;
}

/**
 * Today as `YYYY-MM-DD` on the shop's calendar. A caller that needs it for a
 * default value renders the field only once it has one: a form pre-filled
 * with the wrong day and corrected a moment later is worse than a form that
 * arrives complete.
 *
 * Never cached: a day is stale the moment it turns, so a panel opened after
 * midnight asks again rather than reading the one the last panel got.
 *
 * The refusal is handed back separately from the answer. Both are "no day
 * yet" to a caller that only reads `today`, and the screen that told them
 * apart by that alone would sit on a loading line forever the one time the
 * call failed.
 */
export function useShopToday(): ShopToday {
  const clock = useQuery({
    queryKey: clockQueryKey,
    queryFn: () => api.clock(),
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
  });
  return {
    today: clock.data?.today,
    error: clock.isError ? clock.error : null,
    retry: () => {
      void clock.refetch();
    },
  };
}
