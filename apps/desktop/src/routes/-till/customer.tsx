// Who the basket is for: the fiche, what that fiche already owes, the kind of
// document the sale will be, and the refusal a facture makes when one of the
// two party blocks is short of an identifier.
//
// Every figure here is the core's. The balance and the limit come off the
// fiche, the side and the fields come off the refusal; the screen decides
// none of them (architecture.md rule 2).

import { Link } from "@tanstack/react-router";
import { ApiError, formatCentimes } from "@dzpos/shared";
import type { CustomerDto, SaleKindDto } from "@dzpos/shared";

import { useTranslation, type Key } from "@/i18n";

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
    <>
      <CustomerPicker
        picked={picked}
        rows={rows}
        search={search}
        onSearch={onSearch}
        onPick={onPick}
      />

      <fieldset className="flex flex-wrap gap-3 border-0 p-0">
        <legend className="mb-1">{t("till_kind")}</legend>
        <KindChoice kind="ticket" current={kind} label={t("till_ticket")} onPick={onKind} />
        {/* Loi 04-02 art. 10 decides the paper by who the buyer is, and
            décret 05-468 art. 3 puts that buyer on it, so the choice is
            there once a fiche is picked and not before. */}
        <KindChoice
          kind="facture"
          current={kind}
          label={t("till_facture")}
          title={picked === null ? t("till_facture_needs_customer") : undefined}
          disabled={picked === null}
          onPick={onKind}
        />
        {/* A quotation is the same basket priced and nothing else: it is
            made out to a customer the way a facture is, so it appears on
            the same terms, and it moves neither stock nor debt
            (features.md §3). */}
        <KindChoice
          kind="proforma"
          current={kind}
          label={t("till_kind_proforma")}
          title={picked === null ? t("till_proforma_needs_customer") : undefined}
          disabled={picked === null}
          onPick={onKind}
        />
      </fieldset>
    </>
  );
}

/** Who the sale is for. "Walk-in" is the default and stays the first choice:
 * most baskets at a till belong to nobody in particular, and a cashier must
 * not have to unpick a customer to sell to one.
 *
 * Only active fiches are offered. A closed fiche is one the shop has stopped
 * doing business with, and the core refuses a sale to it; offering it here
 * would be a choice that always fails.
 *
 * The picked fiche is kept whole by the parent, so narrowing the search does
 * not unpick it. It is added back to the options when the search has pushed
 * it out, or the select would show a blank row for a customer who is there.
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
  const active = rows.filter((c) => c.active);
  const options =
    picked !== null && !active.some((c) => c.id === picked.id) ? [picked, ...active] : active;
  const alert = standing(picked);
  return (
    <div className="flex flex-col gap-2 border-b pb-3">
      <label className="flex flex-col gap-1">
        <span>{t("till_customer")}</span>
        <input
          type="search"
          className="rounded border px-2 py-1"
          aria-label={t("till_customer_search")}
          placeholder={t("customers_search_hint")}
          value={search}
          onChange={(e) => onSearch(e.target.value)}
        />
      </label>
      <select
        className="rounded border px-2 py-1"
        aria-label={t("till_customer")}
        value={picked === null ? "" : String(picked.id)}
        onChange={(e) => {
          const id = Number(e.target.value);
          onPick(options.find((c) => c.id === id) ?? null);
        }}
      >
        <option value="">{t("till_walk_in")}</option>
        {options.map((c) => (
          <option key={c.id} value={c.id}>
            {c.name}
          </option>
        ))}
      </select>
      {picked !== null ? (
        <p className="flex items-center justify-between gap-2 text-sm">
          <span>{t("customers_balance")}</span>
          <span data-testid="till-customer-balance" className="font-mono" dir="ltr">
            {formatCentimes(picked.balance_centimes)}
          </span>
        </p>
      ) : null}
      {alert !== null && picked !== null ? (
        <p
          role="status"
          data-testid="till-limit-banner"
          className={alert === "over" ? "text-sm text-red-700" : "text-sm text-amber-700"}
        >
          {`${t(alert === "over" ? "till_over_limit" : "till_near_limit")} · ${formatCentimes(
            picked.balance_centimes,
          )} / ${
            picked.credit_limit_centimes === null
              ? t("till_no_limit")
              : formatCentimes(picked.credit_limit_centimes)
          }`}
        </p>
      ) : null}
    </div>
  );
}

function KindChoice({
  kind,
  current,
  label,
  title,
  disabled = false,
  onPick,
}: {
  kind: SaleKindDto;
  current: SaleKindDto;
  label: string;
  title?: string;
  disabled?: boolean;
  onPick: (kind: SaleKindDto) => void;
}) {
  return (
    <label className="flex items-center gap-2" title={title}>
      <input
        type="radio"
        name="sale_kind"
        value={kind}
        checked={current === kind}
        disabled={disabled}
        onChange={() => onPick(kind)}
      />
      <span>{label}</span>
    </label>
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
    <div
      role="alert"
      data-testid="till-party-ids"
      className="flex flex-col gap-2 rounded border border-red-700 p-3"
    >
      <strong className="text-red-700">{t("error_party_ids")}</strong>
      <p>{t(seller ? "till_party_ids_seller" : "till_party_ids_buyer")}</p>
      <ul data-testid="till-party-ids-missing" className="list-disc ps-5">
        {refusal.missing.map((field) => (
          <li key={field}>{t(field)}</li>
        ))}
      </ul>
      <Link to={seller ? "/settings" : "/customers"} className="underline">
        {t(seller ? "till_party_ids_settings" : "till_party_ids_customer")}
      </Link>
    </div>
  );
}
