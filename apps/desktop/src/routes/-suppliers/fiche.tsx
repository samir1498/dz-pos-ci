// One supplier's statement page, and the panel that edits or closes the
// fiche behind it. `/suppliers/{id}` opens `SupplierFiche` directly, and the
// list screen opens the same `SupplierSheet` from a row, so the fiche cannot
// read one way on the list and another on the statement. The statement
// itself is `-suppliers/statement.tsx`: the balance, the ledger, and the two
// dialogs that write a movement onto it. This file composes that card with
// the page header and reads from it, it does not build it.
//
// Nothing here deletes a supplier: the ledger and the orders hold the fiche,
// so a shop that has stopped buying from somebody closes it, which is the
// close block at the foot of the panel and not a checkbox on the form.
// Closing over an account that is still open asks for a reason, which the
// server records beside the balance.

import { Link } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useForm } from "@tanstack/react-form";
import { ArrowLeft, SquarePen } from "lucide-react";
import { useState } from "react";
import { ApiError } from "@dzpos/shared";
import type { NewSupplierDto, SupplierDto, SupplierWriteDto } from "@dzpos/shared";
import { api, supplierQueryKey, suppliersQueryKey } from "@/api";
import { FormField } from "@/components/FormField";
import { Icon } from "@/components/Icon";
import { Money } from "@/components/Money";
import { MoneyInput } from "@/components/MoneyInput";
import { PageHeader } from "@/components/PageHeader";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { useTranslation, type Key } from "@/i18n";
import { cleared, errorKey, positive } from "@/lib/fields";

import { supplierBalanceLabel } from "./parts";
import { SupplierStatement } from "./statement";

/**
 * One supplier's statement, which is what `/suppliers/{id}` opens. A purchase
 * screen has a supplier id and no fiche, and a link that dropped the operator
 * on the list with a search box to retype would be the shop doing the app's
 * work.
 *
 * The balance, the movements behind it, and the two dialogs that write one.
 * The fiche itself is the same panel the list opens, so the two ways in
 * cannot drift apart.
 */
export function SupplierFiche({ id }: { id: number }) {
  const { t } = useTranslation();
  const [sheetOpen, setSheetOpen] = useState(false);
  const supplier = useQuery({
    queryKey: supplierQueryKey(id),
    queryFn: () => api.getSupplier(id),
  });

  return (
    <div className="flex flex-col gap-4">
      {supplier.isPending ? (
        <p className="text-sm text-muted-foreground">{t("suppliers_loading")}</p>
      ) : null}
      {supplier.isError ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(errorKey(supplier.error))}
        </p>
      ) : null}
      {supplier.isSuccess ? (
        <>
          <PageHeader
            title={supplier.data.name}
            actions={
              <>
                <Button variant="ghost" asChild>
                  <Link to="/suppliers">
                    <Icon as={ArrowLeft} size={18} flip />
                    {t("action_back_to_suppliers")}
                  </Link>
                </Button>
                <Button variant="outline" onClick={() => setSheetOpen(true)}>
                  <Icon as={SquarePen} size={18} />
                  {t("suppliers_edit")}
                </Button>
              </>
            }
          />
          <SupplierStatement supplier={supplier.data} />
          <SupplierSheet
            supplier={supplier.data}
            open={sheetOpen}
            onOpenChange={setSheetOpen}
          />
        </>
      ) : null}
    </div>
  );
}

/** The fiche in a panel: the form, and under a rule the block that stops or
 *  resumes the buying. It slides in from the side the page is not read from,
 *  which `AppShell` computes the same way for the sidebar. */
export function SupplierSheet({
  supplier,
  open,
  onOpenChange,
}: {
  /** `null` is a blank fiche. */
  supplier: SupplierDto | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { t, dir } = useTranslation();
  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      {/* The side is physical on purpose: the panel's edge, its border and
          the half it slides in from have to agree, so the caller picks it
          from the page direction. */}
      <SheetContent
        side={dir === "rtl" ? "left" : "right"}
        data-testid="supplier-sheet"
        className="w-full gap-0 overflow-y-auto sm:max-w-lg"
      >
        <SheetHeader>
          <SheetTitle>{supplier === null ? t("suppliers_new") : supplier.name}</SheetTitle>
          <SheetDescription>{t("suppliers_fiche_hint")}</SheetDescription>
        </SheetHeader>
        <div className="flex flex-col gap-4 px-4 pb-6">
          <SupplierForm
            key={supplier === null ? "new" : supplier.id}
            initial={supplier}
            onDone={() => onOpenChange(false)}
          />
          {supplier === null ? null : (
            <>
              <Separator />
              <CloseBlock supplier={supplier} onDone={() => onOpenChange(false)} />
            </>
          )}
        </div>
      </SheetContent>
    </Sheet>
  );
}

/** What the fiche form holds while it is being filled. The opening debt is
 *  centimes or nothing at all; no float is made on the way through. */
interface FicheValues {
  name: string;
  phone: string;
  address: string;
  rc: string;
  nif: string;
  nis: string;
  ai: string;
  openingDebt: number | null;
  notes: string;
}

/**
 * The blank fiche and an existing one are the same form: the same fields and
 * the same request shape either way. The opening debt is the one difference,
 * and it is only on the blank one: it is a ledger movement, not a column, so
 * an edit that could set it would be a correction nobody could see.
 */
function SupplierForm({
  initial,
  onDone,
}: {
  initial: SupplierDto | null;
  onDone: () => void;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<Key | null>(null);

  const save = useMutation({
    mutationFn: (fiche: NewSupplierDto) =>
      // A blank fiche closes nothing, and `POST /suppliers` refuses a field
      // it does not know, so the create and the update send two shapes.
      initial === null ? api.createSupplier(fiche) : api.updateSupplier(initial.id, whole(fiche)),
    onSuccess: async () => {
      setServerError(null);
      await queryClient.invalidateQueries({ queryKey: suppliersQueryKey });
      if (initial !== null) {
        await queryClient.invalidateQueries({ queryKey: supplierQueryKey(initial.id) });
      }
      onDone();
    },
    onError: (error: unknown) => {
      // On the code and not on the field: a name too long is a `validation`
      // on `name` as well, and it is the one thing this message would be
      // wrong about.
      setServerError(
        error instanceof ApiError && error.code === "conflict"
          ? "error_supplier_name_taken"
          : errorKey(error),
      );
    },
  });

  const blank: FicheValues = {
    name: "",
    phone: "",
    address: "",
    rc: "",
    nif: "",
    nis: "",
    ai: "",
    openingDebt: null,
    notes: "",
  };

  const form = useForm({
    defaultValues:
      initial === null
        ? blank
        : {
            ...blank,
            name: initial.name,
            phone: initial.phone ?? "",
            address: initial.address ?? "",
            rc: initial.rc ?? "",
            nif: initial.nif ?? "",
            nis: initial.nis ?? "",
            ai: initial.ai ?? "",
            notes: initial.notes ?? "",
          },
    onSubmit: async ({ value }) => {
      // The rejection is swallowed on purpose: onError has already turned the
      // server's code into a translated message on the form.
      await save
        .mutateAsync({
          name: value.name,
          phone: cleared(value.phone),
          address: cleared(value.address),
          rc: cleared(value.rc),
          nif: cleared(value.nif),
          nis: cleared(value.nis),
          ai: cleared(value.ai),
          notes: cleared(value.notes),
          // A fiche keeps the state it is in: the close block below is what
          // changes it.
          active: initial === null ? true : initial.active,
          opening_debt_centimes: initial === null ? value.openingDebt : null,
        })
        .catch(() => undefined);
    },
  });

  /** One optional text field of the fiche: a phone, an identifier, a note. */
  const optionalText = (
    name: "phone" | "address" | "rc" | "nif" | "nis" | "ai" | "notes",
    label: string,
    ltr = false,
  ) => (
    <form.Field name={name}>
      {(field) => (
        <FormField label={label}>
          {(parts) => (
            <Input
              {...parts}
              dir={ltr ? "ltr" : undefined}
              className={ltr ? "font-numeric tabular-nums" : undefined}
              value={field.state.value}
              onChange={(e) => field.handleChange(e.target.value)}
            />
          )}
        </FormField>
      )}
    </form.Field>
  );

  return (
    <form
      noValidate
      className="flex flex-col gap-4"
      onSubmit={(e) => {
        e.preventDefault();
        void form.handleSubmit();
      }}
    >
      <form.Field
        name="name"
        validators={{
          onSubmit: ({ value }) => {
            const name = value.trim();
            if (name === "") return "error_name_required";
            // The bound the core stores under (`MAX_FIELD_CHARS`), held here
            // so a paste gone wrong is said in the field's own words rather
            // than making the round trip.
            return name.length > 200 ? "error_name_too_long" : undefined;
          },
        }}
      >
        {(field) => (
          <FormField label={t("field_name")} error={messageOf(field.state.meta.errors, t)}>
            {(parts) => (
              <Input
                {...parts}
                value={field.state.value}
                onChange={(e) => field.handleChange(e.target.value)}
                onBlur={field.handleBlur}
              />
            )}
          </FormField>
        )}
      </form.Field>

      {optionalText("phone", t("field_phone"), true)}
      {optionalText("address", t("field_address"))}
      {optionalText("rc", t("field_rc"), true)}
      {optionalText("nif", t("field_nif"), true)}
      {optionalText("nis", t("field_nis"), true)}
      {optionalText("ai", t("field_ai"), true)}

      {initial === null ? (
        <form.Field name="openingDebt">
          {(field) => (
            <FormField
              label={t("field_supplier_opening_debt")}
              hint={t("field_supplier_opening_debt_hint")}
            >
              {(parts) => (
                <MoneyInput
                  {...parts}
                  value={field.state.value}
                  onChange={field.handleChange}
                />
              )}
            </FormField>
          )}
        </form.Field>
      ) : null}

      {optionalText("notes", t("field_notes"))}

      {serverError !== null ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(serverError)}
        </p>
      ) : null}

      <div className="flex flex-wrap gap-2">
        <Button type="submit" disabled={save.isPending}>
          {save.isPending ? t("action_saving") : t("action_save")}
        </Button>
        <Button type="button" variant="ghost" onClick={onDone}>
          {t("action_cancel")}
        </Button>
      </div>
    </form>
  );
}

/**
 * Stopping, or resuming, the buying. The reason is asked for whenever the
 * screen can see the account is open, and the server asks for it again when
 * only it can see that an order is still unpaid at a nil balance; the refusal
 * names the `reason` field, so the box stays and the message goes with it.
 */
function CloseBlock({ supplier, onDone }: { supplier: SupplierDto; onDone: () => void }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [reason, setReason] = useState("");
  const [serverError, setServerError] = useState<Key | null>(null);
  const [reasonRefused, setReasonRefused] = useState(false);

  const change = useMutation({
    mutationFn: (input: { close: boolean }) =>
      input.close
        ? api.closeSupplier(supplier.id, { reason: cleared(reason) })
        : api.updateSupplier(supplier.id, {
            name: supplier.name,
            phone: supplier.phone,
            address: supplier.address,
            rc: supplier.rc,
            nif: supplier.nif,
            nis: supplier.nis,
            ai: supplier.ai,
            notes: supplier.notes,
            active: true,
            close_reason: null,
          }),
    onSuccess: async () => {
      setServerError(null);
      setReasonRefused(false);
      setReason("");
      await queryClient.invalidateQueries({ queryKey: suppliersQueryKey });
      await queryClient.invalidateQueries({ queryKey: supplierQueryKey(supplier.id) });
      // The panel steps out of the way: what changed is on the list and on
      // the statement behind it, and a panel left open over them hides the
      // answer to the thing that was just done.
      onDone();
    },
    onError: (error: unknown) => {
      setReasonRefused(error instanceof ApiError && error.field === "reason");
      setServerError(errorKey(error));
    },
  });

  if (!supplier.active) {
    return (
      <section className="flex flex-col gap-2">
        <h3 className="text-md font-semibold">{t("suppliers_closed")}</h3>
        <p className="text-sm text-muted-foreground">{t("suppliers_closed_still_pays")}</p>
        {serverError !== null ? (
          <p role="alert" className="text-sm text-fg-danger">
            {t(serverError)}
          </p>
        ) : null}
        <div>
          <Button
            type="button"
            variant="outline"
            disabled={change.isPending}
            onClick={() => void change.mutateAsync({ close: false }).catch(() => undefined)}
          >
            {t("action_reopen_supplier")}
          </Button>
        </div>
      </section>
    );
  }

  const asksForReason = supplier.balance_centimes !== 0 || reasonRefused;

  return (
    <section className="flex flex-col gap-2 rounded-lg border border-warn bg-warn-soft p-3">
      <h3 className="text-md font-semibold">{t("suppliers_close")}</h3>
      <p className="text-sm text-muted-foreground">{t("suppliers_close_hint")}</p>
      {asksForReason ? (
        <>
          <p className="flex items-center gap-2 text-sm">
            <span>{t(supplierBalanceLabel(supplier.balance_centimes))}</span>
            <Money centimes={positive(supplier.balance_centimes)} />
          </p>
          <FormField label={t("suppliers_close_reason")}>
            {(parts) => (
              <Input
                {...parts}
                data-testid="supplier-close-reason"
                value={reason}
                onChange={(e) => setReason(e.target.value)}
              />
            )}
          </FormField>
        </>
      ) : null}
      {serverError !== null ? (
        <p role="alert" className="text-sm text-fg-danger">
          {t(serverError)}
        </p>
      ) : null}
      <div>
        <Button
          type="button"
          variant="destructive"
          disabled={change.isPending}
          onClick={() => void change.mutateAsync({ close: true }).catch(() => undefined)}
        >
          {t("action_close_supplier")}
        </Button>
      </div>
    </section>
  );
}

/** A field validator returns a translation key, never a sentence. */
function messageOf(messages: unknown[], t: (key: Key) => string): string | undefined {
  const key = messages.find((m): m is string => typeof m === "string");
  if (key === undefined) return undefined;
  return isSupplierErrorKey(key) ? t(key) : t("error_unknown");
}

/** The keys this screen's own validators hand back. Spelled out rather than
 *  cast, so a key the dictionaries do not carry fails the parity test rather
 *  than printing itself on a fiche. */
const OWN_ERRORS: readonly Key[] = ["error_name_required", "error_name_too_long"];

function isSupplierErrorKey(value: string): value is Key {
  return OWN_ERRORS.some((key) => key === value);
}

/** The update body: the whole fiche without the opening debt, which is a
 *  create-only field, and with no close reason, which the close route
 *  carries. */
function whole(input: NewSupplierDto): SupplierWriteDto {
  const { opening_debt_centimes: _opening, ...fiche } = input;
  return { ...fiche, close_reason: null };
}
