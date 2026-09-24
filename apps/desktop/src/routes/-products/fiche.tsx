// The fiche: add and edit are one form, on a `Sheet` that slides in over the
// list rather than pushing it down. Labels. The one product's label belongs
// here and renders inside the sheet, under the form; the sheet of labels for
// several ticked rows is a dialog the route file opens over the list, off
// `LabelPanel`'s other state.

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useForm } from "@tanstack/react-form";
import { formatQty, parseQtyToMilli } from "@dzpos/shared";
import type { CategoryDto, NewProductDto, ProductDto } from "@dzpos/shared";
import type { UnitDto } from "@dzpos/shared";
import { Printer } from "lucide-react";
import { useState } from "react";

import { api, productsQueryKey } from "@/api";
import { FormField } from "@/components/FormField";
import { Icon } from "@/components/Icon";
import { LabelPanel, type LabelAsk } from "@/components/LabelPanel";
import { MoneyInput } from "@/components/MoneyInput";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useTranslation, type Key } from "@/i18n";
import { errorKey, fieldErrorMessage } from "@/lib/fields";
import { useHasPermission } from "@/lib/session";

import { CONTENANCE_SYMBOL, CONTENANCE_UNITS, NO_CONTENANCE, readContenance } from "./contenance";
import { NO_CATEGORY, UNITS, UNIT_KEY, rateOptions } from "./parts";

const FALLBACK_RATE_BPS = 1900;

/**
 * What the fiche holds while it is being typed. The three prices are integer
 * centimes or `null` for a blank field, because `MoneyInput` never makes a
 * float and "no amount" is not the same answer as zero. The two quantities
 * stay text: they are milli-units read by `parseQtyToMilli`, and "2,5" kg is
 * not two dinars fifty.
 */
interface FormValues {
  readonly name: string;
  readonly barcode: string;
  /** A category id as text, or `NO_CATEGORY`. */
  readonly category: string;
  /** Basis points as text, which is what a `SelectItem` value is. */
  readonly rate: string;
  readonly unit: string;
  readonly cost: number | null;
  readonly price: number | null;
  readonly wholesale: number | null;
  readonly stock: string;
  readonly lowStock: string;
  readonly active: boolean;
  /** The pack size, text like the other quantities (T13). */
  readonly contenance: string;
  /** One of `CONTENANCE_UNITS`, or `NO_CONTENANCE`. */
  readonly contenanceUnit: string;
}

/**
 * Add and edit are one form: the same fields, the same validators, one
 * request shape (`NewProductDto` is the whole product either way). With
 * `initial` the fields start from the stored row and the save is a PUT to
 * that row; without it they start blank and the save is a POST.
 *
 * The three prices are integer centimes in the form's own state, held by
 * `MoneyInput`; no float is ever made. The two quantities are milli-units
 * and stay text, because they go through `parseQtyToMilli` and not through
 * the money parser: a quantity of "2,5" kg is not two dinars fifty.
 */
export function ProductForm({
  categories,
  initial,
  onDone,
  label,
  onPrintLabel,
}: {
  categories: CategoryDto[];
  initial: ProductDto | null;
  onDone: () => void;
  /** The label this fiche is showing, if the shop asked for it. */
  label: LabelAsk | null;
  onPrintLabel: (id: number) => void;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<Key | null>(null);
  // GET /products stays open to every signed-in role (the till needs the
  // list to ring a sale up; route_gates.rs asserts the route itself carries
  // no row), but the server no longer hands cost to a caller who lacks
  // see_cost_and_margin: routes/products.rs::redact_cost nulls
  // cost_centimes and wholesale_centimes on the way out, so a cashier's
  // ProductDto genuinely carries null and not a value merely hidden here
  // (M4 T5 review, 2026-09-11). This still hides the field on the fiche for
  // everybody else, on top of that: a manager who briefly loses the
  // permission should not see a stale cost sitting in a cached list either.
  const seeCostAndMargin = useHasPermission("see_cost_and_margin");

  const save = useMutation({
    mutationFn: (input: NewProductDto) =>
      initial === null ? api.createProduct(input) : api.updateProduct(initial.id, input),
    onSuccess: async () => {
      setServerError(null);
      await queryClient.invalidateQueries({ queryKey: productsQueryKey });
      onDone();
    },
    onError: (error: unknown) => setServerError(errorKey(error)),
  });

  const first = categories[0];
  // Spelled out rather than inferred from the two branches below: the add
  // branch starts the three prices at `null` and the edit branch at a stored
  // integer, and the inferred union of the two makes `null` unassignable to a
  // field the fiche has to be able to empty.
  const defaults: FormValues =
    initial === null
      ? {
            name: "",
            barcode: "",
            category: first === undefined ? NO_CATEGORY : String(first.id),
            rate: String(first?.default_rate_bps ?? FALLBACK_RATE_BPS),
            unit: "piece",
            cost: null,
            price: null,
            wholesale: null,
            stock: "",
            lowStock: "",
            active: true,
            contenance: "",
            contenanceUnit: NO_CONTENANCE,
          }
        : {
            name: initial.name,
            barcode: initial.barcode ?? "",
            category: initial.category_id === null ? NO_CATEGORY : String(initial.category_id),
            rate: String(initial.rate_bps),
            unit: initial.unit,
            cost: initial.cost_centimes,
            price: initial.selling_centimes,
            wholesale: initial.wholesale_centimes,
            stock: formatQty(initial.qty_on_hand_milli),
            lowStock: initial.low_stock_at_milli === 0 ? "" : formatQty(initial.low_stock_at_milli),
            active: initial.active,
            contenance:
              initial.contenance_milli === null ? "" : formatQty(initial.contenance_milli),
            contenanceUnit: initial.contenance_unit ?? NO_CONTENANCE,
          };

  const form = useForm({
    defaultValues: defaults,
    onSubmit: async ({ value }) => {
      // The validators have already refused anything unreadable, so these
      // fall back only for the blank case they allow.
      const stock = optional(value.stock, parseQtyToMilli) ?? 0;
      const lowStock = optional(value.lowStock, parseQtyToMilli) ?? 0;
      const pack = readContenance(value.contenance, value.contenanceUnit);
      // The rejection is deliberately swallowed: onError has already turned
      // the server's code into a translated message on the form.
      await save
        .mutateAsync({
          name: value.name,
          barcode: value.barcode.trim() === "" ? null : value.barcode.trim(),
          category_id: value.category === NO_CATEGORY ? null : Number(value.category),
          unit: toUnit(value.unit),
          // A blank cost is zero; a blank wholesale price is "none", which
          // is a different answer from a price of nothing.
          cost_centimes: value.cost ?? 0,
          selling_centimes: value.price ?? 0,
          wholesale_centimes: value.wholesale,
          qty_on_hand_milli: stock,
          low_stock_at_milli: lowStock,
          // Sent, never left to the server to infer: the shop chose a rate on
          // this screen and the product has to carry the one it chose.
          rate_bps: Number(value.rate),
          active: value.active,
          contenance_milli: pack.kind === "set" ? pack.milli : null,
          contenance_unit: pack.kind === "set" ? pack.unit : null,
        })
        .catch(() => undefined);
    },
  });

  return (
    <form
      noValidate
      className="flex flex-1 flex-col gap-4 p-4"
      onSubmit={(e) => {
        e.preventDefault();
        void form.handleSubmit();
      }}
    >
      <form.Field
        name="name"
        validators={{
          onSubmit: ({ value }) => (value.trim() === "" ? "error_name_required" : undefined),
        }}
      >
        {(field) => (
          <FormField
            label={t("field_name")}
            required
            error={fieldErrorMessage(field.state.meta.errors, t)}
          >
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

      <form.Field name="barcode">
        {(field) => (
          <FormField
            label={t("field_barcode")}
            hint={initial === null ? t("field_barcode_hint") : undefined}
          >
            {(parts) => (
              <Input
                {...parts}
                dir="ltr"
                className="font-numeric tabular-nums"
                value={field.state.value}
                onChange={(e) => field.handleChange(e.target.value)}
              />
            )}
          </FormField>
        )}
      </form.Field>

      <form.Field name="category">
        {(field) => (
          <FormField label={t("field_category")}>
            {(parts) => (
              <Select
                value={field.state.value}
                onValueChange={(next) => {
                  field.handleChange(next);
                  // The category's rate is the offer, not the decision: the
                  // rate select below stays editable after this.
                  const chosen = categories.find((c) => String(c.id) === next);
                  if (chosen !== undefined) {
                    form.setFieldValue("rate", String(chosen.default_rate_bps));
                  }
                }}
              >
                <SelectTrigger id={parts.id} className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value={NO_CATEGORY}>{t("category_none")}</SelectItem>
                  {categories.map((c) => (
                    <SelectItem key={c.id} value={String(c.id)}>
                      {c.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
          </FormField>
        )}
      </form.Field>

      <form.Field name="rate">
        {(field) => (
          <FormField label={t("field_rate")}>
            {(parts) => (
              <Select value={field.state.value} onValueChange={field.handleChange}>
                <SelectTrigger id={parts.id} className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {rateOptions(categories, t, initial?.rate_bps).map((rate) => (
                    <SelectItem key={rate.value} value={rate.value}>
                      {rate.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
          </FormField>
        )}
      </form.Field>

      <form.Field name="unit">
        {(field) => (
          <FormField label={t("field_unit")}>
            {(parts) => (
              <Select value={field.state.value} onValueChange={field.handleChange}>
                <SelectTrigger id={parts.id} className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {UNITS.map((u) => (
                    <SelectItem key={u} value={u}>
                      {t(UNIT_KEY[u])}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
          </FormField>
        )}
      </form.Field>

      <div className="grid grid-cols-2 items-start gap-4">
        <form.Field
          name="contenance"
          validators={{
            onSubmit: ({ value, fieldApi }) =>
              readContenance(value, fieldApi.form.getFieldValue("contenanceUnit")).kind ===
              "incomplete"
                ? "error_contenance_incomplete"
                : undefined,
          }}
        >
          {(field) => (
            <FormField
              label={t("field_contenance")}
              hint={t("contenance_hint")}
              error={fieldErrorMessage(field.state.meta.errors, t)}
            >
              {(parts) => (
                <Input
                  {...parts}
                  dir="ltr"
                  inputMode="decimal"
                  autoComplete="off"
                  className="font-numeric tabular-nums"
                  value={field.state.value}
                  onChange={(e) => field.handleChange(e.target.value)}
                />
              )}
            </FormField>
          )}
        </form.Field>
        <form.Field name="contenanceUnit">
          {(field) => (
            <FormField label={t("field_contenance_unit")}>
              {(parts) => (
                <Select value={field.state.value} onValueChange={field.handleChange}>
                  <SelectTrigger id={parts.id} className="w-full">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value={NO_CONTENANCE}>{t("contenance_unit_none")}</SelectItem>
                    {CONTENANCE_UNITS.map((u) => (
                      <SelectItem key={u} value={u}>
                        {CONTENANCE_SYMBOL[u]}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}
            </FormField>
          )}
        </form.Field>
      </div>

      <form.Field
        name="price"
        validators={{
          onSubmit: ({ value }) => (value === null ? "error_price_invalid" : undefined),
        }}
      >
        {(field) => (
          <FormField
            label={t("field_price")}
            required
            error={fieldErrorMessage(field.state.meta.errors, t)}
          >
            {(parts) => (
              <MoneyInput {...parts} value={field.state.value} onChange={field.handleChange} />
            )}
          </FormField>
        )}
      </form.Field>

      {seeCostAndMargin ? (
        <form.Field name="cost">
          {(field) => (
            <FormField label={t("field_cost")} hint={t("field_blank_is_zero")}>
              {(parts) => (
                <MoneyInput {...parts} value={field.state.value} onChange={field.handleChange} />
              )}
            </FormField>
          )}
        </form.Field>
      ) : null}

      <form.Field name="wholesale">
        {(field) => (
          <FormField label={t("field_wholesale")} hint={t("field_blank_is_none")}>
            {(parts) => (
              <MoneyInput {...parts} value={field.state.value} onChange={field.handleChange} />
            )}
          </FormField>
        )}
      </form.Field>

      <form.Field
        name="stock"
        validators={{
          onSubmit: ({ value }) =>
            optional(value, parseQtyToMilli) === undefined ? "error_stock_invalid" : undefined,
        }}
      >
        {(field) => (
          <FormField
            label={t("field_stock")}
            error={fieldErrorMessage(field.state.meta.errors, t)}
          >
            {(parts) => (
              <Input
                {...parts}
                dir="ltr"
                inputMode="decimal"
                className="font-numeric tabular-nums text-end"
                value={field.state.value}
                // The ledger owns the quantity once the product exists; the
                // fiche shows it and the core ignores it on an update.
                readOnly={initial !== null}
                onChange={(e) => field.handleChange(e.target.value)}
              />
            )}
          </FormField>
        )}
      </form.Field>

      <form.Field
        name="lowStock"
        validators={{
          onSubmit: ({ value }) =>
            optional(value, parseQtyToMilli) === undefined ? "error_low_stock_invalid" : undefined,
        }}
      >
        {(field) => (
          <FormField
            label={t("field_low_stock")}
            error={fieldErrorMessage(field.state.meta.errors, t)}
          >
            {(parts) => (
              <Input
                {...parts}
                dir="ltr"
                inputMode="decimal"
                className="font-numeric tabular-nums text-end"
                value={field.state.value}
                onChange={(e) => field.handleChange(e.target.value)}
              />
            )}
          </FormField>
        )}
      </form.Field>

      <form.Field name="active">
        {(field) => (
          <FormField
            label={t("field_active")}
            className="flex-row-reverse items-center justify-end gap-2"
          >
            {(parts) => (
              <Checkbox
                id={parts.id}
                checked={field.state.value}
                onCheckedChange={(next) => field.handleChange(next === true)}
              />
            )}
          </FormField>
        )}
      </form.Field>

      {serverError !== null ? (
        <p role="alert" className="text-fg-danger">
          {t(serverError)}
        </p>
      ) : null}

      <div className="flex flex-wrap gap-2">
        <Button type="submit" disabled={save.isPending}>
          {save.isPending ? t("action_saving") : t("action_save")}
        </Button>
        <Button type="button" variant="outline" onClick={onDone}>
          {t("action_cancel")}
        </Button>
        {/* Only on a stored product: a label is a picture of a barcode, and
            a product being typed has no id to print one for. The refusal
            when it has no EAN-13 comes from the server, so the button is
            offered and the reason is read rather than guessed here. */}
        {initial === null ? null : (
          <Button
            type="button"
            variant="outline"
            data-testid="print-label"
            onClick={() => onPrintLabel(initial.id)}
          >
            <Icon as={Printer} size={18} />
            {t("action_print_label")}
          </Button>
        )}
      </div>

      {label === null ? null : (
        <Card>
          <CardContent>
            <LabelPanel ask={label} />
          </CardContent>
        </Card>
      )}
    </form>
  );
}

/**
 * A field a shop may leave empty. Blank is zero; anything the parser cannot
 * read is `undefined`, which is what blocks the submit. `?? 0` used to
 * flatten both cases, so "60 kg" was stored as a stock of nothing.
 */
function optional(text: string, parse: (t: string) => number | null): number | undefined {
  if (text.trim() === "") return 0;
  return parse(text) ?? undefined;
}

function toUnit(value: string): UnitDto {
  const found = UNITS.find((u) => u === value);
  return found ?? "piece";
}
