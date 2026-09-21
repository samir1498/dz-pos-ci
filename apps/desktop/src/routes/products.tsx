// The products screen, on the kit. Everything it shows comes from the API
// over HTTP; the types are generated from the Rust structs (packages/shared).
// Every visible string goes through i18n and nothing here is left or right,
// so Arabic mirrors the layout without a second stylesheet.
//
// Three parts. A filter bar over the catalogue, because a shop with nine
// hundred products does not scroll: it searches, narrows to a shelf, or asks
// what is running out. The list itself, `-products/table.tsx`, which is
// `DataTable` and therefore the same table as every other list in the app.
// And the fiche, `-products/fiche.tsx`, which is a `Sheet`: add and edit are
// one form, it slides in over the list rather than pushing it down, and the
// list stays where the shop left it.
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
import { useQuery } from "@tanstack/react-query";
import { Package, SearchX, Tags } from "lucide-react";
import { useMemo, useState } from "react";
import type { ProductDto } from "@dzpos/shared";
import { api, categoriesQueryKey, productsQueryKey } from "@/api";
import { EmptyState } from "@/components/EmptyState";
import { FormField } from "@/components/FormField";
import { Icon } from "@/components/Icon";
import { LabelPanel, type LabelAsk } from "@/components/LabelPanel";
import { PageHeader } from "@/components/PageHeader";
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
import { useTranslation } from "@/i18n";
import { errorKey } from "@/lib/fields";

import { ProductForm } from "./-products/fiche";
import { ANY_CATEGORY, NO_CATEGORY, keeps } from "./-products/parts";
import { ProductTable } from "./-products/table";

export const Route = createFileRoute("/products")({ component: ProductsScreen });

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
          {/* The "name or barcode" help is the input's placeholder, not a
              hint under it: a hint makes this field taller than its
              neighbours, and `items-end` then sets the category select
              lower than the search box. Customers and suppliers already
              do it this way. */}
          <FormField label={t("products_search")} className="min-w-64 max-w-md flex-1">
            {(parts) => (
              <Input
                {...parts}
                type="search"
                autoComplete="off"
                placeholder={t("products_search_hint")}
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
              at the control by id. `pb-2` centers the row on the inputs'
              line: the row bottom-aligns with them (`items-end`) and at
              20px it is 16px shorter than a 36px input, so 8px. */}
          <FormField
            label={t("products_filter_low")}
            className="flex-row-reverse items-center justify-end gap-2 pb-2"
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
