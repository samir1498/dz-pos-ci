// What the list and the statement both say about a supplier, written once.
// The folder is `-suppliers`, which the router leaves alone, the way the
// customers screen keeps its panel in `-customers` and the till keeps its
// panels in `-till`.
//
// Nothing here fetches and nothing here computes an amount: a balance is the
// core's and arrives already added up.

import type { SupplierDebtKindDto } from "@dzpos/shared";

import type { Key } from "@/i18n";

export const SUPPLIER_KIND_KEY: Record<SupplierDebtKindDto, Key> = {
  opening: "debt_opening",
  purchase: "supplier_debt_purchase",
  payment: "debt_payment",
  return: "supplier_debt_return",
  adjustment: "debt_adjustment",
};

/**
 * What the fiche and the list call the stored balance. A supplier's balance
 * is one signed number: positive is what the shop owes, and a return or an
 * advance past what was due drives it below zero, at which point the supplier
 * owes the shop. That is an advance and not a credit: the shop is not holding
 * anybody's money, it has handed money over.
 */
export function supplierBalanceLabel(balance_centimes: number): Key {
  return balance_centimes < 0 ? "suppliers_advance" : "suppliers_balance";
}

/** What a balance is, in the kit's words. Nil is settled; anything else is
 *  an account still running, in either direction. */
export function balanceStatus(centimes: number): "paid" | "open" {
  return centimes === 0 ? "paid" : "open";
}
