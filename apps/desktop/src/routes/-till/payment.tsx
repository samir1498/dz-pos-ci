// The cash box: how the sale is paid, what was handed over, what comes back,
// and the refusal a credit limit makes.
//
// The screen refuses a cash sale below the net to pay before the core does,
// which is the same rule kept in two places on purpose: it keeps a cashier
// from posting a basket the core would reject, and the core still refuses it
// (the comment at the top of till.tsx).
//
// The amount handed over is integer centimes from end to end. `MoneyInput`
// takes the typing and `Keypad` takes the thumb, and both hand back the same
// integer through `keyedAmount`: no float is made here and no string is
// parsed twice.

import { Link } from "@tanstack/react-router";
import { ApiError } from "@dzpos/shared";
import type { NewSaleDto, PaymentModeDto } from "@dzpos/shared";

import { FormField } from "@/components/FormField";
import { Keypad, keyedAmount, type KeypadKey } from "@/components/Keypad";
import { Money } from "@/components/Money";
import { MoneyInput } from "@/components/MoneyInput";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { useTranslation, type Key } from "@/i18n";

import { Choice, ChoiceGroup } from "./choice";

/** The credit refusal, with the two amounts the server sent. The screen
 * shows them and never works them out: what a customer owes has one answer
 * and it is the core's (architecture.md rule 2). */
export interface CreditRefusal {
  readonly balanceAfterCentimes: number;
  readonly creditLimitCentimes: number;
  readonly body: NewSaleDto;
}

/** The refusal, if this error is one and carried both amounts. An error that
 * says `credit_limit` and carries neither is a server the screen does not
 * recognise, and it falls back to the plain message. */
export function creditRefusal(error: unknown, body: NewSaleDto): CreditRefusal | null {
  if (!(error instanceof ApiError) || error.code !== "credit_limit") return null;
  const { balanceAfterCentimes, creditLimitCentimes } = error;
  if (balanceAfterCentimes === undefined || creditLimitCentimes === undefined) return null;
  return { balanceAfterCentimes, creditLimitCentimes, body };
}

/** The mode, the amount tendered, the change and the credit refusal.
 * `showChange` is the parent's answer to whether there is anything to give
 * back yet: an empty box on a cash sale is a cashier who has not counted the
 * notes, not a mistake. */
export function PaymentPanel({
  mode,
  onMode,
  creditAllowed,
  tendered,
  onTendered,
  onEnter,
  showChange,
  change,
  problem,
  refusal,
  onOverride,
  pending,
}: {
  mode: PaymentModeDto;
  onMode: (mode: PaymentModeDto) => void;
  creditAllowed: boolean;
  /** Centimes, or null for a box nobody has typed in yet. */
  tendered: number | null;
  onTendered: (centimes: number | null) => void;
  /** The pad's wide key: the same thing F9 and the brass button do. */
  onEnter: () => void;
  showChange: boolean;
  change: number;
  problem: Key | null;
  refusal: CreditRefusal | null;
  onOverride: () => void;
  pending: boolean;
}) {
  const { t } = useTranslation();

  function onKey(key: KeypadKey) {
    if (key === "enter") {
      onEnter();
      return;
    }
    onTendered(keyedAmount(tendered, key));
  }

  return (
    <>
      <ChoiceGroup label={t("payment_mode")}>
        <Choice
          checked={mode === "cash"}
          label={t("pay_cash")}
          onPick={() => onMode("cash")}
        />
        <Choice
          checked={mode === "card"}
          label={t("pay_card")}
          onPick={() => onMode("card")}
        />
        {/* On credit the sale goes on a customer's ledger, so the choice
            is there once a customer who may buy on credit is picked. A
            limit of zero is no credit at all (features.md §1). */}
        <Choice
          checked={mode === "credit"}
          label={t("pay_credit")}
          title={creditAllowed ? undefined : t("pay_credit_needs_customer")}
          disabled={!creditAllowed}
          onPick={() => onMode("credit")}
        />
      </ChoiceGroup>

      {mode === "cash" ? (
        <div className="flex flex-col gap-3">
          <FormField label={t("field_tendered")} error={problem === null ? undefined : t(problem)}>
            {(parts) => <MoneyInput {...parts} value={tendered} onChange={onTendered} />}
          </FormField>
          {/* The pad a thumb hits. It types the same integer the box above
              takes, so the two ways of counting the notes cannot disagree. */}
          <Keypad onKey={onKey} disabled={pending} />
        </div>
      ) : null}
      {mode === "cash" && problem === null && showChange ? (
        <p className="flex items-center justify-between gap-2">
          <span>{t("till_change")}</span>
          <Money centimes={change} data-testid="till-change" className="text-lg" />
        </p>
      ) : null}

      {refusal !== null ? (
        <CreditRefused refusal={refusal} onOverride={onOverride} pending={pending} />
      ) : null}
    </>
  );
}

/** The credit limit refusing the sale, in the two amounts the server sent.
 * The override button sends the same basket again with the flag on; the
 * server writes an audit row for it, and until roles arrive in M4 anyone at
 * the till may take that decision (features.md §1). */
function CreditRefused({
  refusal,
  onOverride,
  pending,
}: {
  refusal: CreditRefusal;
  onOverride: () => void;
  pending: boolean;
}) {
  const { t } = useTranslation();
  return (
    <Card role="alert" className="gap-2 border-line-danger p-3">
      <strong className="text-fg-danger">{t("error_credit_limit")}</strong>
      <p className="flex items-center justify-between gap-2">
        <span>{t("till_balance_after")}</span>
        <Money centimes={refusal.balanceAfterCentimes} data-testid="till-balance-after" />
      </p>
      <p className="flex items-center justify-between gap-2">
        <span>{t("field_credit_limit")}</span>
        <Money centimes={refusal.creditLimitCentimes} data-testid="till-credit-limit" />
      </p>
      <Button type="button" variant="outline" disabled={pending} onClick={onOverride}>
        {t("till_override")}
      </Button>
      {/* The other answer to this refusal, and the one a shop usually wants:
          take money off what the customer already owes. The link opens that
          customer's own fiche rather than the list, because the cashier is
          looking at a refusal that named them and should not have to type the
          name back into a search box. */}
      {refusal.body.customer_id !== null ? (
        <Link
          to="/customers/$id"
          params={{ id: String(refusal.body.customer_id) }}
          className="underline"
        >
          {t("till_open_fiche")}
        </Link>
      ) : null}
    </Card>
  );
}
