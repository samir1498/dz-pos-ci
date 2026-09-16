// The store block a ticket prints as the seller.

import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";

import { SettingsLoad } from "@/components/settings/SettingsLoad";
import { StoreForm } from "@/components/settings/StoreForm";

export const Route = createFileRoute("/settings/shop")({ component: ShopRoom });

export function ShopRoom() {
  // Lives here, not in the form: a save refetches the settings and the form
  // is remounted on the fresh block (its key), which would drop the message.
  const [saved, setSaved] = useState(false);
  return (
    <SettingsLoad>
      {(settings) => (
        <StoreForm
          key={JSON.stringify(settings.store)}
          initial={settings.store}
          saved={saved}
          onSaved={setSaved}
        />
      )}
    </SettingsLoad>
  );
}
