// The fiche itself: who the customer is, what the facture will call them, and
// how far the till may let them go. It is a panel rather than a block on the
// page because it is a form the shop fills once and comes back to rarely,
// while the account under it is what the counter reads every day.
//
// The blank fiche and an existing one are the same form: the same fields and
// the same request shape either way. The opening debt is the one difference,
// and it is only on the blank one: it is a ledger movement, not a column, so
// an edit that could set it would be a correction nobody could see
// (features.md §2).

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useForm } from "@tanstack/react-form";
import { ApiError } from "@dzpos/shared";
import type { CustomerDto, CustomerWriteDto, NewCustomerDto, PartyKindDto } from "@dzpos/shared";
import { useId, useState } from "react";

import { ChoiceRow } from "@/components/ChoiceRow";
import { FormField } from "@/components/FormField";
import { MoneyInput } from "@/components/MoneyInput";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { customersQueryKey, api } from "@/api";
import { useTranslation, type Key } from "@/i18n";
import { cleared, errorKey } from "@/lib/fields";

import { PARTY_KEY, PARTY_KINDS, balanceLabel, balanceShown, useFieldError } from "./parts";

/** What the form holds while it is being filled. The three amounts are
 *  centimes or nothing at all: `MoneyInput` never makes a float and blank is
 *  not zero, because "no limit" and "a limit of nothing" are different
 *  answers and the till acts on them differently. */
interface FicheValues {
  name: string;
  partyKind: PartyKindDto;
  phone: string;
  address: string;
  rc: string;
  nif: string;
  nis: string;
  ai: string;
  creditLimit: number | null;
  warnThreshold: number | null;
  openingDebt: number | null;
  notes: string;
  active: boolean;
  closeReason: string;
}

function blank(): FicheValues {
  return {
    name: "",
    partyKind: "company",
    phone: "",
    address: "",
    rc: "",
    nif: "",
    nis: "",
    ai: "",
    creditLimit: null,
    warnThreshold: null,
    openingDebt: null,
    notes: "",
    active: true,
    closeReason: "",
  };
}

function filled(customer: CustomerDto): FicheValues {
  return {
    name: customer.name,
    partyKind: customer.party_kind,
    phone: customer.phone ?? "",
    address: customer.address ?? "",
    rc: customer.rc ?? "",
    nif: customer.nif ?? "",
    nis: customer.nis ?? "",
    ai: customer.ai ?? "",
    creditLimit: customer.credit_limit_centimes,
    warnThreshold: customer.warn_threshold_centimes,
    openingDebt: null,
    notes: customer.notes ?? "",
    active: customer.active,
    closeReason: "",
  };
}

/**
 * The panel. It slides in from the side opposite the reading direction, which
 * `AppShell` computes the same way for the navigation: the panel's edge, its
 * border and the half it comes in from all have to agree, so the side stays
 * physical and the direction decides it.
 */
export function CustomerFicheSheet({
  open,
  initial,
  onOpenChange,
}: {
  open: boolean;
  /** `null` is the blank fiche. */
  initial: CustomerDto | null;
  onOpenChange: (open: boolean) => void;
}) {
  const { t, dir } = useTranslation();
  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent
        side={dir === "rtl" ? "left" : "right"}
        className="w-full gap-0 sm:max-w-xl"
        data-testid="customer-fiche"
      >
        <SheetHeader className="border-b border-border">
          <SheetTitle>{initial === null ? t("customers_new") : initial.name}</SheetTitle>
          <SheetDescription>{t("customers_fiche_hint")}</SheetDescription>
        </SheetHeader>
        <CustomerForm
          key={initial === null ? "new" : initial.id}
          initial={initial}
          onDone={() => onOpenChange(false)}
        />
      </SheetContent>
    </Sheet>
  );
}

function CustomerForm({
  initial,
  onDone,
}: {
  initial: CustomerDto | null;
  onDone: () => void;
}) {
  const { t } = useTranslation();
  const said = useFieldError();
  const queryClient = useQueryClient();
  const activeId = useId();
  const [serverError, setServerError] = useState<Key | null>(null);
  // The server refuses a close over an open account without a reason
  // (features.md §2). The screen asks for one whenever it can see the account
  // is open, and this flag is for when it cannot: a fiche whose balance is
  // nil can still have a document asking to be paid, and the refusal is what
  // says so.
  const [reasonRefused, setReasonRefused] = useState(false);

  const save = useMutation({
    mutationFn: ({ close_reason, ...fiche }: NewCustomerDto & { close_reason: string | null }) =>
      // A blank fiche closes nothing, and `POST /customers` refuses a field
      // it does not know, so the reason only travels on the update.
      initial === null
        ? api.createCustomer(fiche)
        : api.updateCustomer(initial.id, whole({ ...fiche, close_reason })),
    onSuccess: async () => {
      setServerError(null);
      setReasonRefused(false);
      // The prefix covers the list and the one fiche the account page reads:
      // `customerQueryKey(id)` is `customersQueryKey` with the id after it.
      await queryClient.invalidateQueries({ queryKey: customersQueryKey });
      onDone();
    },
    onError: (error: unknown) => {
      setServerError(errorKey(error));
      setReasonRefused(error instanceof ApiError && error.field === "reason");
    },
  });

  const form = useForm({
    defaultValues: initial === null ? blank() : filled(initial),
    onSubmit: async ({ value }) => {
      // The rejection is swallowed on purpose: onError has already turned the
      // server's code into a translated message on the form.
      await save
        .mutateAsync({
          name: value.name,
          party_kind: value.partyKind,
          phone: cleared(value.phone),
          address: cleared(value.address),
          rc: cleared(value.rc),
          nif: cleared(value.nif),
          nis: cleared(value.nis),
          ai: cleared(value.ai),
          credit_limit_centimes: value.creditLimit,
          warn_threshold_centimes: value.warnThreshold,
          notes: cleared(value.notes),
          active: value.active,
          opening_debt_centimes: initial === null ? value.openingDebt : null,
          close_reason: cleared(value.closeReason),
        })
        .catch(() => undefined);
    },
  });

  /** One optional text field of the fiche: a phone, an identifier. Blank
   *  clears the column (`cleared` above). An identifier is set in the figure
   *  face and reads left to right in Arabic too, the way it is printed. */
  const optionalText = (
    name: "phone" | "address" | "rc" | "nif" | "nis" | "ai",
    label: string,
    figures = false,
  ) => (
    <form.Field name={name}>
      {(field) => (
        <FormField label={label}>
          {(parts) => (
            <Input
              {...parts}
              dir={figures ? "ltr" : undefined}
              className={figures ? "font-numeric" : undefined}
              value={field.state.value}
              onChange={(event) => field.handleChange(event.target.value)}
            />
          )}
        </FormField>
      )}
    </form.Field>
  );

  return (
    <form
      noValidate
      className="flex min-h-0 flex-1 flex-col"
      onSubmit={(event) => {
        event.preventDefault();
        void form.handleSubmit();
      }}
    >
      <div className="grid min-h-0 flex-1 gap-4 overflow-y-auto p-4 sm:grid-cols-2">
        <form.Field
          name="name"
          validators={{
            onSubmit: ({ value }) => (value.trim() === "" ? "error_name_required" : undefined),
          }}
        >
          {(field) => (
            // Not `required`: the kit puts a star inside the label, and the
            // label is then "Nom*", which is not what a test or a screen
            // reader asks for. The validator above refuses a blank one, and
            // the server refuses it again.
            <FormField
              label={t("field_name")}
              error={said(field.state.meta.errors)}
              className="sm:col-span-2"
            >
              {(parts) => (
                <Input
                  {...parts}
                  value={field.state.value}
                  onBlur={field.handleBlur}
                  onChange={(event) => field.handleChange(event.target.value)}
                />
              )}
            </FormField>
          )}
        </form.Field>

        {/* Asked for, never inferred from whether an RC was typed in: loi
            04-02 art. 10 decides ticket against facture by who the buyer is
            (features.md §2). */}
        <form.Field name="partyKind">
          {(field) => (
            <ChoiceRow
              label={t("field_party_kind")}
              value={field.state.value}
              onChange={field.handleChange}
              options={PARTY_KINDS.map((kind) => ({ value: kind, label: t(PARTY_KEY[kind]) }))}
            />
          )}
        </form.Field>

        {optionalText("phone", t("field_phone"), true)}
        {optionalText("address", t("field_address"))}
        {optionalText("rc", t("field_rc"), true)}
        {optionalText("nif", t("field_nif"), true)}
        {optionalText("nis", t("field_nis"), true)}
        {optionalText("ai", t("field_ai"), true)}

        <form.Field name="creditLimit">
          {(field) => (
            <FormField label={t("field_credit_limit")}>
              {(parts) => (
                <MoneyInput {...parts} value={field.state.value} onChange={field.handleChange} />
              )}
            </FormField>
          )}
        </form.Field>

        <form.Field name="warnThreshold">
          {(field) => (
            <FormField label={t("field_warn_threshold")}>
              {(parts) => (
                <MoneyInput {...parts} value={field.state.value} onChange={field.handleChange} />
              )}
            </FormField>
          )}
        </form.Field>

        {initial === null ? (
          <form.Field name="openingDebt">
            {(field) => (
              <FormField
                label={t("field_opening_debt")}
                hint={t("field_opening_debt_hint")}
                className="sm:col-span-2"
              >
                {(parts) => (
                  <MoneyInput {...parts} value={field.state.value} onChange={field.handleChange} />
                )}
              </FormField>
            )}
          </form.Field>
        ) : null}

        <form.Field name="notes">
          {(field) => (
            <FormField label={t("field_notes")} className="sm:col-span-2">
              {(parts) => (
                <Textarea
                  {...parts}
                  rows={3}
                  value={field.state.value}
                  onChange={(event) => field.handleChange(event.target.value)}
                />
              )}
            </FormField>
          )}
        </form.Field>

        <form.Field name="active">
          {(field) => (
            <div className="flex items-center gap-2 sm:col-span-2">
              <Checkbox
                id={activeId}
                checked={field.state.value}
                onCheckedChange={(next) => field.handleChange(next === true)}
              />
              <Label htmlFor={activeId}>{t("field_customer_active")}</Label>
            </div>
          )}
        </form.Field>

        {/* Closing a fiche says the shop has stopped trading with that
            customer, and doing it over an account that is still open is a
            decision rather than a tidy-up: the reason goes in the audit log
            beside the balance (features.md §2). The block appears when the
            balance says the account is open, and when the server says so
            about a document the screen cannot see. */}
        <form.Subscribe selector={(state) => state.values.active}>
          {(active) =>
            initial !== null &&
            initial.active &&
            !active &&
            (initial.balance_centimes !== 0 || reasonRefused) ? (
              <form.Field name="closeReason">
                {(field) => (
                  <div className="rounded-lg border border-warn bg-warn-soft p-3 sm:col-span-2">
                    <FormField
                      label={t("customers_close_reason")}
                      hint={`${t("customers_close_reason_hint")} ${t(balanceLabel(initial.balance_centimes))} ${balanceShown(initial.balance_centimes)}`}
                    >
                      {(parts) => (
                        <Input
                          {...parts}
                          data-testid="customer-close-reason"
                          value={field.state.value}
                          onChange={(event) => field.handleChange(event.target.value)}
                        />
                      )}
                    </FormField>
                  </div>
                )}
              </form.Field>
            ) : null
          }
        </form.Subscribe>

        {serverError === null ? null : (
          <p role="alert" className="text-sm text-fg-danger sm:col-span-2">
            {t(serverError)}
          </p>
        )}
      </div>

      <SheetFooter className="flex-row justify-end border-t border-border">
        <Button type="button" variant="ghost" onClick={onDone}>
          {t("action_cancel")}
        </Button>
        <Button type="submit" disabled={save.isPending}>
          {save.isPending ? t("action_saving") : t("action_save")}
        </Button>
      </SheetFooter>
    </form>
  );
}

/** The update body: the whole fiche without the opening debt, which is a
 *  create-only field, plus the reason a close over an open account needs. */
function whole(input: NewCustomerDto & { close_reason: string | null }): CustomerWriteDto {
  const { opening_debt_centimes: _opening, ...fiche } = input;
  return fiche;
}
