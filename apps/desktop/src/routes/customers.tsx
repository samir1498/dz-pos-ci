// The customers screen: who the shop sells to and what each one owes.
// Everything it shows comes from the API over HTTP and the balance is the
// core's, never added up here.
//
// The list is the whole screen. A row's name opens that customer's account
// (`/customers/{id}`), where the movements, the payments and the papers are;
// the row's own button opens the fiche panel, which is the form and nothing
// else. Two ways in, two different questions: "what has this customer done"
// and "what does the facture call them".
//
// Nothing on this screen deletes a customer: the ledger holds the fiche, so a
// shop that has stopped dealing with somebody clears the active box instead,
// and the till's picker then leaves them out.

import { Link, createFileRoute } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import type { CustomerDto } from "@dzpos/shared";
import { SquarePen, Users } from "lucide-react";
import { useState } from "react";

import { DataTable, type Column } from "@/components/DataTable";
import { EmptyState } from "@/components/EmptyState";
import { FormField } from "@/components/FormField";
import { Icon } from "@/components/Icon";
import { Money } from "@/components/Money";
import { PageHeader } from "@/components/PageHeader";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { api, customersQueryKey } from "@/api";
import { useTranslation } from "@/i18n";
import { errorKey } from "@/lib/fields";

import { CustomerFicheSheet } from "./-customers/fiche";
import { CustomerStatus } from "./-customers/parts";

export const Route = createFileRoute("/customers")({ component: CustomersScreen });

export function CustomersScreen() {
  const { t } = useTranslation();
  const [search, setSearch] = useState("");
  // One panel, two jobs: "new" is the blank fiche, a customer is that
  // customer's own.
  const [open, setOpen] = useState<"new" | CustomerDto | null>(null);
  const customers = useQuery({
    queryKey: [...customersQueryKey, search.trim()],
    queryFn: () => api.listCustomers(search),
  });

  // The panel reads the row from the list, so it opens on the balance the
  // last answer carried rather than the one it was opened with.
  const opened =
    open === null || open === "new"
      ? open
      : (customers.data?.find((c) => c.id === open.id) ?? open);

  const add = (
    <Button onClick={() => setOpen("new")}>
      <Icon as={Users} size={18} />
      {t("customers_add")}
    </Button>
  );

  const columns: readonly Column<CustomerDto>[] = [
    {
      id: "name",
      // `field_name`, not `col_name`: the catalogue's column is a
      // "désignation", which is a thing on a shelf and not a person or a
      // company.
      header: t("field_name"),
      cell: (customer) => (
        <div className="flex flex-wrap items-center gap-2">
          <Link
            to="/customers/$id"
            params={{ id: String(customer.id) }}
            className="font-medium text-foreground underline-offset-4 hover:underline"
          >
            {customer.name}
          </Link>
          {customer.active ? null : (
            <Badge variant="secondary">{t("customers_inactive")}</Badge>
          )}
        </div>
      ),
    },
    {
      id: "phone",
      header: t("col_phone"),
      // `dir="ltr"` on the number itself, not on the cell: a phone reads left
      // to right with Western digits whatever the screen's language, and the
      // cell keeps its own padding on the reading side.
      cell: (customer) =>
        customer.phone === null ? null : (
          <span dir="ltr" className="font-numeric text-muted-foreground">
            {customer.phone}
          </span>
        ),
    },
    {
      id: "debt",
      header: t("col_debt"),
      money: true,
      cell: (customer) => (
        <span className="inline-flex items-center gap-2">
          {customer.balance_centimes < 0 ? (
            <Badge variant="outline">{t("customers_credit")}</Badge>
          ) : null}
          <Money centimes={Math.abs(customer.balance_centimes)} />
        </span>
      ),
    },
    {
      id: "limit",
      header: t("col_limit"),
      money: true,
      cell: (customer) =>
        customer.credit_limit_centimes === null ? null : (
          <Money centimes={customer.credit_limit_centimes} className="text-muted-foreground" />
        ),
    },
    {
      id: "status",
      header: t("col_status"),
      cell: (customer) => <CustomerStatus customer={customer} />,
    },
  ];

  return (
    <section className="flex flex-col gap-4">
      <PageHeader
        title={t("customers_title")}
        description={t("customers_subtitle")}
        actions={add}
      />

      <FormField label={t("customers_search")} className="max-w-sm">
        {(parts) => (
          <Input
            {...parts}
            type="search"
            placeholder={t("customers_search_hint")}
            value={search}
            onChange={(event) => setSearch(event.target.value)}
          />
        )}
      </FormField>

      {customers.isPending ? (
        <div className="flex flex-col gap-2">
          <span className="sr-only">{t("customers_loading")}</span>
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
        </div>
      ) : null}
      {customers.isError ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(customers.error))}
        </p>
      ) : null}
      {customers.isSuccess ? (
        <DataTable
          columns={columns}
          rows={customers.data}
          rowKey={(customer) => customer.id}
          caption={t("customers_title")}
          // No action of its own: the one button that opens a blank fiche is
          // already in the header, a hand's width above, and a second copy of
          // it would be two places to look for one thing.
          empty={
            <EmptyState
              icon={Users}
              title={t("customers_empty")}
              description={t("customers_empty_hint")}
            />
          }
          actions={(customer) => (
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label={`${t("customers_edit")} ${customer.name}`}
              onClick={() => setOpen(customer)}
            >
              <Icon as={SquarePen} size={18} />
            </Button>
          )}
        />
      ) : null}

      <CustomerFicheSheet
        open={opened !== null}
        initial={opened === "new" || opened === null ? null : opened}
        onOpenChange={(next) => {
          if (!next) setOpen(null);
        }}
      />
    </section>
  );
}
