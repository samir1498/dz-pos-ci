-- `contenance_unit` first: its CHECK names `contenance_milli`, and SQLite
-- refuses to drop a column another column's CHECK names.
ALTER TABLE products DROP COLUMN contenance_unit;
ALTER TABLE products DROP COLUMN contenance_milli;
