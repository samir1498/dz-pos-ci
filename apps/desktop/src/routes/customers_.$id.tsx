// One customer's account, addressable. What the counter reads: what is owed
// now, every movement behind that figure, the payments and what each one
// settled, and the two papers the shop can hand over. Anything that names a
// customer links here: the till's credit refusal, the documents screen, and
// the list's own rows.
//
// The trailing underscore on `customers_` keeps this out from under the list
// screen's route rather than nesting inside it: `/customers` is a whole page
// of its own and not a layout with an outlet.
//
// No amount on this page is worked out here. The balance and the running
// column of the ledger are the core's (`services::debt`), the statement and
// the debt slip are pages the core rendered, and they go into an iframe as
// they came: the shop is looking at what the printer will put on paper
// rather than at a second rendering of the same balances.
//
// The ledger, the payments and the correction are `-customers/statement.tsx`;
// the two printed papers are `-customers/papers.tsx`; the create/edit sheet
// both this page and the list open is `-customers/fiche.tsx`.

import type { CustomerDto } from "@dzpos/shared";
import { useQuery } from "@tanstack/react-query";
import { Link, createFileRoute } from "@tanstack/react-router";
import { ArrowLeft, SquarePen } from "lucide-react";
import { useState } from "react";

import { Icon } from "@/components/Icon";
import { Money } from "@/components/Money";
import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { api, customerQueryKey } from "@/api";
import { useTranslation } from "@/i18n";
import { errorKey } from "@/lib/fields";

import { CustomerFicheSheet } from "./-customers/fiche";
import { DebtSlipPanel, StatementPanel } from "./-customers/papers";
import { CustomerStatus, balanceLabel } from "./-customers/parts";
import { AdjustPanel, Ledger, PaymentsPanel } from "./-customers/statement";

export const Route = createFileRoute("/customers_/$id")({ component: OneCustomer });

function OneCustomer() {
  const { id } = Route.useParams();
  // A path is text, and `/customers/abc` is a link somebody mistyped rather
  // than a customer this shop does not have. `Number` on it is NaN, which
  // would go to the API as `/customers/NaN` and come back a bad request; the
  // page answers it here instead, and says the one thing there is to say.
  const parsed = Number(id);
  if (!Number.isInteger(parsed)) return <NotACustomer />;
  return <CustomerFiche id={parsed} />;
}

function BackToList() {
  const { t } = useTranslation();
  return (
    <Button variant="ghost" asChild>
      <Link to="/customers">
        <Icon as={ArrowLeft} size={18} flip />
        {t("action_back_to_customers")}
      </Link>
    </Button>
  );
}

function NotACustomer() {
  const { t } = useTranslation();
  return (
    <section className="flex flex-col gap-4">
      <PageHeader title={t("customers_title")} actions={<BackToList />} />
      <p role="alert" className="text-sm text-fg-danger">
        {t("error_not_found")}
      </p>
    </section>
  );
}

export function CustomerFiche({ id }: { id: number }) {
  const { t } = useTranslation();
  const [editing, setEditing] = useState(false);
  const customer = useQuery({
    queryKey: customerQueryKey(id),
    queryFn: () => api.getCustomer(id),
  });

  if (customer.isPending) {
    return (
      <section className="flex flex-col gap-4">
        <PageHeader title={t("customers_title")} actions={<BackToList />} />
        <span className="sr-only">{t("customers_loading")}</span>
        <Skeleton className="h-24 w-full" />
        <Skeleton className="h-64 w-full" />
      </section>
    );
  }

  if (customer.isError) {
    return (
      <section className="flex flex-col gap-4">
        <PageHeader title={t("customers_title")} actions={<BackToList />} />
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(customer.error))}
        </p>
      </section>
    );
  }

  const shown = customer.data;
  /* The phone reads left to right with Western digits whatever the screen's
     language, the same rule the customers list and the suppliers list already
     hold. Without it the bidi algorithm takes each group of digits for a run
     of its own and lays the groups out right to left, so `0770 11 22 33` came
     out as `33 22 11 0770` on the Arabic fiche and a shop would have dialled
     it that way. */
  const said =
    shown.phone === null && shown.address === null ? undefined : (
      <>
        {shown.phone === null ? null : (
          <span dir="ltr" data-testid="customer-phone" className="font-numeric">
            {shown.phone}
          </span>
        )}
        {shown.phone === null || shown.address === null ? null : " · "}
        {/* An address is free text and can be in either script, so it is
            isolated rather than forced: `bdi` keeps whatever is inside it
            from reordering the line around it, and an Algerian address
            carries a lot number and a postcode that would otherwise come
            apart the way the phone did. */}
        {shown.address === null ? null : <bdi>{shown.address}</bdi>}
      </>
    );

  return (
    <section className="flex flex-col gap-6">
      <PageHeader
        title={shown.name}
        description={said}
        actions={
          <>
            <BackToList />
            <Button variant="outline" onClick={() => setEditing(true)}>
              <Icon as={SquarePen} size={18} />
              {t("customers_modify")}
            </Button>
          </>
        }
      />

      <Figures customer={shown} />
      <Ledger customer={shown} />
      <PaymentsPanel customer={shown} />
      <div className="grid gap-4 lg:grid-cols-2">
        <StatementPanel customer={shown} />
        <DebtSlipPanel customer={shown} />
      </div>
      <AdjustPanel customer={shown} />

      <CustomerFicheSheet open={editing} initial={shown} onOpenChange={setEditing} />
    </section>
  );
}

/** What is owed, what the till will allow, and where the warning starts. The
 *  three figures the counter asks for before it sells anything on credit. */
function Figures({ customer }: { customer: CustomerDto }) {
  const { t } = useTranslation();
  return (
    <div className="grid gap-4 sm:grid-cols-3">
      <Card data-testid="customer-balance">
        <CardHeader>
          <CardDescription>{t(balanceLabel(customer.balance_centimes))}</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-wrap items-center justify-between gap-2">
          <Money centimes={Math.abs(customer.balance_centimes)} className="text-2xl" />
          <CustomerStatus customer={customer} />
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardDescription>{t("col_limit")}</CardDescription>
        </CardHeader>
        <CardContent>
          {customer.credit_limit_centimes === null ? (
            <span className="text-md text-muted-foreground">{t("customers_no_limit")}</span>
          ) : (
            <Money centimes={customer.credit_limit_centimes} className="text-2xl" />
          )}
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          {/* The short form: the form's own label carries "(DA)", which a
              card whose whole content is an amount does not need. */}
          <CardDescription>{t("customers_warn_threshold")}</CardDescription>
        </CardHeader>
        <CardContent>
          {customer.warn_threshold_centimes === null ? (
            <span className="text-md text-muted-foreground">{t("customers_no_limit")}</span>
          ) : (
            <Money centimes={customer.warn_threshold_centimes} className="text-2xl" />
          )}
        </CardContent>
      </Card>
    </div>
  );
}
