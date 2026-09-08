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
import type { NewProductDto, ProductDto, UnitDto } from "@dzpos/shared";
import { api, productsQueryKey } from "@/api";
import { isKey, useTranslation, type Key } from "@/i18n";

export const Route = createFileRoute("/products")({ component: ProductsScreen });

const UNITS: readonly UnitDto[] = ["piece", "kg", "litre", "box"];

const UNIT_KEY: Record<UnitDto, Key> = {
  piece: "unit_piece",
  kg: "unit_kg",
  litre: "unit_litre",
  box: "unit_box",
};

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
  const [adding, setAdding] = useState(false);
  const products = useQuery({ queryKey: productsQueryKey, queryFn: () => api.listProducts() });

  return (
    <section className="flex flex-col gap-4">
      <header className="flex items-center justify-between gap-4">
        <h1 className="text-xl font-semibold">{t("products_title")}</h1>
        <button
          type="button"
          className="rounded border px-3 py-1.5"
          onClick={() => setAdding((open) => !open)}
        >
          {adding ? t("action_cancel") : t("products_add")}
        </button>
      </header>

      {adding ? <AddProductForm onDone={() => setAdding(false)} /> : null}

      {products.isPending ? <p>{t("products_loading")}</p> : null}
      {products.isError ? (
        <p role="alert" className="text-red-700">
          {t(errorKey(products.error))}
        </p>
      ) : null}
      {products.isSuccess ? <ProductTable rows={products.data} /> : null}
    </section>
  );
}

function ProductTable({ rows }: { rows: ProductDto[] }) {
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
          <th scope="col" className="text-end pb-2">{t("col_stock")}</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((p) => (
          <tr key={p.id} className="border-t">
            <td className="py-1.5 pe-3">{p.name}</td>
            <td className="py-1.5 pe-3 font-mono">{p.barcode ?? ""}</td>
            <td className="py-1.5 pe-3">{t(UNIT_KEY[p.unit])}</td>
            <td className="py-1.5 ps-3 text-end font-mono">
              {formatCentimes(p.selling_centimes)}
            </td>
            <td className="py-1.5 ps-3 text-end font-mono">{formatQty(p.qty_on_hand_milli)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function AddProductForm({ onDone }: { onDone: () => void }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [serverError, setServerError] = useState<Key | null>(null);

  const create = useMutation({
    mutationFn: (input: NewProductDto) => api.createProduct(input),
    onSuccess: async () => {
      setServerError(null);
      await queryClient.invalidateQueries({ queryKey: productsQueryKey });
      onDone();
    },
    onError: (error: unknown) => setServerError(errorKey(error)),
  });

  const form = useForm({
    defaultValues: {
      name: "",
      barcode: "",
      unit: "piece",
      cost: "",
      price: "",
      stock: "",
    },
    onSubmit: async ({ value }) => {
      const cost = parseAmountToCentimes(value.cost) ?? 0;
      const price = parseAmountToCentimes(value.price) ?? 0;
      const stock = parseQtyToMilli(value.stock) ?? 0;
      // The rejection is deliberately swallowed: onError has already turned
      // the server's code into a translated message on the form.
      await create
        .mutateAsync({
          name: value.name,
          barcode: value.barcode.trim() === "" ? null : value.barcode.trim(),
          category_id: 1,
          unit: toUnit(value.unit),
          cost_centimes: cost,
          selling_centimes: price,
          wholesale_centimes: null,
          qty_on_hand_milli: stock,
          low_stock_at_milli: 0,
          rate_bps: null,
          active: true,
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
          <label className="flex flex-col gap-1">
            <span>{t("field_price")}</span>
            <input
              inputMode="decimal"
              className="rounded border px-2 py-1 font-mono text-end"
              value={field.state.value}
              onChange={(e) => field.handleChange(e.target.value)}
            />
            <FieldError messages={field.state.meta.errors} />
          </label>
        )}
      </form.Field>

      <form.Field name="cost">
        {(field) => (
          <label className="flex flex-col gap-1">
            <span>{t("field_cost")}</span>
            <input
              inputMode="decimal"
              className="rounded border px-2 py-1 font-mono text-end"
              value={field.state.value}
              onChange={(e) => field.handleChange(e.target.value)}
            />
          </label>
        )}
      </form.Field>

      <form.Field name="stock">
        {(field) => (
          <label className="flex flex-col gap-1">
            <span>{t("field_stock")}</span>
            <input
              inputMode="decimal"
              className="rounded border px-2 py-1 font-mono text-end"
              value={field.state.value}
              onChange={(e) => field.handleChange(e.target.value)}
            />
          </label>
        )}
      </form.Field>

      {serverError !== null ? (
        <p role="alert" className="text-red-700">
          {t(serverError)}
        </p>
      ) : null}

      <div className="flex gap-2">
        <button type="submit" className="rounded border px-3 py-1.5" disabled={create.isPending}>
          {create.isPending ? t("action_saving") : t("action_save")}
        </button>
        <button type="button" className="rounded border px-3 py-1.5" onClick={onDone}>
          {t("action_cancel")}
        </button>
      </div>
    </form>
  );
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
