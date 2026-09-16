// `/settings` on its own is not a screen: it is the rail plus whichever room
// is open. Landing there opens the shop block, which is the one every role
// may read and the one a shop fills in first.

import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute("/settings/")({
  beforeLoad: () => {
    throw redirect({ to: "/settings/shop" });
  },
});
