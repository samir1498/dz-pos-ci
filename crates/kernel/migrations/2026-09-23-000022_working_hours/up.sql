-- The cabinet's working hours, the first of the book tools (C5b of
-- context/plans/20260923-the-first-clinic-module-patients-queue-appointments.md).
-- One row is one open range of one weekday: a lunch break is two rows for
-- that day, and a day with no row is closed. A shop with no row at all has
-- never set its hours, and the book refuses nothing on their account; the
-- service never writes a week with no open range, so "no row" cannot mean
-- "closed all week".
--
-- In the one shared migration list, like the other clinic tables. Additive,
-- so the revert is a DROP.

CREATE TABLE working_hours (
    -- A UUID v7 as its 36-character text, made by the clinic service, for
    -- the reason `patients.id` gives.
    id             TEXT PRIMARY KEY NOT NULL CHECK (length(id) = 36),
    -- The kernel's tenant. RESTRICT, with every other table.
    shop_id        INTEGER NOT NULL REFERENCES shops(id) ON DELETE RESTRICT,
    -- The Algerian week, Sunday first: 0 is Sunday, 5 Friday, 6 Saturday
    -- (Samir, 2026-09-23). The number chrono's `num_days_from_sunday` gives.
    weekday        INTEGER NOT NULL CHECK (weekday BETWEEN 0 AND 6),
    -- Minutes from midnight, so a range that runs to midnight closes at
    -- 1440 rather than at a `24:00:00` no time function reads. The range is
    -- half-open: a visit may end on `closes_minute`, not start on it.
    opens_minute   INTEGER NOT NULL CHECK (opens_minute >= 0),
    closes_minute  INTEGER NOT NULL CHECK (closes_minute <= 1440),
    -- The shop's clock, no DEFAULT, for the reason `patients` gives. No
    -- `updated_at`: a new week replaces every row of the shop.
    created_at     TEXT NOT NULL,
    CHECK (opens_minute < closes_minute)
) STRICT;

-- One range per start per weekday, and the order the service reads a day in.
-- That two ranges of a day do not overlap is the service's check.
CREATE UNIQUE INDEX idx_working_hours_shop_day
    ON working_hours (shop_id, weekday, opens_minute);
