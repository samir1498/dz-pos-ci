// Who the basket is for: the fiche, what that fiche already owes, the kind of
// document the sale will be, and the refusal a facture makes when one of the
// two party blocks is short of an identifier.
//
// Every figure here is the core's. The balance and the limit come off the
// fiche, the side and the fields come off the refusal; the screen decides
// none of them (architecture.md rule 2).
//
// The picker was a `<select>` and is now the search box with its answers
// under it. Two reasons, one of each kind. A select on this kit is a Radix
// popover, which needs pointer capture and `scrollIntoView` and so cannot be
// driven in jsdom at all, and picking a customer is the first step of the
// credit, facture and proforma flows this screen is tested on. And at a
// counter the shop already types a name or a phone number into the box above
// it: the answers are what the shop asked for, so showing them as the list
// they are costs a cashier one look instead of one look and one click.

import { Link } from "@tanstack/react-router";
import { ApiError } from "@dzpos/shared";
import type { CustomerDto, SaleKindDto } from "@dzpos/shared";
import { Search, UserRound } from "lucide-react";

import { FormField } from "@/components/FormField";
import { Icon } from "@/components/Icon";
import { Money } from "@/components/Money";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { useTranslation, type Key } from "@/i18n";

import { Choice, ChoiceGroup } from "./choice";

/** A facture the party blocks refuse, as the server described it: which
 * half is short and of which identifiers. Both come off the wire; the screen
 * decides neither, the way it decides neither credit amount. */
export interface PartyRefusal {
  readonly side: "seller" | "buyer";
  readonly missing: readonly Key[];
}

/** The identifiers a facture can be short of, as their own labels. The
 * server sends the field names décret 05-468 art. 3 names; anything else is
 * a server this screen does not recognise, and the refusal then falls back
 * to its one line of text rather than showing a raw key. */
const PARTY_FIELD: Record<string, Key> = {
  rc: "field_rc",
  nis: "field_nis",
  name: "field_name",
  address: "field_address",
};

/** The refusal, if this error is one and named a side and fields the screen
 * knows. */
export function partyRefusal(error: unknown): PartyRefusal | null {
  if (!(error instanceof ApiError) || error.code !== "party_ids") return null;
  const { partySide, missingIds } = error;
  if (partySide !== "seller" && partySide !== "buyer") return null;
  if (missingIds === undefined || missingIds.length === 0) return null;
  const missing: Key[] = [];
  for (const field of missingIds) {
    const key = PARTY_FIELD[field];
    if (key === undefined) return null;
    missing.push(key);
  }
  return { side: partySide, missing };
}

/** What the picked customer's own standing says, before this basket. Over
 * the limit is what the shop has already let happen; near it is the warning
 * threshold reached. A customer with no limit is never either. */
function standing(customer: CustomerDto | null): "over" | "near" | null {
  if (customer === null) return null;
  const { balance_centimes: balance, credit_limit_centimes: limit } = customer;
  if (limit !== null && balance > limit) return "over";
  const warn = customer.warn_threshold_centimes;
  if (warn !== null && balance >= warn) return "near";
  return null;
}

/** Whether this customer may buy on credit at all: a limit of zero is no
 * credit, a null limit is no limit. Two different answers, and the screen
 * acts on them differently (features.md §1). */
export function takesCredit(customer: CustomerDto | null): boolean {
  if (customer === null) return false;
  const limit = customer.credit_limit_centimes;
  return limit === null || limit > 0;
}

/** The fiche and the document it decides. The two are one panel because the
 * paper follows the buyer: loi 04-02 art. 10 decides it by who they are and
 * décret 05-468 art. 3 puts them on it, so neither choice means anything
 * without the other. */
export function CustomerPanel({
  picked,
  rows,
  search,
  onSearch,
  onPick,
  kind,
  onKind,
}: {
  picked: CustomerDto | null;
  rows: CustomerDto[];
  search: string;
  onSearch: (value: string) => void;
  onPick: (customer: CustomerDto | null) => void;
  kind: SaleKindDto;
  onKind: (kind: SaleKindDto) => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col gap-3 border-b border-border pb-3">
      <CustomerPicker
        picked={picked}
        rows={rows}
        search={search}
        onSearch={onSearch}
        onPick={onPick}
      />

      <ChoiceGroup label={t("till_kind")}>
        <Choice
          checked={kind === "ticket"}
          label={t("till_ticket")}
          onPick={() => onKind("ticket")}
        />
        {/* Loi 04-02 art. 10 decides the paper by who the buyer is, and
            décret 05-468 art. 3 puts that buyer on it, so the choice is
            there once a fiche is picked and not before. */}
        <Choice
          checked={kind === "facture"}
          label={t("till_facture")}
          title={picked === null ? t("till_facture_needs_customer") : undefined}
          disabled={picked === null}
          onPick={() => onKind("facture")}
        />
        {/* A quotation is the same basket priced and nothing else: it is
            made out to a customer the way a facture is, so it appears on
            the same terms, and it moves neither stock nor debt
            (features.md §3). */}
        <Choice
          checked={kind === "proforma"}
          label={t("till_kind_proforma")}
          title={picked === null ? t("till_proforma_needs_customer") : undefined}
          disabled={picked === null}
          onPick={() => onKind("proforma")}
        />
      </ChoiceGroup>
    </div>
  );
}

/** Who the sale is for. Nobody in particular is the default and stays it:
 * most baskets at a till belong to a walk-in, and a cashier must not have to
 * unpick a customer to sell to one.
 *
 * Only active fiches are offered. A closed fiche is one the shop has stopped
 * doing business with, and the core refuses a sale to it; offering it here
 * would be a choice that always fails.
 *
 * The picked fiche is kept whole by the parent, so narrowing the search does
 * not unpick it: it is shown above the list rather than inside it, and the
 * way back to the walk-in is the button beside it.
 */
function CustomerPicker({
  picked,
  rows,
  search,
  onSearch,
  onPick,
}: {
  picked: CustomerDto | null;
  rows: CustomerDto[];
  search: string;
  onSearch: (value: string) => void;
  onPick: (customer: CustomerDto | null) => void;
}) {
  const { t } = useTranslation();
  const offered = rows.filter((c) => c.active && c.id !== picked?.id);
  const alert = standing(picked);
  return (
    <div className="flex flex-col gap-2">
      <FormField label={t("till_customer_search")}>
        {(parts) => (
          <div className="relative">
            <Icon
              as={Search}
              size={18}
              className="pointer-events-none absolute inset-y-0 start-3 my-auto text-faint"
            />
            <Input
              {...parts}
              type="search"
              className="ps-9"
              placeholder={t("customers_search_hint")}
              value={search}
              onChange={(event) => onSearch(event.target.value)}
            />
          </div>
        )}
      </FormField>

      {picked !== null ? (
        <Card className="gap-2 p-3">
          <div className="flex items-center justify-between gap-2">
            <span className="font-medium">{picked.name}</span>
            <Button type="button" variant="ghost" size="sm" onClick={() => onPick(null)}>
              {t("till_walk_in")}
            </Button>
          </div>
          <p className="flex items-center justify-between gap-2 text-sm">
            <span className="text-muted-foreground">{t("customers_balance")}</span>
            <Money centimes={picked.balance_centimes} data-testid="till-customer-balance" />
          </p>
          {/* The balance against the limit, both as the fiche carries them
              and both through `Money`, so the two amounts a cashier compares
              are set in the same figures as every other amount on the screen.
              A fiche with no limit says so in words rather than showing an
              amount nobody set. */}
          {alert !== null ? (
            <p
              role="status"
              data-testid="till-limit-banner"
              className={alert === "over" ? "text-sm text-fg-danger" : "text-sm text-warn"}
            >
              {`${t(alert === "over" ? "till_over_limit" : "till_near_limit")} · `}
              {/* Balance over limit is one expression and reads left to
                  right whole. Each `Money` carries its own `dir` and the
                  slash between them sits in the sentence, which on an Arabic
                  till made two islands laid out right to left: the ceiling
                  came out on the left of the slash and the balance on its
                  right, so a cashier reading the figures the way figures are
                  written read the comparison backwards. */}
              <span dir="ltr" data-testid="till-limit-pair">
                <Money centimes={picked.balance_centimes} className="text-sm" />
                {" / "}
                {picked.credit_limit_centimes === null ? (
                  t("till_no_limit")
                ) : (
                  <Money centimes={picked.credit_limit_centimes} className="text-sm" />
                )}
              </span>
            </p>
          ) : null}
        </Card>
      ) : null}

      {/* A plain overflow rather than the kit's scroll area: the list is a
          few rows the server already narrowed, and Radix's scroller wants a
          ResizeObserver, which the shop's oldest webview and the test
          environment both do without. */}
      <div className="max-h-48 overflow-y-auto">
        <div role="group" aria-label={t("till_customer")} className="flex flex-col gap-1">
          {offered.length === 0 ? (
            <p className="text-sm text-muted-foreground">{t("till_no_customer")}</p>
          ) : null}
          {offered.map((customer) => (
            <Button
              key={customer.id}
              type="button"
              variant="ghost"
              className="justify-between font-normal"
              onClick={() => onPick(customer)}
            >
              <span className="flex items-center gap-2 truncate">
                <Icon as={UserRound} size={18} className="text-faint" />
                {customer.name}
              </span>
              <Money centimes={customer.balance_centimes} className="text-xs" />
            </Button>
          ))}
        </div>
      </div>
    </div>
  );
}

/** The facture the party blocks refuse, with the side and the fields the
 * server named and a way to go and fill them in. Two screens own the two
 * halves, so the link goes to the one that can fix this refusal and not to
 * a generic "settings". */
export function PartyIdsRefused({ refusal }: { refusal: PartyRefusal }) {
  const { t } = useTranslation();
  const seller = refusal.side === "seller";
  return (
    <Card role="alert" data-testid="till-party-ids" className="gap-2 border-line-danger p-3">
      <strong className="text-fg-danger">{t("error_party_ids")}</strong>
      <p>{t(seller ? "till_party_ids_seller" : "till_party_ids_buyer")}</p>
      <ul data-testid="till-party-ids-missing" className="list-disc ps-5">
        {refusal.missing.map((field) => (
          <li key={field}>{t(field)}</li>
        ))}
      </ul>
      <Link to={seller ? "/settings" : "/customers"} className="underline">
        {t(seller ? "till_party_ids_settings" : "till_party_ids_customer")}
      </Link>
    </Card>
  );
}
