-- Create expense_categories lookup table
CREATE TABLE expense_categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Seed default categories (no Tabac/Chema per user request)
INSERT INTO expense_categories (name) VALUES ('Livraison');
INSERT INTO expense_categories (name) VALUES ('Électricité');
INSERT INTO expense_categories (name) VALUES ('Eau');
INSERT INTO expense_categories (name) VALUES ('Loyer');
INSERT INTO expense_categories (name) VALUES ('Autre');