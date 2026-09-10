// One schema per generated DTO, grouped by the family it belongs to. Each is
// declared `satisfies z.ZodType<Dto>` against `src/generated` and carries the
// other half of the drift check beside it (`./drift`), so a schema that stops
// matching the Rust struct fails `just types-check`.

export { day, exactInteger } from "./common";
export type { Assert, Covers, Equal, Matches, SameKeys } from "./drift";
export { apiErrorPayloadSchema, apiErrorSchema } from "./error";
export { categorySchema, productSchema, unitSchema } from "./catalogue";
export {
  backupSchema,
  backupsSchema,
  clockSchema,
  datedRegimeSchema,
  healthSchema,
  regimeSchema,
  restoreSchema,
  settingsSchema,
  storeSchema,
} from "./settings";
export {
  documentKindSchema,
  documentStatusSchema,
  paymentModeSchema,
  saleBalanceSchema,
  saleCancelEffectSchema,
  saleCancellationSchema,
  saleKindSchema,
  saleLineSchema,
  saleSchema,
  saleTotalsSchema,
  saleTvaSchema,
  saleWarningSchema,
} from "./sale";
export {
  cashPositionSchema,
  expenseCategorySchema,
  expenseSchema,
  expensesSchema,
  month,
  newExpenseSchema,
  outgoingsSchema,
  takingsSchema,
} from "./expense";
export {
  customerLedgerSchema,
  customerPaymentsSchema,
  customerSchema,
  debtEntrySchema,
  debtKindSchema,
  partyKindSchema,
  paymentAllocationSchema,
  paymentMethodSchema,
  paymentSchema,
} from "./customer";
export {
  supplierAllocationSchema,
  supplierDebtKindSchema,
  supplierEntrySchema,
  supplierLedgerSchema,
  supplierSchema,
  supplierStatementSchema,
} from "./supplier";
