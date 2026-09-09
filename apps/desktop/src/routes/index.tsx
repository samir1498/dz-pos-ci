// The till is the home. A cashier opens this window to sell, so the first
// screen is the one that sells; the redirect runs before the route loads,
// which keeps "dz-pos" from flashing on every launch.

import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute("/")({
  beforeLoad: () => {
    throw redirect({ to: "/till" });
  },
});
