// The split into `client/` per domain costs two compile errors the single
// object literal used to give for free, and this file buys both back.
//
// Dropping a `...xClient(transport)` line from `createClient` deletes every
// method that file owned, and nothing else in this package notices: the
// suites here call a minority of the domain methods, so most of them can
// vanish silently. `claimPairing` is the sharpest case, with no caller in the
// desktop or the phone either, so no `tsc` run anywhere would go red.
//
// A method name used by two domain files is the other one. In one literal a
// repeated key was an error; across two spreads the later one quietly wins
// and `tsc --strict` says nothing, checked on 2026-09-20 with two factories
// both returning `hi`, one typed number and one string: exit 0, and the
// result typed as the second.
//
// So the list below is written out by hand rather than read off the client,
// which would assert only that the client equals itself.

import { describe, expect, test } from "vitest";
import { createClient } from "../src/client";
import type { Transport } from "../src/client";
import { auditClient } from "../src/client/audit";
import { bookClient } from "../src/client/book";
import { authClient } from "../src/client/auth";
import { backupsClient } from "../src/client/backups";
import { categoriesClient } from "../src/client/categories";
import { customersClient } from "../src/client/customers";
import { dashboardClient } from "../src/client/dashboard";
import { expensesClient } from "../src/client/expenses";
import { exportClient } from "../src/client/export";
import { importClient } from "../src/client/import";
import { pairingClient } from "../src/client/pairing";
import { patientsClient } from "../src/client/patients";
import { productsClient } from "../src/client/products";
import { purchasesClient } from "../src/client/purchases";
import { queueClient } from "../src/client/queue";
import { salesClient } from "../src/client/sales";
import { settingsClient } from "../src/client/settings";
import { stockClient } from "../src/client/stock";
import { suppliersClient } from "../src/client/suppliers";
import { supportClient } from "../src/client/support";
import { tillClient } from "../src/client/till";
import { usersClient } from "../src/client/users";

/** Every call the client answers, by the file that owns it. A method added
 * to a domain file is added here in the same commit; that is the point of
 * writing it twice. */
const SURFACE: Readonly<Record<string, readonly string[]>> = {
  "client.ts": ["setSession", "logout", "health", "getBuildInfo", "clock"],
  audit: ["listAuditLog"],
  auth: ["login", "claimFirstOwner", "me", "sessionIdle", "listStaff"],
  backups: ["listBackups", "createBackup", "restoreBackup"],
  categories: ["listCategories"],
  customers: [
    "listCustomers",
    "getCustomer",
    "createCustomer",
    "updateCustomer",
    "customerLedger",
    "adjustCustomerDebt",
    "customerPayments",
    "payCustomer",
    "customerStatement",
    "customerDebtSlip",
  ],
  dashboard: ["dashboard", "dashboardSeries"],
  expenses: ["listExpenses", "listExpenseCategories", "createExpense", "cashPosition"],
  export: ["exportWorkbook"],
  import: ["importTemplate", "dryRunProductImport", "applyProductImport"],
  pairing: ["createPairingQr", "listPairedDevices", "revokePairedDevice", "claimPairing"],
  products: ["listProducts", "updateProduct", "createProduct", "getProductLabel", "getLabelSheet"],
  purchases: [
    "listPurchases",
    "getPurchase",
    "createPurchase",
    "receivePurchase",
    "returnPurchase",
    "cancelPurchase",
    "closeShortPurchase",
  ],
  sales: [
    "createSale",
    "getSale",
    "getSaleTicket",
    "getSaleFacture",
    "listSales",
    "createAvoir",
    "listAvoirs",
    "cancelSale",
  ],
  settings: [
    "getSettings",
    "updateStore",
    "changeRegime",
    "setDiscountThreshold",
    "setTheme",
    "setFactureLayout",
    "setPrintLang",
    "setThermalMode",
  ],
  stock: ["lastStockRecount", "recountStock"],
  suppliers: [
    "listSuppliers",
    "getSupplier",
    "createSupplier",
    "updateSupplier",
    "closeSupplier",
    "supplierLedger",
    "paySupplier",
    "adjustSupplierDebt",
    "supplierStatement",
  ],
  support: ["supportBundle"],
  till: ["openShift", "closeShift", "getOpenShift", "getShift", "listShifts"],
  users: ["listUsers", "createUser", "setUserPin", "deactivateUser", "reactivateUser"],
  patients: ["listPatients", "getPatient", "createPatient", "updatePatient", "archivePatient"],
  queue: [
    "listQueue",
    "addToQueue",
    "callNextInQueue",
    "callInQueue",
    "markSeenInQueue",
    "markLeftInQueue",
  ],
  book: [
    "listAppointments",
    "getAppointment",
    "bookAppointment",
    "cancelAppointment",
    "moveAppointment",
    "markAppointmentNoShow",
    "clearAppointmentNoShow",
    "getSlotMinutes",
    "setSlotMinutes",
    "getWorkingHours",
    "setWorkingHours",
    "listAbsenceBlocks",
    "createAbsenceBlock",
    "removeAbsenceBlock",
    "listVisitTypes",
    "createVisitType",
    "updateVisitType",
    "removeVisitType",
    "nextFreeSlot",
    "dayList",
  ],
};

/** A transport that would fail loudly if a factory called it while being
 * built. Nothing here makes a request: the subject is which names exist. */
const unused: Transport = {
  send() {
    throw new Error("the surface test makes no request");
  },
  sendText() {
    throw new Error("the surface test makes no request");
  },
  sendFile() {
    throw new Error("the surface test makes no request");
  },
};

const FACTORIES: Readonly<Record<string, () => object>> = {
  audit: () => auditClient(unused),
  auth: () => authClient(unused),
  backups: () => backupsClient(unused),
  categories: () => categoriesClient(unused),
  customers: () => customersClient(unused),
  dashboard: () => dashboardClient(unused),
  expenses: () => expensesClient(unused),
  export: () => exportClient(unused),
  import: () => importClient(unused),
  pairing: () => pairingClient(unused),
  products: () => productsClient(unused),
  purchases: () => purchasesClient(unused),
  sales: () => salesClient(unused),
  settings: () => settingsClient(unused),
  stock: () => stockClient(unused),
  suppliers: () => suppliersClient(unused),
  support: () => supportClient(unused),
  till: () => tillClient(unused),
  users: () => usersClient(unused),
  patients: () => patientsClient(unused),
  queue: () => queueClient(unused),
  book: () => bookClient(unused),
};

function methodsOf(client: object): Record<string, unknown> {
  const flat: Record<string, unknown> = { ...client };
  return flat;
}

describe("the client's surface", () => {
  const client = methodsOf(createClient("http://127.0.0.1:4317"));

  for (const [owner, names] of Object.entries(SURFACE)) {
    for (const name of names) {
      test(`${owner} answers ${name}`, () => {
        expect(typeof client[name]).toBe("function");
      });
    }
  }

  test("and answers nothing beyond the list", () => {
    const listed = Object.values(SURFACE).flat();
    const found = Object.keys(client).filter((key) => typeof client[key] === "function");
    expect([...found].sort()).toEqual([...listed].sort());
  });

  test("with every domain's names its own", () => {
    const seen = new Map<string, string>();
    const clash: string[] = [];
    for (const [owner, build] of Object.entries(FACTORIES)) {
      for (const name of Object.keys(build())) {
        const first = seen.get(name);
        if (first === undefined) {
          seen.set(name, owner);
        } else {
          clash.push(`${name}: ${first} and ${owner}`);
        }
      }
    }
    expect(clash).toEqual([]);
  });
});
