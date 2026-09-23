-- The waiting queue, the clinic module's second table (C4 of
-- context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md).
-- Patients in arrival order, called one by one, marked seen: the shape many
-- cabinets run on paper
-- (context/research/20260922-a-doctors-cabinet-day-on-paper.md). One row is
-- one arrival; a patient who comes back after being seen is a second row.
--
-- In the one shared migration list, like `patients`: a shop install carries
-- this table empty.
--
-- Additive. No existing table is rebuilt and no row is moved, so the revert
-- is a DROP.

CREATE TABLE queue_entries (
    -- A UUID v7 as its 36-character text, made by the clinic service, for
    -- the reason `patients.id` gives.
    id          TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    -- The kernel's tenant. RESTRICT, with every other table.
    shop_id     INTEGER NOT NULL REFERENCES shops(id) ON DELETE RESTRICT,
    -- RESTRICT: a patient file is archived, never deleted, and the day's
    -- queue has to read the name back. That the patient is of the same shop
    -- is the service's check: `patients` has no (shop_id, id) key for a
    -- two-column reference to point at.
    patient_id  TEXT NOT NULL REFERENCES patients(id) ON DELETE RESTRICT,
    -- The shop clock's local day of the arrival, ISO text (YYYY-MM-DD). A
    -- column of its own rather than read off arrived_at, so the day's list
    -- and the one-live-entry index below are both a plain equality. `IS` and
    -- not `=`, for the reason `patients.date_of_birth` gives.
    day         TEXT NOT NULL CHECK (date(day) IS day),
    -- The shop's clock, stamped by the service. The four moments of an
    -- entry: it arrives, is called in, and is then seen or leaves unseen.
    arrived_at  TEXT NOT NULL,
    called_at   TEXT,
    seen_at     TEXT,
    left_at     TEXT,
    -- The shop's clock, no DEFAULT, for the reason `patients` gives.
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    -- The day is the arrival's own: an entry cannot sit in one day's list
    -- having arrived on another.
    CHECK (date(arrived_at) IS day),
    -- Called needs arrived. `arrived_at NOT NULL` already holds it on every
    -- row; written out so the order of the four moments reads whole here.
    CHECK (called_at IS NULL OR arrived_at IS NOT NULL),
    -- Seen needs called: nobody is seen who was never called in.
    CHECK (seen_at IS NULL OR called_at IS NOT NULL),
    -- Left means left without being seen, so an entry is one or the other.
    CHECK (seen_at IS NULL OR left_at IS NULL)
) STRICT;

-- One live entry (neither seen nor gone) per patient per shop per day. The
-- service refuses a second one first, with a field name; this index is what
-- holds when two desks add the same patient in the same instant. Seen or
-- left, the entry stops counting, so a patient seen in the morning can come
-- back in the afternoon.
CREATE UNIQUE INDEX idx_queue_entries_live
    ON queue_entries (shop_id, patient_id, day)
    WHERE seen_at IS NULL AND left_at IS NULL;
-- The day's list in arrival order, which is the only way the service reads
-- the table.
CREATE INDEX idx_queue_entries_shop_day ON queue_entries (shop_id, day, arrived_at);
