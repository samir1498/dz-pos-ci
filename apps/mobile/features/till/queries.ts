// What the till has to read before it can sell anything.
//
// Two reads, and the régime one is not optional. It decides whether a price
// tag is the whole price or only the HT part, so a basket cannot be priced
// without it — the till shows no total and refuses to ring rather than
// guessing `reel` and charging an IFU shop's customer a TVA they do not owe.

import { useQuery } from "@tanstack/react-query";

import type { ProductDto, RegimeDto, SettingsDto } from "@dzpos/shared";

import { get } from "../../lib/api";
import type { Session } from "../../lib/session";

const credentialsOf = (session: Session) => ({
  deviceToken: session.deviceToken,
  sessionToken: session.sessionToken,
});

export function useProducts(session: Session) {
  return useQuery({
    // The session token is part of the key: sign out, sign in as someone
    // else, and the new signer reads with their own credentials instead of
    // the previous one's cached answer.
    queryKey: ["products", session.sessionToken],
    queryFn: () => get<ProductDto[]>("/products", credentialsOf(session)),
  });
}

export function useRegime(session: Session) {
  return useQuery({
    queryKey: ["settings", session.sessionToken],
    queryFn: async (): Promise<RegimeDto> => {
      const settings = await get<SettingsDto>("/settings", credentialsOf(session));
      // `regime` is the one in force today; `regime_planned` is a change the
      // owner has dated ahead and is not what a sale rung now is priced on.
      return settings.regime.regime;
    },
  });
}
