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
import { createClient } from "./client";
import type { Transport } from "./client";
import { auditClient } from "./client/audit";
import { authClient } from "./client/auth";
import { backupsClient } from "./client/backups";
import { categoriesClient } from "./client/categories";
import { customersClient } from "./client/customers";
import { dashboardClient } from "./client/dashboard";
import { expensesClient } from "./client/expenses";
import { exportClient } from "./client/export";
import { importClient } from "./client/import";
import { pairingClient } from "./client/pairing";
import { productsClient } from "./client/products";
import { purchasesClient } from "./client/purchases";
import { salesClient } from "./client/sales";
import { settingsClient } from "./client/settings";
import { stockClient } from "./client/stock";
import { suppliersClient } from "./client/suppliers";
import { supportClient } from "./client/support";
import { tillClient } from "./client/till";
import { usersClient } from "./client/users";

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
