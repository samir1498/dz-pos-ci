// The two papers a customer's account can print: the statement over a
// range, and the debt slip for right now. Both are pages the core rendered,
// dropped into a sandboxed iframe as they came: the shop is looking at what
// the printer will put on paper rather than at a second rendering of the
// same balances.

import { useQuery } from "@tanstack/react-query";
import type { CustomerDto } from "@dzpos/shared";
import { useState } from "react";

import { FormField } from "@/components/FormField";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader } from "@/components/ui/card";
import { DateField } from "@/components/ui/date-field";
import { Skeleton } from "@/components/ui/skeleton";
import { api, customerDebtSlipQueryKey, customerStatementQueryKey } from "@/api";
import { useTranslation } from "@/i18n";
import { useShopToday } from "@/lib/clock";
import { errorKey } from "@/lib/fields";

export function StatementPanel({ customer }: { customer: CustomerDto }) {
  const { t } = useTranslation();
  const clock = useShopToday();

  return (
    <Card>
      <CardHeader>
        <h3 className="text-md leading-none font-semibold">{t("customers_statement")}</h3>
        <CardDescription>{t("customers_statement_hint")}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        {/* The range defaults to the shop's own day, which the server owns,
            so the fields wait for it rather than opening on the browser's.
            A refusal is said out loud with a way to ask again: waiting is
            what a call in flight looks like, not what a failed one does. */}
        {clock.error !== null ? (
          <>
            <p role="alert" className="text-sm text-fg-danger">
              {t(errorKey(clock.error))}
            </p>
            <div>
              <Button variant="outline" onClick={clock.retry}>
                {t("action_retry")}
              </Button>
            </div>
          </>
        ) : clock.today === undefined ? (
          <Skeleton className="h-10 w-full" />
        ) : (
          <StatementRange customer={customer} today={clock.today} />
        )}
      </CardContent>
    </Card>
  );
}

/** The range and the page it asks for, once the shop's day is known. */
function StatementRange({ customer, today }: { customer: CustomerDto; today: string }) {
  const { t, lang } = useTranslation();
  const [from, setFrom] = useState(`${today.slice(0, 4)}-01-01`);
  const [to, setTo] = useState(today);
  const [asked, setAsked] = useState<{ from: string; to: string } | null>(null);
  const backwards = from > to;

  const statement = useQuery({
    queryKey: customerStatementQueryKey(customer.id, asked?.from ?? "", asked?.to ?? "", lang),
    queryFn: () => api.customerStatement(customer.id, asked?.from ?? "", asked?.to ?? "", lang),
    enabled: asked !== null,
  });

  return (
    <>
      <div className="flex flex-wrap items-end gap-3">
        <FormField label={t("field_statement_from")}>
          {(parts) => (
            <DateField {...parts} data-testid="statement-from" value={from} onChange={setFrom} />
          )}
        </FormField>
        <FormField label={t("field_statement_to")}>
          {(parts) => <DateField {...parts} data-testid="statement-to" value={to} onChange={setTo} />}
        </FormField>
        <Button
          variant="outline"
          disabled={backwards}
          onClick={() => setAsked(asked === null ? { from, to } : null)}
        >
          {asked === null ? t("action_statement") : t("action_statement_close")}
        </Button>
      </div>
      {/* Caught here as well as by the server: a range the wrong way round is
          a typing mistake, and a call that can only be refused is a call not
          worth making. */}
      {backwards ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t("error_statement_range_invalid")}
        </p>
      ) : null}
      {statement.isPending && asked !== null ? <Skeleton className="h-96 w-full" /> : null}
      {statement.isError ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(statement.error))}
        </p>
      ) : null}
      {asked !== null && statement.isSuccess ? (
        <iframe
          title={t("customers_statement_title")}
          srcDoc={statement.data}
          // An empty sandbox: the page carries no script and needs no origin,
          // so it cannot reach this one even if a customer name ever slipped
          // past the template's escaping.
          sandbox=""
          className="h-96 w-full rounded-lg border border-border bg-card"
          data-testid="customer-statement"
        />
      ) : null}
    </>
  );
}

/**
 * One button and no fields. The slip is about what the customer owes now, so
 * there is no range to pick, and the newest ten movements are the paper's
 * length rather than a choice the screen offers.
 */
export function DebtSlipPanel({ customer }: { customer: CustomerDto }) {
  const { t, lang } = useTranslation();
  const [open, setOpen] = useState(false);

  const slip = useQuery({
    queryKey: customerDebtSlipQueryKey(customer.id, lang),
    queryFn: () => api.customerDebtSlip(customer.id, lang),
    enabled: open,
  });

  return (
    <Card>
      <CardHeader>
        <h3 className="text-md leading-none font-semibold">{t("customers_debt_slip")}</h3>
        <CardDescription>{t("customers_debt_slip_hint")}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <div>
          <Button variant="outline" onClick={() => setOpen(!open)} data-testid="customer-debt-slip-button">
            {open ? t("action_debt_slip_close") : t("action_debt_slip")}
          </Button>
        </div>
        {slip.isPending && open ? <Skeleton className="h-96 w-full" /> : null}
        {slip.isError ? (
          <p role="alert" className="text-sm text-fg-danger">
            {t(errorKey(slip.error))}
          </p>
        ) : null}
        {open && slip.isSuccess ? (
          <iframe
            title={t("customers_debt_slip_title")}
            srcDoc={slip.data}
            // An empty sandbox, for the reason the statement's carries one:
            // the page has no script and needs no origin.
            sandbox=""
            className="h-96 w-full rounded-lg border border-border bg-card"
            data-testid="customer-debt-slip"
          />
        ) : null}
      </CardContent>
    </Card>
  );
}
