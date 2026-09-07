-- Add UNIQUE constraint on suppliers.name to prevent duplicate suppliers
CREATE UNIQUE INDEX IF NOT EXISTS idx_suppliers_name ON suppliers (name);