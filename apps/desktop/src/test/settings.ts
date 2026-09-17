// The settings answer a screen test stubs when it is not about the settings.
//
// Four test files stub `GET /settings` only so the shell can name the shop
// and the till can read the discount threshold. Written out four times, a
// field added to the DTO breaks all four, and the four get fixed differently:
// that is how three of them ended up with a different shop name than the
// fourth. A test that is about a particular value spreads this and overrides
// the one field it cares about.

import type { SettingsDto } from "@dzpos/shared";

export const SHOP_NAME = "Mon magasin";

export const SETTINGS: SettingsDto = {
  store: {
    name: SHOP_NAME,
    rc: null,
    nif: null,
    nis: null,
    ai: null,
    address: null,
    phone: null,
  },
  regime: { regime: "reel", valid_from: "2026-01-01" },
  regime_planned: null,
  theme: null,
  facture_layout: "standard",
  facture_layouts: ["standard", "compact"],
  discount_threshold_bps: 0,
};
