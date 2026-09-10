// The kit page's route, and the one thing it decides: this page exists while
// developing and nowhere else.
//
// `import.meta.env.DEV` is a constant the bundler replaces, so in a shipped
// build the guard is `if (false)` and the branch that reaches the page is
// dead code. The import is dynamic for that reason and not for loading
// speed: a static one would keep the page, the sample data and every
// component it shows in the shipped bundle, unreachable but paid for. This
// is the one place the app splits a chunk, and it is the one place where the
// chunk is never fetched in production because the branch that would fetch
// it is gone.
//
// The route is not in the sidebar either (AppShell's `NAV` does not carry
// it), which is why `activeItem` answers nothing for `/kit` and the topbar
// falls back to the app's name.

import { createFileRoute, lazyRouteComponent, notFound } from "@tanstack/react-router";

export const Route = createFileRoute("/kit")({
  beforeLoad: () => {
    if (!import.meta.env.DEV) throw notFound();
  },
  component: import.meta.env.DEV
    ? lazyRouteComponent(() => import("@/kit/KitPage"), "KitPage")
    : () => null,
});
