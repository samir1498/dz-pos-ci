-- Absence blocks, the second of the book tools (C5b of
-- context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md):
-- a period the doctor is away (a morning, a congress, a holiday, a Hijri
-- feast confirmed the evening before, per loi 23-10, which the desk closes by
-- hand). The book refuses a booking that overlaps one; the appointments a
-- new block lands on stay booked until the desk moves or cancels each one.
--
-- In the one shared migration list, like the other clinic tables. Additive,
-- so the revert is a DROP.

CREATE TABLE absence_blocks (
    -- A UUID v7 as its 36-character text, made by the clinic service, for
    -- the reason `patients.id` gives.
    id          TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    -- The kernel's tenant. RESTRICT, with every other table.
    shop_id     INTEGER NOT NULL REFERENCES shops(id) ON DELETE RESTRICT,
    -- The shop clock's local start and end, `YYYY-MM-DD HH:MM:SS` on a whole
    -- minute, the book's own stamp format and CHECKs. Half-open: an
    -- appointment may end on `starts_at` or start on `ends_at`.
    starts_at   TEXT NOT NULL CHECK (datetime(starts_at) IS starts_at),
    ends_at     TEXT NOT NULL CHECK (datetime(ends_at) IS ends_at),
    -- A short word for the desk ("congrès"), optional. Never a reason about a
    -- patient: every role that reads the book reads this.
    label       TEXT CHECK (label IS NULL OR (length(trim(label)) > 0 AND length(label) <= 60)),
    -- The shop's clock, no DEFAULT, for the reason `patients` gives. No
    -- `updated_at`: a block is made and removed, never edited.
    created_at  TEXT NOT NULL,
    CHECK (strftime('%S', starts_at) = '00' AND strftime('%S', ends_at) = '00'),
    CHECK (ends_at > starts_at)
) STRICT;

-- The book's overlap read and the list of blocks to come both go by shop and
-- start.
CREATE INDEX idx_absence_blocks_shop_start ON absence_blocks (shop_id, starts_at);
