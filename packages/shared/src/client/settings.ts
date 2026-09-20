// The settings page: reading it, and the handful of writes that each answer
// the whole page back rather than patching the caller's own copy.

import type { DiscountThresholdChangeDto } from "../generated/DiscountThresholdChangeDto";
import type { FactureLayoutDto } from "../generated/FactureLayoutDto";
import type { RegimeChangeDto } from "../generated/RegimeChangeDto";
import type { SettingsDto } from "../generated/SettingsDto";
import type { StoreDto } from "../generated/StoreDto";
import type { ThemeDto } from "../generated/ThemeDto";
import { narrow } from "../client-response";
import type { Transport } from "../client";
import { settingsSchema, storeSchema } from "../schemas/settings";

export function settingsClient({ send }: Transport) {
  return {
    async getSettings(): Promise<SettingsDto> {
      return narrow(await send("/settings"), settingsSchema, "settings");
    },

    /** The whole store block; a null clears that field. */
    async updateStore(input: StoreDto): Promise<StoreDto> {
      const body = await send("/settings/store", {
        method: "PUT",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, storeSchema, "store block");
    },

    /** Appends a dated régime change; the answer is the whole settings page
     * again, since the change is current or planned depending on its day. */
    async changeRegime(input: RegimeChangeDto): Promise<SettingsDto> {
      const body = await send("/settings/regime", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, settingsSchema, "settings");
    },

    /** Appends a dated change to the discount a cashier may give without
     * asking anyone, in basis points of the basket. Answers the whole
     * settings page, like the régime change it rides beside. */
    async setDiscountThreshold(input: DiscountThresholdChangeDto): Promise<SettingsDto> {
      const body = await send("/settings/discount-threshold", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(input),
      });
      return narrow(body, settingsSchema, "settings");
    },

    /** Records the shop's theme, or `null` to follow the machine. The answer
     * is the whole settings page, the way a régime change answers, so the
     * screen reads one shape back instead of patching its own copy. */
    async setTheme(theme: ThemeDto | null): Promise<SettingsDto> {
      const body = await send("/settings/theme", {
        method: "PUT",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ theme }),
      });
      return narrow(body, settingsSchema, "settings");
    },

    /** Records which layout the shop's factures are drawn in. Answers the
     * whole settings page, the way `setTheme` does, so the screen reads one
     * shape back instead of patching its own copy. */
    async setFactureLayout(layout: FactureLayoutDto): Promise<SettingsDto> {
      const body = await send("/settings/facture-layout", {
        method: "PUT",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ facture_layout: layout }),
      });
      return narrow(body, settingsSchema, "settings");
    },
  };
}
