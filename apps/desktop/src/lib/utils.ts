// The class merger every shadcn/ui component imports as `cn`. Two jobs in
// one call: clsx flattens the conditional forms a component takes (arrays,
// objects, undefined), and tailwind-merge then drops the earlier of two
// utilities that set the same property, so a caller's `p-4` beats the
// component's own `p-2` instead of both landing and the cascade deciding by
// source order.

import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}
