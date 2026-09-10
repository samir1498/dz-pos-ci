// The one icon set. Lucide, taken as a direct dependency at an exact version
// (MIT, stroke icons, the set Samir's own brand kit uses); nothing in this app
// draws an icon by hand except the logo.
//
// The component takes the Lucide component itself rather than a name string.
// A name would mean a lookup table holding every icon in the set, and the
// bundler would then ship all of them; passing the component keeps the import
// at the call site and lets tree shaking drop the rest.
//
// Three sizes, because a scale with a size for every occasion has no scale.
// An icon is decoration beside a label unless a caller gives it one of its
// own, so `aria-hidden` is the default and `label` is the way out of it.

import type { LucideProps } from "lucide-react";
import type { ComponentType } from "react";

import { cn } from "@/lib/utils";

/** 18 beside text, 20 in a control, 24 for a heading or an empty state. */
export type IconSize = 18 | 20 | 24;

export function Icon({
  as: Glyph,
  size = 20,
  flip = false,
  label,
  className,
}: {
  as: ComponentType<LucideProps>;
  size?: IconSize;
  /**
   * For an icon that points somewhere: an arrow, a chevron, an undo. Mirrors
   * with the page direction, so "next" points at the next thing in Arabic
   * too. A clock, a printer or a coin must not carry this: mirroring those
   * makes them wrong rather than translated.
   */
  flip?: boolean;
  label?: string;
  className?: string;
}) {
  return (
    <Glyph
      size={size}
      aria-hidden={label === undefined}
      aria-label={label}
      role={label === undefined ? undefined : "img"}
      className={cn(flip && "rtl:-scale-x-100", className)}
    />
  );
}
