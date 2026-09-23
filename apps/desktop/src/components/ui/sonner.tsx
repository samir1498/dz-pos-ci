// The toast host, mounted once in the shell.
//
// shadcn ships this file reading `useTheme()` from next-themes and handing
// sonner a `theme` prop. Two reasons it does not here. The app has no
// next-themes: the theme is one attribute on the document element and a block
// of CSS variables per theme, and a component that asked which theme was on
// would be the theme-conditional code `tests/theme.test.ts` fails the gates for.
// And sonner's own `theme` prop only picks between its two built-in palettes,
// neither of which is ours.
//
// So the toast wears our roles directly. The four variables below are
// sonner's own hooks for exactly this, and they point at the popover roles a
// dialog and a dropdown already use, which means a toast follows a theme
// switch with the rest of the app and no code here runs at all.

import {
  CircleCheckIcon,
  InfoIcon,
  Loader2Icon,
  OctagonXIcon,
  TriangleAlertIcon,
} from "lucide-react";
import { Toaster as Sonner, type ToasterProps } from "sonner";

function Toaster({ ...props }: ToasterProps) {
  return (
    <Sonner
      className="toaster group"
      icons={{
        success: <CircleCheckIcon className="size-4" />,
        info: <InfoIcon className="size-4" />,
        warning: <TriangleAlertIcon className="size-4" />,
        error: <OctagonXIcon className="size-4" />,
        loading: <Loader2Icon className="size-4 animate-spin" />,
      }}
      style={{
        "--normal-bg": "var(--popover)",
        "--normal-text": "var(--popover-foreground)",
        "--normal-border": "var(--border)",
        "--border-radius": "var(--radius)",
      }}
      {...props}
    />
  );
}

export { Toaster };
