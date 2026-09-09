// The products screen. Everything it shows comes from the API over HTTP;
// the types are generated from the Rust structs (packages/shared).
// Every visible string goes through i18n and every box uses logical CSS
// properties, so Arabic mirrors the layout without a second stylesheet.

import { createFileRoute } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useForm } from "@tanstack/react-form";
import { useState } from "react";
import {
  ApiError,
  formatCentimes,
  formatQty,
  parseAmountToCentimes,
  parseQtyToMilli,
} from "@dzpos/shared";
import type { CategoryDto, NewProductDto, ProductDto, UnitDto } from "@dzpos/shared";
import { api, categoriesQueryKey, productsQueryKey } from "@/api";
import { isKey, useTranslation, type Key } from "@/i18n";

export const Route = createFileRoute("/products")({ component: ProductsScreen });

const UNITS: readonly UnitDto[] = ["piece", "kg", "litre", "box"];

const UNIT_KEY: Record<UnitDto, Key> = {
  piece: "unit_piece",
  kg: "unit_kg",
  litre: "unit_litre",
  box: "unit_box",
};

/** features.md, TVA rates row: 19 % standard, 9 % reduced, 0 % exempt. */
const RATES: readonly { bps: number; key: Key }[] = [
  { bps: 1900, key: "rate_1900" },
  { bps: 900, key: "rate_900" },
  { bps: 0, key: "rate_0" },
];

const FALLBACK_RATE_BPS = 1900;

/** "7 %" for 700 bps: the label of a rate the fixed list does not carry. */
function rateLabel(bps: number): string {
  const percent = bps / 100;
  return `${Number.isInteger(percent) ? percent : percent.toFixed(2).replace(".", ",")} %`;
}

/**
 * The fixed choices plus any category default (or stored product rate) the
 * list does not carry. The migration allows any rate between 0 and 10 000
 * bps on a category; without this a 700 bps category showed "19 %" while
 * the form posted 700.
 */
function rateOptions(
  categories: readonly CategoryDto[],
  t: (key: Key) => string,
  stored?: number,
): { value: string; label: string }[] {
  const fixed = RATES.map((rate) => ({ value: String(rate.bps), label: t(rate.key) }));
  const known = new Set(RATES.map((rate) => rate.bps));
  const candidates = categories.map((c) => c.default_rate_bps);
  // A product edited later keeps showing the rate it was stored with, even
  // one no category offers any more.
  if (stored !== undefined) candidates.push(stored);
  const extra = [...new Set(candidates)]
    .filter((bps) => !known.has(bps))
    .sort((a, b) => b - a)
    .map((bps) => ({ value: String(bps), label: rateLabel(bps) }));
  return [...fixed, ...extra];
}

const ERROR_KEY: Record<string, Key> = {
  validation: "error_validation",
  duplicate_barcode: "error_duplicate_barcode",
  not_found: "error_not_found",
  money: "error_money",
  storage: "error_storage",
  bad_request: "error_bad_request",
  bad_response: "error_bad_response",
  unreachable: "error_unreachable",
};

/** The server sends a code, never a sentence; the UI owns the wording. */
function errorKey(error: unknown): Key {
  if (error instanceof ApiError) {
    return ERROR_KEY[error.code] ?? "error_unknown";
  }
  return "error_unknown";
}

export function ProductsScreen() {
  const { t } = useTranslation();
  // One form, two jobs: `null` is closed, "new" is the add form, a product
  // is the edit form for that row.
  const [open, setOpen] = useState<"new" | ProductDto | null>(null);
  const products = useQuery({ queryKey: productsQueryKey, queryFn: () => api.listProducts() });
  // The form needs the shop's real categories before it can offer one, so
  // the query lives here and the form is rendered once it has answered.
  const categories = useQuery({
    queryKey: categoriesQueryKey,
    queryFn: () => api.listCategories(),
  });

  return (
    <section className="flex flex-col gap-4">
      <header className="flex items-center justify-between gap-4">
        <h1 className="text-xl font-semibold">{t("products_title")}</h1>
        <button
          type="button"
          className="rounded border px-3 py-1.5"
          onClick={() => setOpen((current) => (current === null ? "new" : null))}
        >
          {open !== null ? t("action_cancel") : t("products_add")}
        </button>
      </header>

      {open !== null && categories.isSuccess ? (
        <ProductForm
          key={open === "new" ? "new" : open.id}
          categories={categories.data}
          initial={open === "new" ? null : open}
          onDone={() => setOpen(null)}
        />
      ) : null}
      {open !== null && categories.isPending ? <p>{t("products_loading")}</p> : null}
      {open !== null && categories.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(categories.error))}
        </p>
      ) : null}

      {products.isPending ? <p>{t("products_loading")}</p> : null}
      {products.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(products.error))}
        </p>
      ) : null}
      {products.isSuccess ? (
        <ProductTable rows={products.data} onEdit={(row) => setOpen(row)} />
      ) : null}
    </section>
  );
}

function ProductTable({
  rows,
  onEdit,
}: {
  rows: ProductDto[];
  onEdit: (row: ProductDto) => void;
}) {
  const { t } = useTranslation();
  if (rows.length === 0) return <p>{t("products_empty")}</p>;
  return (
    <table className="w-full text-start">
      <caption className="sr-only">{t("products_title")}</caption>
      <thead>
        <tr>
          <th scope="col" className="text-start pb-2">{t("col_name")}</th>
          <th scope="col" className="text-start pb-2">{t("col_barcode")}</th>
          <th scope="col" className="text-start pb-2">{t("col_unit")}</th>
          <th scope="col" className="text-end pb-2">{t("col_price")}</th>
          <th scope="col" className="text-end pb-2">{t("col_rate")}</th>
          <th scope="col" className="text-end pb-2">{t("col_stock")}</th>
          <th scope="col" className="pb-2">
            <span className="sr-only">{t("products_edit")}</span>
          </th>
        </tr>
      </thead>
      <tbody>
        {rows.map((p) => (
          <tr key={p.id} className={p.active ? "border-t" : "border-t opacity-60"}>
            <td className="py-1.5 pe-3">
              {p.name}
              {p.active ? null : (
                <span className="ms-2 rounded border px-1 text-xs uppercase">
                  {t("products_inactive")}
                </span>
              )}
            </td>
            <td className="py-1.5 pe-3 font-mono">{p.barcode ?? ""}</td>
            <td className="py-1.5 pe-3">{t(UNIT_KEY[p.unit])}</td>
            <td className="py-1.5 ps-3 text-end font-mono">
              {formatCentimes(p.selling_centimes)}
            </td>
            <td className="py-1.5 ps-3 text-end font-mono">{rateLabel(p.rate_bps)}</td>
            <td className="py-1.5 ps-3 text-end font-mono">{formatQty(p.qty_on_hand_milli)}</td>
            <td className="py-1.5 ps-3 text-end">
              <button
                type="button"
                className="rounded border px-2 py-0.5 text-sm"
                aria-label={`${t("products_edit")} ${p.name}`}
                onClick={() => onEdit(p)}
              >
                {t("products_edit")}
              </button>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/**
 * Add and edit are one form: the same fields, the same validators, one
 * request shape (`NewProductDto` is the whole product either way). With
 * `initial` the fields start from the stored row and the save is a PUT to
 * that row; without it they start blank and the save is a POST.
 */
function ProductForm({
  categories,
  initial,
  onDone,
}: {
  categories: CategoryDto[];
  initial: ProductDto | null;
  onDone: () => void;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<Key | null>(null);

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
  const form = useForm({
    defaultValues:
      initial === null
        ? {
            name: "",
            barcode: "",
            category: first === undefined ? "" : String(first.id),
            rate: String(first?.default_rate_bps ?? FALLBACK_RATE_BPS),
            unit: "piece",
            cost: "",
            price: "",
            wholesale: "",
            stock: "",
            lowStock: "",
            active: true,
          }
        : {
            name: initial.name,
            barcode: initial.barcode ?? "",
            category: initial.category_id === null ? "" : String(initial.category_id),
            rate: String(initial.rate_bps),
            unit: initial.unit,
            cost: formatCentimes(initial.cost_centimes),
            price: formatCentimes(initial.selling_centimes),
            wholesale:
              initial.wholesale_centimes === null ? "" : formatCentimes(initial.wholesale_centimes),
            stock: formatQty(initial.qty_on_hand_milli),
            lowStock: initial.low_stock_at_milli === 0 ? "" : formatQty(initial.low_stock_at_milli),
            active: initial.active,
          },
    onSubmit: async ({ value }) => {
      // The validators have already refused anything unreadable, so these
      // fall back only for the blank case they allow.
      const cost = optional(value.cost, parseAmountToCentimes) ?? 0;
      const price = parseAmountToCentimes(value.price) ?? 0;
      const stock = optional(value.stock, parseQtyToMilli) ?? 0;
      const lowStock = optional(value.lowStock, parseQtyToMilli) ?? 0;
      // A blank wholesale price is "none", not a price of zero.
      const wholesale =
        value.wholesale.trim() === "" ? null : parseAmountToCentimes(value.wholesale);
      // The rejection is deliberately swallowed: onError has already turned
      // the server's code into a translated message on the form.
      await save
        .mutateAsync({
          name: value.name,
          barcode: value.barcode.trim() === "" ? null : value.barcode.trim(),
          category_id: value.category === "" ? null : Number(value.category),
          unit: toUnit(value.unit),
          cost_centimes: cost,
          selling_centimes: price,
          wholesale_centimes: wholesale,
          qty_on_hand_milli: stock,
          low_stock_at_milli: lowStock,
          // Sent, never left to the server to infer: the shop chose a rate on
          // this screen and the product has to carry the one it chose.
          rate_bps: Number(value.rate),
          active: value.active,
        })
        .catch(() => undefined);
    },
  });

  return (
    <form
      noValidate
      className="flex flex-col gap-3 rounded border p-4"
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
          <label className="flex flex-col gap-1">
            <span>{t("field_name")}</span>
            <input
              className="rounded border px-2 py-1"
              value={field.state.value}
              onChange={(e) => field.handleChange(e.target.value)}
              onBlur={field.handleBlur}
            />
            <FieldError messages={field.state.meta.errors} />
          </label>
        )}
      </form.Field>

      <form.Field name="barcode">
        {(field) => (
          <label className="flex flex-col gap-1">
            <span>{t("field_barcode")}</span>
            <input
              className="rounded border px-2 py-1 font-mono"
              value={field.state.value}
              onChange={(e) => field.handleChange(e.target.value)}
            />
            <span className="text-sm opacity-70">{t("field_barcode_hint")}</span>
          </label>
        )}
      </form.Field>

      <form.Field name="category">
        {(field) => (
          <label className="flex flex-col gap-1">
            <span>{t("field_category")}</span>
            <select
              className="rounded border px-2 py-1"
              value={field.state.value}
              onChange={(e) => {
                field.handleChange(e.target.value);
                // The category's rate is the offer, not the decision: the
                // rate select below stays editable after this.
                const chosen = categories.find((c) => String(c.id) === e.target.value);
                if (chosen !== undefined) {
                  form.setFieldValue("rate", String(chosen.default_rate_bps));
                }
              }}
            >
              {categories.map((c) => (
                <option key={c.id} value={String(c.id)}>
                  {c.name}
                </option>
              ))}
            </select>
          </label>
        )}
      </form.Field>

      <form.Field name="rate">
        {(field) => (
          <label className="flex flex-col gap-1">
            <span>{t("field_rate")}</span>
            <select
              className="rounded border px-2 py-1"
              value={field.state.value}
              onChange={(e) => field.handleChange(e.target.value)}
            >
              {rateOptions(categories, t, initial?.rate_bps).map((rate) => (
                <option key={rate.value} value={rate.value}>
                  {rate.label}
                </option>
              ))}
            </select>
          </label>
        )}
      </form.Field>

      <form.Field name="unit">
        {(field) => (
          <label className="flex flex-col gap-1">
            <span>{t("field_unit")}</span>
            <select
              className="rounded border px-2 py-1"
              value={field.state.value}
              onChange={(e) => field.handleChange(e.target.value)}
            >
              {UNITS.map((u) => (
                <option key={u} value={u}>
                  {t(UNIT_KEY[u])}
                </option>
              ))}
            </select>
          </label>
        )}
      </form.Field>

      <form.Field
        name="price"
        validators={{
          onSubmit: ({ value }) =>
            parseAmountToCentimes(value) === null ? "error_price_invalid" : undefined,
        }}
      >
        {(field) => (
          <AmountField
            label={t("field_price")}
            value={field.state.value}
            onChange={field.handleChange}
            errors={field.state.meta.errors}
          />
        )}
      </form.Field>

      <form.Field
        name="cost"
        validators={{
          onSubmit: ({ value }) =>
            optional(value, parseAmountToCentimes) === undefined
              ? "error_cost_invalid"
              : undefined,
        }}
      >
        {(field) => (
          <AmountField
            label={t("field_cost")}
            value={field.state.value}
            onChange={field.handleChange}
            errors={field.state.meta.errors}
          />
        )}
      </form.Field>

      <form.Field
        name="wholesale"
        validators={{
          onSubmit: ({ value }) =>
            value.trim() !== "" && parseAmountToCentimes(value) === null
              ? "error_wholesale_invalid"
              : undefined,
        }}
      >
        {(field) => (
          <AmountField
            label={t("field_wholesale")}
            value={field.state.value}
            onChange={field.handleChange}
            errors={field.state.meta.errors}
          />
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
          <AmountField
            label={t("field_stock")}
            value={field.state.value}
            onChange={field.handleChange}
            errors={field.state.meta.errors}
          />
        )}
      </form.Field>

      <form.Field
        name="lowStock"
        validators={{
          onSubmit: ({ value }) =>
            optional(value, parseQtyToMilli) === undefined
              ? "error_low_stock_invalid"
              : undefined,
        }}
      >
        {(field) => (
          <AmountField
            label={t("field_low_stock")}
            value={field.state.value}
            onChange={field.handleChange}
            errors={field.state.meta.errors}
          />
        )}
      </form.Field>

      <form.Field name="active">
        {(field) => (
          <label className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={field.state.value}
              onChange={(e) => field.handleChange(e.target.checked)}
            />
            <span>{t("field_active")}</span>
          </label>
        )}
      </form.Field>

      {serverError !== null ? (
        <p role="alert" className="text-red-700">
          {t(serverError)}
        </p>
      ) : null}

      <div className="flex gap-2">
        <button type="submit" className="rounded border px-3 py-1.5" disabled={save.isPending}>
          {save.isPending ? t("action_saving") : t("action_save")}
        </button>
        <button type="button" className="rounded border px-3 py-1.5" onClick={onDone}>
          {t("action_cancel")}
        </button>
      </div>
    </form>
  );
}

/** A decimal typed by a person: an amount in dinars or a quantity. */
function AmountField({
  label,
  value,
  onChange,
  errors,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  errors: unknown[];
}) {
  return (
    <label className="flex flex-col gap-1">
      <span>{label}</span>
      <input
        inputMode="decimal"
        className="rounded border px-2 py-1 font-mono text-end"
        value={value}
        onChange={(e) => onChange(e.target.value)}
      />
      <FieldError messages={errors} />
    </label>
  );
}

/**
 * A field a shop may leave empty. Blank is zero; anything the parser cannot
 * read is `undefined`, which is what blocks the submit. `?? 0` used to
 * flatten both cases, so "12 DA" was stored as a cost of nothing.
 */
function optional(text: string, parse: (t: string) => number | null): number | undefined {
  if (text.trim() === "") return 0;
  return parse(text) ?? undefined;
}

/** Field validators return translation keys, never sentences. */
function FieldError({ messages }: { messages: unknown[] }) {
  const { t } = useTranslation();
  const key = messages.find((m): m is string => typeof m === "string");
  if (key === undefined) return null;
  return (
    <span role="alert" className="text-sm text-red-700">
      {t(isKey(key) ? key : "error_unknown")}
    </span>
  );
}

function toUnit(value: string): UnitDto {
  const found = UNITS.find((u) => u === value);
  return found ?? "piece";
}
