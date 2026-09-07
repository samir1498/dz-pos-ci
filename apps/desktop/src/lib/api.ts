import { invoke } from "@tauri-apps/api/core";

// ── Types matching the Rust commands ──

export interface Product {
  id: number;
  name: string;
  barcode: string | null;
  cost_price: number;
  selling_price: number;
  quantity_on_hand: number;
  created_at: string;
}

export interface ProductInput {
  name: string;
  barcode: string | null;
  cost_price: number;
  selling_price: number;
  quantity_on_hand: number;
}

export interface Transaction {
  id: number;
  timestamp: string;
  total: number;
}

export interface TransactionItem {
  id: number;
  transaction_id: number;
  product_id: number;
  product_name: string;
  quantity: number;
  selling_price: number;
  cost_price: number;
  line_total: number;
}

export interface SaleResult {
  transaction: Transaction;
  items: TransactionItem[];
}

export interface SaleInput {
  timestamp: string;
  items: SaleItemInput[];
}

export interface SaleItemInput {
  product_id: number;
  product_name: string;
  quantity: number;
  selling_price: number;
  cost_price: number;
}

export interface PurchaseOrder {
  id: number;
  purchase_order_number: string;
  supplier_id: number | null;
  status: string;
  notes: string | null;
  total: number;
  created_at: string;
}

export interface PurchaseOrderItem {
  id: number;
  purchase_order_id: number;
  product_name: string;
  quantity: number;
  unit_cost: number;
  line_total: number;
}

export interface CreatePurchaseOrderInput {
  purchase_order_number: string;
  supplier_id: number | null;
  notes: string | null;
  items: PurchaseOrderItemInput[];
}

export interface PurchaseOrderItemInput {
  product_name: string;
  quantity: number;
  unit_cost: number;
}

export interface Expense {
  id: number;
  date: string;
  expense_category_id: number;
  description: string;
  amount: number;
}

export interface ExpenseCategory {
  id: number;
  name: string;
}

export interface CreateExpenseInput {
  date: string;
  category: string;
  description: string;
  amount: number;
}

export interface Supplier {
  id: number;
  name: string;
  phone: string | null;
  city: string | null;
  notes: string | null;
}

export interface SupplierInput {
  name: string;
  phone: string | null;
  notes: string | null;
}

export interface DashboardData {
  sales_total: number;
  expenses_total: number;
  transaction_count: number;
  cost_total: number;
  profit: number;
}

export interface DbInfo {
  db_path: string;
  product_count: number;
}

export interface PaginatedResult<T> {
  items: T[];
  total: number;
  page: number;
  per_page: number;
}

// ── Invoke helpers ──

function cmd<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args);
}

// Products
export const listProducts = (page?: number, perPage?: number) =>
  cmd<PaginatedResult<Product>>("list_products", { page, perPage });

export const getProduct = (id: number) =>
  cmd<Product | null>("get_product", { id });

export const createProduct = (input: ProductInput) =>
  cmd<Product>("create_product", { input });

export const updateProduct = (id: number, input: ProductInput) =>
  cmd<void>("update_product", { id, input });

export const deleteProduct = (id: number) =>
  cmd<boolean>("delete_product", { id });

// Sales
export const recordSale = (input: SaleInput) =>
  cmd<SaleResult>("record_sale", { input });

export const listSales = (date: string, page?: number, perPage?: number) =>
  cmd<PaginatedResult<Transaction>>("list_sales", { date, page, perPage });

// Purchase Orders
export const listPurchaseOrders = (page?: number, perPage?: number) =>
  cmd<PaginatedResult<PurchaseOrder>>("list_purchase_orders", { page, perPage });

export const createPurchaseOrder = (input: CreatePurchaseOrderInput) =>
  cmd<{ order: PurchaseOrder; items: PurchaseOrderItem[] }>("create_purchase_order", { input });

export const receivePurchaseOrder = (id: number) =>
  cmd<void>("receive_purchase_order", { id });

// Expenses
export const listExpenses = (startDate: string, endDate: string, page?: number, perPage?: number) =>
  cmd<PaginatedResult<Expense>>("list_expenses", { start_date: startDate, end_date: endDate, page, perPage });

export const createExpense = (input: CreateExpenseInput) =>
  cmd<Expense>("create_expense", { input });

// Categories
export const listExpenseCategories = () =>
  cmd<ExpenseCategory[]>("list_expense_categories");

// Dashboard
export const getDashboardData = (date: string) =>
  cmd<DashboardData>("get_dashboard_data", { date });

export const getLowStockProducts = (threshold?: number) =>
  cmd<Product[]>("get_low_stock_products", { threshold });

// Suppliers
export const listSuppliers = () =>
  cmd<Supplier[]>("list_suppliers");

export const createSupplier = (input: SupplierInput) =>
  cmd<Supplier>("create_supplier", { input });

export const updateSupplier = (id: number, input: SupplierInput) =>
  cmd<void>("update_supplier", { id, input });

export const deleteSupplier = (id: number) =>
  cmd<boolean>("delete_supplier", { id });

export const deleteExpense = (id: number) =>
  cmd<boolean>("delete_expense", { id });

export const deletePurchaseOrder = (id: number) =>
  cmd<boolean>("delete_purchase_order", { id });

// Info
export const getDbInfo = () =>
  cmd<DbInfo>("get_db_info");