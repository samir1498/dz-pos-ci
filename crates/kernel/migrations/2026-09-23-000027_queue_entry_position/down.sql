-- The up added one index and one column and moved no row. The index goes
-- first: SQLite refuses to drop a column an index names. Every entry stays;
-- the day's list reads in arrival order again.
DROP INDEX idx_queue_entries_order;
ALTER TABLE queue_entries DROP COLUMN position;
