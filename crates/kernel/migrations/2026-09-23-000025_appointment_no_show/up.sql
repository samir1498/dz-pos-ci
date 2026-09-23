-- The no-show mark, the fifth of the book tools (C5b of
-- context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md):
-- the moment the desk marked a past appointment as one the patient did not
-- come to, kept on the row. NULL while unmarked. Never on a cancelled row:
-- a slot given back was not missed. That the start is past is the
-- service's check, the clock being the shop's. The stamp is the shop
-- clock's, fractions of a second and all, like `cancelled_at`, so no
-- `datetime()` CHECK holds it to whole seconds.
--
-- A column added, no row moved, so the revert drops the column. Nothing
-- indexes it and no other table names it.

ALTER TABLE appointments ADD COLUMN no_show_at TEXT
    CHECK (no_show_at IS NULL OR cancelled_at IS NULL);
