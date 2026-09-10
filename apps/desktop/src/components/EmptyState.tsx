// What a screen shows when there is nothing yet. Not an error, and not a
// blank rectangle either: an empty products table on the first morning is the
// correct state, and the screen's job there is to say what would fill it and
// offer the one action that does.
//
// The icon is decoration (`Icon` hides it from a screen reader unless it is
// given a label of its own); the title carries the meaning.

import type { LucideProps } from "lucide-react";
import type { ComponentType, ReactNode } from "react";

import { Icon } from "@/components/Icon";
import { cn } from "@/lib/utils";

export function EmptyState({
  icon,
  title,
  description,
  action,
  className,
  "data-testid": testId,
}: {
  icon: ComponentType<LucideProps>;
  title: string;
  description?: string;
  /** The one thing to do about it. Two buttons here is a screen deciding. */
  action?: ReactNode;
  className?: string;
  "data-testid"?: string;
}) {
  return (
    <div
      data-testid={testId ?? "empty-state"}
      className={cn(
        "flex flex-col items-center justify-center gap-3 rounded-lg border border-dashed border-border px-6 py-12 text-center",
        className,
      )}
    >
      <Icon as={icon} size={24} className="text-faint" />
      <p className="text-md font-medium text-foreground">{title}</p>
      {description === undefined ? null : (
        <p className="max-w-prose text-sm text-muted-foreground">{description}</p>
      )}
      {action === undefined ? null : <div className="pt-1">{action}</div>}
    </div>
  );
}
