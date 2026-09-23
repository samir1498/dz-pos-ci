-- The link from a queue entry to the appointment it came in for (C6b of
-- context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md):
-- the desk marks a booked patient arrived, and that adds them to the day's
-- queue with this column naming the booking. NULL for a walk-in, which is
-- every entry written before this migration.
--
-- A column added, no row moved, so the revert drops the index and the
-- column.

-- RESTRICT for the reason `queue_entries.patient_id` gives: an appointment
-- is cancelled, never deleted. That it is of the same shop is the
-- service's check, as the patient's is. The CHECK is the id's own length,
-- the one `appointments.id` holds.
ALTER TABLE queue_entries ADD COLUMN appointment_id TEXT
    REFERENCES appointments(id) ON DELETE RESTRICT
    CHECK (appointment_id IS NULL OR length(appointment_id) = 36);

-- One entry per appointment, ever: marking a booked patient arrived twice
-- adds no second entry. The service returns the first one before it gets
-- here; this index is what holds when two desks mark the same booking in
-- the same instant. Walk-ins (NULL) do not count.
CREATE UNIQUE INDEX idx_queue_entries_appointment
    ON queue_entries (appointment_id)
    WHERE appointment_id IS NOT NULL;
