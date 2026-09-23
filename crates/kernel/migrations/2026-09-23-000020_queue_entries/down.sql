-- The indexes go with the table they are on. The up added a table and
-- touched no row of any other, so this is the whole of the reverse.
DROP INDEX idx_queue_entries_shop_day;
DROP INDEX idx_queue_entries_live;
DROP TABLE queue_entries;
