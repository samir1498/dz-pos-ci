// Every list in the app is this table. A screen describes its columns and
// hands over its rows; it does not write `<table>`, and the eslint rule in
// eslint.config.js is what keeps that true.
//
// Three things it does that a hand-written table on each screen kept getting
// wrong in different ways:
//
// - A money column is right-aligned in the reading sense (`text-end`) and set
//   in the figure face with tabular figures, so a column of totals lines up
//   on the digit. Declaring `money: true` on the column is what turns that
//   on; the cell still renders `Money`, because the grouping and the `ltr`
//   belong to the amount rather than to the table.
// - The header band and the row hover come from the token roles once, so the
//   dark themes get a dark band instead of the light one a screen picked by
//   eye.
// - An empty list renders the caller's empty state inside the same frame
//   instead of an empty `<tbody>`, which is a table that looks broken rather
//   than a list that is not filled yet.
//
// Row actions live in a last column with no header text; the header cell
// still exists and carries an off-screen label, because a header cell with no
// accessible name is what makes a screen reader announce the column as blank.

import type { ReactNode } from "react";

import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { useTranslation } from "@/i18n";
import { cn } from "@/lib/utils";

export interface Column<Row> {
  /** Stable across renders; it is the React key of the cell. */
  readonly id: string;
  /** Already translated. The table does no i18n of its own. */
  readonly header: string;
  /**
   * An amount. Sets the figure face, tabular figures and end alignment on
   * both the heading and the cells. The cell itself still renders `Money`.
   */
  readonly money?: boolean;
  /** A count or a quantity: aligned like an amount, set in the body face. */
  readonly numeric?: boolean;
  readonly cell: (row: Row) => ReactNode;
}

export function DataTable<Row>({
  columns,
  rows,
  rowKey,
  caption,
  empty,
  actions,
  className,
  "data-testid": testId,
}: {
  readonly columns: readonly Column<Row>[];
  readonly rows: readonly Row[];
  /** What makes a row itself. An index would reorder wrongly on a sort. */
  readonly rowKey: (row: Row) => string | number;
  /** Read by a screen reader before the table; not painted. */
  readonly caption: string;
  /** Shown in place of the body when there are no rows. */
  readonly empty?: ReactNode;
  readonly actions?: (row: Row) => ReactNode;
  readonly className?: string;
  readonly "data-testid"?: string;
}) {
  const { t } = useTranslation();
  const align = (column: Column<Row>) =>
    column.money === true || column.numeric === true ? "text-end" : "text-start";
  const face = (column: Column<Row>) =>
    column.money === true ? "font-numeric tabular-nums" : column.numeric === true ? "tabular-nums" : "";

  if (rows.length === 0 && empty !== undefined) {
    return <div data-testid={testId}>{empty}</div>;
  }

  return (
    <div
      data-testid={testId}
      className={cn("overflow-hidden rounded-lg border border-border bg-card", className)}
    >
      <Table>
        <caption className="sr-only">{caption}</caption>
        <TableHeader className="bg-muted">
          <TableRow>
            {columns.map((column) => (
              <TableHead key={column.id} className={cn(align(column), face(column))}>
                {column.header}
              </TableHead>
            ))}
            {actions === undefined ? null : (
              <TableHead className="text-end">
                <span className="sr-only">{t("table_actions")}</span>
              </TableHead>
            )}
          </TableRow>
        </TableHeader>
        <TableBody>
          {rows.map((row) => (
            <TableRow key={rowKey(row)}>
              {columns.map((column) => (
                <TableCell key={column.id} className={cn(align(column), face(column))}>
                  {column.cell(row)}
                </TableCell>
              ))}
              {actions === undefined ? null : (
                <TableCell className="text-end">
                  <div className="flex items-center justify-end gap-1">{actions(row)}</div>
                </TableCell>
              )}
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}
