// The dashboard: what the shop did today and this month, and the thirty days
// behind it.
//
// Nothing on this screen is computed here. Every figure is one the core
// derived from the ledgers when the call was made (features.md §1, "no column
// stores a total"), so the screen's whole job is to put the server's integers
// where a shopkeeper can read them. The one arithmetic in this file is the
// chart's y axis, which divides centimes into whole dinars for a tick label
// and never for a total.
//
// The day comes from `/clock` and never from `new Date()`, for the reason
// `lib/clock.ts` gives: the core dates every row on Algeria's calendar and a
// machine in another zone would ask for a day the ledger has not reached. The
// day is then sent explicitly, so the two calls, the query key and the e2e
// assertions all name the same day rather than each reading the clock again a
// second apart.
//
// The chart reads left to right in Arabic too. It is the decision `Money`
// already takes for an amount and the till takes for a barcode: a time axis
// with Western digits mirrored into an Arabic page puts the oldest day on the
// right of a series recharts drew for the left, and no reader gains anything
// from it. The words around it mirror; the plot does not.

import { useQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { AlertTriangle, PackageSearch, TrendingUp } from "lucide-react";
import { useState, type ReactNode } from "react";
import { Bar, CartesianGrid, ComposedChart, Line, XAxis, YAxis } from "recharts";

import type {
  DashboardDto,
  DashboardSeriesDto,
  DashboardSeriesPointDto,
  LowStockDto,
  TopProductDto,
} from "@dzpos/shared";
import { formatQty } from "@dzpos/shared";

import { api, dashboardQueryKey, dashboardSeriesQueryKey } from "@/api";
import { DataTable, type Column } from "@/components/DataTable";
import { EmptyState } from "@/components/EmptyState";
import { Money } from "@/components/Money";
import { PageHeader } from "@/components/PageHeader";
import { StatusPill } from "@/components/StatusPill";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  ChartContainer,
  ChartLegend,
  ChartLegendContent,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from "@/components/ui/chart";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useTranslation, type Key } from "@/i18n";
import { useShopToday } from "@/lib/clock";
import { errorKey } from "@/lib/fields";
import { useHasPermission } from "@/lib/session";

/** The window the chart draws, and the one the API defaults to. */
const CHART_DAYS = 30;

export const Route = createFileRoute("/dashboard")({ component: DashboardScreen });

export function DashboardScreen() {
  const { t } = useTranslation();
  const today = useShopToday();

  if (today.error !== null) {
    return (
      <section className="flex flex-col">
        <PageHeader title={t("dashboard_title")} />
        <Refusal error={today.error} onRetry={today.retry} />
      </section>
    );
  }
  if (today.today === undefined) {
    return (
      <section className="flex flex-col">
        <PageHeader title={t("dashboard_title")} />
        <p>{t("dashboard_loading")}</p>
      </section>
    );
  }
  return <Day day={today.today} />;
}

function Day({ day }: { day: string }) {
  const { t } = useTranslation();
  // GET /dashboard and GET /dashboard/series now refuse a caller without
  // SeeReports outright (crates/api/src/gates/, M4 T5 review,
  // 2026-09-11), and AppShell hides the nav entry the same way, so a
  // cashier never reaches this screen with data to draw. The finer-grained
  // check below is a second permission, not a second layer of the same one:
  // SeeReports is "may see this screen at all", SeeCostAndMargin is "may
  // see the shop's margin on it", and today only a role holding both ever
  // gets past the route gate to render this component — but the two stay
  // independent because a custom role could one day hold the first without
  // the second, and it is this flag, not the route gate, that would still
  // hide the margin figure and the top-by-margin card from them.
  const seeCostAndMargin = useHasPermission("see_cost_and_margin");
  const dashboard = useQuery({
    queryKey: dashboardQueryKey(day),
    queryFn: () => api.dashboard(day),
  });
  const series = useQuery({
    queryKey: dashboardSeriesQueryKey(day, CHART_DAYS),
    queryFn: () => api.dashboardSeries({ day, days: CHART_DAYS }),
  });

  return (
    <section className="flex flex-col" data-testid="dashboard">
      <PageHeader title={t("dashboard_title")} description={t("dashboard_description")} />

      {dashboard.isPending ? <p>{t("dashboard_loading")}</p> : null}
      {dashboard.isError ? (
        <Refusal
          error={dashboard.error}
          onRetry={() => {
            void dashboard.refetch();
          }}
        />
      ) : null}

      {dashboard.isSuccess ? (
        <div className="flex flex-col gap-4">
          <Figures dashboard={dashboard.data} seeCostAndMargin={seeCostAndMargin} />

          <Card data-testid="dashboard-chart-card">
            <CardHeader>
              <CardTitle>{t("dashboard_chart_title")}</CardTitle>
            </CardHeader>
            <CardContent>
              {series.isPending ? <p>{t("dashboard_loading")}</p> : null}
              {series.isError ? (
                <Refusal
                  error={series.error}
                  onRetry={() => {
                    void series.refetch();
                  }}
                />
              ) : null}
              {series.isSuccess ? (
                <Chart series={series.data} seeCostAndMargin={seeCostAndMargin} />
              ) : null}
            </CardContent>
          </Card>

          <div className="grid gap-4 lg:grid-cols-2">
            <LowStockCard rows={dashboard.data.low_stock} className="lg:col-span-2" />
            <TopCard
              title="dashboard_top_quantity"
              rows={dashboard.data.top_by_quantity}
              testId="dashboard-top-quantity"
            />
            {/* Nothing but a margin column: hiding the figure and keeping
                the card would be an empty table with a name on it. */}
            {seeCostAndMargin ? (
              <TopCard
                title="dashboard_top_margin"
                rows={dashboard.data.top_by_margin}
                testId="dashboard-top-margin"
              />
            ) : null}
          </div>
        </div>
      ) : null}
    </section>
  );
}

/** A refusal the server sent, as the word this app has for that code, with the
 *  one thing to do about it. */
function Refusal({ error, onRetry }: { error: unknown; onRetry: () => void }) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col items-start gap-2">
      <p role="alert" className="text-fg-danger">
        {t(errorKey(error))}
      </p>
      <Button variant="outline" onClick={onRetry}>
        {t("action_retry")}
      </Button>
    </div>
  );
}

// ---- the figure cards ----

function Figures({
  dashboard,
  seeCostAndMargin,
}: {
  dashboard: DashboardDto;
  seeCostAndMargin: boolean;
}) {
  const { t } = useTranslation();
  const { today, this_month: month } = dashboard;
  return (
    <div className="flex flex-col gap-4">
      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <FigureCard
          label="dashboard_sales"
          centimes={today.sales_ttc_centimes}
          monthCentimes={month.sales_ttc_centimes}
          count={month.sales_count}
          countLabel={t("dashboard_documents")}
          testId="figure-sales"
        />
        {seeCostAndMargin ? (
          <FigureCard
            label="dashboard_margin"
            centimes={today.margin_centimes}
            monthCentimes={month.margin_centimes}
            testId="figure-margin"
          />
        ) : null}
        <FigureCard
          label="dashboard_expenses"
          centimes={today.expenses_centimes}
          monthCentimes={month.expenses_centimes}
          testId="figure-expenses"
        />
        <FigureCard
          label="dashboard_cash"
          centimes={dashboard.cash_today.cash_centimes}
          monthCentimes={dashboard.cash_this_month.cash_centimes}
          testId="figure-cash"
        />
      </div>
      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
        <OwedCard
          label="dashboard_customer_debt"
          centimes={dashboard.customer_debt.total_centimes}
          parties={dashboard.customer_debt.parties}
          testId="figure-customer-debt"
        />
        <OwedCard
          label="dashboard_supplier_debt"
          centimes={dashboard.supplier_debt.total_centimes}
          parties={dashboard.supplier_debt.parties}
          testId="figure-supplier-debt"
        />
        <Card data-testid="figure-open-purchases">
          <CardHeader>
            <CardTitle className="text-sm font-medium text-muted-foreground">
              {t("dashboard_open_purchases")}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-2xl font-numeric font-medium tabular-nums" dir="ltr">
              {dashboard.open_purchases}
            </p>
          </CardContent>
          {/* No sub-line: an order is either open or it is not, and there is
              no second period to compare it against. */}
        </Card>
      </div>
    </div>
  );
}

/**
 * One figure of the day with the same figure of the month under it. Two
 * numbers and not one, because "today" alone says nothing on a slow Tuesday
 * and the month is what the shop is actually running against.
 */
function FigureCard({
  label,
  centimes,
  monthCentimes,
  count,
  countLabel,
  testId,
}: {
  label: Key;
  centimes: number;
  monthCentimes: number;
  count?: number;
  countLabel?: string;
  testId: string;
}) {
  const { t } = useTranslation();
  return (
    <Card data-testid={testId}>
      <CardHeader>
        <CardTitle className="text-sm font-medium text-muted-foreground">{t(label)}</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-1">
        <Money centimes={centimes} className="text-2xl" data-testid={`${testId}-today`} />
        <SubLine label={t("dashboard_this_month")}>
          <Money centimes={monthCentimes} data-testid={`${testId}-month`} />
        </SubLine>
        {count === undefined || countLabel === undefined ? null : (
          <SubLine label={countLabel}>
            <Count value={count} testId={`${testId}-count`} />
          </SubLine>
        )}
      </CardContent>
    </Card>
  );
}

/**
 * A word at the start of the line and its figure at the end.
 *
 * The figure never sits directly after the number's noun, which is the shape
 * that reads as "1 comptes" in French and needs a plural rule the dictionary
 * has no way to carry. Label, then figure: the same line works for one and for
 * a hundred, in the three languages, and it is the shape the expenses screen's
 * cash panel already uses.
 */
function SubLine({ label, children }: { label: string; children: ReactNode }) {
  return (
    <p className="flex items-baseline justify-between gap-4 text-sm text-muted-foreground">
      <span>{label}</span>
      {children}
    </p>
  );
}

/** A count. The figure face and `dir="ltr"`, for the reason an amount has
 *  them; it is not money, so it does not go through `Money`. */
function Count({ value, testId }: { value: number; testId: string }) {
  return (
    <span dir="ltr" data-testid={testId} className="font-numeric font-medium tabular-nums">
      {value}
    </span>
  );
}

/** What is owed one way or the other, and by how many accounts. */
function OwedCard({
  label,
  centimes,
  parties,
  testId,
}: {
  label: Key;
  centimes: number;
  parties: number;
  testId: string;
}) {
  const { t } = useTranslation();
  return (
    <Card data-testid={testId}>
      <CardHeader>
        <CardTitle className="text-sm font-medium text-muted-foreground">{t(label)}</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-1">
        <Money centimes={centimes} className="text-2xl" data-testid={`${testId}-total`} />
        <SubLine label={t("dashboard_accounts")}>
          <Count value={parties} testId={`${testId}-parties`} />
        </SubLine>
      </CardContent>
    </Card>
  );
}

// ---- the chart ----

/** A hundred dinars, as centimes. Only the axis divides by it. */
const CENTIMES_PER_DINAR = 100;

/**
 * A tick label: whole dinars, grouped, no centimes. `Math.trunc` on a safe
 * integer is exact and this is a label on an axis, never a total; every amount
 * a reader is meant to trust on this screen goes through `Money`.
 */
export function wholeDinars(centimes: number): string {
  const negative = centimes < 0;
  const dinars = Math.trunc(Math.abs(centimes) / CENTIMES_PER_DINAR);
  // U+202F, the narrow no-break space `formatCentimes` groups with
  // (features.md, "Numbers are Western digits in every language"). An
  // ordinary space here put `1 284` on the axis against `1 284,00` on the
  // card below it, two different separators for the same figure.
  const grouped = String(dinars).replace(/\B(?=(\d{3})+(?!\d))/g, "\u202f");
  return `${negative ? "-" : ""}${grouped}`;
}

/** `2026-09-10` shown as `10/09`. The year is the window's, said once in the
 *  card's title, so repeating it thirty times along an axis costs room and
 *  tells nobody anything. */
function shortDay(day: string): string {
  return `${day.slice(8, 10)}/${day.slice(5, 7)}`;
}

interface Bucket {
  readonly key: string;
  readonly label: string;
  readonly range: string;
  readonly sales: number;
  readonly margin: number;
  readonly expenses: number;
}

function bucket(point: DashboardSeriesPointDto): Bucket {
  return {
    key: `${point.from}/${point.to}`,
    label: shortDay(point.to),
    range: point.from === point.to ? shortDay(point.to) : `${shortDay(point.from)} — ${shortDay(point.to)}`,
    sales: point.figures.sales_ttc_centimes,
    margin: point.figures.margin_centimes,
    expenses: point.figures.expenses_centimes,
  };
}

type Grain = "days" | "weeks";

function Chart({
  series,
  seeCostAndMargin,
}: {
  series: DashboardSeriesDto;
  seeCostAndMargin: boolean;
}) {
  const { t } = useTranslation();
  const [grain, setGrain] = useState<Grain>("days");
  const points = grain === "days" ? series.days : series.weeks;
  const buckets = points.map(bucket);

  // The colours are role variables, so each of the four themes paints its own
  // version of the same three series and no code here learns which is on.
  // The margin entry is left out of the config, not just off the plot,
  // recharts and the legend both read this map for what exists.
  const config: ChartConfig = seeCostAndMargin
    ? {
        sales: { label: t("dashboard_sales"), color: "var(--color-primary)" },
        margin: { label: t("dashboard_margin"), color: "var(--color-money)" },
        expenses: { label: t("dashboard_expenses"), color: "var(--color-warn)" },
      }
    : {
        sales: { label: t("dashboard_sales"), color: "var(--color-primary)" },
        expenses: { label: t("dashboard_expenses"), color: "var(--color-warn)" },
      };

  return (
    <div className="flex flex-col gap-3">
      <Tabs
        value={grain}
        onValueChange={(value) => setGrain(value === "weeks" ? "weeks" : "days")}
      >
        <TabsList>
          <TabsTrigger value="days" data-testid="chart-grain-days">
            {t("dashboard_chart_days")}
          </TabsTrigger>
          <TabsTrigger value="weeks" data-testid="chart-grain-weeks">
            {t("dashboard_chart_weeks")}
          </TabsTrigger>
        </TabsList>
      </Tabs>

      {buckets.length === 0 ? (
        <EmptyState
          icon={TrendingUp}
          title={t("dashboard_chart_empty")}
          data-testid="dashboard-chart-empty"
        />
      ) : (
        // `dir="ltr"`: see the note at the top of this file.
        // `data-buckets` is how a test tells the day view from the week view.
        // recharts draws a rectangle only for a bar with a height, so counting
        // the bars counts the days the shop sold on rather than the days the
        // chart covers, and the two views of one quiet month look identical
        // through it.
        <div dir="ltr" data-testid="dashboard-chart" data-buckets={buckets.length}>
          <ChartContainer config={config} className="aspect-auto h-64 w-full">
            <ComposedChart data={buckets} margin={{ top: 8, right: 8, bottom: 0, left: 8 }}>
              <CartesianGrid vertical={false} />
              <XAxis
                dataKey="label"
                tickLine={false}
                axisLine={false}
                tickMargin={8}
                minTickGap={16}
              />
              <YAxis
                tickLine={false}
                axisLine={false}
                width={64}
                tickFormatter={wholeDinars}
              />
              <ChartTooltip
                content={
                  <ChartTooltipContent
                    labelFormatter={(_label, payload) => rangeOf(payload)}
                    formatter={(value, name) => <TooltipRow value={value} name={name} />}
                  />
                }
              />
              <ChartLegend content={<ChartLegendContent />} />
              {/* No entry animation on any of the three. recharts grows a bar
                  out of nothing over a second and a half, which on a screen a
                  shopkeeper opens to read a number is a second and a half of
                  wrong numbers, and it makes a committed screenshot a picture
                  of whichever frame the shutter caught. */}
              <Bar
                dataKey="sales"
                fill="var(--color-sales)"
                radius={2}
                maxBarSize={24}
                isAnimationActive={false}
              />
              {seeCostAndMargin ? (
                <Line
                  dataKey="margin"
                  stroke="var(--color-margin)"
                  strokeWidth={2}
                  dot={false}
                  type="monotone"
                  isAnimationActive={false}
                />
              ) : null}
              <Line
                dataKey="expenses"
                stroke="var(--color-expenses)"
                strokeWidth={2}
                strokeDasharray="4 3"
                dot={false}
                type="monotone"
                isAnimationActive={false}
              />
            </ComposedChart>
          </ChartContainer>
        </div>
      )}
    </div>
  );
}

/** The two days a bucket covers, off the first row of the tooltip's payload.
 *  A week reads `04/09 — 10/09`; a day reads once. */
function rangeOf(payload: unknown): string {
  if (!Array.isArray(payload)) return "";
  const [first] = payload;
  if (typeof first !== "object" || first === null) return "";
  const datum: unknown = Reflect.get(first, "payload");
  if (typeof datum !== "object" || datum === null) return "";
  const range: unknown = Reflect.get(datum, "range");
  return typeof range === "string" ? range : "";
}

/** One line of the tooltip: what the series is, and the amount as an amount.
 *  Without this the chart component would print the centimes as a plain
 *  number, which is the figure divided by a hundred and read as dinars. */
function TooltipRow({ value, name }: { value: unknown; name: unknown }) {
  return (
    <span className="flex w-full items-center justify-between gap-4">
      <span className="text-muted-foreground">{typeof name === "string" ? name : ""}</span>
      {typeof value === "number" && Number.isSafeInteger(value) ? (
        <Money centimes={value} />
      ) : null}
    </span>
  );
}

// ---- the three lists ----

function LowStockCard({ rows, className }: { rows: readonly LowStockDto[]; className?: string }) {
  const { t } = useTranslation();
  const columns: readonly Column<LowStockDto>[] = [
    { id: "name", header: t("col_name"), cell: (row) => row.name },
    {
      id: "stock",
      header: t("col_stock"),
      numeric: true,
      cell: (row) => (
        <span dir="ltr">{formatQty(row.qty_on_hand_milli)}</span>
      ),
    },
    {
      id: "threshold",
      header: t("col_threshold"),
      numeric: true,
      cell: (row) => <span dir="ltr">{formatQty(row.low_stock_at_milli)}</span>,
    },
    {
      id: "status",
      header: t("col_status"),
      // Every row here is under its threshold, so the pill is not telling one
      // row from another: it is the colour a shopkeeper scans the list for,
      // and it is the kit's pill rather than a coloured cell so it says the
      // word too, on paper and on the ink theme alike.
      cell: () => <StatusPill status="low" data-testid="low-stock-pill" />,
    },
  ];
  return (
    <Card className={className} data-testid="dashboard-low-stock">
      <CardHeader>
        <CardTitle>{t("dashboard_low_stock")}</CardTitle>
      </CardHeader>
      <CardContent>
        <DataTable
          columns={columns}
          rows={rows}
          rowKey={(row) => row.product_id}
          caption={t("dashboard_low_stock")}
          empty={
            <EmptyState
              icon={AlertTriangle}
              title={t("dashboard_low_stock_empty")}
              data-testid="dashboard-low-stock-empty"
            />
          }
        />
      </CardContent>
    </Card>
  );
}

function TopCard({
  title,
  rows,
  testId,
}: {
  title: Key;
  rows: readonly TopProductDto[];
  testId: string;
}) {
  const { t } = useTranslation();
  const columns: readonly Column<TopProductDto>[] = [
    { id: "name", header: t("col_name"), cell: (row) => row.name },
    {
      id: "qty",
      header: t("col_qty"),
      numeric: true,
      cell: (row) => <span dir="ltr">{formatQty(row.qty_milli)}</span>,
    },
    {
      id: "margin",
      header: t("col_margin"),
      money: true,
      cell: (row) => <Money centimes={row.margin_centimes} />,
    },
  ];
  return (
    <Card data-testid={testId}>
      <CardHeader>
        <CardTitle>{t(title)}</CardTitle>
      </CardHeader>
      <CardContent>
        <DataTable
          columns={columns}
          rows={rows}
          rowKey={(row) => row.product_id}
          caption={t(title)}
          empty={
            <EmptyState
              icon={PackageSearch}
              title={t("dashboard_top_empty")}
              data-testid={`${testId}-empty`}
            />
          }
        />
      </CardContent>
    </Card>
  );
}
