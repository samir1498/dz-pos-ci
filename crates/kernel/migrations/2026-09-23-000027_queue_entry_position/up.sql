-- The desk's own order of the waiting room (C6b of
-- context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md,
-- Samir 2026-09-23 19:32): staff drag patients into the order they will
-- go in, a booked patient who came early included, so the queue needs an
-- order of its own and not arrival time alone. `position` is the entry's
-- place in its day's list, 1 first; a new arrival takes the day's last
-- place plus one, and a reorder rewrites the day's places 1 to n. No rule
-- of the software moves anybody.
--
-- Not unique: SQLite checks a UNIQUE index row by row inside one UPDATE,
-- so a reorder that swaps two places would trip on its own half-way state.
-- Ties cannot come from the service; the reads break one by arrival and id
-- all the same.
--
-- `NOT NULL` needs a DEFAULT to be added to a table with rows. 1 is only
-- what the ALTER writes before the UPDATE below numbers the rows already
-- there; the service always writes the place itself.
--
-- A column added and filled, no row moved, so the revert drops the index
-- and the column.

ALTER TABLE queue_entries ADD COLUMN position INTEGER NOT NULL DEFAULT 1
    CHECK (position > 0);

-- The entries already there keep the order they had, arrival then id: each
-- takes the count of its day's entries up to and including itself.
UPDATE queue_entries
SET position = (
    SELECT COUNT(*)
    FROM queue_entries AS earlier
    WHERE earlier.shop_id = queue_entries.shop_id
      AND earlier.day = queue_entries.day
      AND (earlier.arrived_at < queue_entries.arrived_at
           OR (earlier.arrived_at = queue_entries.arrived_at
               AND earlier.id <= queue_entries.id))
);

-- The day's list in the desk's order, which is how the service reads it now.
CREATE INDEX idx_queue_entries_order ON queue_entries (shop_id, day, position);
