// Words that belong to no one screen. Kept small on purpose: a key here is
// one every domain may use, and a key used once belongs in its domain.

import type { Messages } from "../message";

export const common = {
  app_name: "Dinar",
  action_back: "رجوع",
  action_try_again: "إعادة المحاولة",
  currency_suffix: "دج",
} as const satisfies Messages;
