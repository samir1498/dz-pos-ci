-- The appointment book, the clinic module's third table (C5 of
-- context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md).
-- One doctor, one slot length set in settings, and a refusal when a second
-- patient takes a booked slot. No recurrence, no reminders, no rota.
--
-- In the one shared migration list, like `patients` and `queue_entries`: a
-- shop install carries this table empty.
--
-- Additive. No existing table is rebuilt and no row is moved, so the revert
-- is a DROP.

CREATE TABLE appointments (
    -- A UUID v7 as its 36-character text, made by the clinic service, for
    -- the reason `patients.id` gives.
    id            TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    -- The kernel's tenant. RESTRICT, with every other table.
    shop_id       INTEGER NOT NULL REFERENCES shops(id) ON DELETE RESTRICT,
    -- RESTRICT for the reason `queue_entries.patient_id` gives. That the
    -- patient is of the same shop is the service's check.
    patient_id    TEXT NOT NULL REFERENCES patients(id) ON DELETE RESTRICT,
    -- The shop clock's local time the slot starts, `YYYY-MM-DD HH:MM:SS`,
    -- the queue's stamp format. `datetime()` reads anything else as another
    -- text or as NULL, so `IS` refuses a fraction, a `T` or a stray zone;
    -- the second CHECK keeps it on a whole minute. That it sits on the slot
    -- grid is the service's check: the grid is a setting, not a column.
    starts_at     TEXT NOT NULL CHECK (datetime(starts_at) IS starts_at),
    -- The slot length in force when the appointment was booked (or last
    -- moved). Stored per row, because the setting can change under a book
    -- already full: yesterday's 30-minute slots stay 30 minutes long. 240 is
    -- a ceiling well above the setting's own 120, so a later wider setting
    -- needs no rebuild of this table.
    slot_minutes  INTEGER NOT NULL CHECK (slot_minutes > 0 AND slot_minutes <= 240),
    -- A short reason for the visit as the desk writes it ("contrôle",
    -- "certificat"), optional. Not the clinical notes: those live on the
    -- patient's file, behind whatever permission the notes end up with.
    note          TEXT CHECK (note IS NULL OR length(trim(note)) > 0),
    -- The shop's clock when the slot was given back, NULL while it is live.
    -- A moment rather than a status column: cancelled is the only way out
    -- of the book (arriving and being seen are the queue's), and the moment
    -- is what the audit and the screen want.
    cancelled_at  TEXT,
    -- The shop's clock, no DEFAULT, for the reason `patients` gives.
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,
    CHECK (strftime('%S', starts_at) = '00')
) STRICT;

-- One live appointment per shop per start. The service refuses a taken
-- slot first, and any overlap with a live slot of another length; this
-- index is what holds when two desks book the same start in the same
-- instant. A cancelled row stops counting, so its slot can be booked again.
-- Also the index every read uses: the day and the week read live rows only,
-- by `(shop_id, starts_at)`.
CREATE UNIQUE INDEX idx_appointments_live
    ON appointments (shop_id, starts_at)
    WHERE cancelled_at IS NULL;
