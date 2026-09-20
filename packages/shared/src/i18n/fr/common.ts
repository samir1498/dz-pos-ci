// Words that belong to no one screen. Kept small on purpose: a key here is
// one every domain may use, and a key used once belongs in its domain.

import type { Messages } from "../message";

export const common = {
  app_name: "Dinar",
  action_back: "Retour",
  action_try_again: "Réessayer",
  currency_suffix: "DA",
} as const satisfies Messages;
