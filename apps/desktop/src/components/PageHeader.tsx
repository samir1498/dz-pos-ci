// The first thing on every screen: what this page is, one line about it when
// it needs one, and the actions that belong to the page rather than to a row.
//
// It starts at `h2`. The `h1` is the page's name in the topbar (AppShell),
// which every screen has and none of them writes; a screen heading itself
// again at level one would give the document two, and a screen reader's
// outline would then have no page level at all. So this is the second level
// and a screen's own sections are the third.
//
// The actions sit at the end of the row through `ms-auto`, which is the end
// of the row in Arabic too.

import type { ReactNode } from "react";

import { cn } from "@/lib/utils";

export function PageHeader({
  title,
  description,
  actions,
  className,
}: {
  title: string;
  /** A node rather than a string: a line that carries a phone number or any
   *  other value the shop did not write needs that value in its own `dir`,
   *  and a string cannot hold one. */
  description?: ReactNode;
  /** Buttons for the page as a whole. A row's own actions go in its row. */
  actions?: ReactNode;
  className?: string;
}) {
  return (
    <div
      data-testid="page-header"
      className={cn("flex flex-wrap items-start gap-4 pb-4", className)}
    >
      <div className="min-w-0">
        <h2 className="truncate text-xl font-semibold text-foreground">{title}</h2>
        {description === undefined || description === null ? null : (
          <p className="mt-1 text-sm text-muted-foreground">{description}</p>
        )}
      </div>
      {actions === undefined ? null : (
        <div className="ms-auto flex flex-wrap items-center gap-2">{actions}</div>
      )}
    </div>
  );
}
