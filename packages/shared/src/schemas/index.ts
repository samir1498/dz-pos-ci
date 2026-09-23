// One schema per generated DTO, grouped by the family it belongs to. Each is
// declared `satisfies z.ZodType<Dto>` against `src/generated` and carries the
// other half of the drift check beside it (`./drift`), so a schema that stops
// matching the Rust struct fails `just types-check`.

export { day, exactInteger } from "./common";
export type { Assert, Covers, Equal, Matches, SameKeys } from "./drift";
export { apiErrorPayloadSchema, apiErrorSchema } from "./error";
export { auditEntrySchema, auditLogSchema, auditUserSchema } from "./audit";
export { categorySchema, productSchema, unitSchema } from "./catalogue";
export { deviceTokenSchema, pairedDeviceSchema, pairingQrSchema } from "./pairing";
export { staffSchema } from "./staff";
export {
  importAppliedSchema,
  importDryRunSchema,
  importOutcomeSchema,
  importRowSchema,
  labelSheetSchema,
  LABEL_SHEET_MAX,
} from "./import";
export {
  dashboardFiguresSchema,
  dashboardSchema,
  lowStockSchema,
  owedSchema,
  topProductSchema,
} from "./dashboard";
export {
  meSchema,
  permissionSchema,
  roleSchema,
  sessionIdleSchema,
  sessionSchema,
} from "./session";
export {
  backupSchema,
  backupsSchema,
  buildInfoSchema,
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
export { lastStockRecountSchema, stockDriftSchema, stockRecountSchema } from "./stock";
export { shiftReportSchema, shiftSchema } from "./till";
export {
  closeOrderSchema,
  newPurchaseLineSchema,
  newPurchaseSchema,
  newReceiptSchema,
  paidNowSchema,
  purchaseDetailSchema,
  purchaseLineSchema,
  purchaseReceiptLineSchema,
  purchaseReceiptSchema,
  purchaseSchema,
  purchaseStatusSchema,
  receiveLineSchema,
} from "./purchase";
export {
  supplierAllocationSchema,
  supplierDebtKindSchema,
  supplierEntrySchema,
  supplierLedgerSchema,
  supplierSchema,
  supplierStatementSchema,
} from "./supplier";
export { userSchema } from "./user";
export { patientSchema, patientWriteSchema, sexSchema } from "./patient";
export { queueAddSchema, queueEntrySchema } from "./queue";
