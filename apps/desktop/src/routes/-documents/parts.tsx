// Small pieces the documents list and the open row both read: how a refusal
// is shown, the day a document carries, and how a reversal is settled.
// Nothing here decides what to show, only how to say it — and nothing here
// works out an amount, which is the rule the refund block below is written
// against.

import type { RefundDto, SaleDto } from "@dzpos/shared";

import { ChoiceRow } from "@/components/ChoiceRow";
import { Money } from "@/components/Money";
import { useTranslation } from "@/i18n";
import { errorKey } from "@/lib/fields";

/** The refusal, wherever one is shown. One shape, so a failed list, a failed
 *  avoir and a failed sheet all read the same and all wear the danger role
 *  rather than a colour picked per call site. */
export function Refusal({ error }: { error: unknown }) {
  const { t } = useTranslation();
  return (
    <p role="alert" className="text-sm text-fg-danger">
      {t(errorKey(error))}
    </p>
  );
}

/** The day a document was issued, as the list column and the detail card
 *  both show it. The API answers `YYYY-MM-DD HH:MM:SS` already on the shop's
 *  calendar, so the day is the first ten characters and never a `Date` this
 *  screen builds: a parse here would drag the browser's timezone into a
 *  figure the core already decided (features.md, the shop's clock). */
export function day(issuedAt: string): string {
  return issuedAt.slice(0, 10);
}

/** How the shop settles a reversal, as the two dialogs ask it.
 *
 *  `ledger` is the name for "no notes move", which is the account credit on a
 *  document naming a customer and simply nothing at all on an anonymous
 *  ticket. One value for both because the wire has one: the `refund` field is
 *  left out, which is what every caller sent before the field existed
 *  (`CancelDocumentDto`, `NewAvoirDto`). */
export type Settlement = "cash" | "ledger";

/** The `refund` field as the two bodies carry it. `RefundDto` is a bare
 *  string on the wire (`export type RefundDto = "cash"`), not an object with
 *  a kind, so this is the whole of the translation and there is no assertion
 *  anywhere in it. */
export function refundField(settlement: Settlement): { refund?: RefundDto } {
  return settlement === "cash" ? { refund: "cash" } : {};
}

/** Whether notes may come out of the drawer for this paper at all.
 *
 *  A credit sale's money went on the customer's account and comes off it
 *  there; notes on top would hand it back twice, and
 *  `services::cancellation` refuses the body outright. The screen asks the
 *  same question so the option is never offered rather than offered and
 *  refused — and it asks it of `payment_mode` alone, which is the field the
 *  server's own guard reads. */
export function cashMayGoBack(document: SaleDto): boolean {
  return document.payment_mode !== "credit";
}

/**
 * How the money goes back, on the cancel dialog and on the avoir dialog.
 *
 * Three shapes, decided by the paper and never by what is convenient:
 *
 * - a credit sale offers nothing to choose. The row is replaced by the
 *   sentence the server would answer with, so a cashier reads the reason
 *   here rather than as a refusal after confirming.
 * - a cash or card sale naming a customer chooses between notes and the
 *   account.
 * - a cash or card sale naming nobody chooses between notes and nothing at
 *   all: there is no account to credit, and a mis-rung ticket the customer
 *   already walked away from is undone without the drawer moving.
 *
 * `ledger` is the default in all three, because absent is what the wire
 * carried before this choice existed and a dialog that hands money over
 * unless somebody notices is the wrong way round.
 *
 * **The figure, and why only one dialog carries it.** Neither dialog works
 * out what leaves the drawer; that subtraction is the core's
 * (`services::cancellation::cash_going_back`).
 *
 * A cancellation hands back the paper the cashier is holding, so `figures`
 * shows the two stored totals the rule is written on — what the customer
 * handed over, and the droit de timbre inside it that never comes back
 * (features.md §1, "Cash handed back") — with the hint that a credit note
 * already written lowers it.
 *
 * An avoir hands back its own `net_to_pay`, which carries no stamp at all
 * and depends on the lines somebody is about to type. The facture's totals
 * are the wrong figure there from the first avoir onwards, not merely after
 * a partial one, so `AvoirPanel` passes no `figures` and the dialog says in
 * words that the credit note's own amount is what comes back. A preview of
 * the exact figure is a number only the server can give, and it is the
 * follow-up named in the plan's T6 paragraph.
 */
export function RefundChoice({
  document,
  value,
  onChange,
  figures,
}: {
  document: SaleDto;
  value: Settlement;
  onChange: (next: Settlement) => void;
  /** Left out by a dialog whose amount is not this document's totals. */
  figures?: boolean;
}) {
  const { t } = useTranslation();

  if (!cashMayGoBack(document)) {
    return (
      <p
        data-testid="refund-credit-only"
        className="rounded-md bg-muted px-3 py-2 text-sm text-muted-foreground"
      >
        {t("documents_refund_credit_only")}
      </p>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      <ChoiceRow<Settlement>
        label={t("documents_refund")}
        value={value}
        onChange={onChange}
        options={[
          { value: "cash", label: t("documents_refund_cash") },
          {
            value: "ledger",
            label:
              document.customer_id === null
                ? t("documents_refund_nothing")
                : t("documents_refund_account"),
          },
        ]}
      />
      {value === "cash" && figures === true ? (
        <div data-testid="refund-figures" className="flex flex-col gap-1 text-sm">
          <p className="flex items-baseline justify-between gap-4">
            <span className="text-muted-foreground">{t("documents_refund_paid_in")}</span>
            <Money centimes={document.totals.net_to_pay_centimes} data-testid="refund-paid-in" />
          </p>
          {document.totals.stamp_centimes === 0 ? null : (
            <p className="flex items-baseline justify-between gap-4">
              <span className="text-muted-foreground">{t("documents_refund_stamp_kept")}</span>
              <Money centimes={document.totals.stamp_centimes} data-testid="refund-stamp-kept" />
            </p>
          )}
          <p className="text-xs text-muted-foreground">{t("documents_refund_hint")}</p>
        </div>
      ) : null}
      {value === "cash" && figures !== true ? (
        <p data-testid="refund-avoir-note" className="text-xs text-muted-foreground">
          {t("documents_refund_avoir_amount")}
        </p>
      ) : null}
    </div>
  );
}
