// The products screen, on the kit. Everything it shows comes from the API
// over HTTP; the types are generated from the Rust structs (packages/shared).
// Every visible string goes through i18n and nothing here is left or right,
// so Arabic mirrors the layout without a second stylesheet.
//
// Three parts. A filter bar over the catalogue, because a shop with nine
// hundred products does not scroll: it searches, narrows to a shelf, or asks
// what is running out. The list itself, which is `DataTable` and therefore
// the same table as every other list in the app. And the fiche, which is a
// `Sheet`: add and edit are one form, it slides in over the list rather than
// pushing it down, and the list stays where the shop left it.
//
// The filters are applied here and not asked of the server. The catalogue is
// one request the screen already makes and a shop's catalogue is thousands of
// rows at the very most, so a second endpoint would buy latency and a cache
// to invalidate and nothing else.
//
// Labels. The one product's label belongs to the fiche and renders inside the
// sheet, under the form; the sheet of labels for the ticked rows is a dialog
// off the list. They are the same `LabelPanel` and the same state, split that
// way on purpose: a dialog opened on top of the sheet would mark the sheet
// inert, and the form under it would stop answering.

import { createFileRoute } from "@tanstack/react-router";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useForm } from "@tanstack/react-form";
import { Package, Printer, SearchX, Tags } from "lucide-react";
import { useMemo, useState, type ReactNode } from "react";
import { ApiError, formatQty, parseQtyToMilli } from "@dzpos/shared";
import type { CategoryDto, NewProductDto, ProductDto, UnitDto } from "@dzpos/shared";
import { api, categoriesQueryKey, productsQueryKey } from "@/api";
import { DataTable, type Column } from "@/components/DataTable";
import { EmptyState } from "@/components/EmptyState";
import { FormField } from "@/components/FormField";
import { Icon } from "@/components/Icon";
import { LabelPanel, type LabelAsk } from "@/components/LabelPanel";
import { Money } from "@/components/Money";
import { MoneyInput } from "@/components/MoneyInput";
import { PageHeader } from "@/components/PageHeader";
import { stockState } from "@/components/ProductTile";
import { StatusPill } from "@/components/StatusPill";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from "@/components/ui/sheet";
import { isKey, useTranslation, type Key } from "@/i18n";
import { RATES, rateCellLabel, rateLabel } from "@/lib/rate";
import { useHasPermission } from "@/lib/session";

export const Route = createFileRoute("/products")({ component: ProductsScreen });

const UNITS: readonly UnitDto[] = ["piece", "kg", "litre", "box"];

const UNIT_KEY: Record<UnitDto, Key> = {
  piece: "unit_piece",
  kg: "unit_kg",
  litre: "unit_litre",
  box: "unit_box",
};

const FALLBACK_RATE_BPS = 1900;

/**
 * Two sentinels, because a `SelectItem` refuses an empty value: Radix uses
 * the empty string internally for "nothing chosen" and throws on an item
 * that claims it. They are spelled here once and turned back into `null` and
 * "no filter" at the two places that read them.
 */
const NO_CATEGORY = "none";
const ANY_CATEGORY = "all";

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
  const percentSign = t("percent_sign");
  const decimalSeparator = t("decimal_separator");
  const extra = [...new Set(candidates)]
    .filter((bps) => !known.has(bps))
    .sort((a, b) => b - a)
    .map((bps) => ({ value: String(bps), label: rateLabel(bps, percentSign, decimalSeparator) }));
  return [...fixed, ...extra];
}

const ERROR_KEY: Record<string, Key> = {
  validation: "error_validation",
  duplicate_barcode: "error_duplicate_barcode",
  not_found: "error_not_found",
  money: "error_money",
  storage: "error_storage",
  restart_needed: "error_restart_needed",
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
}

/** What the filter bar is set to. All three off is the whole catalogue. */
interface Filters {
  readonly search: string;
  readonly category: string;
  readonly lowOnly: boolean;
}

/**
 * The filter bar applied to one product. Search reads the name and the
 * barcode, because a shop holding the box in its hand types the digits off
 * it rather than the name printed on it; both are compared case-folded so a
 * catalogue typed in capitals still answers.
 *
 * "Running out" is the product's own threshold and not a number this screen
 * picked, which is the same rule the tile draws its chip from.
 */
export function keeps(product: ProductDto, filters: Filters): boolean {
  const needle = filters.search.trim().toLocaleLowerCase();
  if (needle !== "") {
    const haystack = `${product.name} ${product.barcode ?? ""}`.toLocaleLowerCase();
    if (!haystack.includes(needle)) return false;
  }
  if (filters.category === NO_CATEGORY) {
    if (product.category_id !== null) return false;
  } else if (filters.category !== ANY_CATEGORY) {
    if (String(product.category_id) !== filters.category) return false;
  }
  if (filters.lowOnly && stockState(product.qty_on_hand_milli, product.low_stock_at_milli) === "ok") {
    return false;
  }
  return true;
}

export function ProductsScreen() {
  const { t, dir } = useTranslation();
  // One form, two jobs: `null` is closed, "new" is the add form, a product
  // is the edit form for that row.
  const [open, setOpen] = useState<"new" | ProductDto | null>(null);
  // The labels the screen is showing, or none. One state for the fiche's
  // single label and for the list's sheet: they are the same page at two
  // sizes, and only one of them is on screen at a time.
  const [labels, setLabels] = useState<LabelAsk | null>(null);
  // The rows ticked for a sheet of labels. Ids and not rows: a product
  // edited while it is ticked stays ticked, and the sheet is rendered from
  // what the server holds rather than from a copy this screen kept.
  const [picked, setPicked] = useState<readonly number[]>([]);
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState<string>(ANY_CATEGORY);
  const [lowOnly, setLowOnly] = useState(false);
  const products = useQuery({ queryKey: productsQueryKey, queryFn: () => api.listProducts() });
  // The form needs the shop's real categories before it can offer one, so
  // the query lives here and the form is rendered once it has answered.
  const categories = useQuery({
    queryKey: categoriesQueryKey,
    queryFn: () => api.listCategories(),
  });

  const all = useMemo(() => products.data ?? [], [products.data]);
  const shown = useMemo(
    () => all.filter((row) => keeps(row, { search, category, lowOnly })),
    [all, search, category, lowOnly],
  );
  const filtering = search.trim() !== "" || category !== ANY_CATEGORY || lowOnly;

  const clearFilters = () => {
    setSearch("");
    setCategory(ANY_CATEGORY);
    setLowOnly(false);
  };

  const closeFiche = () => {
    setOpen(null);
    setLabels((current) => (current !== null && current.kind === "one" ? null : current));
  };

  return (
    <div className="flex flex-col gap-4">
      <PageHeader
        title={t("products_title")}
        description={`${shown.length} ${t(shown.length < 2 ? "products_count_one" : "products_count")}`}
        actions={
          <>
            <Button
              variant="outline"
              data-testid="print-selected-labels"
              disabled={picked.length === 0}
              onClick={() =>
                setLabels((current) =>
                  current !== null && current.kind === "sheet" ? null : { kind: "sheet", ids: picked },
                )
              }
            >
              <Icon as={Tags} size={18} />
              {labels !== null && labels.kind === "sheet"
                ? t("action_labels_close")
                : t("action_print_labels")}
            </Button>
            <Button onClick={() => setOpen("new")}>{t("products_add")}</Button>
          </>
        }
      />

      <Card>
        <CardContent className="flex flex-wrap items-end gap-4">
          <FormField
            label={t("products_search")}
            hint={t("products_search_hint")}
            className="min-w-64 max-w-md flex-1"
          >
            {(parts) => (
              <Input
                {...parts}
                type="search"
                autoComplete="off"
                data-testid="products-search"
                value={search}
                onChange={(event) => setSearch(event.target.value)}
              />
            )}
          </FormField>

          <FormField label={t("products_filter_category")} className="min-w-48">
            {(parts) => (
              <Select value={category} onValueChange={setCategory}>
                <SelectTrigger id={parts.id} data-testid="products-filter-category" className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value={ANY_CATEGORY}>{t("products_filter_all")}</SelectItem>
                  <SelectItem value={NO_CATEGORY}>{t("category_none")}</SelectItem>
                  {(categories.data ?? []).map((one) => (
                    <SelectItem key={one.id} value={String(one.id)}>
                      {one.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            )}
          </FormField>

          {/* A tick wants its label beside it, not over it, so the field is
              turned into a row; the label is still the kit's and still points
              at the control by id. */}
          <FormField
            label={t("products_filter_low")}
            className="flex-row-reverse items-center justify-end gap-2"
          >
            {(parts) => (
              <Checkbox
                id={parts.id}
                data-testid="products-filter-low"
                checked={lowOnly}
                onCheckedChange={(next) => setLowOnly(next === true)}
              />
            )}
          </FormField>

          {filtering ? (
            <Button variant="ghost" data-testid="products-clear-filters" onClick={clearFilters}>
              {t("products_filter_clear")}
            </Button>
          ) : null}
        </CardContent>
      </Card>

      {products.isPending ? <p className="text-muted-foreground">{t("products_loading")}</p> : null}
      {products.isError ? (
        <p role="alert" className="text-fg-danger">
          {t(errorKey(products.error))}
        </p>
      ) : null}
      {open !== null && categories.isError ? (
        <p role="alert" className="text-fg-danger">
          {t(errorKey(categories.error))}
        </p>
      ) : null}

      {products.isSuccess ? (
        <ProductTable
          rows={shown}
          empty={
            all.length === 0 ? (
              <EmptyState
                icon={Package}
                title={t("products_empty")}
                description={t("products_empty_hint")}
                action={
                  <Button data-testid="products-add-first" onClick={() => setOpen("new")}>
                    {t("products_add_first")}
                  </Button>
                }
              />
            ) : (
              <EmptyState
                icon={SearchX}
                title={t("products_no_match")}
                description={t("products_no_match_hint")}
                action={
                  <Button variant="outline" onClick={clearFilters}>
                    {t("products_filter_clear")}
                  </Button>
                }
              />
            )
          }
          onEdit={(row) => setOpen(row)}
          picked={picked}
          onPick={(id, on) => {
            setLabels((current) => (current !== null && current.kind === "sheet" ? null : current));
            setPicked((current) =>
              on ? [...current, id] : current.filter((other) => other !== id),
            );
          }}
        />
      ) : null}

      <Sheet
        open={open !== null && categories.isSuccess}
        onOpenChange={(next) => {
          if (!next) closeFiche();
        }}
      >
        <SheetContent
          side={dir === "rtl" ? "left" : "right"}
          showCloseButton={false}
          className="w-full overflow-y-auto sm:max-w-lg"
        >
          <SheetHeader>
            <SheetTitle>{open === null || open === "new" ? t("products_add") : open.name}</SheetTitle>
            <SheetDescription>{t("products_fiche_hint")}</SheetDescription>
          </SheetHeader>
          {open !== null && categories.isSuccess ? (
            <ProductForm
              key={open === "new" ? "new" : open.id}
              categories={categories.data}
              initial={open === "new" ? null : open}
              onDone={closeFiche}
              label={labels !== null && labels.kind === "one" ? labels : null}
              onPrintLabel={(id) =>
                setLabels((current) =>
                  current !== null && current.kind === "one" && current.id === id
                    ? null
                    : { kind: "one", id },
                )
              }
            />
          ) : null}
        </SheetContent>
      </Sheet>

      <Dialog
        open={labels !== null && labels.kind === "sheet"}
        onOpenChange={(next) => {
          if (!next) setLabels(null);
        }}
      >
        {/* `sm:` on purpose: DialogContent's own cap is `sm:max-w-lg`, and
            an unprefixed override loses to it at every width the media
            query covers, which is the only width a sheet of labels is read
            at. */}
        <DialogContent className="sm:max-w-3xl">
          <DialogHeader>
            <DialogTitle>{t("labels_title")}</DialogTitle>
            <DialogDescription>{t("labels_sheet_hint")}</DialogDescription>
          </DialogHeader>
          {labels !== null && labels.kind === "sheet" ? <LabelPanel ask={labels} /> : null}
        </DialogContent>
      </Dialog>
    </div>
  );
}

function ProductTable({
  rows,
  empty,
  onEdit,
  picked,
  onPick,
}: {
  rows: readonly ProductDto[];
  empty: ReactNode;
  onEdit: (row: ProductDto) => void;
  picked: readonly number[];
  onPick: (id: number, on: boolean) => void;
}) {
  const { t } = useTranslation();

  // `dir="ltr"` on the four cells below: a barcode, a price, a rate and a
  // quantity are read left to right with Western digits regardless of the
  // screen's language (a decision, features.md names no rule for it).
  // Without it the Unicode bidi algorithm is free to reorder the space and
  // the sign around the digits inside an RTL row. `Money` carries its own.
  const columns: readonly Column<ProductDto>[] = [
    {
      id: "pick",
      header: t("col_pick"),
      cell: (row) => (
        <Checkbox
          aria-label={`${t("col_pick")} ${row.name}`}
          checked={picked.includes(row.id)}
          onCheckedChange={(next) => onPick(row.id, next === true)}
        />
      ),
    },
    {
      id: "name",
      header: t("col_name"),
      cell: (row) => (
        <span className="flex flex-wrap items-center gap-2">
          <span className={row.active ? "" : "text-muted-foreground"}>{row.name}</span>
          {row.active ? null : <Badge variant="outline">{t("products_inactive")}</Badge>}
        </span>
      ),
    },
    {
      id: "barcode",
      header: t("col_barcode"),
      cell: (row) => (
        <span dir="ltr" data-testid="cell-barcode" className="font-numeric tabular-nums">
          {row.barcode ?? ""}
        </span>
      ),
    },
    { id: "unit", header: t("col_unit"), cell: (row) => t(UNIT_KEY[row.unit]) },
    {
      id: "price",
      header: t("col_price"),
      money: true,
      cell: (row) => <Money centimes={row.selling_centimes} data-testid="cell-price" />,
    },
    {
      id: "rate",
      header: t("col_rate"),
      numeric: true,
      cell: (row) => (
        <span dir="ltr" data-testid="cell-rate">
          {rateCellLabel(row.rate_bps, t)}
        </span>
      ),
    },
    {
      id: "stock",
      header: t("col_stock"),
      numeric: true,
      cell: (row) => (
        <span className="flex items-center justify-end gap-2">
          <span dir="ltr" data-testid="cell-stock">
            {formatQty(row.qty_on_hand_milli)}
          </span>
          {stockState(row.qty_on_hand_milli, row.low_stock_at_milli) === "ok" ? null : (
            <StatusPill status="low" data-testid="cell-low" />
          )}
        </span>
      ),
    },
  ];

  return (
    <DataTable
      columns={columns}
      rows={rows}
      rowKey={(row) => row.id}
      caption={t("products_title")}
      empty={empty}
      data-testid="products-table"
      actions={(row) => (
        <Button
          variant="ghost"
          size="sm"
          aria-label={`${t("products_edit")} ${row.name}`}
          onClick={() => onEdit(row)}
        >
          {t("products_edit")}
        </Button>
      )}
    />
  );
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
function ProductForm({
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
          };

  const form = useForm({
    defaultValues: defaults,
    onSubmit: async ({ value }) => {
      // The validators have already refused anything unreadable, so these
      // fall back only for the blank case they allow.
      const stock = optional(value.stock, parseQtyToMilli) ?? 0;
      const lowStock = optional(value.lowStock, parseQtyToMilli) ?? 0;
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
          <FormField label={t("field_name")} required error={firstError(field.state.meta.errors, t)}>
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
            error={firstError(field.state.meta.errors, t)}
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
          <FormField label={t("field_stock")} error={firstError(field.state.meta.errors, t)}>
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
          <FormField label={t("field_low_stock")} error={firstError(field.state.meta.errors, t)}>
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

/** Field validators return translation keys, never sentences. */
function firstError(messages: unknown[], t: (key: Key) => string): string | undefined {
  const key = messages.find((m): m is string => typeof m === "string");
  if (key === undefined) return undefined;
  return t(isKey(key) ? key : "error_unknown");
}

function toUnit(value: string): UnitDto {
  const found = UNITS.find((u) => u === value);
  return found ?? "piece";
}
