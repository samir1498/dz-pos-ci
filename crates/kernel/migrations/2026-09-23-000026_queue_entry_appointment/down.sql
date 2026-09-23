-- The up added one index and one column and moved no row. The index goes
-- first: SQLite refuses to drop a column an index names. Every entry stays;
-- a booked arrival reads as a walk-in again.
DROP INDEX idx_queue_entries_appointment;
ALTER TABLE queue_entries DROP COLUMN appointment_id;
