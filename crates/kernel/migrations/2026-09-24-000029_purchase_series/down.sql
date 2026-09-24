-- The index first: SQLite refuses to drop a column an index names. The
-- counters the up seeded, and every one `take_next` created since, go with
-- the numbers they handed out.
DROP INDEX idx_purchases_series_number;
DELETE FROM counters WHERE name LIKE 'purchase:%';
ALTER TABLE purchases DROP COLUMN number;
ALTER TABLE purchases DROP COLUMN series_year;
