// shadcn's Calendar, on react-day-picker 9, trimmed to what the date field
// asks of it: one month, one day picked. Written by hand in the CLI's shape
// and with the corrections frontend-conventions lists: logical properties
// only, no `dark:`, no size the scale has no name for.
//
// The chevrons point the way the page reads. In Arabic the previous month is
// on the right, and react-day-picker lays the nav out by `dir`, so the icons
// flip with it through `Icon`'s `flip`.

import * as React from "react";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { DayPicker, getDefaultClassNames } from "react-day-picker";

import { Icon } from "@/components/Icon";
import { buttonVariants } from "@/components/ui/button";
import { cn } from "@/lib/utils";

function Calendar({
  className,
  classNames,
  showOutsideDays = true,
  ...props
}: React.ComponentProps<typeof DayPicker>) {
  const defaults = getDefaultClassNames();
  return (
    <DayPicker
      showOutsideDays={showOutsideDays}
      className={cn("bg-popover p-1", className)}
      classNames={{
        root: cn("w-fit", defaults.root),
        months: cn("relative flex flex-col gap-4", defaults.months),
        month: cn("flex w-full flex-col gap-2", defaults.month),
        nav: cn(
          "absolute inset-x-0 top-0 flex w-full items-center justify-between gap-1",
          defaults.nav,
        ),
        button_previous: cn(
          buttonVariants({ variant: "ghost", size: "icon-sm" }),
          defaults.button_previous,
        ),
        button_next: cn(
          buttonVariants({ variant: "ghost", size: "icon-sm" }),
          defaults.button_next,
        ),
        month_caption: cn("flex h-8 w-full items-center justify-center", defaults.month_caption),
        caption_label: cn("text-sm font-medium", defaults.caption_label),
        month_grid: "w-full border-collapse",
        weekdays: cn("flex", defaults.weekdays),
        weekday: cn(
          "flex-1 text-center text-xs font-normal text-muted-foreground",
          defaults.weekday,
        ),
        week: cn("mt-1 flex w-full", defaults.week),
        day: cn("relative p-0 text-center", defaults.day),
        day_button: cn(
          buttonVariants({ variant: "ghost", size: "icon-sm" }),
          "font-numeric tabular-nums",
          defaults.day_button,
        ),
        selected: cn(
          "[&>button]:bg-primary [&>button]:text-primary-foreground",
          defaults.selected,
        ),
        today: cn("[&>button]:border [&>button]:border-border", defaults.today),
        outside: cn("text-muted-foreground opacity-50", defaults.outside),
        disabled: cn("opacity-50", defaults.disabled),
        hidden: cn("invisible", defaults.hidden),
        ...classNames,
      }}
      components={{
        Chevron: ({ orientation }) => (
          <Icon as={orientation === "left" ? ChevronLeft : ChevronRight} size={18} flip />
        ),
      }}
      {...props}
    />
  );
}

export { Calendar };
