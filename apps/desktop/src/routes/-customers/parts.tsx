// What the list and the account page both say about a customer, written
// once. The folder is `-customers`, which the router leaves alone, the way
// the till keeps its panels in `-till`.
//
// Nothing here fetches and nothing here computes an amount: a balance is the
// core's (`services::debt`) and arrives already added up.

import type { CustomerDto, DebtKindDto, PartyKindDto } from "@dzpos/shared";
import { useId } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useTranslation, type Key } from "@/i18n";
import { fieldErrorMessage, shownPositive } from "@/lib/fields";
import { cn } from "@/lib/utils";

export const PARTY_KINDS: readonly PartyKindDto[] = ["company", "consumer"];

export const PARTY_KEY: Readonly<Record<PartyKindDto, Key>> = {
  company: "party_company",
  consumer: "party_consumer",
};

export const DEBT_KIND_KEY: Readonly<Record<DebtKindDto, Key>> = {
  opening: "debt_opening",
  sale: "debt_sale",
  payment: "debt_payment",
  avoir: "debt_avoir",
  adjustment: "debt_adjustment",
};

/** What the fiche and the list call the stored balance.
 *
 * A customer's balance is one signed number: positive is what they owe, and
 * an avoir past what was outstanding drives it below zero, at which point the
 * shop is holding their money. "Créance -1 000,00" is not a sentence anyone
 * says at a counter, so the sign is spent on the word instead of on the
 * figure and the amount is always shown positive.
 *
 * features.md §3 (avoir) is where a negative balance comes from.
 */
export function balanceLabel(balance_centimes: number): Key {
  return balance_centimes < 0 ? "customers_credit" : "customers_balance";
}

export function balanceShown(balance_centimes: number): string {
  return shownPositive(balance_centimes);
}

/**
 * The tag one customer earns, in the order the shop reads them: a balance
 * past the limit is the fact that stops a sale, and it outranks the others
 * even when the limit is zero. Then no credit at all (a zero limit, which is
 * not the same answer as no limit), then the warning threshold.
 *
 * features.md §2 names the thresholds; which one wins when two apply is a
 * decision this screen takes.
 */
export type StatusKey =
  | "status_over_limit"
  | "status_no_credit"
  | "status_near_limit"
  | "status_ok";

export function statusKey(customer: CustomerDto): StatusKey {
  const { balance_centimes, credit_limit_centimes, warn_threshold_centimes } = customer;
  if (credit_limit_centimes !== null && balance_centimes > credit_limit_centimes) {
    return "status_over_limit";
  }
  if (credit_limit_centimes === 0) return "status_no_credit";
  if (warn_threshold_centimes !== null && balance_centimes >= warn_threshold_centimes) {
    return "status_near_limit";
  }
  return "status_ok";
}

/**
 * The four states a customer account can be in, as a `Badge` rather than a
 * `StatusPill`: the kit's pill carries five words (issued, cancelled, paid,
 * open, low) and none of them is "plafond dépassé". The tones are the same
 * soft-surface roles the pill uses, so a tag on the ink theme is the ink
 * theme's version of the same idea and no code here learns which theme is on.
 */
const TONE: Readonly<Record<StatusKey, string>> = {
  status_over_limit: "bg-danger-soft text-fg-danger",
  status_near_limit: "bg-warn-soft text-warn",
  status_no_credit: "bg-muted text-muted-foreground",
  status_ok: "bg-primary-soft text-fg-success",
};

export function CustomerStatus({
  customer,
  className,
}: {
  customer: CustomerDto;
  className?: string;
}) {
  const { t } = useTranslation();
  const key = statusKey(customer);
  return (
    <Badge data-status={key} className={cn("border-transparent", TONE[key], className)}>
      {t(key)}
    </Badge>
  );
}

/**
 * A choice between two or three named things: the party kind, the payment
 * mode. It is a row of kit buttons wearing `role="radio"` inside a
 * `radiogroup` rather than `<input type="radio">`, which the kit's lint rule
 * refuses and which wears the browser's own dot on every theme.
 *
 * The accessible name of each option is its own label, so a test and a
 * screen reader both find "Entreprise" rather than "option 1 of 2".
 */
export function ChoiceRow<Value extends string>({
  label,
  options,
  value,
  onChange,
}: {
  label: string;
  readonly options: readonly { readonly value: Value; readonly label: string }[];
  value: Value;
  onChange: (next: Value) => void;
}) {
  const id = useId();
  return (
    <div className="flex flex-col gap-1.5 text-start">
      <span id={id} className="text-sm font-medium text-foreground">
        {label}
      </span>
      <div role="radiogroup" aria-labelledby={id} className="flex flex-wrap gap-2">
        {options.map((option) => (
          <Button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={value === option.value}
            variant={value === option.value ? "secondary" : "outline"}
            onClick={() => onChange(option.value)}
          >
            {option.label}
          </Button>
        ))}
      </div>
    </div>
  );
}

/**
 * A field validator answers with a translation key, never a sentence, and
 * `FormField` wants the sentence. This is the one place the two meet.
 */
export function useFieldError(): (messages: readonly unknown[]) => string | undefined {
  const { t } = useTranslation();
  return (messages) => fieldErrorMessage(messages, t);
}
