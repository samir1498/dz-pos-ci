-- The confirmation call, kept on the appointment (C6b of
-- context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md):
-- the desk rings the day before and records what came of it, confirmed or
-- no answer, with the moment. Samir, 2026-09-23 19:34: the desk marks a
-- call's outcome and decides what follows; nothing here cancels, moves or
-- marks anything because of it. The desk may clear it again.
--
-- Two columns added, no row moved, so the revert drops them.

-- What came of the last call: 'confirmed' or 'no_answer', NULL before any
-- call or once cleared.
ALTER TABLE appointments ADD COLUMN call_outcome TEXT
    CHECK (call_outcome IS NULL OR call_outcome IN ('confirmed', 'no_answer'));

-- The shop clock's moment the outcome was recorded, fractions of a second
-- and all like `no_show_at`. Set exactly when there is an outcome.
ALTER TABLE appointments ADD COLUMN call_at TEXT
    CHECK ((call_at IS NULL) = (call_outcome IS NULL));
