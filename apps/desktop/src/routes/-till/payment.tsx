// The cash box: how the sale is paid, what was handed over, what comes back,
// and the refusal a credit limit makes.
//
// The screen refuses a cash sale below the net to pay before the core does,
// which is the same rule kept in two places on purpose: it keeps a cashier
// from posting a basket the core would reject, and the core still refuses it
// (the comment at the top of till.tsx).

import { Link } from "@tanstack/react-router";
import { ApiError, formatCentimes } from "@dzpos/shared";
import type { NewSaleDto, PaymentModeDto } from "@dzpos/shared";

import { useTranslation, type Key } from "@/i18n";

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
  tenderedText,
  onTendered,
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
  tenderedText: string;
  onTendered: (value: string) => void;
  showChange: boolean;
  change: number;
  problem: Key | null;
  refusal: CreditRefusal | null;
  onOverride: () => void;
  pending: boolean;
}) {
  const { t } = useTranslation();
  return (
    <>
      <fieldset className="flex flex-wrap gap-3 border-0 p-0">
        <legend className="mb-1">{t("payment_mode")}</legend>
        <PaymentChoice mode="cash" current={mode} label={t("pay_cash")} onPick={onMode} />
        <PaymentChoice mode="card" current={mode} label={t("pay_card")} onPick={onMode} />
        {/* On credit the sale goes on a customer's ledger, so the choice
            is there once a customer who may buy on credit is picked. A
            limit of zero is no credit at all (features.md §1). */}
        <PaymentChoice
          mode="credit"
          current={mode}
          label={t("pay_credit")}
          title={creditAllowed ? undefined : t("pay_credit_needs_customer")}
          disabled={!creditAllowed}
          onPick={onMode}
        />
      </fieldset>

      {mode === "cash" ? (
        <label className="flex flex-col gap-1">
          <span>{t("field_tendered")}</span>
          <input
            dir="ltr"
            inputMode="decimal"
            className="rounded border px-2 py-1 text-end font-mono"
            value={tenderedText}
            onChange={(e) => onTendered(e.target.value)}
          />
        </label>
      ) : null}
      {mode === "cash" && problem === null && showChange ? (
        <p className="flex items-center justify-between gap-2">
          <span>{t("till_change")}</span>
          <span data-testid="till-change" className="font-mono" dir="ltr">
            {formatCentimes(change)}
          </span>
        </p>
      ) : null}
      {problem !== null ? (
        <p role="alert" className="text-sm text-red-700">
          {t(problem)}
        </p>
      ) : null}

      {refusal !== null ? (
        <CreditRefused refusal={refusal} onOverride={onOverride} pending={pending} />
      ) : null}
    </>
  );
}

function PaymentChoice({
  mode,
  current,
  label,
  title,
  disabled = false,
  onPick,
}: {
  mode: PaymentModeDto;
  current: PaymentModeDto;
  label: string;
  title?: string;
  disabled?: boolean;
  onPick: (mode: PaymentModeDto) => void;
}) {
  return (
    <label className="flex items-center gap-2" title={title}>
      <input
        type="radio"
        name="payment_mode"
        value={mode}
        checked={current === mode}
        disabled={disabled}
        onChange={() => onPick(mode)}
      />
      <span>{label}</span>
    </label>
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
    <div role="alert" className="flex flex-col gap-2 rounded border border-red-700 p-3">
      <strong className="text-red-700">{t("error_credit_limit")}</strong>
      <p className="flex items-center justify-between gap-2">
        <span>{t("till_balance_after")}</span>
        <span data-testid="till-balance-after" className="font-mono" dir="ltr">
          {formatCentimes(refusal.balanceAfterCentimes)}
        </span>
      </p>
      <p className="flex items-center justify-between gap-2">
        <span>{t("field_credit_limit")}</span>
        <span data-testid="till-credit-limit" className="font-mono" dir="ltr">
          {formatCentimes(refusal.creditLimitCentimes)}
        </span>
      </p>
      <button
        type="button"
        className="rounded border px-3 py-1.5"
        disabled={pending}
        onClick={onOverride}
      >
        {t("till_override")}
      </button>
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
    </div>
  );
}
