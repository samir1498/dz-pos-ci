// A product's pack size as the fiche types it (T13): a quantity read the way
// every quantity is (`parseQtyToMilli`, "1,5" is 1500 thousandths) and one of
// four units. The name the screens show with it is the server's
// (`ProductDto.display_name`); nothing here builds it.

import { parseQtyToMilli } from "@dzpos/shared";
import type { ContenanceUnitDto } from "@dzpos/shared";

export const CONTENANCE_UNITS = ["g", "kg", "ml", "l"] as const satisfies readonly ContenanceUnitDto[];

/** The select's value for "no pack size": a Radix select item cannot be "". */
export const NO_CONTENANCE = "none";

/** The symbol a unit is shown with, the same in the three languages. */
export const CONTENANCE_SYMBOL: Record<ContenanceUnitDto, string> = {
  g: "g",
  kg: "kg",
  ml: "mL",
  l: "L",
};

export type ReadContenance =
  | { readonly kind: "none" }
  | { readonly kind: "set"; readonly milli: number; readonly unit: ContenanceUnitDto }
  | { readonly kind: "incomplete" };

/** Both boxes blank is no pack size; one without the other, or a quantity
 *  that is not above zero, is a half-typed figure the fiche refuses. */
export function readContenance(qty: string, unit: string): ReadContenance {
  const chosen = CONTENANCE_UNITS.find((u) => u === unit);
  if (qty.trim() === "" && chosen === undefined) return { kind: "none" };
  const milli = parseQtyToMilli(qty);
  if (chosen === undefined || milli === null || milli <= 0) return { kind: "incomplete" };
  return { kind: "set", milli, unit: chosen };
}
