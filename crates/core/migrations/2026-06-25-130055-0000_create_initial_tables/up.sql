-- Suppliers
CREATE TABLE suppliers (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL,
    phone       TEXT,
    notes       TEXT
);

-- Products
CREATE TABLE products (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT NOT NULL,
    barcode         TEXT UNIQUE,
    cost_price      REAL NOT NULL DEFAULT 0,
    selling_price   REAL NOT NULL DEFAULT 0,
    quantity_on_hand INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Purchase Orders
CREATE TABLE purchase_orders (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    purchase_order_number TEXT NOT NULL UNIQUE,
    supplier_id           INTEGER REFERENCES suppliers(id),
    status                TEXT NOT NULL DEFAULT 'Draft',
    notes                 TEXT,
    total                 REAL NOT NULL DEFAULT 0,
    created_at            TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Purchase Order Items
CREATE TABLE purchase_order_items (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    purchase_order_id  INTEGER NOT NULL REFERENCES purchase_orders(id) ON DELETE CASCADE,
    product_name       TEXT NOT NULL,
    quantity           INTEGER NOT NULL DEFAULT 1,
    unit_cost          REAL NOT NULL DEFAULT 0,
    line_total         REAL NOT NULL DEFAULT 0
);

-- Expenses
CREATE TABLE expenses (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    date            TEXT NOT NULL,
    category        TEXT NOT NULL,
    description     TEXT,
    amount          REAL NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Transactions
CREATE TABLE transactions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp       TEXT NOT NULL DEFAULT (datetime('now')),
    total           REAL NOT NULL DEFAULT 0
);

-- Transaction Items
CREATE TABLE transaction_items (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    transaction_id  INTEGER NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
    product_id      INTEGER NOT NULL REFERENCES products(id),
    product_name    TEXT NOT NULL,
    quantity        INTEGER NOT NULL DEFAULT 1,
    selling_price   REAL NOT NULL DEFAULT 0,
    cost_price      REAL NOT NULL DEFAULT 0,
    line_total      REAL NOT NULL DEFAULT 0
);
